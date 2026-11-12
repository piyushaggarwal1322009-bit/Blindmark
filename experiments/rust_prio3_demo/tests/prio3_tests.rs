//! Integration & Adversarial Unit Tests for Prio3Histogram (12 Buckets)

use prio::codec::{Encode, ParameterizedDecode};
use prio::vdaf::{Aggregator, Client, VerifyTransition};
use rand::Rng;
use rust_prio3_demo::{create_stipend_histogram_vdaf, run_histogram_simulation, NUM_BUCKETS};
use std::io::Cursor;

#[test]
fn test_single_valid_bucket() {
    let dataset = vec![5];
    let hist = run_histogram_simulation(&dataset).expect("Single valid submission should succeed");
    assert_eq!(hist.len(), NUM_BUCKETS);
    assert_eq!(hist[5], 1);
    for i in 0..NUM_BUCKETS {
        if i != 5 {
            assert_eq!(hist[i], 0);
        }
    }
}

#[test]
fn test_multiple_valid_buckets() {
    let dataset = vec![0, 11, 0, 4, 7, 11, 11];
    let hist = run_histogram_simulation(&dataset).expect("Multiple valid submissions should succeed");
    assert_eq!(hist[0], 2);
    assert_eq!(hist[4], 1);
    assert_eq!(hist[7], 1);
    assert_eq!(hist[11], 3);
}

#[test]
fn test_boundary_buckets() {
    // Bucket 0 (lowest boundary) and Bucket 11 (highest boundary)
    let dataset = vec![0, 11];
    let hist = run_histogram_simulation(&dataset).expect("Boundary buckets should succeed");
    assert_eq!(hist[0], 1);
    assert_eq!(hist[11], 1);
}

#[test]
fn test_out_of_range_bucket_rejected() {
    // Bucket 12 is out of range for a 12-bucket histogram (0..11)
    let dataset = vec![12];
    let res = run_histogram_simulation(&dataset);
    assert!(res.is_err(), "Bucket 12 must be rejected as out of range");
}

#[test]
fn test_tampered_input_share_rejection() {
    let vdaf = create_stipend_histogram_vdaf().expect("VDAF creation failed");
    let verify_key: [u8; 32] = rand::thread_rng().gen();
    let nonce: [u8; 16] = rand::thread_rng().gen();
    let bucket = 3;

    let (public_share, input_shares) = vdaf.shard(&[], &bucket, &nonce).expect("Sharding failed");

    let (state_0, msg_share_0) = vdaf
        .verify_init(&verify_key, &[], 0, &(), &nonce, &public_share, &input_shares[0])
        .expect("Init 0 should succeed");

    let (_state_1, msg_share_1) = vdaf
        .verify_init(&verify_key, &[], 1, &(), &nonce, &public_share, &input_shares[1])
        .expect("Init 1 should succeed");

    // Tamper with prep message share 1 (simulating malicious wire modification)
    let mut msg_1_bytes = Vec::new();
    msg_share_1.encode(&mut msg_1_bytes).expect("Encoding succeeds");
    // Flip a byte within the verifiers payload (each Field128 is 16 bytes)
    if !msg_1_bytes.is_empty() {
        msg_1_bytes[0] ^= 0x01;
    }

    let corrupted_msg_1 =
        prio::vdaf::prio3::Prio3VerifierShare::decode_with_param(&state_0, &mut Cursor::new(&msg_1_bytes))
            .expect("Decoding succeeds");

    let prep_msg_res = vdaf.verifier_shares_to_message(&[], &(), vec![msg_share_0.clone(), corrupted_msg_1]);
    assert!(prep_msg_res.is_err(), "Proof verifier check must fail on tampered verifier share");

    // With untampered shares, combination and verify_next succeed
    let prep_msg = vdaf
        .verifier_shares_to_message(&[], &(), vec![msg_share_0, msg_share_1])
        .expect("Combination succeeds for valid shares");

    let out_0 = vdaf.verify_next(&[], state_0, prep_msg);
    assert!(matches!(out_0, Ok(VerifyTransition::Finish(_))), "Valid aggregator reaches finish transition");
}
