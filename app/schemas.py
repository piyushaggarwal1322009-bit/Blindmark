from datetime import datetime
from typing import Optional, List, Dict, Any
from pydantic import BaseModel, Field, ConfigDict

# Community Schemas
class CommunityCreate(BaseModel):
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


# Token Schemas
class TokenIssueRequest(BaseModel):
    participant_ref: str = Field(..., min_length=3, max_length=128)

class TokenIssueResponse(BaseModel):
    token: str
    expires_at: datetime
    note: str = "Store this token securely. It cannot be recovered and will be invalidated upon single use."

class TokenRedeemRequest(BaseModel):
    token: str = Field(..., min_length=16)

class TokenRedeemResponse(BaseModel):
    ok: bool
    message: str
    redeemed_at: datetime


# Result Schemas
class ResultReleaseRequest(BaseModel):
    # Simulated aggregate shares or unshared histogram for testing/release
    histogram: List[int]
    participant_count: int

class ResultResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    campaign_id: str
    histogram: Optional[List[int]] = None
    participant_count: int
    release_status: str
    released_at: Optional[datetime] = None
    message: Optional[str] = None

