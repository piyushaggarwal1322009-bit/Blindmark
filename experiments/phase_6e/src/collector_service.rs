//! Collector Service interface definition and future collection workflow.

use blindmark_phase_6c::collector::HistogramCollector;
use blindmark_phase_6c::protocol::ProtocolError;

pub struct CollectorService {
    collector: HistogramCollector,
}

impl CollectorService {
    pub fn new() -> Result<Self, ProtocolError> {
        let collector = HistogramCollector::new()?;
        Ok(Self { collector })
    }

    /// Collector client interface for retrieving aggregate shares and unsharding.
    /// CRITICAL PRIVACY GUARANTEE: Never releases unverified or arbitrary histograms.
    pub fn unshard_from_shares(
        &self,
        leader_share_encoded: &[u8],
        helper_share_encoded: &[u8],
        count: usize,
    ) -> Result<Vec<u128>, ProtocolError> {
        let response = blindmark_phase_6c::protocol::CollectionResponse {
            job_id: "ephemeral_collection".into(),
            leader_share: blindmark_phase_6c::protocol::AggregateShareMessage {
                agg_id: blindmark_phase_6c::protocol::AggregatorId::Leader,
                aggregate_share_encoded: leader_share_encoded.to_vec(),
                report_count: count,
            },
            helper_share: blindmark_phase_6c::protocol::AggregateShareMessage {
                agg_id: blindmark_phase_6c::protocol::AggregatorId::Helper,
                aggregate_share_encoded: helper_share_encoded.to_vec(),
                report_count: count,
            },
        };

        self.collector.reconstruct_histogram(&response)
    }
}

