import secrets
import hashlib
from datetime import datetime, timezone, timedelta
from typing import Tuple
from sqlalchemy.orm import Session
from sqlalchemy import update
from app.models import CampaignToken, CampaignEligibility, Campaign

TOKEN_EXPIRY_HOURS = 24

def generate_secure_token() -> str:
    """Generates a high-entropy 256-bit URL-safe token."""
    return secrets.token_urlsafe(32)

def hash_token(raw_token: str) -> str:
    """Computes SHA-256 hash of the raw token to create the nullifier."""
    return hashlib.sha256(raw_token.encode("utf-8")).hexdigest()

def register_campaign_eligibility(db: Session, campaign_id: str, participant_ref: str) -> CampaignEligibility:
    """
    Registers an eligible participant for a campaign before token issuance.
    Strictly verifies campaign existence and prevents duplicate enrollment.
    """
    campaign = db.query(Campaign).filter(Campaign.id == campaign_id).first()
    if not campaign:
        raise ValueError(f"Campaign '{campaign_id}' not found.")

    existing = db.query(CampaignEligibility).filter(
        CampaignEligibility.campaign_id == campaign_id,
        CampaignEligibility.participant_ref == participant_ref
    ).first()
    if existing:
        raise ValueError(f"Participant '{participant_ref}' is already registered for this campaign.")

    eligibility = CampaignEligibility(
        campaign_id=campaign_id,
        participant_ref=participant_ref,
        token_issued=False,
        issued_at=None
    )
    db.add(eligibility)
    db.commit()
    db.refresh(eligibility)
    return eligibility

def issue_campaign_token(db: Session, campaign_id: str, participant_ref: str) -> Tuple[str, datetime]:
    """
    Issues a one-time single-use token for a PRE-REGISTERED eligible participant.
    
    Enforces:
    1. Campaign exists and is active.
    2. Participant reference MUST ALREADY BE REGISTERED as eligible. No auto-enrollment.
    3. Participant has not already claimed a token for this campaign.
    4. Only SHA-256 nullifier_hash is stored in campaign_tokens.
    """
    # 1. Check campaign
    campaign = db.query(Campaign).filter(Campaign.id == campaign_id).first()
    if not campaign:
        raise ValueError("Campaign not found")
    if campaign.status != "active":
        raise ValueError(f"Campaign is not active (status: {campaign.status})")

    # 2. Check pre-registered eligibility (Strict whitelist enforcement)
    eligibility = db.query(CampaignEligibility).filter(
        CampaignEligibility.campaign_id == campaign_id,
        CampaignEligibility.participant_ref == participant_ref
    ).first()

    if eligibility is None:
        raise ValueError(
            f"Participant '{participant_ref}' is not registered as eligible for campaign '{campaign_id}'."
        )

    if eligibility.token_issued:
        raise ValueError("Token has already been issued to this participant for this campaign")

    # Mark as issued
    eligibility.token_issued = True
    eligibility.issued_at = datetime.now(timezone.utc)

    # 3. Generate raw token and nullifier hash
    raw_token = generate_secure_token()
    nullifier = hash_token(raw_token)
    expires_at = datetime.now(timezone.utc) + timedelta(hours=TOKEN_EXPIRY_HOURS)

    token_record = CampaignToken(
        campaign_id=campaign_id,
        nullifier_hash=nullifier,
        expires_at=expires_at,
        is_redeemed=False,
        is_revoked=False
    )
    db.add(token_record)
    db.commit()

    return raw_token, expires_at

def redeem_campaign_token(db: Session, campaign_id: str, raw_token: str) -> datetime:
    """
    Validates and redeems a one-time token using an atomic database-level update.
    
    Works reliably across SQLite, PostgreSQL, and multi-worker ASGI processes:
    - Verifies token existence, campaign scope, revocation, and expiration.
    - Executes an atomic conditional UPDATE (is_redeemed = True WHERE id = :id AND is_redeemed = False).
    - If affected row count == 0, the token was already redeemed concurrently.
    """
    nullifier = hash_token(raw_token)
    now = datetime.now(timezone.utc)

    # Find the token record by nullifier hash
    token_record = db.query(CampaignToken).filter(
        CampaignToken.nullifier_hash == nullifier
    ).first()

    if not token_record:
        raise ValueError("Invalid token")

    if token_record.campaign_id != campaign_id:
        raise ValueError("Token was issued for a different campaign")

    if token_record.is_revoked:
        raise ValueError("Token has been revoked")

    exp = token_record.expires_at
    if exp.tzinfo is None:
        exp = exp.replace(tzinfo=timezone.utc)

    if now > exp:
        raise ValueError("Token has expired")

    if token_record.is_redeemed:
        raise ValueError("Token has already been redeemed")

    # Atomic database-level conditional update
    # Only updates if is_redeemed is still False at the instant of execution
    stmt = (
        update(CampaignToken)
        .where(
            CampaignToken.id == token_record.id,
            CampaignToken.is_redeemed == False
        )
        .values(
            is_redeemed=True,
            redeemed_at=now
        )
    )
    result = db.execute(stmt)
    db.commit()

    if result.rowcount == 0:
        # Another concurrent request redeemed this token first
        raise ValueError("Token has already been redeemed")

    return now
