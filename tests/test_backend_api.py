import os
import pytest
from datetime import datetime, timezone, timedelta
from concurrent.futures import ThreadPoolExecutor
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from app.database import Base, get_db
from app.main import app
from app.models import Campaign, CampaignToken, Community, CampaignEligibility
from app.services.token_service import hash_token
from app.auth import ADMIN_API_KEY, AGGREGATOR_API_KEY

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

ADMIN_HEADERS = {"X-API-Key": ADMIN_API_KEY}
AGGREGATOR_HEADERS = {"X-Service-Key": AGGREGATOR_API_KEY}

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

# ==============================================================================
# 1. Community Authorization & Creation Tests
# ==============================================================================

def test_community_creation_requires_admin(client):
    # Anonymous request must fail
    anon_res = client.post("/api/communities", json={
        "name": "Unauthorized Guild",
        "description": "Should be rejected."
    })
    assert anon_res.status_code == 401

    # Invalid admin key must fail
    bad_res = client.post("/api/communities", json={
        "name": "Bad Guild"
    }, headers={"X-API-Key": "wrong-key"})
    assert bad_res.status_code == 403

def test_community_creation_success_and_duplicate(client):
    response = client.post("/api/communities", json={
        "name": "Rust Engineers Guild",
        "description": "A community of systems developers."
    }, headers=ADMIN_HEADERS)
    assert response.status_code == 201
    data = response.json()
    assert data["name"] == "Rust Engineers Guild"
    assert "id" in data

    # Test duplicate name rejection
    dup_res = client.post("/api/communities", json={
        "name": "Rust Engineers Guild"
    }, headers=ADMIN_HEADERS)
    assert dup_res.status_code == 400

# ==============================================================================
# 2. Campaign Authorization & Creation Tests
# ==============================================================================

def test_campaign_creation_requires_admin(client):
    c_res = client.post("/api/communities", json={"name": "Auth Test Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]

    # Anonymous campaign creation rejected
    anon_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "Anon Campaign",
        "category": "Salary",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 5
    })
    assert anon_res.status_code == 401

def test_campaign_creation_success(client):
    c_res = client.post("/api/communities", json={"name": "Frontend Guild"}, headers=ADMIN_HEADERS)
    community_id = c_res.json()["id"]

    buckets = [f"Bucket_{i}" for i in range(20)]
    res = client.post("/api/campaigns", json={
        "community_id": community_id,
        "title": "2026 Salary Benchmark",
        "category": "Salary",
        "bucket_configuration": buckets,
        "minimum_participants": 5
    }, headers=ADMIN_HEADERS)
    assert res.status_code == 201
    data = res.json()
    assert data["title"] == "2026 Salary Benchmark"
    assert len(data["bucket_configuration"]) == 20
    assert data["minimum_participants"] == 5

def test_invalid_campaign_configuration(client):
    c_res = client.post("/api/communities", json={"name": "Data Science Guild"}, headers=ADMIN_HEADERS)
    community_id = c_res.json()["id"]

    # Less than 2 buckets rejected
    res = client.post("/api/campaigns", json={
        "community_id": community_id,
        "title": "Invalid Campaign",
        "category": "Salary",
        "bucket_configuration": ["Single Bucket"],
        "minimum_participants": 5
    }, headers=ADMIN_HEADERS)
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
    }, headers=ADMIN_HEADERS)
    assert res_date.status_code == 400
    assert "strictly after the start date" in res_date.json()["detail"]

# ==============================================================================
# 3. Eligibility Whitelist & Token Issuance Tests
# ==============================================================================

def test_unregistered_participant_token_rejection(client):
    c_res = client.post("/api/communities", json={"name": "Eligibility Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "Strict Eligibility Camp",
        "category": "Compensation",
        "bucket_configuration": ["Tier1", "Tier2"],
        "minimum_participants": 3
    }, headers=ADMIN_HEADERS)
    campaign_id = camp_res.json()["id"]

    # Unregistered participant attempt MUST be rejected
    res = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={
        "participant_ref": "unregistered_user_999"
    })
    assert res.status_code == 400
    assert "not registered as eligible" in res.json()["detail"]

def test_eligibility_registration_and_token_issuance(client):
    c_res = client.post("/api/communities", json={"name": "AI Engineers Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "LLM Salary Benchmark",
        "category": "Compensation",
        "bucket_configuration": ["Tier1", "Tier2"],
        "minimum_participants": 3
    }, headers=ADMIN_HEADERS)
    campaign_id = camp_res.json()["id"]

    # 1. Admin registers eligible participant
    reg_res = client.post(f"/api/campaigns/{campaign_id}/eligibility", json={
        "participant_ref": "user_hash_123"
    }, headers=ADMIN_HEADERS)
    assert reg_res.status_code == 201

    # Duplicate registration is rejected
    dup_reg = client.post(f"/api/campaigns/{campaign_id}/eligibility", json={
        "participant_ref": "user_hash_123"
    }, headers=ADMIN_HEADERS)
    assert dup_reg.status_code == 400

    # 2. Registered participant requests token
    issue_res = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={
        "participant_ref": "user_hash_123"
    })
    assert issue_res.status_code == 200
    token = issue_res.json()["token"]
    assert len(token) > 20

    # 3. Participant cannot claim another token
    dup_issue = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={
        "participant_ref": "user_hash_123"
    })
    assert dup_issue.status_code == 400
    assert "already been issued" in dup_issue.json()["detail"]

    # 4. Successful redemption
    redeem_res = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={
        "token": token
    })
    assert redeem_res.status_code == 200
    assert redeem_res.json()["ok"] is True

def test_cross_campaign_eligibility_rejection(client):
    c_res = client.post("/api/communities", json={"name": "Cross Camp Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]

    camp1 = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Camp 1", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    }, headers=ADMIN_HEADERS).json()["id"]

    camp2 = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Camp 2", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    }, headers=ADMIN_HEADERS).json()["id"]

    # Register participant only in Camp 1
    client.post(f"/api/campaigns/{camp1}/eligibility", json={"participant_ref": "alice_uid"}, headers=ADMIN_HEADERS)

    # Camp 2 cannot issue token for alice
    res = client.post(f"/api/campaigns/{camp2}/tokens/issue", json={"participant_ref": "alice_uid"})
    assert res.status_code == 400
    assert "not registered as eligible" in res.json()["detail"]

# ==============================================================================
# 4. Token Redemption & Concurrency Tests
# ==============================================================================

def test_duplicate_token_redemption(client):
    c_res = client.post("/api/communities", json={"name": "Security Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "AppSec Benchmark",
        "category": "Security",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 3
    }, headers=ADMIN_HEADERS)
    campaign_id = camp_res.json()["id"]

    client.post(f"/api/campaigns/{campaign_id}/eligibility", json={"participant_ref": "sec_user_1"}, headers=ADMIN_HEADERS)
    issue_res = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={"participant_ref": "sec_user_1"})
    token = issue_res.json()["token"]

    r1 = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": token})
    assert r1.status_code == 200

    r2 = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": token})
    assert r2.status_code == 400
    assert "already been redeemed" in r2.json()["detail"]

def test_expired_token(client):
    db = TestingSessionLocal()
    c_res = client.post("/api/communities", json={"name": "DevOps Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "SRE Benchmark",
        "category": "DevOps",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 3
    }, headers=ADMIN_HEADERS)
    campaign_id = camp_res.json()["id"]

    raw_token = "expired_secret_token_1234567890"
    nullifier = hash_token(raw_token)
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

def test_revoked_token(client):
    db = TestingSessionLocal()
    c_res = client.post("/api/communities", json={"name": "Cloud Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "Cloud Architect Benchmark",
        "category": "Cloud",
        "bucket_configuration": ["B1", "B2"],
        "minimum_participants": 3
    }, headers=ADMIN_HEADERS)
    campaign_id = camp_res.json()["id"]

    raw_token = "revoked_secret_token_1234567890"
    nullifier = hash_token(raw_token)
    token_record = CampaignToken(
        campaign_id=campaign_id,
        nullifier_hash=nullifier,
        expires_at=datetime.now(timezone.utc) + timedelta(hours=2),
        is_redeemed=False,
        is_revoked=True
    )
    db.add(token_record)
    db.commit()
    db.close()

    res = client.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": raw_token})
    assert res.status_code == 400
    assert "revoked" in res.json()["detail"]

def test_cross_campaign_token(client):
    c_res = client.post("/api/communities", json={"name": "Cross Campaign Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]

    camp1 = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Camp 1", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    }, headers=ADMIN_HEADERS).json()["id"]

    camp2 = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Camp 2", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    }, headers=ADMIN_HEADERS).json()["id"]

    client.post(f"/api/campaigns/{camp1}/eligibility", json={"participant_ref": "user_1"}, headers=ADMIN_HEADERS)
    tok1 = client.post(f"/api/campaigns/{camp1}/tokens/issue", json={"participant_ref": "user_1"}).json()["token"]

    res = client.post(f"/api/campaigns/{camp2}/tokens/redeem", json={"token": tok1})
    assert res.status_code == 400
    assert "different campaign" in res.json()["detail"]

def test_concurrent_token_redemption(client):
    c_res = client.post("/api/communities", json={"name": "Concurrency Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Concurrency Benchmark", "category": "Compensation", "bucket_configuration": ["B1", "B2"]
    }, headers=ADMIN_HEADERS)
    campaign_id = camp_res.json()["id"]

    client.post(f"/api/campaigns/{campaign_id}/eligibility", json={"participant_ref": "conc_u"}, headers=ADMIN_HEADERS)
    issue_res = client.post(f"/api/campaigns/{campaign_id}/tokens/issue", json={"participant_ref": "conc_u"})
    token = issue_res.json()["token"]

    def attempt_redeem():
        c = TestClient(app)
        return c.post(f"/api/campaigns/{campaign_id}/tokens/redeem", json={"token": token})

    with ThreadPoolExecutor(max_workers=5) as executor:
        futures = [executor.submit(attempt_redeem) for _ in range(5)]
        responses = [f.result() for f in futures]

    successes = [r for r in responses if r.status_code == 200]
    failures = [r for r in responses if r.status_code == 400]

    assert len(successes) == 1
    assert len(failures) == 4

# ==============================================================================
# 5. Result Evaluation Authorization & Input Validation Tests
# ==============================================================================

def test_result_evaluation_requires_aggregator_auth(client):
    c_res = client.post("/api/communities", json={"name": "Agg Auth Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Auth Camp", "category": "Salary", "bucket_configuration": ["B1", "B2"], "minimum_participants": 5
    }, headers=ADMIN_HEADERS).json()["id"]

    # Anonymous attempt rejected
    anon_res = client.post(f"/api/campaigns/{camp}/results/evaluate", json={
        "histogram": [3, 4], "participant_count": 7
    })
    assert anon_res.status_code == 401

    # Bad service key rejected
    bad_res = client.post(f"/api/campaigns/{camp}/results/evaluate", json={
        "histogram": [3, 4], "participant_count": 7
    }, headers={"X-Service-Key": "invalid-key"})
    assert bad_res.status_code == 403

def test_result_evaluation_histogram_validation(client):
    c_res = client.post("/api/communities", json={"name": "Validation Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp = client.post("/api/campaigns", json={
        "community_id": cid, "title": "Val Camp", "category": "Salary", "bucket_configuration": ["B1", "B2", "B3"], "minimum_participants": 5
    }, headers=ADMIN_HEADERS).json()["id"]

    # Negative count rejected
    neg_res = client.post(f"/api/campaigns/{camp}/results/evaluate", json={
        "histogram": [2, -1, 4], "participant_count": 5
    }, headers=AGGREGATOR_HEADERS)
    assert neg_res.status_code in [400, 422]

    # Dimension mismatch rejected (2 buckets instead of 3)
    dim_res = client.post(f"/api/campaigns/{camp}/results/evaluate", json={
        "histogram": [2, 3], "participant_count": 5
    }, headers=AGGREGATOR_HEADERS)
    assert dim_res.status_code in [400, 422]

    # Unexpected extra fields rejected by schema extra='forbid'
    extra_res = client.post(f"/api/campaigns/{camp}/results/evaluate", json={
        "histogram": [2, 3, 2], "participant_count": 7, "malicious_field": "injected"
    }, headers=AGGREGATOR_HEADERS)
    assert extra_res.status_code == 422

def test_minimum_participant_threshold_and_unreleased_results(client):
    c_res = client.post("/api/communities", json={"name": "Threshold Guild"}, headers=ADMIN_HEADERS)
    cid = c_res.json()["id"]
    camp_res = client.post("/api/campaigns", json={
        "community_id": cid,
        "title": "Strict Threshold Benchmark",
        "category": "Salary",
        "bucket_configuration": ["<$50k", "$50k-$100k", ">$100k"],
        "minimum_participants": 10
    }, headers=ADMIN_HEADERS)
    campaign_id = camp_res.json()["id"]

    # 1. Below threshold: 4 participants when 10 required
    eval_below = client.post(f"/api/campaigns/{campaign_id}/results/evaluate", json={
        "histogram": [1, 2, 1],
        "participant_count": 4
    }, headers=AGGREGATOR_HEADERS)
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
    }, headers=AGGREGATOR_HEADERS)
    assert eval_met.status_code == 200
    met_data = eval_met.json()
    assert met_data["release_status"] == "released"
    assert met_data["histogram"] == [3, 6, 3]

    # Public results now show released histogram
    public_res = client.get(f"/api/campaigns/{campaign_id}/results")
    assert public_res.status_code == 200
    assert public_res.json()["release_status"] == "released"
    assert public_res.json()["histogram"] == [3, 6, 3]
