//! Collector component for Phase 6C.
//!
//! Initiates collection through the Leader interface.
//! Receives `CollectionResponse` containing aggregate shares from Leader and Helper.
//! Decodes both aggregate shares and reconstructs the aggregate histogram via `vdaf.unshard`.
//! Releases the histogram according to collection policy.
//!
//! # Security Boundaries
//!
//! - Collector receives only additive aggregate shares, never individual input shares.
//! - Unsharding requires matching report counts and valid additive secret reconstruction.
//! - In this phase, Collector verifies that both aggregators agree on the measurement count.

use crate::protocol::*;
use prio::vdaf::Collector;

pub struct HistogramCollector {
    vdaf: prio::vdaf::prio3::Prio3Histogram,
}

impl HistogramCollector {
    /// Constructs a new `HistogramCollector` instance.
    pub fn new() -> Result<Self, ProtocolError> {
        let vdaf = create_vdaf()?;
        Ok(Self { vdaf })
    }

    /// Reconstructs the final histogram from a `CollectionResponse`.
    ///
    /// Validates that both aggregate shares are present, from distinct aggregators,
    /// and have identical report counts before invoking `vdaf.unshard`.
    pub fn reconstruct_histogram(
        &self,
        response: &CollectionResponse,
    ) -> Result<Vec<u128>, ProtocolError> {
        if response.leader_share.agg_id != AggregatorId::Leader {
            return Err(ProtocolError::WrongAggregator {
                expected: AggregatorId::Leader,
                got: response.leader_share.agg_id,
            });
        }
        if response.helper_share.agg_id != AggregatorId::Helper {
            return Err(ProtocolError::WrongAggregator {
                expected: AggregatorId::Helper,
                got: response.helper_share.agg_id,
            });
        }

        let num_measurements = response.leader_share.report_count;
        if num_measurements != response.helper_share.report_count {
            return Err(ProtocolError::InvalidMessage(format!(
                "count mismatch between Leader ({}) and Helper ({})",
                num_measurements, response.helper_share.report_count
            )));
        }

        if num_measurements == 0 {
            return Ok(vec![0; NUM_BUCKETS]);
        }

        let leader_agg =
            decode_aggregate_share(&self.vdaf, &response.leader_share.aggregate_share_encoded)?;
        let helper_agg =
            decode_aggregate_share(&self.vdaf, &response.helper_share.aggregate_share_encoded)?;

        let final_histogram =
            self.vdaf
                .unshard(&(), vec![leader_agg, helper_agg], num_measurements)?;

        Ok(final_histogram)
    }
}

