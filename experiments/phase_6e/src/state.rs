//! State management for Leader and Helper aggregator services.

use blindmark_phase_6c::helper::HelperAggregator;
use blindmark_phase_6c::leader::LeaderAggregator;
use blindmark_phase_6c::protocol::SEED_SIZE;
use std::sync::Mutex;

pub struct LeaderState {
    pub aggregator: Mutex<LeaderAggregator>,
    pub verify_key: [u8; SEED_SIZE],
}

impl LeaderState {
    pub fn new(verify_key: [u8; SEED_SIZE]) -> Result<Self, blindmark_phase_6c::protocol::ProtocolError> {
        let aggregator = LeaderAggregator::new(verify_key)?;
        Ok(Self {
            aggregator: Mutex::new(aggregator),
            verify_key,
        })
    }
}

pub struct HelperState {
    pub aggregator: Mutex<HelperAggregator>,
    pub verify_key: [u8; SEED_SIZE],
}

impl HelperState {
    pub fn new(verify_key: [u8; SEED_SIZE]) -> Result<Self, blindmark_phase_6c::protocol::ProtocolError> {
        let aggregator = HelperAggregator::new(verify_key)?;
        Ok(Self {
            aggregator: Mutex::new(aggregator),
            verify_key,
        })
    }
}

