//! Phase 6C Demonstration CLI / binary.
//!
//! Demonstrates the four-component independent aggregator architecture
//! executing the Prio3Histogram workflow over the in-memory transport.

use blindmark_phase_6c::client::HistogramClient;
use blindmark_phase_6c::collector::HistogramCollector;
use blindmark_phase_6c::helper::HelperAggregator;
use blindmark_phase_6c::leader::LeaderAggregator;
use blindmark_phase_6c::protocol::*;
use blindmark_phase_6c::transport::{CollectorInbound, HelperInbound, LeaderInbound, TransportHub};
use rand::Rng;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Blindmark Phase 6C: Independent Aggregator Architecture ===");
    println!(
        "Configuration: {} buckets, {} aggregators, chunk length {}",
        NUM_BUCKETS, NUM_AGGREGATORS, CHUNK_LENGTH
    );

    // 1. Shared setup: verification key shared between aggregators (out-of-band in DAP)
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();

    // 2. Instantiate independent logical components
    let client = HistogramClient::new()?;
    let mut leader = LeaderAggregator::new(verify_key)?;
    let mut helper = HelperAggregator::new(verify_key)?;
    let collector = HistogramCollector::new()?;
    let mut transport = TransportHub::new();

    // 3. Dataset of stipend bucket submissions (from Phase 6A/6B design)
    // Buckets: 1 ($2,000-$3,499), 3 ($5,000-$6,499), 5 ($8,000-$9,499), 11 (>= $20,000)
    let dataset = vec![1, 3, 5, 5, 3, 5, 11];
    println!("Submitting {} valid measurements: {:?}", dataset.len(), dataset);

    // 4. Processing Loop through explicit message transport
    for &measurement in &dataset {
        // Step 4.1: Client generates two independent reports
        let (leader_report, helper_report) = client.generate_reports(measurement)?;

        // Deliver reports via transport
        transport.to_leader.send(LeaderInbound::ClientReport(leader_report));
        transport.to_helper.send(HelperInbound::ClientReport(helper_report));

        // Step 4.2: Leader receives and processes its report (verify_init round 1)
        if let Some(LeaderInbound::ClientReport(rep)) = transport.to_leader.recv() {
            leader.handle_client_report(rep)?;
        }

        // Step 4.3: Helper receives and processes its report (verify_init round 1)
        let helper_share_msg = if let Some(HelperInbound::ClientReport(rep)) = transport.to_helper.recv() {
            helper.handle_client_report(rep)?
        } else {
            return Err("Missing helper report in transport".into());
        };

        // Helper sends verifier share to Leader
        transport
            .to_leader
            .send(LeaderInbound::HelperVerifierShare(helper_share_msg));

        // Step 4.4: Leader receives Helper's verifier share, combines them, executes verify_next,
        // and broadcasts the combined verifier message to Helper
        let broadcast_msg = if let Some(LeaderInbound::HelperVerifierShare(vs)) = transport.to_leader.recv() {
            leader.handle_helper_verifier_share(vs)?
        } else {
            return Err("Missing helper verifier share at leader".into());
        };

        transport
            .to_helper
            .send(HelperInbound::VerifierMessage(broadcast_msg));

        // Step 4.5: Helper receives broadcast message and executes verify_next
        if let Some(HelperInbound::VerifierMessage(vm)) = transport.to_helper.recv() {
            helper.handle_verifier_message(vm)?;
        }
    }

    println!(
        "Verification complete. Leader accepted: {}, Helper accepted: {}",
        leader.accepted_count(),
        helper.accepted_count()
    );

    // 5. Collection Phase
    let job_id = "job-stipend-q4-2026".to_string();
    transport
        .to_leader
        .send(LeaderInbound::CollectionRequest(CollectionRequest {
            job_id: job_id.clone(),
        }));

    // Leader handles collection request: requests Helper aggregate share
    if let Some(LeaderInbound::CollectionRequest(req)) = transport.to_leader.recv() {
        transport
            .to_helper
            .send(HelperInbound::AggregateShareRequest {
                job_id: req.job_id.clone(),
            });

        let helper_agg_msg = if let Some(HelperInbound::AggregateShareRequest { .. }) = transport.to_helper.recv() {
            helper.produce_aggregate_share()?
        } else {
            return Err("Missing aggregate share request at helper".into());
        };

        let collection_response = leader.build_collection_response(req.job_id, helper_agg_msg)?;
        transport
            .to_collector
            .send(CollectorInbound::CollectionResponse(collection_response));
    }

    // Collector receives response and unshards
    let final_histogram = if let Some(CollectorInbound::CollectionResponse(resp)) = transport.to_collector.recv() {
        collector.reconstruct_histogram(&resp)?
    } else {
        return Err("Missing collection response at collector".into());
    };

    println!("\nSuccessfully aggregated and unshared histogram via 4-component protocol:");
    for (idx, count) in final_histogram.iter().enumerate() {
        println!("  Bucket {:2}: {} participant(s)", idx, count);
    }

    assert_eq!(final_histogram[1], 1);
    assert_eq!(final_histogram[3], 2);
    assert_eq!(final_histogram[5], 3);
    assert_eq!(final_histogram[11], 1);
    println!("\nProtocol equality check PASSED.");

    Ok(())
}

