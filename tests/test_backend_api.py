import os
import pytest
from datetime import datetime, timezone, timedelta
from concurrent.futures import ThreadPoolExecutor
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from app.database import Base, get_db
from app.main import app
from app.models import Campaign, CampaignToken, Community
from app.services.token_service import hash_token

# Use a test SQLite database
TEST_DB_URL = "sqlite:///./test_blindmark.db"
test_engine = create_engine(TEST_DB_URL, connect_args={"check_same_thread": False})
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=test_engine)

def override_get_db():
    db = TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()

app.dependency_overrides[get_db] = override_get_db

@pytest.fixture(scope="session", autouse=True)
def setup_test_database():
    Base.metadata.drop_all(bind=test_engine)
    Base.metadata.create_all(bind=test_engine)
    yield
    Base.metadata.drop_all(bind=test_engine)
    if os.path.exists("./test_blindmark.db"):
        try:
            os.remove("./test_blindmark.db")
        except Exception:
            pass

@pytest.fixture
def client():
    return TestClient(app)

# 1. Community Creation Test
def test_community_creation(client):
    response = client.post("/api/communities", json={
        "name": "Rust Engineers Guild",
        "description": "A community of systems developers."
    })
    assert response.status_code == 201
    data = response.json()
    assert data["name"] == "Rust Engineers Guild"
    assert "id" in data

    # Test duplicate name rejection
    dup_res = client.post("/api/communities", json={
        "name": "Rust Engineers Guild"
    })
    assert dup_res.status_code == 400

# 2. Campaign Creation Test
def test_campaign_creation(client):
    # Setup community
    c_res = client.post("/api/communities", json={"name": "Frontend Guild"})
    community_id = c_res.json()["id"]

    # Valid campaign creation
    buckets = [f"Bucket_{i}" for i in range(20)]
    res = client.post("/api/campaigns", json={
        "community_id": community_id,
        "title": "2026 Salary Benchmark",
        "category": "Salary",
        "bucket_configuration": buckets,
        "minimum_participants": 5
    })
    assert res.status_code == 201
    data = res.json()
    assert data["title"] == "2026 Salary Benchmark"
    assert len(data["bucket_configuration"]) == 20
    assert data["minimum_participants"] == 5

# 3. Invalid Campaign Configuration Test
def test_invalid_campaign_configuration(client):
    c_res = client.post("/api/communities", json={"name": "Data Science Guild"})
    community_id = c_res.json()["id"]

    # Less than 2 buckets (schema validation rejects min_length=2 on bucket_configuration)
    res = client.post("/api/campaigns", json={
        "community_id": community_id,
        "title": "Invalid Campaign",
        "category": "Salary",
        "bucket_configuration": ["Single Bucket"],
        "minimum_participants": 5
    })
    assert res.status_code in [400, 422]

    # End date before start date
    now = datetime.now(timezone.utc)
    res_date = client.post("/api/campaigns", json={
        "community_id": community_id,
        "title": "Invalid Date Campaign",
        "category": "Salary",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 5,
        "start_at": now.isoformat(),
        "end_at": (now - timedelta(days=1)).isoformat()
    })
    assert res_date.status_code == 400
    assert "strictly after the start date" in res_date.json()["detail"]

# 4. Token Issuance & Successful Redemption
def test_token_issuance_and_redemption(client):
    # Create community & campaign
    c_res = client.post("/api/communities", json={"name": "AI Engineers Guild"})
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "LLM Salary Benchmark",
        "category": "Compensation",
        "bucket_configuration": ["Tier1", "Tier2"],
        "minimum_participants": 3
    })
    campaign_id = camp_res.json()["id"]

    # Issue token
    issue_res = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={
        "participant_ref": "user_hash_123"
    })
    assert issue_res.status_code == 200
    token = issue_res.json()["token"]
    assert len(token) > 20

    # Participant cannot claim another token
    dup_issue = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={
        "participant_ref": "user_hash_123"
    })
    assert dup_issue.status_code == 400

    # Successful redemption
    redeem_res = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={
        "token": token
    })
    assert redeem_res.status_code == 200
    assert redeem_res.json()["ok"] is True

# 5. Duplicate Token Redemption Test
def test_duplicate_token_redemption(client):
    # Setup
    c_res = client.post("/api/communities", json={"name": "Security Guild"})
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "AppSec Benchmark",
        "category": "Security",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 3
    })
    campaign_id = camp_res.json()["id"]

    issue_res = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={
        "participant_ref": "sec_user_1"
    })
    token = issue_res.json()["token"]

    # First redemption succeeds
    r1 = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": token})
    assert r1.status_code == 200

    # Second redemption must fail
    r2 = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": token})
    assert r2.status_code == 400
    assert "already been redeemed" in r2.json()["detail"]

# 6. Expired Token Test
def test_expired_token(client):
    db = TestingSessionLocal()
    c_res = client.post("/api/communities", json={"name": "DevOps Guild"})
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "SRE Benchmark",
        "category": "DevOps",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 3
    })
    campaign_id = camp_res.json()["id"]

    raw_token = "expired_secret_token_1234567890"
    nullifier = hash_token(raw_token)
    # Insert token expired 2 hours ago
    token_record = CampaignToken(
        campaign_id=campaign_id,
        nullifier_hash=nullifier,
        expires_at=datetime.now(timezone.utc) - timedelta(hours=2),
        is_redeemed=False,
        is_revoked=False
    )
    db.add(token_record)
    db.commit()
    db.close()

    res = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": raw_token})
    assert res.status_code == 400
    assert "expired" in res.json()["detail"]

# 7. Revoked Token Test
def test_revoked_token(client):
    db = TestingSessionLocal()
    c_res = client.post("/api/communities", json={"name": "Cloud Guild"})
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "Cloud Architect Benchmark",
        "category": "Cloud",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 3
    })
    campaign_id = camp_res.json()["id"]

    raw_token = "revoked_secret_token_1234567890"
    nullifier = hash_token(raw_token)
    token_record = CampaignToken(
        campaign_id=campaign_id,
        nullifier_hash=nullifier,
        expires_at=datetime.now(timezone.utc) + timedelta(hours=2),
        is_redeemed=False,
        is_revoked=True  # Revoked
    )
    db.add(token_record)
    db.commit()
    db.close()

    res = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": raw_token})
    assert res.status_code == 400
    assert "revoked" in res.json()["detail"]

# 8. Cross-Campaign Token Redemption Test
def test_cross_campaign_token(client):
    c_res = client.post("/api/communities", json={"name": "Cross Campaign Guild"})
    cid = c_res.json()["id"]

    # Campaign 1
    camp1 = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Camp 1", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    }).json()["id"]

    # Campaign 2
    camp2 = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Camp 2", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    }).json()["id"]

    # Token issued for Campaign 1 (min 3 chars for participant_ref)
    tok1 = client.post(f"/api/campaigns/{camp1}/tokens/issue", json={"participant_ref": "user_1"}).json()["token"]

    # Attempt to redeem token on Campaign 2
    res = client.post(f"/api/campaigns/{camp2}/tokens/redeem", json={"token": tok1})
    assert res.status_code == 400
    assert "different campaign" in res.json()["detail"]

# 9. Concurrent Token Redemption Test
def test_concurrent_token_redemption(client):
    c_res = client.post("/api/communities", json={"name": "Concurrency Guild"})
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Concurrency Benchmark", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    })
    campaign_id = camp_res.json()["id"]

    issue_res = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={"participant_ref": "conc_u"})
    token = issue_res.json()["token"]

    def attempt_redeem():
        c = TestClient(app)
        return c.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": token})

    # Launch 5 concurrent requests attempting to redeem the exact same token
    with ThreadPoolExecutor(max_workers=5) as executor:
        futures = [executor.submit(attempt_redeem) for _ in range(5)]
        responses = [f.result() for f in futures]

    successes = [r for r in responses if r.status_code == 200]
    failures = [r for r in responses if r.status_code == 400]

    # Exactly 1 request must succeed; all others must be rejected
    assert len(successes) == 1
    assert len(failures) == 4

# 10. Minimum Participant Threshold & Unreleased Campaign Results Test
def test_minimum_participant_threshold_and_unreleased_results(client):
    c_res = client.post("/api/communities", json={"name": "Threshold Guild"})
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "Strict Threshold Benchmark",
        "category": "Salary",
        "bucket_configuration": ["<$50k", "$50k-$100k", ">$100k"],
        "minimum_participants": 10
    })
    campaign_id = camp_res.json()["id"]

    # 1. Below threshold: 4 participants when 10 required
    eval_below = client.post(f"/api/campaigns/{campaign_id}/results/evaluate", json={
        "histogram": [1, 2, 1],
        "participant_count": 4
    })
    assert eval_below.status_code == 200
    below_data = eval_below.json()
    assert below_data["release_status"] == "withheld"
    assert below_data["histogram"] is None  # MUST NOT LEAK
    assert "Cohort threshold not reached" in below_data["message"]

    # Querying public results returns unreleased/withheld status
    res_query = client.get(f"/api/campaigns/{campaign_id}/results")
    assert res_query.status_code == 200
    assert res_query.json()["release_status"] == "withheld"
    assert res_query.json()["histogram"] is None

    # 2. Meeting or exceeding threshold: 12 participants
    eval_met = client.post(f"/api/campaigns/{campaign_id}/results/evaluate", json={
        "histogram": [3, 6, 3],
        "participant_count": 12
    })
    assert eval_met.status_code == 200
    met_data = eval_met.json()
    assert met_data["release_status"] == "released"
    assert met_data["histogram"] == [3, 6, 3]

    # Public results now show released histogram
    public_res = client.get(f"/api/campaigns/{campaign_id}/results")
    assert public_res.status_code == 200
    assert public_res.json()["release_status"] == "released"
    assert public_res.json()["histogram"] == [3, 6, 3]
