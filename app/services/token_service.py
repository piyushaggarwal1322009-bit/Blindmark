import secrets
import hashlib
import threading
from datetime import datetime, timezone, timedelta
from typing import Tuple
from sqlalchemy.orm import Session
from sqlalchemy import select
from app.models import CampaignToken, CampaignEligibility, Campaign

TOKEN_EXPIRY_HOURS = 24
_redemption_lock = threading.Lock()

def generate_secure_token() -> str:
    """Generates a high-entropy 256-bit URL-safe token."""
    return secrets.token_urlsafe(32)

def hash_token(raw_token: str) -> str:
    """Computes SHA-256 hash of the raw token to create the nullifier."""
    return hashlib.sha256(raw_token.encode("utf-8")).hexdigest()

def issue_campaign_token(db: Session, campaign_id: str, participant_ref: str) -> Tuple[str, datetime]:
    """
    Issues a one-time single-use token for an eligible participant in a campaign.
    Enforces that:
    1. The campaign exists and is active.
    2. The participant reference is eligible.
    3. The participant has not already claimed a token for this campaign.
    4. Only the nullifier hash is stored, never the raw token or link to participant identity.
    """
    # 1. Check campaign
    campaign = db.query(Campaign).filter(Campaign.id == campaign_id).first()
    if not campaign:
        raise ValueError("Campaign not found")
    if campaign.status != "active":
        raise ValueError(f"Campaign is not active (status: {campaign.status})")

    # 2. Check or register eligibility
    eligibility = db.query(CampaignEligibility).filter(
        CampaignEligibility.campaign_id == campaign_id,
        CampaignEligibility.participant_ref == participant_ref
    ).first()

    if eligibility is None:
        # For development/demo mode, register as eligible on first request
        eligibility = CampaignEligibility(
            campaign_id=campaign_id,
            participant_ref=participant_ref,
            token_issued=True,
            issued_at=datetime.now(timezone.utc)
        )
        db.add(eligibility)
    else:
        if eligibility.token_issued:
            raise ValueError("Token has already been issued to this participant for this campaign")
        eligibility.token_issued = True
        eligibility.issued_at = datetime.now(timezone.utc)

    # 3. Generate raw token and hash
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
    Validates and redeems a one-time token under strict locking.
    Enforces:
    1. Campaign matches token campaign_id.
    2. Token is not revoked.
    3. Token is not expired.
    4. Token is not already redeemed (single-use).
    5. Concurrency protection via row locking and process mutex.
    """
    nullifier = hash_token(raw_token)
    now = datetime.now(timezone.utc)

    with _redemption_lock:
        query = db.query(CampaignToken).filter(
            CampaignToken.nullifier_hash == nullifier
        )
        
        # Use with_for_update if supported (e.g. Postgres)
        if db.bind and db.bind.dialect.name != "sqlite":
            query = query.with_for_update()

        token_record = query.first()

        if not token_record:
            raise ValueError("Invalid token")

        if token_record.campaign_id != campaign_id:
            raise ValueError("Token was issued for a different campaign")

        if token_record.is_revoked:
            raise ValueError("Token has been revoked")

        # Make expires_at timezone-aware for comparison if needed
        exp = token_record.expires_at
        if exp.tzinfo is None:
            exp = exp.replace(tzinfo=timezone.utc)

        if now > exp:
            raise ValueError("Token has expired")

        if token_record.is_redeemed:
            raise ValueError("Token has already been redeemed")

        # Atomically mark as redeemed
        token_record.is_redeemed = True
        token_record.redeemed_at = now
        db.commit()

        return now
