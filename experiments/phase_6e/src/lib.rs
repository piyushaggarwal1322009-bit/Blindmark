//! Blindmark Phase 6E: Rust Aggregator HTTP Services
//!
//! Provides independently addressable HTTP services for Leader and Helper aggregators,
//! using standard library HTTP server foundations to ensure zero external dependency bloat
//! and full compatibility with the existing offline toolchain.

pub mod config;
pub mod error;
pub mod state;
pub mod http;
pub mod routes;
pub mod leader_service;
pub mod helper_service;
pub mod collector_service;
pub mod storage;

