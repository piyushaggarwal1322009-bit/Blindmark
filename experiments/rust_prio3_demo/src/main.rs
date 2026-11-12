//! Blindmark — Phase 6B: Isolated Prio3Histogram Prototype
//!
//! Demonstrates the complete two-aggregator Prio3Histogram workflow using
//! the `prio` crate (`0.18.1`), the exact VDAF engine powering Janus.
//!
//! Workflow:
//! 1. Instantiates `Prio3::new_histogram` with K=12 buckets and chunk_length=4.
//! 2. Encodes discrete internship stipend buckets (0..11) into 1-hot measurement shares.
//! 3. Generates zero-knowledge Fully Linear Proofs (FLPs) over 128-bit field (Field128).
//! 4. Simulates two independent aggregators (Leader and Helper) executing multi-round preparation.
//! 5. Verifies and accumulates valid output shares.
//! 6. Combines aggregate shares in the Collector via `vdaf.unshard`.

use prio::vdaf::prio3::{Prio3, Prio3Histogram};
use prio::vdaf::{Aggregatable, Aggregator, Client, Collector, VerifyTransition};
use rand::Rng;
use std::error::Error;

/// Number of discrete internship stipend buckets configured in Phase 6A.
pub const NUM_BUCKETS: usize = 12;

/// Number of aggregators in standard DAP non-colluding deployment.
pub const NUM_AGGREGATORS: u8 = 2;

/// Gadget chunk length for parallel sum proof circuits.
pub const CHUNK_LENGTH: usize = 4;

/// Constructs the canonical 12-bucket Prio3Histogram VDAF instance.
pub fn create_stipend_histogram_vdaf() -> Result<Prio3Histogram, Box<dyn Error>> {
    let vdaf = Prio3::new_histogram(NUM_AGGREGATORS, NUM_BUCKETS, CHUNK_LENGTH)?;
    Ok(vdaf)
}

/// Simulates the complete end-to-end Prio3Histogram lifecycle for a dataset of bucket selections.
///
/// Returns the reconstructed aggregate histogram vector of length 12.
pub fn run_histogram_simulation(
    measurements: &[usize],
) -> Result<Vec<u128>, Box<dyn Error>> {
    let vdaf = create_stipend_histogram_vdaf()?;
    let verify_key: [u8; 32] = rand::thread_rng().gen();

    let mut agg_share_0: Option<prio::vdaf::AggregateShare<prio::field::Field128>> = None;
    let mut agg_share_1: Option<prio::vdaf::AggregateShare<prio::field::Field128>> = None;
    let mut accepted_count = 0;

    for &bucket in measurements {
        if bucket >= NUM_BUCKETS {
            return Err(format!("Bucket index {} out of range (max {})", bucket, NUM_BUCKETS - 1).into());
        }

        // 1. Client Sharding
        let nonce: [u8; 16] = rand::thread_rng().gen();
        let (public_share, input_shares) = vdaf.shard(&[], &bucket, &nonce)?;
        assert_eq!(input_shares.len(), 2, "Must produce shares for exactly 2 aggregators");

        // 2. Aggregator Preparation (Round 1: verify_init)
        let (state_0, msg_share_0) = vdaf.verify_init(&verify_key, &[], 0, &(), &nonce, &public_share, &input_shares[0])?;
        let (state_1, msg_share_1) = vdaf.verify_init(&verify_key, &[], 1, &(), &nonce, &public_share, &input_shares[1])?;

        // 3. Message Combination (Prep message exchange)
        let prep_msg = vdaf.verifier_shares_to_message(&[], &(), vec![msg_share_0, msg_share_1])?;

        // 4. Aggregator Preparation (Round 2: verify_next to finish)
        let out_share_0 = match vdaf.verify_next(&[], state_0, prep_msg.clone())? {
            VerifyTransition::Finish(share) => share,
            _ => return Err("Aggregator 0 failed to reach Finish state".into()),
        };
        let out_share_1 = match vdaf.verify_next(&[], state_1, prep_msg)? {
            VerifyTransition::Finish(share) => share,
            _ => return Err("Aggregator 1 failed to reach Finish state".into()),
        };

        // 5. Accumulate into running aggregate shares
        match agg_share_0.as_mut() {
            Some(agg) => agg.accumulate(&out_share_0)?,
            None => agg_share_0 = Some(prio::vdaf::AggregateShare::from(out_share_0)),
        }
        match agg_share_1.as_mut() {
            Some(agg) => agg.accumulate(&out_share_1)?,
            None => agg_share_1 = Some(prio::vdaf::AggregateShare::from(out_share_1)),
        }

        accepted_count += 1;
    }

    if accepted_count == 0 {
        return Ok(vec![0; NUM_BUCKETS]);
    }

    // 6. Collector Unsharding
    let agg_shares = vec![agg_share_0.unwrap(), agg_share_1.unwrap()];
    let final_histogram = vdaf.unshard(&(), agg_shares, accepted_count)?;

    Ok(final_histogram)
}

fn main() {
    println!("=== Blindmark Isolated Prio3Histogram Prototype ===");
    println!("Configuration: {} buckets, {} aggregators, chunk length {}", NUM_BUCKETS, NUM_AGGREGATORS, CHUNK_LENGTH);

    // Test dataset representing diverse monthly stipend submissions
    // Buckets:
    // Bucket 1: $2,000 - $3,499 (1 report)
    // Bucket 3: $5,000 - $6,499 (2 reports)
    // Bucket 5: $8,000 - $9,499 (3 reports)
    // Bucket 11: >= $20,000 (1 report)
    let dataset = vec![1, 3, 5, 5, 3, 5, 11];
    println!("Submitting {} valid measurements: {:?}", dataset.len(), dataset);

    match run_histogram_simulation(&dataset) {
        Ok(hist) => {
            println!("\nSuccessfully aggregated and unshared histogram:");
            for (idx, count) in hist.iter().enumerate() {
                println!("  Bucket {:2}: {} participant(s)", idx, count);
            }
            assert_eq!(hist[1], 1);
            assert_eq!(hist[3], 2);
            assert_eq!(hist[5], 3);
            assert_eq!(hist[11], 1);
            println!("\nVerification equality check PASSED.");
        }
        Err(e) => {
            eprintln!("Error during simulation: {}", e);
            std::process::exit(1);
        }
    }
}
