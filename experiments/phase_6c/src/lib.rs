//! Blindmark Phase 6C: Independent Aggregator Architecture
//!
//! This crate demonstrates a modular four-component Prio3Histogram aggregation
//! architecture with explicit message-passing interfaces between Client, Leader,
//! Helper, and Collector.
//!
//! All communication occurs through typed messages over an in-memory transport.
//! No network deployment or production DAP endpoints are introduced.

pub mod client;
pub mod collector;
pub mod helper;
pub mod leader;
pub mod protocol;
pub mod transport;

