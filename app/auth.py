import os
from typing import Optional
from fastapi import Header, HTTPException, status, Depends
from pydantic import BaseModel

# For development / internal service authorization
# In production, keys/tokens should be loaded from secure secrets management
ADMIN_API_KEY = os.getenv("BLINDMARK_ADMIN_API_KEY", "dev-admin-secret-key-blindmark")
AGGREGATOR_API_KEY = os.getenv("BLINDMARK_AGGREGATOR_API_KEY", "dev-aggregator-secret-key-blindmark")

class AuthPrincipal(BaseModel):
    role: str
    identifier: str

def require_admin(x_api_key: Optional[str] = Header(None, alias="X-API-Key")) -> AuthPrincipal:
    """
    Enforces administrative authentication for sensitive management endpoints:
    - Community creation
    - Campaign creation
    - Participant eligibility enrollment
    """
    if not x_api_key:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Missing required authentication credential (X-API-Key header required)."
        )
    if x_api_key != ADMIN_API_KEY:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Forbidden: Invalid administrative credentials."
        )
    return AuthPrincipal(role="admin", identifier="admin-user")

def require_aggregator(x_service_key: Optional[str] = Header(None, alias="X-Service-Key")) -> AuthPrincipal:
    """
    Enforces trusted service-to-service authentication for MPC aggregators:
    - Result evaluation & publication
    """
    if not x_service_key:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Missing required service authentication credential (X-Service-Key header required)."
        )
    if x_service_key != AGGREGATOR_API_KEY:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Forbidden: Caller is not an authorized aggregation service."
        )
    return AuthPrincipal(role="aggregator", identifier="aggregator-service")
