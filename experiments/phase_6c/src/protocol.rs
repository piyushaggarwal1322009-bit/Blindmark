//! Protocol message types and shared configuration for the Prio3Histogram VDAF.
//!
//! Defines the serializable message types exchanged between Client, Leader,
//! Helper, and Collector. Each message documents its sender, recipient, purpose,
//! and validation rules.
//!
//! # Security note
//!
//! These message types carry encoded cryptographic material. The encoding uses
//! the `prio` crate's `Encode`/`Decode` traits. No plaintext measurements,
//! private input shares, or secret verification material are logged.

use prio::codec::{Encode, ParameterizedDecode};
use prio::field::Field128;
use prio::vdaf::prio3::{
    Prio3Histogram, Prio3InputShare, Prio3PublicShare, Prio3VerifierMessage, Prio3VerifierShare,
    Prio3VerifyState,
};
use prio::vdaf::AggregateShare;
use std::io::Cursor;
use thiserror::Error;

/// SEED_SIZE for Prio3Histogram (XofTurboShake128).
pub const SEED_SIZE: usize = 32;

/// NONCE_SIZE for Prio3 (fixed by spec).
pub const NONCE_SIZE: usize = 16;

/// Number of aggregators in standard DAP deployment.
pub const NUM_AGGREGATORS: u8 = 2;

/// Number of histogram buckets (internship stipend ranges from Phase 6A).
pub const NUM_BUCKETS: usize = 12;

/// Gadget chunk length for the parallel sum proof circuit.
pub const CHUNK_LENGTH: usize = 4;

/// Aggregator identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregatorId {
    /// Aggregator 0 — the Leader that coordinates the protocol.
    Leader = 0,
    /// Aggregator 1 — the Helper that independently verifies.
    Helper = 1,
}

impl AggregatorId {
    /// Returns the numeric index for use with the prio crate API.
    pub fn index(&self) -> usize {
        match self {
            AggregatorId::Leader => 0,
            AggregatorId::Helper => 1,
        }
    }
}

/// Errors that can occur during protocol execution.
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("VDAF error: {0}")]
    Vdaf(#[from] prio::vdaf::VdafError),

    #[error("codec error: {0}")]
    Codec(#[from] prio::codec::CodecError),

    #[error("measurement {0} out of range [0, {1})")]
    MeasurementOutOfRange(usize, usize),

    #[error("missing aggregator response from {0:?}")]
    MissingAggregatorResponse(AggregatorId),

    #[error("unexpected aggregator id: expected {expected:?}, got {got:?}")]
    WrongAggregator {
        expected: AggregatorId,
        got: AggregatorId,
    },

    #[error("verification failed: {0}")]
    VerificationFailed(String),

    #[error("aggregation not ready: no accepted measurements")]
    NoMeasurements,

    #[error("invalid message: {0}")]
    InvalidMessage(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("duplicate report id: {0}")]
    DuplicateReport(String),
}

/// Unique identifier for a client report (measurement submission).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReportId(pub String);

impl ReportId {
    /// Generate a new random report identifier.
    pub fn random() -> Self {
        use rand::Rng;
        let id: u64 = rand::thread_rng().gen();
        ReportId(format!("report-{:016x}", id))
    }
}

// ---------------------------------------------------------------------------
// Message types
// ---------------------------------------------------------------------------

/// Message: Client → Leader and Client → Helper
///
/// Contains the client's encoded input share for one aggregator, along with
/// the public share and nonce needed for verification.
///
/// **Sender:** Client
/// **Recipient:** Leader (agg_id=0) or Helper (agg_id=1)
/// **Purpose:** Deliver the aggregator's designated input share for verification.
/// **Validation:** The recipient must check that `target` matches its own id.
/// **Contents:** Public data (public_share, nonce) + one input share (private to this aggregator).
#[derive(Debug, Clone)]
pub struct ClientReport {
    /// Unique report identifier.
    pub report_id: ReportId,
    /// Which aggregator this report is addressed to.
    pub target: AggregatorId,
    /// The nonce used during sharding (shared between both aggregators).
    pub nonce: [u8; NONCE_SIZE],
    /// Encoded public share (shared between both aggregators).
    pub public_share_encoded: Vec<u8>,
    /// Encoded input share designated for this aggregator only.
    pub input_share_encoded: Vec<u8>,
}

/// Message: Leader → Helper (forwarding the verification request)
///
/// Contains the Helper's own input share plus protocol metadata needed to
/// begin verification.
///
/// **Sender:** Leader
/// **Recipient:** Helper
/// **Purpose:** Initiate verification on the Helper side.
/// **Validation:** Helper must check target == Helper.
/// **Contents:** Public data + Helper's input share.
#[derive(Debug, Clone)]
pub struct VerifyRequest {
    pub report_id: ReportId,
    pub nonce: [u8; NONCE_SIZE],
    pub public_share_encoded: Vec<u8>,
    pub input_share_encoded: Vec<u8>,
}

/// Message: Aggregator → Leader (verifier share from verify_init)
///
/// Contains the encoded verifier share produced by one aggregator during
/// the first round of verification.
///
/// **Sender:** Leader or Helper
/// **Recipient:** Leader (for combining)
/// **Purpose:** Provide this aggregator's verifier share for message combination.
/// **Contents:** Encoded verifier share (cryptographic material, not plaintext).
#[derive(Debug, Clone)]
pub struct VerifierShareMessage {
    pub report_id: ReportId,
    pub agg_id: AggregatorId,
    pub verifier_share_encoded: Vec<u8>,
}

/// Message: Leader → Helper (combined verifier message for verify_next)
///
/// Contains the combined verifier message produced by
/// `verifier_shares_to_message`.
///
/// **Sender:** Leader
/// **Recipient:** Helper
/// **Purpose:** Deliver the combined verifier message so Helper can complete verification.
/// **Contents:** Encoded combined verifier message (cryptographic material).
#[derive(Debug, Clone)]
pub struct VerifierMessageBroadcast {
    pub report_id: ReportId,
    pub verifier_message_encoded: Vec<u8>,
}

/// Message: Aggregator → Collector (aggregate share for collection)
///
/// Contains one aggregator's aggregate share after processing all accepted
/// measurements.
///
/// **Sender:** Leader or Helper
/// **Recipient:** Collector
/// **Purpose:** Provide aggregate share for final unsharding.
/// **Contents:** Encoded aggregate share (additive share, not the final result).
#[derive(Debug, Clone)]
pub struct AggregateShareMessage {
    pub agg_id: AggregatorId,
    pub aggregate_share_encoded: Vec<u8>,
    pub report_count: usize,
}

/// Message: Collector → Leader (collection request)
///
/// **Sender:** Collector
/// **Recipient:** Leader
/// **Purpose:** Request aggregate shares from both aggregators.
#[derive(Debug, Clone)]
pub struct CollectionRequest {
    /// Opaque collection job identifier.
    pub job_id: String,
}

/// Message: Leader → Collector (collection response)
///
/// **Sender:** Leader
/// **Recipient:** Collector
/// **Purpose:** Return both aggregate shares for unsharding.
#[derive(Debug, Clone)]
pub struct CollectionResponse {
    pub job_id: String,
    pub leader_share: AggregateShareMessage,
    pub helper_share: AggregateShareMessage,
}

// ---------------------------------------------------------------------------
// VDAF helpers
// ---------------------------------------------------------------------------

/// Constructs the canonical Prio3Histogram VDAF instance.
pub fn create_vdaf() -> Result<Prio3Histogram, ProtocolError> {
    let vdaf = Prio3Histogram::new_histogram(NUM_AGGREGATORS, NUM_BUCKETS, CHUNK_LENGTH)?;
    Ok(vdaf)
}

/// Encode a public share to bytes.
pub fn encode_public_share(share: &Prio3PublicShare<SEED_SIZE>) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::new();
    share.encode(&mut buf)?;
    Ok(buf)
}

/// Decode a public share from bytes.
pub fn decode_public_share(
    vdaf: &Prio3Histogram,
    bytes: &[u8],
) -> Result<Prio3PublicShare<SEED_SIZE>, ProtocolError> {
    let share = Prio3PublicShare::decode_with_param(vdaf, &mut Cursor::new(bytes))?;
    Ok(share)
}

/// Encode an input share to bytes.
pub fn encode_input_share(
    share: &Prio3InputShare<Field128, SEED_SIZE>,
) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::new();
    share.encode(&mut buf)?;
    Ok(buf)
}

/// Decode an input share from bytes.
pub fn decode_input_share(
    vdaf: &Prio3Histogram,
    agg_id: usize,
    bytes: &[u8],
) -> Result<Prio3InputShare<Field128, SEED_SIZE>, ProtocolError> {
    let share =
        Prio3InputShare::decode_with_param(&(vdaf, agg_id), &mut Cursor::new(bytes))?;
    Ok(share)
}

/// Encode a verifier share to bytes.
pub fn encode_verifier_share(
    share: &Prio3VerifierShare<Field128, SEED_SIZE>,
) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::new();
    share.encode(&mut buf)?;
    Ok(buf)
}

/// Decode a verifier share from bytes using a verify state as decoding parameter.
pub fn decode_verifier_share(
    state: &Prio3VerifyState<Field128, SEED_SIZE>,
    bytes: &[u8],
) -> Result<Prio3VerifierShare<Field128, SEED_SIZE>, ProtocolError> {
    let share = Prio3VerifierShare::decode_with_param(state, &mut Cursor::new(bytes))?;
    Ok(share)
}

/// Encode a verifier message to bytes.
pub fn encode_verifier_message(
    msg: &Prio3VerifierMessage<SEED_SIZE>,
) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::new();
    msg.encode(&mut buf)?;
    Ok(buf)
}

/// Decode a verifier message from bytes using a verify state as decoding parameter.
pub fn decode_verifier_message(
    state: &Prio3VerifyState<Field128, SEED_SIZE>,
    bytes: &[u8],
) -> Result<Prio3VerifierMessage<SEED_SIZE>, ProtocolError> {
    let msg = Prio3VerifierMessage::decode_with_param(state, &mut Cursor::new(bytes))?;
    Ok(msg)
}

/// Encode an aggregate share to bytes.
pub fn encode_aggregate_share(
    share: &AggregateShare<Field128>,
) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::new();
    share.encode(&mut buf)?;
    Ok(buf)
}

/// Decode an aggregate share from bytes.
pub fn decode_aggregate_share(
    vdaf: &Prio3Histogram,
    bytes: &[u8],
) -> Result<AggregateShare<Field128>, ProtocolError> {
    let share =
        AggregateShare::decode_with_param(&(vdaf, &()), &mut Cursor::new(bytes))?;
    Ok(share)
}

