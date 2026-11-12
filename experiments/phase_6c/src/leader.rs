//! Leader Aggregator component for Phase 6C.
//!
//! Receives input shares addressed to `AggregatorId::Leader`.
//! Executes Prio3 verification round 1 (`verify_init`) for Leader.
//! Combines verifier shares from Leader and Helper (`verifier_shares_to_message`).
//! Broadcasts `VerifierMessageBroadcast` to Helper and executes `verify_next` for Leader.
//! Maintains its own independent aggregation state.
//! Coordinates collection by requesting Helper's aggregate share and packaging the response for Collector.
//!
//! # Security Boundaries
//!
//! - Leader never sees Helper's input share.
//! - Leader maintains its own independent `AggregateShare`.
//! - Verification must complete independently for both aggregators before accumulation.

use crate::protocol::*;
use prio::vdaf::{Aggregatable, Aggregator, VerifyTransition};
use std::collections::{HashMap, HashSet};

pub struct LeaderAggregator {
    vdaf: prio::vdaf::prio3::Prio3Histogram,
    verify_key: [u8; SEED_SIZE],
    /// Pending verification state per report_id (Leader's own verify_state + Leader's verifier_share).
    pending_verifications: HashMap<
        ReportId,
        (
            prio::vdaf::prio3::Prio3VerifyState<prio::field::Field128, SEED_SIZE>,
            prio::vdaf::prio3::Prio3VerifierShare<prio::field::Field128, SEED_SIZE>,
        ),
    >,
    /// Independent running aggregate share maintained by Leader.
    aggregate_share: Option<prio::vdaf::AggregateShare<prio::field::Field128>>,
    /// Count of successfully verified and accumulated reports.
    accepted_count: usize,
    /// Set of processed report IDs to prevent duplicate processing.
    processed_reports: HashSet<ReportId>,
}

impl LeaderAggregator {
    /// Constructs a new `LeaderAggregator` sharing the configured verification key.
    pub fn new(verify_key: [u8; SEED_SIZE]) -> Result<Self, ProtocolError> {
        let vdaf = create_vdaf()?;
        Ok(Self {
            vdaf,
            verify_key,
            pending_verifications: HashMap::new(),
            aggregate_share: None,
            accepted_count: 0,
            processed_reports: HashSet::new(),
        })
    }

    /// Process an inbound ClientReport addressed to Leader.
    /// Executes `verify_init` (Round 1) for Leader and stores pending state.
    pub fn handle_client_report(
        &mut self,
        report: ClientReport,
    ) -> Result<(), ProtocolError> {
        if report.target != AggregatorId::Leader {
            return Err(ProtocolError::WrongAggregator {
                expected: AggregatorId::Leader,
                got: report.target,
            });
        }

        if self.processed_reports.contains(&report.report_id) {
            return Err(ProtocolError::DuplicateReport(report.report_id.0));
        }

        let public_share = decode_public_share(&self.vdaf, &report.public_share_encoded)?;
        let input_share = decode_input_share(&self.vdaf, AggregatorId::Leader.index(), &report.input_share_encoded)?;

        // Aggregator 0 (Leader) executes verify_init
        let (verify_state, verifier_share) = self.vdaf.verify_init(
            &self.verify_key,
            &[],
            AggregatorId::Leader.index(),
            &(),
            &report.nonce,
            &public_share,
            &input_share,
        )?;

        self.pending_verifications
            .insert(report.report_id, (verify_state, verifier_share));

        Ok(())
    }

    /// Receive Helper's verifier share, combine verifier shares into a prep message,
    /// complete Leader's `verify_next` and aggregate, and produce the broadcast message for Helper.
    pub fn handle_helper_verifier_share(
        &mut self,
        helper_msg: VerifierShareMessage,
    ) -> Result<VerifierMessageBroadcast, ProtocolError> {
        if helper_msg.agg_id != AggregatorId::Helper {
            return Err(ProtocolError::WrongAggregator {
                expected: AggregatorId::Helper,
                got: helper_msg.agg_id,
            });
        }

        let (leader_state, leader_verifier_share) = self
            .pending_verifications
            .remove(&helper_msg.report_id)
            .ok_or_else(|| {
                ProtocolError::InvalidMessage(format!(
                    "no pending verification for report {:?}",
                    helper_msg.report_id
                ))
            })?;

        // Helper verifier share is decoded using leader_state or helper verify parameters
        let helper_verifier_share =
            decode_verifier_share(&leader_state, &helper_msg.verifier_share_encoded)?;

        // Combine verifier shares into a single verifier message
        let combined_msg = self.vdaf.verifier_shares_to_message(
            &[],
            &(),
            vec![leader_verifier_share, helper_verifier_share],
        )?;

        let verifier_message_encoded = encode_verifier_message(&combined_msg)?;

        // Leader executes verify_next
        let out_share = match self.vdaf.verify_next(&[], leader_state, combined_msg)? {
            VerifyTransition::Finish(share) => share,
            VerifyTransition::Continue(..) => {
                return Err(ProtocolError::VerificationFailed(
                    "unexpected intermediate continue state in 1-round FLP".into(),
                ))
            }
        };

        // Accumulate into Leader's isolated aggregate share
        match self.aggregate_share.as_mut() {
            Some(agg) => agg.accumulate(&out_share)?,
            None => self.aggregate_share = Some(prio::vdaf::AggregateShare::from(out_share)),
        }

        self.processed_reports.insert(helper_msg.report_id.clone());
        self.accepted_count += 1;

        Ok(VerifierMessageBroadcast {
            report_id: helper_msg.report_id,
            verifier_message_encoded,
        })
    }

    /// Produce Leader's aggregate share for collection.
    pub fn produce_aggregate_share(&self) -> Result<AggregateShareMessage, ProtocolError> {
        let agg_share = self.aggregate_share.as_ref().ok_or(ProtocolError::NoMeasurements)?;
        let aggregate_share_encoded = encode_aggregate_share(agg_share)?;

        Ok(AggregateShareMessage {
            agg_id: AggregatorId::Leader,
            aggregate_share_encoded,
            report_count: self.accepted_count,
        })
    }

    /// Package the final collection response containing both aggregate shares.
    pub fn build_collection_response(
        &self,
        job_id: String,
        helper_share: AggregateShareMessage,
    ) -> Result<CollectionResponse, ProtocolError> {
        let leader_share = self.produce_aggregate_share()?;

        if leader_share.report_count != helper_share.report_count {
            return Err(ProtocolError::InvalidMessage(format!(
                "report count mismatch: leader has {}, helper has {}",
                leader_share.report_count, helper_share.report_count
            )));
        }

        Ok(CollectionResponse {
            job_id,
            leader_share,
            helper_share,
        })
    }

    pub fn accepted_count(&self) -> usize {
        self.accepted_count
    }
}

