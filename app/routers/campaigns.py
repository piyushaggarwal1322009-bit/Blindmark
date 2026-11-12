from typing import List, Optional
from datetime import datetime, timezone
from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy.orm import Session
from app.database import get_db
from app.models import Campaign, Community, AggregationResult, CampaignEligibility
from app.schemas import (
    CampaignCreate, CampaignResponse,
    EligibilityRegisterRequest, EligibilityRegisterResponse,
    TokenIssueRequest, TokenIssueResponse,
    TokenRedeemRequest, TokenRedeemResponse,
    ResultReleaseRequest, ResultResponse
)
from app.services.token_service import (
    issue_campaign_token, redeem_campaign_token, register_campaign_eligibility
)
from app.services.release_policy import DevelopmentReleasePolicy
from app.auth import require_admin, require_aggregator, AuthPrincipal
from core.validation import validate_bucket_index

router = APIRouter(prefix="/api/campaigns", tags=["Campaigns"])

# Campaign Creation (Protected: Admin Only)
@router.post("", response_model=CampaignResponse, status_code=status.HTTP_201_CREATED)
def create_campaign(
    payload: CampaignCreate,
    db: Session = Depends(get_db),
    admin: AuthPrincipal = Depends(require_admin)
):
    community = db.query(Community).filter(Community.id == payload.community_id).first()
    if not community:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Community '{payload.community_id}' not found."
        )

    # Validate bucket configuration
    if len(payload.bucket_configuration) < 2:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Campaign bucket configuration must define at least 2 discrete buckets."
        )

    # Date sanity checks
    start = payload.start_at or datetime.now(timezone.utc)
    if payload.end_at and payload.end_at <= start:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Campaign end date must be strictly after the start date."
        )

    campaign = Campaign(
        community_id=payload.community_id,
        title=payload.title,
        description=payload.description,
        category=payload.category,
        bucket_configuration=payload.bucket_configuration,
        minimum_participants=payload.minimum_participants,
        status="active",
        start_at=start,
        end_at=payload.end_at
    )
    db.add(campaign)
    db.commit()
    db.refresh(campaign)
    return campaign

# Campaign Listing (Public metadata)
@router.get("", response_model=List[CampaignResponse])
def list_campaigns(community_id: Optional[str] = None, db: Session = Depends(get_db)):
    query = db.query(Campaign)
    if community_id:
        query = query.filter(Campaign.community_id == community_id)
    return query.all()

# Campaign Retrieval (Public metadata)
@router.get("/{campaign_id}", response_model=CampaignResponse)
def get_campaign(campaign_id: str, db: Session = Depends(get_db)):
    campaign = db.query(Campaign).filter(Campaign.id == campaign_id).first()
    if not campaign:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Campaign '{campaign_id}' not found."
        )
    return campaign

# Participant Eligibility Registration (Protected: Admin Only)
@router.post("/{campaign_id}/eligibility", response_model=EligibilityRegisterResponse, status_code=status.HTTP_201_CREATED)
def register_eligibility(
    campaign_id: str,
    payload: EligibilityRegisterRequest,
    db: Session = Depends(get_db),
    admin: AuthPrincipal = Depends(require_admin)
):
    try:
        eligibility = register_campaign_eligibility(db, campaign_id, payload.participant_ref)
        return EligibilityRegisterResponse(
            campaign_id=campaign_id,
            participant_ref=eligibility.participant_ref,
            token_issued=eligibility.token_issued,
            registered=True,
            message="Participant reference successfully registered as eligible."
        )
    except ValueError as e:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(e))

# Token Issuance Endpoint (Available to registered participants)
@router.post("/{campaign_id}/tokens/issue", response_model=TokenIssueResponse)
def issue_token(campaign_id: str, payload: TokenIssueRequest, db: Session = Depends(get_db)):
    try:
        raw_token, expires_at = issue_campaign_token(db, campaign_id, payload.participant_ref)
        return TokenIssueResponse(token=raw_token, expires_at=expires_at)
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )

# Token Redemption Endpoint
@router.post("/{campaign_id}/tokens/redeem", response_model=TokenRedeemResponse)
def redeem_token(campaign_id: str, payload: TokenRedeemRequest, db: Session = Depends(get_db)):
    try:
        redeemed_at = redeem_campaign_token(db, campaign_id, payload.token)
        return TokenRedeemResponse(
            ok=True,
            message="Token successfully validated and consumed in nullifier set.",
            redeemed_at=redeemed_at
        )
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )

# Submit & Evaluate Results Endpoint (Protected: Trusted Aggregator Only)
@router.post("/{campaign_id}/results/evaluate", response_model=ResultResponse)
def evaluate_results(
    campaign_id: str,
    payload: ResultReleaseRequest,
    db: Session = Depends(get_db),
    aggregator: AuthPrincipal = Depends(require_aggregator)
):
    campaign = db.query(Campaign).filter(Campaign.id == campaign_id).first()
    if not campaign:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Campaign not found")

    # Strict input validation: histogram length and values
    expected_buckets = len(campaign.bucket_configuration)
    if len(payload.histogram) != expected_buckets:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Histogram dimension mismatch: campaign defines {expected_buckets} buckets, received {len(payload.histogram)}."
        )

    # Validate each bucket count using validation logic
    for i, count in enumerate(payload.histogram):
        if not isinstance(count, int) or isinstance(count, bool):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Bucket {i} count must be an integer, got {type(count).__name__}."
            )
        if count < 0:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Bucket {i} count cannot be negative: {count}."
            )

    status_str, sanitized_hist, msg = DevelopmentReleasePolicy.evaluate_release(
        campaign, payload.histogram, payload.participant_count
    )

    result_record = AggregationResult(
        campaign_id=campaign_id,
        histogram=sanitized_hist if sanitized_hist is not None else [],
        participant_count=payload.participant_count,
        release_status=status_str,
        released_at=datetime.now(timezone.utc) if status_str == "released" else None
    )
    db.add(result_record)
    db.commit()
    db.refresh(result_record)

    return ResultResponse(
        id=result_record.id,
        campaign_id=campaign_id,
        histogram=sanitized_hist,
        participant_count=payload.participant_count,
        release_status=status_str,
        released_at=result_record.released_at,
        message=msg
    )

# Retrieve Campaign Results Endpoint (Public, only returns released data)
@router.get("/{campaign_id}/results", response_model=ResultResponse)
def get_campaign_results(campaign_id: str, db: Session = Depends(get_db)):
    campaign = db.query(Campaign).filter(Campaign.id == campaign_id).first()
    if not campaign:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Campaign not found")

    result = db.query(AggregationResult).filter(
        AggregationResult.campaign_id == campaign_id,
        AggregationResult.release_status == "released"
    ).order_by(AggregationResult.created_at.desc()).first()

    if not result:
        return ResultResponse(
            id="none",
            campaign_id=campaign_id,
            histogram=None,
            participant_count=0,
            release_status="withheld",
            message="No released results available for this campaign. Cohort threshold or release schedule has not been met."
        )

    return ResultResponse(
        id=result.id,
        campaign_id=result.campaign_id,
        histogram=result.histogram,
        participant_count=result.participant_count,
        release_status=result.release_status,
        released_at=result.released_at,
        message="Verified aggregate benchmark release."
    )
