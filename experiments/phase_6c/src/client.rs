//! Client component for Phase 6C.
//!
//! Accepts a valid histogram measurement (a bucket index in `[0, NUM_BUCKETS)`),
//! uses the Prio3Histogram client API to generate the public share and input shares,
//! and creates separate `ClientReport` messages addressed to the Leader and Helper.
//!
//! # Security Boundaries
//!
//! - The client never discloses the original measurement to either aggregator.
//! - Aggregator 0 (Leader) receives only input share 0.
//! - Aggregator 1 (Helper) receives only input share 1.
//! - Nonce and public share are shared between both aggregators.

use crate::protocol::*;
use prio::vdaf::Client;
use rand::Rng;

pub struct HistogramClient {
    vdaf: prio::vdaf::prio3::Prio3Histogram,
}

impl HistogramClient {
    /// Creates a new `HistogramClient` with the default 12-bucket VDAF configuration.
    pub fn new() -> Result<Self, ProtocolError> {
        let vdaf = create_vdaf()?;
        Ok(Self { vdaf })
    }

    /// Shards a measurement into two distinct `ClientReport`s, one for Leader and one for Helper.
    ///
    /// Returns `(leader_report, helper_report)`.
    pub fn generate_reports(
        &self,
        measurement: usize,
    ) -> Result<(ClientReport, ClientReport), ProtocolError> {
        if measurement >= NUM_BUCKETS {
            return Err(ProtocolError::MeasurementOutOfRange(measurement, NUM_BUCKETS));
        }

        let report_id = ReportId::random();
        let nonce: [u8; NONCE_SIZE] = rand::thread_rng().gen();

        // Prio3 client sharding: generates public share + 2 input shares
        let (public_share, input_shares) = self.vdaf.shard(&[], &measurement, &nonce)?;
        if input_shares.len() != 2 {
            return Err(ProtocolError::InvalidMessage(format!(
                "expected 2 input shares, got {}",
                input_shares.len()
            )));
        }

        let public_share_encoded = encode_public_share(&public_share)?;
        let leader_input_share_encoded = encode_input_share(&input_shares[0])?;
        let helper_input_share_encoded = encode_input_share(&input_shares[1])?;

        let leader_report = ClientReport {
            report_id: report_id.clone(),
            target: AggregatorId::Leader,
            nonce,
            public_share_encoded: public_share_encoded.clone(),
            input_share_encoded: leader_input_share_encoded,
        };

        let helper_report = ClientReport {
            report_id,
            target: AggregatorId::Helper,
            nonce,
            public_share_encoded,
            input_share_encoded: helper_input_share_encoded,
        };

        Ok((leader_report, helper_report))
    }
}

