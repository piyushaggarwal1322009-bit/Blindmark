//! Helper Aggregator component for Phase 6C.
//!
//! Receives only input shares addressed to `AggregatorId::Helper`.
//! Independently executes Prio3 verification.
//! Maintains its own independent aggregation state.
//! Produces its own aggregate share upon collection.
//!
//! # Security Boundaries
//!
//! - The Helper never receives the Leader's private input share.
//! - The Helper verifies using its own copy of the VDAF and verification key.
//! - The Helper accumulates valid output shares into its own separate `AggregateShare`.

use crate::protocol::*;
use prio::vdaf::{Aggregatable, Aggregator, VerifyTransition};
use std::collections::{HashMap, HashSet};

pub struct HelperAggregator {
    vdaf: prio::vdaf::prio3::Prio3Histogram,
    verify_key: [u8; SEED_SIZE],
    /// Pending verification state per report_id (after verify_init, awaiting prep message).
    pending_verifications: HashMap<ReportId, prio::vdaf::prio3::Prio3VerifyState<prio::field::Field128, SEED_SIZE>>,
    /// Independent running aggregate share maintained by Helper.
    aggregate_share: Option<prio::vdaf::AggregateShare<prio::field::Field128>>,
    /// Count of successfully verified and accumulated reports.
    accepted_count: usize,
    /// Set of processed report IDs to prevent duplicate processing.
    processed_reports: HashSet<ReportId>,
}

impl HelperAggregator {
    /// Constructs a new `HelperAggregator` sharing the configured verification key.
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

    /// Process an inbound ClientReport addressed to Helper.
    /// Executes `verify_init` (Round 1) and returns a `VerifierShareMessage` to be sent to Leader.
    pub fn handle_client_report(
        &mut self,
        report: ClientReport,
    ) -> Result<VerifierShareMessage, ProtocolError> {
        if report.target != AggregatorId::Helper {
            return Err(ProtocolError::WrongAggregator {
                expected: AggregatorId::Helper,
                got: report.target,
            });
        }

        if self.processed_reports.contains(&report.report_id) {
            return Err(ProtocolError::DuplicateReport(report.report_id.0));
        }

        let public_share = decode_public_share(&self.vdaf, &report.public_share_encoded)?;
        let input_share = decode_input_share(&self.vdaf, AggregatorId::Helper.index(), &report.input_share_encoded)?;

        // Aggregator 1 (Helper) executes verify_init
        let (verify_state, verifier_share) = self.vdaf.verify_init(
            &self.verify_key,
            &[],
            AggregatorId::Helper.index(),
            &(),
            &report.nonce,
            &public_share,
            &input_share,
        )?;

        let verifier_share_encoded = encode_verifier_share(&verifier_share)?;

        self.pending_verifications
            .insert(report.report_id.clone(), verify_state);

        Ok(VerifierShareMessage {
            report_id: report.report_id,
            agg_id: AggregatorId::Helper,
            verifier_share_encoded,
        })
    }

    /// Process the combined verifier message broadcast by Leader (Round 2).
    /// Executes `verify_next` to finish transition, then accumulates into Helper's aggregate share.
    pub fn handle_verifier_message(
        &mut self,
        msg: VerifierMessageBroadcast,
    ) -> Result<(), ProtocolError> {
        let verify_state = self
            .pending_verifications
            .remove(&msg.report_id)
            .ok_or_else(|| {
                ProtocolError::InvalidMessage(format!(
                    "no pending verification for report {:?}",
                    msg.report_id
                ))
            })?;

        let verifier_msg = decode_verifier_message(&verify_state, &msg.verifier_message_encoded)?;

        let out_share = match self.vdaf.verify_next(&[], verify_state, verifier_msg)? {
            VerifyTransition::Finish(share) => share,
            VerifyTransition::Continue(..) => {
                return Err(ProtocolError::VerificationFailed(
                    "unexpected intermediate continue state in 1-round FLP".into(),
                ))
            }
        };

        // Accumulate into Helper's isolated aggregate share
        match self.aggregate_share.as_mut() {
            Some(agg) => agg.accumulate(&out_share)?,
            None => self.aggregate_share = Some(prio::vdaf::AggregateShare::from(out_share)),
        }

        self.processed_reports.insert(msg.report_id);
        self.accepted_count += 1;
        Ok(())
    }

    /// Produce Helper's aggregate share for collection.
    pub fn produce_aggregate_share(&self) -> Result<AggregateShareMessage, ProtocolError> {
        let agg_share = self.aggregate_share.as_ref().ok_or(ProtocolError::NoMeasurements)?;
        let aggregate_share_encoded = encode_aggregate_share(agg_share)?;

        Ok(AggregateShareMessage {
            agg_id: AggregatorId::Helper,
            aggregate_share_encoded,
            report_count: self.accepted_count,
        })
    }

    pub fn accepted_count(&self) -> usize {
        self.accepted_count
    }
}

