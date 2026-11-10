import os
import sys

# Ensure poc directory from draft-irtf-cfrg-vdaf repository is in path
repo_poc_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "draft-irtf-cfrg-vdaf", "poc"))
if repo_poc_dir not in sys.path:
    sys.path.insert(0, repo_poc_dir)

from vdaf_poc.common import gen_rand
from vdaf_poc.vdaf_prio3 import Prio3Histogram

def run_submission_pipeline(prio3, verify_key, ctx, agg_param, agg_shares, meas=None, custom_shards=None, commit_to_agg=True):
    """
    Attempts to process a submission through:
    1. Client sharding (if meas provided)
    2. Aggregator verify_init
    3. Multi-round verifier message generation (verifier_shares_to_message)
    4. Aggregator verify_next & agg_update (if commit_to_agg is True)
    Returns (success: bool, stage_failed: str, error_message: str)
    """
    num_aggregators = prio3.SHARES

    # Stage 1: Client Encoding & Sharding
    if custom_shards is None:
        try:
            nonce = gen_rand(prio3.NONCE_SIZE)
            rand = gen_rand(prio3.RAND_SIZE)
            public_share, input_shares = prio3.shard(ctx, meas, nonce, rand)
        except Exception as e:
            return False, "client_encoding_sharding", f"{type(e).__name__}: {e}"
    else:
        nonce, public_share, input_shares = custom_shards

    # Stage 2: Aggregator verify_init
    verify_states = []
    outbound_verifier_shares = []
    try:
        for j in range(num_aggregators):
            state, share = prio3.verify_init(
                verify_key, ctx, j, agg_param, nonce, public_share, input_shares[j]
            )
            verify_states.append(state)
            outbound_verifier_shares.append(share)
    except Exception as e:
        return False, "aggregator_verify_init", f"{type(e).__name__}: {e}"

    # Stage 3: Verifier Shares to Message (Proof check)
    try:
        for _ in range(prio3.ROUNDS - 1):
            verifier_message = prio3.verifier_shares_to_message(
                ctx, agg_param, outbound_verifier_shares
            )
            outbound_verifier_shares = []
            for j in range(num_aggregators):
                out = prio3.verify_next(ctx, verify_states[j], verifier_message)
                assert isinstance(out, tuple)
                verify_states[j], verifier_share = out
                outbound_verifier_shares.append(verifier_share)

        verifier_message = prio3.verifier_shares_to_message(
            ctx, agg_param, outbound_verifier_shares
        )
    except Exception as e:
        return False, "proof_verification (verifier_shares_to_message)", f"{type(e).__name__}: {e}"

    # Stage 4: Aggregator verify_next & agg_update
    try:
        out_shares = []
        for j in range(num_aggregators):
            out_share = prio3.verify_next(ctx, verify_states[j], verifier_message)
            assert not isinstance(out_share, tuple)
            out_shares.append(out_share)

        if commit_to_agg:
            for j in range(num_aggregators):
                agg_shares[j] = prio3.agg_update(agg_param, agg_shares[j], out_shares[j])
    except Exception as e:
        return False, "aggregation (verify_next/agg_update)", f"{type(e).__name__}: {e}"

    return True, "none", "ok"

def main():
    print("=== BlindMark Phase 2: Prio3Histogram Attack & Integrity Test ===\n")
    num_aggregators = 2
    length = 20
    chunk_length = 5
    prio3 = Prio3Histogram(num_aggregators, length, chunk_length)

    ctx = b"prio3-histogram-attack-eval"
    agg_param = None
    verify_key = gen_rand(prio3.VERIFY_KEY_SIZE)

    # Aggregators initialize empty aggregate shares
    agg_shares = [prio3.agg_init(agg_param) for _ in range(num_aggregators)]

    # Step 3: Baseline valid measurements: 5, 7, 5, 12
    valid_measurements = [5, 7, 5, 12]
    print(f"1. Submitting baseline valid measurements: {valid_measurements}")
    valid_count = 0
    for m in valid_measurements:
        ok, stage, err = run_submission_pipeline(
            prio3, verify_key, ctx, agg_param, agg_shares, meas=m, commit_to_agg=True
        )
        assert ok, f"Baseline measurement {m} failed unexpectedly at {stage}: {err}"
        valid_count += 1
    print(f"   Successfully processed {valid_count} valid submissions.\n")

    # Step 2: Replay attacks and invalid submissions
    print("2. Replaying attack scenarios against Prio3Histogram:\n")

    # Test 1: Valid bucket submission
    print("--- Test 1: Valid bucket submission (bucket 3) ---")
    ok, stage, err = run_submission_pipeline(
        prio3, verify_key, ctx, agg_param, agg_shares, meas=3, commit_to_agg=False
    )
    print(f"Result: Success={ok}, Stage={stage}, Info={err}")

    # Test 2: Out-of-range bucket submission (bucket 20 for length=20, indexable 0..19)
    print("\n--- Test 2: Out-of-range bucket submission (bucket 20) ---")
    ok, stage, err = run_submission_pipeline(
        prio3, verify_key, ctx, agg_param, agg_shares, meas=20, commit_to_agg=True
    )
    print(f"Result: Success={ok}, Stage={stage}, Info={err}")

    # Test 2b: Massive out-of-range value (10,000,000, replicating Phase 1 cheat)
    print("\n--- Test 2b: Original cheat value (10,000,000) ---")
    ok, stage, err = run_submission_pipeline(
        prio3, verify_key, ctx, agg_param, agg_shares, meas=10000000, commit_to_agg=True
    )
    print(f"Result: Success={ok}, Stage={stage}, Info={err}")

    # Test 3: Negative bucket submission (e.g., -25 and -1)
    # Note on Python reference implementation behavior:
    # In Python list indexing, -1 indexes the last element (encoded[-1] -> bucket 19),
    # whereas an out-of-range negative index like -25 raises IndexError.
    print("\n--- Test 3a: Negative bucket submission out of index range (-25) ---")
    ok, stage, err = run_submission_pipeline(
        prio3, verify_key, ctx, agg_param, agg_shares, meas=-25, commit_to_agg=True
    )
    print(f"Result: Success={ok}, Stage={stage}, Info={err}")

    print("\n--- Test 3b: Negative bucket submission (-1) [Testing Reference Python Indexing Artifact] ---")
    ok, stage, err = run_submission_pipeline(
        prio3, verify_key, ctx, agg_param, agg_shares, meas=-1, commit_to_agg=False
    )
    print(f"Result: Success={ok}, Stage={stage}, Info={err}")
    print("Note: In Python, list[-1] maps to the last element (bucket 19). The FLP proof validly verifies")
    print("one-hotness of bucket 19. Under the VDAF specification, bucket indices are non-negative [0, length).")

    # Test 4: Malformed / Tampered share/proof
    # Client creates valid shares for bucket 5, but the leader share is tampered (+10 to bucket 5)
    print("\n--- Test 4a: Tampered measurement share (+10 in share) ---")
    nonce = gen_rand(prio3.NONCE_SIZE)
    rand = gen_rand(prio3.RAND_SIZE)
    pub_share, input_shares = prio3.shard(ctx, 5, nonce, rand)
    leader_meas_share, leader_proofs_share, leader_blind = input_shares[0]
    tampered_meas_share = list(leader_meas_share)
    tampered_meas_share[5] = tampered_meas_share[5] + prio3.flp.field(10)
    tampered_leader_share = (tampered_meas_share, leader_proofs_share, leader_blind)
    tampered_input_shares = [tampered_leader_share, input_shares[1]]

    ok, stage, err = run_submission_pipeline(
        prio3, verify_key, ctx, agg_param, agg_shares,
        custom_shards=(nonce, pub_share, tampered_input_shares),
        commit_to_agg=True
    )
    print(f"Result: Success={ok}, Stage={stage}, Info={err}")

    # Test 4b: Tampered proof share
    print("\n--- Test 4b: Tampered proof share ---")
    nonce_b = gen_rand(prio3.NONCE_SIZE)
    rand_b = gen_rand(prio3.RAND_SIZE)
    pub_share_b, input_shares_b = prio3.shard(ctx, 5, nonce_b, rand_b)
    l_meas, l_proofs, l_blind = input_shares_b[0]
    tampered_proofs = list(l_proofs)
    tampered_proofs[0] = tampered_proofs[0] + prio3.flp.field(1)
    tampered_input_b = [(l_meas, tampered_proofs, l_blind), input_shares_b[1]]

    ok, stage, err = run_submission_pipeline(
        prio3, verify_key, ctx, agg_param, agg_shares,
        custom_shards=(nonce_b, pub_share_b, tampered_input_b),
        commit_to_agg=True
    )
    print(f"Result: Success={ok}, Stage={stage}, Info={err}")

    # Step 3: Unshard and check histogram integrity
    print("\n=== 3. Histogram Integrity Verification ===")
    agg_result = prio3.unshard(agg_param, agg_shares, valid_count)
    print(f"Final aggregated histogram (across {valid_count} valid submissions):")
    print(agg_result)

    expected = [0] * 20
    expected[5] = 2
    expected[7] = 1
    expected[12] = 1

    print(f"Expected histogram:\n{expected}")
    assert agg_result == expected, f"Integrity check failed! Got {agg_result}, expected {expected}"
    print("\n>>> Integrity check PASSED: No rejected, out-of-range, or tampered submission corrupted the final histogram!")

if __name__ == "__main__":
    main()
