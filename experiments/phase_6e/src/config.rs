//! Service configuration module.

use blindmark_phase_6c::protocol::AggregatorId;

/// Maximum payload size allowed for HTTP requests (64 KB).
pub const MAX_PAYLOAD_BYTES: usize = 65536;

/// Development service authorization key.
/// WARNING: Development only. Production requires mutual TLS (mTLS) with client certs.
pub const DEV_SERVICE_KEY: &str = "dev-aggregator-secret-key-blindmark";

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
    pub aggregator_id: AggregatorId,
    pub max_payload_bytes: usize,
    pub auth_key: String,
}

impl ServiceConfig {
    pub fn default_leader() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8001,
            aggregator_id: AggregatorId::Leader,
            max_payload_bytes: MAX_PAYLOAD_BYTES,
            auth_key: DEV_SERVICE_KEY.into(),
        }
    }

    pub fn default_helper() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8002,
            aggregator_id: AggregatorId::Helper,
            max_payload_bytes: MAX_PAYLOAD_BYTES,
            auth_key: DEV_SERVICE_KEY.into(),
        }
    }
}

