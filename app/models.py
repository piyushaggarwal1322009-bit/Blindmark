import uuid
from datetime import datetime, timezone
from sqlalchemy import (
    Column, String, Text, Integer, DateTime, ForeignKey, Boolean, JSON, UniqueConstraint, Index
)
from sqlalchemy.orm import relationship
from app.database import Base

def utc_now():
    return datetime.now(timezone.utc)

class Community(Base):
    __tablename__ = "communities"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    name = Column(String(100), nullable=False, unique=True, index=True)
    description = Column(Text, nullable=True)
    created_at = Column(DateTime(timezone=True), default=utc_now, nullable=False)

    campaigns = relationship("Campaign", back_populates="community", cascade="all, delete-orphan")


class Campaign(Base):
    __tablename__ = "campaigns"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    community_id = Column(String(36), ForeignKey("communities.id", ondelete="CASCADE"), nullable=False, index=True)
    title = Column(String(200), nullable=False)
    description = Column(Text, nullable=True)
    category = Column(String(100), nullable=False)
    bucket_configuration = Column(JSON, nullable=False)  # List of bucket definitions/labels
    minimum_participants = Column(Integer, default=20, nullable=False)
    status = Column(String(20), default="active", nullable=False)  # draft, active, closed
    start_at = Column(DateTime(timezone=True), default=utc_now, nullable=False)
    end_at = Column(DateTime(timezone=True), nullable=True)
    created_at = Column(DateTime(timezone=True), default=utc_now, nullable=False)

    community = relationship("Community", back_populates="campaigns")
    eligibilities = relationship("CampaignEligibility", back_populates="campaign", cascade="all, delete-orphan")
    tokens = relationship("CampaignToken", back_populates="campaign", cascade="all, delete-orphan")
    results = relationship("AggregationResult", back_populates="campaign", cascade="all, delete-orphan")


class CampaignEligibility(Base):
    __tablename__ = "campaign_eligibilities"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    campaign_id = Column(String(36), ForeignKey("campaigns.id", ondelete="CASCADE"), nullable=False, index=True)
    # Pseudonymized participant eligibility reference (e.g. SHA-256 hash of verified email or member UUID)
    participant_ref = Column(String(128), nullable=False, index=True)
    token_issued = Column(Boolean, default=False, nullable=False)
    issued_at = Column(DateTime(timezone=True), nullable=True)

    __table_args__ = (
        UniqueConstraint("campaign_id", "participant_ref", name="uq_campaign_participant"),
    )

    campaign = relationship("Campaign", back_populates="eligibilities")


class CampaignToken(Base):
    """
    Tracks single-use nullifiers for duplicate prevention.
    CRITICAL: Never stores the raw token or link to participant identity.
    Only the SHA-256 nullifier_hash is recorded upon token issuance or redemption.
    """
    __tablename__ = "campaign_tokens"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    campaign_id = Column(String(36), ForeignKey("campaigns.id", ondelete="CASCADE"), nullable=False, index=True)
    nullifier_hash = Column(String(64), nullable=False, unique=True, index=True)
    expires_at = Column(DateTime(timezone=True), nullable=False)
    is_revoked = Column(Boolean, default=False, nullable=False)
    is_redeemed = Column(Boolean, default=False, nullable=False)
    redeemed_at = Column(DateTime(timezone=True), nullable=True)
    created_at = Column(DateTime(timezone=True), default=utc_now, nullable=False)

    __table_args__ = (
        Index("ix_campaign_token_lookup", "campaign_id", "nullifier_hash"),
    )

    campaign = relationship("Campaign", back_populates="tokens")


class AggregationResult(Base):
    """
    Stores finalized aggregated histogram distributions.
    CRITICAL: Never stores individual measurements or unaggregated shares.
    """
    __tablename__ = "aggregation_results"

    id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    campaign_id = Column(String(36), ForeignKey("campaigns.id", ondelete="CASCADE"), nullable=False, index=True)
    histogram = Column(JSON, nullable=False)  # List of counts per bucket
    participant_count = Column(Integer, nullable=False)
    release_status = Column(String(20), default="withheld", nullable=False)  # withheld, released, aborted
    released_at = Column(DateTime(timezone=True), nullable=True)
    created_at = Column(DateTime(timezone=True), default=utc_now, nullable=False)

    campaign = relationship("Campaign", back_populates="results")

