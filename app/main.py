from fastapi import FastAPI
from app.database import engine, Base
from app.routers import communities, campaigns

# Automatically ensure tables are created
Base.metadata.create_all(bind=engine)

app = FastAPI(
    title="Blindmark Backend API",
    description="Privacy-preserving benchmarking engine using VDAF additive secret sharing and zero-knowledge proofs.",
    version="0.1.0"
)

app.include_router(communities.router)
app.include_router(campaigns.router)

@app.get("/healthz")
def health_check():
    return {"status": "ok", "service": "blindmark-backend"}

