import os
import sys

# Add the poc directory from the draft-irtf-cfrg-vdaf repository next to BlindMark
repo_poc_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "draft-irtf-cfrg-vdaf", "poc"))
if repo_poc_dir not in sys.path:
    sys.path.insert(0, repo_poc_dir)

from vdaf_poc.common import gen_rand
from vdaf_poc.vdaf_prio3 import Prio3Histogram

def main():
    # 2 aggregators, 20 buckets, chunk_length = 5
    num_aggregators = 2
    length = 20
    chunk_length = 5
    prio3 = Prio3Histogram(num_aggregators, length, chunk_length)

    measurements = [5, 7, 5, 12]
    ctx = b"prio3-histogram-demo"
    agg_param = None
    verify_key = gen_rand(prio3.VERIFY_KEY_SIZE)

    # Aggregators initialize empty aggregate shares
    agg_shares = [prio3.agg_init(agg_param) for _ in range(num_aggregators)]

    # Process each client measurement
    for meas in measurements:
        # Sharding: Client creates a public share and input shares for each aggregator
        nonce = gen_rand(prio3.NONCE_SIZE)
        rand = gen_rand(prio3.RAND_SIZE)
        public_share, input_shares = prio3.shard(ctx, meas, nonce, rand)

        # Verification step 1: verify_init on each aggregator
        verify_states = []
        outbound_verifier_shares = []
        for j in range(num_aggregators):
            state, share = prio3.verify_init(
                verify_key, ctx, j, agg_param, nonce, public_share, input_shares[j]
            )
            verify_states.append(state)
            outbound_verifier_shares.append(share)

        # Multi-round verification loop (if ROUNDS > 1)
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

        # Final verifier message
        verifier_message = prio3.verifier_shares_to_message(
            ctx, agg_param, outbound_verifier_shares
        )

        # Aggregation: Each aggregator verifies and gets out_share, then updates agg_share
        for j in range(num_aggregators):
            out_share = prio3.verify_next(ctx, verify_states[j], verifier_message)
            assert not isinstance(out_share, tuple)
            agg_shares[j] = prio3.agg_update(agg_param, agg_shares[j], out_share)

    # Collector unshard
    agg_result = prio3.unshard(agg_param, agg_shares, len(measurements))

    print("Resulting histogram:")
    print(agg_result)
    for bucket_idx, count in enumerate(agg_result):
        if count != 0:
            print(f"bucket {bucket_idx} = {count}")

if __name__ == "__main__":
    main()

