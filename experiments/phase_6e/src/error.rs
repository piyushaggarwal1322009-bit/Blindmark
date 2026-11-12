//! Service error types and HTTP status mappings.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Wrong aggregator target: expected {expected}, got {got}")]
    WrongAggregator { expected: u8, got: u8 },

    #[error("Invalid task ID: {0}")]
    InvalidTaskId(String),

    #[error("Payload too large: maximum allowed is {max_bytes} bytes, received {received_bytes} bytes")]
    PayloadTooLarge { max_bytes: usize, received_bytes: usize },

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Protocol error: {0}")]
    Protocol(#[from] blindmark_phase_6c::protocol::ProtocolError),
}

impl ServiceError {
    /// Maps internal service error to standard HTTP status code and JSON response body.
    pub fn to_http_status_and_body(&self) -> (u16, String) {
        match self {
            ServiceError::Unauthorized(msg) => (
                401,
                format!("{{\"error\":\"Unauthorized\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::WrongAggregator { expected, got } => (
                400,
                format!(
                    "{{\"error\":\"WrongAggregatorTarget\",\"detail\":\"Request addressed to aggregator {}, but this service is aggregator {}\"}}",
                    got, expected
                ),
            ),
            ServiceError::InvalidTaskId(msg) => (
                400,
                format!("{{\"error\":\"InvalidTaskId\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::PayloadTooLarge { max_bytes, received_bytes } => (
                413,
                format!(
                    "{{\"error\":\"PayloadTooLarge\",\"detail\":\"Payload length {} exceeds maximum allowed limit of {} bytes\"}}",
                    received_bytes, max_bytes
                ),
            ),
            ServiceError::BadRequest(msg) => (
                400,
                format!("{{\"error\":\"BadRequest\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::NotImplemented(msg) => (
                501,
                format!("{{\"error\":\"NotImplemented\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::NotFound(msg) => (
                404,
                format!("{{\"error\":\"NotFound\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::Conflict(msg) => (
                409,
                format!("{{\"error\":\"Conflict\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::MethodNotAllowed(msg) => (
                405,
                format!("{{\"error\":\"MethodNotAllowed\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::Internal(msg) => (
                500,
                format!("{{\"error\":\"InternalServerError\",\"detail\":\"{}\"}}", msg),
            ),
            ServiceError::Protocol(err) => (
                500,
                format!("{{\"error\":\"ProtocolError\",\"detail\":\"{}\"}}", err),
            ),
        }
    }
}
