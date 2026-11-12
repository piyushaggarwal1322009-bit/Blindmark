from datetime import datetime
from typing import Optional, List, Dict, Any
from pydantic import BaseModel, Field, ConfigDict, field_validator

# Community Schemas
class CommunityCreate(BaseModel):
    model_config = ConfigDict(extra="forbid")
    name: str = Field(..., min_length=2, max_length=100)
    description: Optional[str] = None

class CommunityResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    name: str
    description: Optional[str] = None
    created_at: datetime


# Campaign Schemas
class CampaignCreate(BaseModel):
    model_config = ConfigDict(extra="forbid")
    community_id: str
    title: str = Field(..., min_length=3, max_length=200)
    description: Optional[str] = None
    category: str = Field(..., min_length=2, max_length=100)
    bucket_configuration: List[str] = Field(..., min_length=2, max_length=50)
    minimum_participants: int = Field(default=20, ge=3)
    start_at: Optional[datetime] = None
    end_at: Optional[datetime] = None

class CampaignResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    community_id: str
    title: str
    description: Optional[str] = None
    category: str
    bucket_configuration: List[str]
    minimum_participants: int
    status: str
    start_at: datetime
    end_at: Optional[datetime] = None
    created_at: datetime


# Participant Eligibility Schemas
class EligibilityRegisterRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    participant_ref: str = Field(..., min_length=3, max_length=128)

class EligibilityRegisterResponse(BaseModel):
    campaign_id: str
    participant_ref: str
    token_issued: bool
    registered: bool
    message: str


# Token Schemas
class TokenIssueRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    participant_ref: str = Field(..., min_length=3, max_length=128)

class TokenIssueResponse(BaseModel):
    token: str
    expires_at: datetime
    note: str = "Store this token securely. It cannot be recovered and will be invalidated upon single use."

class TokenRedeemRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    token: str = Field(..., min_length=16)

class TokenRedeemResponse(BaseModel):
    ok: bool
    message: str
    redeemed_at: datetime


# Result Schemas
class ResultReleaseRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    histogram: List[int]
    participant_count: int = Field(..., ge=1)

    @field_validator("histogram")
    @classmethod
    def validate_histogram_counts(cls, v: List[int]) -> List[int]:
        if not v:
            raise ValueError("Histogram cannot be empty.")
        for count in v:
            if isinstance(count, bool) or not isinstance(count, int):
                raise ValueError(f"Histogram bucket count must be an integer, got {type(count).__name__}")
            if count < 0:
                raise ValueError(f"Histogram count cannot be negative: got {count}")
        return v

class ResultResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    campaign_id: str
    histogram: Optional[List[int]] = None
    participant_count: int
    release_status: str
    released_at: Optional[datetime] = None
    message: Optional[str] = None
