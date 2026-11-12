//! Integration and Protocol Tests for Phase 6C Independent Aggregator Architecture.

use blindmark_phase_6c::client::HistogramClient;
use blindmark_phase_6c::collector::HistogramCollector;
use blindmark_phase_6c::helper::HelperAggregator;
use blindmark_phase_6c::leader::LeaderAggregator;
use blindmark_phase_6c::protocol::*;
use blindmark_phase_6c::transport::{CollectorInbound, HelperInbound, LeaderInbound, TransportHub};
use rand::Rng;

fn run_protocol_pipeline(measurements: &[usize]) -> Result<Vec<u128>, ProtocolError> {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let client = HistogramClient::new()?;
    let mut leader = LeaderAggregator::new(verify_key)?;
    let mut helper = HelperAggregator::new(verify_key)?;
    let collector = HistogramCollector::new()?;
    let mut transport = TransportHub::new();

    for &m in measurements {
        let (leader_rep, helper_rep) = client.generate_reports(m)?;
        transport.to_leader.send(LeaderInbound::ClientReport(leader_rep));
        transport.to_helper.send(HelperInbound::ClientReport(helper_rep));

        let l_rep = match transport.to_leader.recv().unwrap() {
            LeaderInbound::ClientReport(r) => r,
            _ => panic!("wrong message"),
        };
        leader.handle_client_report(l_rep)?;

        let h_rep = match transport.to_helper.recv().unwrap() {
            HelperInbound::ClientReport(r) => r,
            _ => panic!("wrong message"),
        };
        let h_share = helper.handle_client_report(h_rep)?;
        transport
            .to_leader
            .send(LeaderInbound::HelperVerifierShare(h_share));

        let h_vs = match transport.to_leader.recv().unwrap() {
            LeaderInbound::HelperVerifierShare(s) => s,
            _ => panic!("wrong message"),
        };
        let broadcast = leader.handle_helper_verifier_share(h_vs)?;
        transport
            .to_helper
            .send(HelperInbound::VerifierMessage(broadcast));

        let vm = match transport.to_helper.recv().unwrap() {
            HelperInbound::VerifierMessage(m) => m,
            _ => panic!("wrong message"),
        };
        helper.handle_verifier_message(vm)?;
    }

    let helper_agg = helper.produce_aggregate_share()?;
    let collection_resp = leader.build_collection_response("test-job".into(), helper_agg)?;
    transport
        .to_collector
        .send(CollectorInbound::CollectionResponse(collection_resp));

    let resp = match transport.to_collector.recv().unwrap() {
        CollectorInbound::CollectionResponse(r) => r,
    };
    collector.reconstruct_histogram(&resp)
}

#[test]
fn test_single_valid_bucket() {
    let hist = run_protocol_pipeline(&[4]).expect("single bucket pipeline failed");
    assert_eq!(hist.len(), NUM_BUCKETS);
    assert_eq!(hist[4], 1);
    for i in 0..NUM_BUCKETS {
        if i != 4 {
            assert_eq!(hist[i], 0);
        }
    }
}

#[test]
fn test_multiple_valid_buckets() {
    let dataset = vec![0, 11, 0, 4, 7, 11, 11];
    let hist = run_protocol_pipeline(&dataset).expect("multiple bucket pipeline failed");
    assert_eq!(hist[0], 2);
    assert_eq!(hist[4], 1);
    assert_eq!(hist[7], 1);
    assert_eq!(hist[11], 3);
}

#[test]
fn test_first_and_last_valid_buckets() {
    let dataset = vec![0, 11];
    let hist = run_protocol_pipeline(&dataset).expect("boundary bucket pipeline failed");
    assert_eq!(hist[0], 1);
    assert_eq!(hist[11], 1);
}

#[test]
fn test_out_of_range_bucket_rejected_at_client() {
    let client = HistogramClient::new().unwrap();
    let res = client.generate_reports(NUM_BUCKETS);
    assert!(res.is_err(), "Measurement = NUM_BUCKETS must be rejected");
    let res2 = client.generate_reports(100);
    assert!(res2.is_err(), "Measurement = 100 must be rejected");
}

#[test]
fn test_leader_and_helper_receive_distinct_shares() {
    let client = HistogramClient::new().unwrap();
    let (l_rep, h_rep) = client.generate_reports(2).unwrap();

    assert_eq!(l_rep.target, AggregatorId::Leader);
    assert_eq!(h_rep.target, AggregatorId::Helper);
    assert_eq!(l_rep.nonce, h_rep.nonce);
    assert_eq!(l_rep.public_share_encoded, h_rep.public_share_encoded);
    // Crucial: Leader and Helper receive distinct input share encodings!
    assert_ne!(
        l_rep.input_share_encoded, h_rep.input_share_encoded,
        "Leader and Helper input shares must not be identical"
    );
}

#[test]
fn test_independent_verification_and_accumulation() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let client = HistogramClient::new().unwrap();
    let mut leader = LeaderAggregator::new(verify_key).unwrap();
    let mut helper = HelperAggregator::new(verify_key).unwrap();

    let (l_rep, h_rep) = client.generate_reports(3).unwrap();
    leader.handle_client_report(l_rep).unwrap();
    let h_share = helper.handle_client_report(h_rep).unwrap();
    let broadcast = leader.handle_helper_verifier_share(h_share).unwrap();
    helper.handle_verifier_message(broadcast).unwrap();

    assert_eq!(leader.accepted_count(), 1);
    assert_eq!(helper.accepted_count(), 1);

    // Both aggregators independently produce their own aggregate shares
    let l_agg = leader.produce_aggregate_share().unwrap();
    let h_agg = helper.produce_aggregate_share().unwrap();

    assert_eq!(l_agg.agg_id, AggregatorId::Leader);
    assert_eq!(h_agg.agg_id, AggregatorId::Helper);
    // In additive secret sharing over field elements, the two aggregate shares are distinct random shares
    assert_ne!(l_agg.aggregate_share_encoded, h_agg.aggregate_share_encoded);
}

#[test]
fn test_wrong_aggregator_target_rejection() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let client = HistogramClient::new().unwrap();
    let mut leader = LeaderAggregator::new(verify_key).unwrap();
    let mut helper = HelperAggregator::new(verify_key).unwrap();

    let (l_rep, h_rep) = client.generate_reports(1).unwrap();

    // Send Leader report to Helper -> must fail
    let res = helper.handle_client_report(l_rep);
    assert!(matches!(res, Err(ProtocolError::WrongAggregator { .. })));

    // Send Helper report to Leader -> must fail
    let res = leader.handle_client_report(h_rep);
    assert!(matches!(res, Err(ProtocolError::WrongAggregator { .. })));
}

#[test]
fn test_duplicate_report_rejection() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let client = HistogramClient::new().unwrap();
    let mut leader = LeaderAggregator::new(verify_key).unwrap();
    let mut helper = HelperAggregator::new(verify_key).unwrap();

    let (l_rep, h_rep) = client.generate_reports(5).unwrap();

    leader.handle_client_report(l_rep.clone()).unwrap();
    let h_share = helper.handle_client_report(h_rep.clone()).unwrap();
    let broadcast = leader.handle_helper_verifier_share(h_share).unwrap();
    helper.handle_verifier_message(broadcast).unwrap();

    // Re-submitting the exact same report ID to Helper must be rejected
    let res_h = helper.handle_client_report(h_rep);
    assert!(matches!(res_h, Err(ProtocolError::DuplicateReport(..))));

    // Re-submitting to Leader must be rejected
    let res_l = leader.handle_client_report(l_rep);
    assert!(matches!(res_l, Err(ProtocolError::DuplicateReport(..))));
}

#[test]
fn test_malformed_input_share_syntax_rejection() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let client = HistogramClient::new().unwrap();
    let mut helper = HelperAggregator::new(verify_key).unwrap();

    let (_, mut h_rep) = client.generate_reports(2).unwrap();
    // Corrupt the encoding length / structure
    h_rep.input_share_encoded.truncate(4);

    let res = helper.handle_client_report(h_rep);
    assert!(res.is_err(), "Truncated/corrupted share encoding must fail decoding");
}

#[test]
fn test_tampered_verifier_share_cryptographic_proof_failure() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let client = HistogramClient::new().unwrap();
    let mut leader = LeaderAggregator::new(verify_key).unwrap();
    let mut helper = HelperAggregator::new(verify_key).unwrap();

    let (l_rep, h_rep) = client.generate_reports(3).unwrap();
    leader.handle_client_report(l_rep).unwrap();
    let mut h_share = helper.handle_client_report(h_rep).unwrap();

    // Tamper with the bytes of Helper's verifier share (FLPs verifiers)
    if let Some(last) = h_share.verifier_share_encoded.last_mut() {
        *last ^= 0xff;
    }

    // Leader combines verifier shares -> proof verification check fails!
    let res = leader.handle_helper_verifier_share(h_share);
    assert!(
        res.is_err(),
        "Cryptographically tampered verifier share must cause proof check failure"
    );
}

#[test]
fn test_collector_mismatched_report_count_rejection() {
    let collector = HistogramCollector::new().unwrap();
    let mock_response = CollectionResponse {
        job_id: "mismatch-job".into(),
        leader_share: AggregateShareMessage {
            agg_id: AggregatorId::Leader,
            aggregate_share_encoded: vec![0; 32],
            report_count: 5,
        },
        helper_share: AggregateShareMessage {
            agg_id: AggregatorId::Helper,
            aggregate_share_encoded: vec![0; 32],
            report_count: 4, // Mismatch!
        },
    };

    let res = collector.reconstruct_histogram(&mock_response);
    assert!(res.is_err(), "Mismatched report count must be rejected by Collector");
}

