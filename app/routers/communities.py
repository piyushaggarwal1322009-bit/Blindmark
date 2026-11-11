from typing import List
from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy.orm import Session
from app.database import get_db
from app.models import Community
from app.schemas import CommunityCreate, CommunityResponse

router = APIRouter(prefix="/api/communities", tags=["Communities"])

@router.post("", response_model=CommunityResponse, status_code=status.HTTP_201_CREATED)
def create_community(payload: CommunityCreate, db: Session = Depends(get_db)):
    existing = db.query(Community).filter(Community.name == payload.name).first()
    if existing:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Community with name '{payload.name}' already exists."
        )

    community = Community(
        name=payload.name,
        description=payload.description
    )
    db.add(community)
    db.commit()
    db.refresh(community)
    return community

@router.get("/{community_id}", response_model=CommunityResponse)
def get_community(community_id: str, db: Session = Depends(get_db)):
    community = db.query(Community).filter(Community.id == community_id).first()
    if not community:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Community '{community_id}' not found."
        )
    return community

