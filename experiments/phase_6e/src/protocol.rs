//! Canonical Phase 6F.2 message-body encoding.
//!
//! This module defines the byte format only. It does not authenticate a frame,
//! select keys, apply freshness or sequence policy, or enable network ingress.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::convert::TryInto;

pub const DOMAIN_SEPARATOR: [u8; 16] = *b"BLINDMARK_6F2CON";
pub const PROTOCOL_ID: [u8; 4] = *b"BLMK";
pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_IDENTIFIER_BYTES: usize = 64;
pub const HMAC_SHA256_BYTES: usize = 32;
pub const MAX_FRAME_BYTES: usize = 65_536;
pub const MAX_PAYLOAD_BYTES: usize = 65_052;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    LeaderToHelperRequest = 0x0001,
    LeaderToHelperAck = 0x0002,
    HelperToLeaderRequest = 0x0003,
    HelperToLeaderAck = 0x0004,
    CommitDecision = 0x0005,
    CommitStatus = 0x0006,
    ReplayCheck = 0x0007,
    Reject = 0x0008,
}

impl TryFrom<u16> for MessageType {
    type Error = ProtocolError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0001 => Ok(Self::LeaderToHelperRequest),
            0x0002 => Ok(Self::LeaderToHelperAck),
            0x0003 => Ok(Self::HelperToLeaderRequest),
            0x0004 => Ok(Self::HelperToLeaderAck),
            0x0005 => Ok(Self::CommitDecision),
            0x0006 => Ok(Self::CommitStatus),
            0x0007 => Ok(Self::ReplayCheck),
            0x0008 => Ok(Self::Reject),
            _ => Err(ProtocolError::UnsupportedMessageType(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolMessage {
    pub key_version: u16,
    pub message_type: MessageType,
    pub protocol_round: u16,
    pub sender_id: String,
    pub recipient_id: String,
    pub task_id: String,
    pub report_id: String,
    pub batch_id: String,
    pub message_id: String,
    pub sequence_no: u64,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidIdentifier(&'static str),
    InvalidTimestampRange,
    UnsupportedVersion(u16),
    UnsupportedMessageType(u16),
    InvalidDomain,
    InvalidProtocolId,
    PayloadTooLarge(usize),
    FrameTooLarge(usize),
    Truncated,
    TrailingBytes,
    InvalidEncoding,
    InvalidKey,
    InvalidMac,
    KeyVersionMismatch { expected: u16, actual: u16 },
}

type HmacSha256 = Hmac<Sha256>;

/// Serialize and authenticate a message using the supplied key material.
/// Key provisioning, direction binding, and rotation remain caller policy.
pub fn sign_frame(message: &ProtocolMessage, key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if key.is_empty() {
        return Err(ProtocolError::InvalidKey);
    }
    let body = message.canonical_body()?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| ProtocolError::InvalidKey)?;
    mac.update(&body);
    let tag = mac.finalize().into_bytes();

    let mut frame = body;
    frame.extend_from_slice(&tag);
    Ok(frame)
}

/// Parse and authenticate exactly one frame using a caller-selected key.
/// `expected_key_version` must come from the caller's trusted key configuration.
pub fn verify_frame(
    frame: &[u8],
    expected_key_version: u16,
    key: &[u8],
) -> Result<ProtocolMessage, ProtocolError> {
    if key.is_empty() {
        return Err(ProtocolError::InvalidKey);
    }
    if frame.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge(frame.len()));
    }
    if frame.len() < HMAC_SHA256_BYTES {
        return Err(ProtocolError::Truncated);
    }

    let body_len = frame.len() - HMAC_SHA256_BYTES;
    let (body, tag) = frame.split_at(body_len);
    let message = ProtocolMessage::from_canonical_body(body)?;
    if message.key_version != expected_key_version {
        return Err(ProtocolError::KeyVersionMismatch {
            expected: expected_key_version,
            actual: message.key_version,
        });
    }

    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| ProtocolError::InvalidKey)?;
    mac.update(body);
    mac.verify_slice(tag)
        .map_err(|_| ProtocolError::InvalidMac)?;
    Ok(message)
}

impl ProtocolMessage {
    /// Encode the canonical body bytes covered by HMAC-SHA256.
    pub fn canonical_body(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;

        let mut body = Vec::with_capacity(452 + self.payload.len());
        body.extend_from_slice(&DOMAIN_SEPARATOR);
        body.extend_from_slice(&PROTOCOL_ID);
        body.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        body.extend_from_slice(&self.key_version.to_be_bytes());
        body.extend_from_slice(&(self.message_type as u16).to_be_bytes());
        body.extend_from_slice(&self.protocol_round.to_be_bytes());

        for (name, value) in [
            ("sender_id", self.sender_id.as_str()),
            ("recipient_id", self.recipient_id.as_str()),
            ("task_id", self.task_id.as_str()),
            ("report_id", self.report_id.as_str()),
            ("batch_id", self.batch_id.as_str()),
            ("message_id", self.message_id.as_str()),
        ] {
            encode_identifier(&mut body, name, value)?;
        }

        body.extend_from_slice(&self.sequence_no.to_be_bytes());
        body.extend_from_slice(&self.issued_at_ms.to_be_bytes());
        body.extend_from_slice(&self.expires_at_ms.to_be_bytes());
        let payload_len: u32 = self
            .payload
            .len()
            .try_into()
            .map_err(|_| ProtocolError::PayloadTooLarge(self.payload.len()))?;
        body.extend_from_slice(&payload_len.to_be_bytes());
        body.extend_from_slice(&self.payload);

        if body.len() + HMAC_SHA256_BYTES > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(body.len() + HMAC_SHA256_BYTES));
        }
        Ok(body)
    }

    /// Parse exactly one canonical body. The separate 32-byte MAC is not read
    /// here; callers must frame and authenticate it before processing.
    pub fn from_canonical_body(body: &[u8]) -> Result<Self, ProtocolError> {
        if body.len() + HMAC_SHA256_BYTES > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(body.len() + HMAC_SHA256_BYTES));
        }

        let mut cursor = Cursor::new(body);
        if cursor.take(DOMAIN_SEPARATOR.len())? != DOMAIN_SEPARATOR {
            return Err(ProtocolError::InvalidDomain);
        }
        if cursor.take(PROTOCOL_ID.len())? != PROTOCOL_ID {
            return Err(ProtocolError::InvalidProtocolId);
        }

        let version = cursor.u16()?;
        if version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }
        let key_version = cursor.u16()?;
        let message_type = MessageType::try_from(cursor.u16()?)?;
        let protocol_round = cursor.u16()?;
        let sender_id = cursor.identifier("sender_id")?;
        let recipient_id = cursor.identifier("recipient_id")?;
        let task_id = cursor.identifier("task_id")?;
        let report_id = cursor.identifier("report_id")?;
        let batch_id = cursor.identifier("batch_id")?;
        let message_id = cursor.identifier("message_id")?;
        let sequence_no = cursor.u64()?;
        let issued_at_ms = cursor.u64()?;
        let expires_at_ms = cursor.u64()?;
        let payload_len = cursor.u32()? as usize;

        if payload_len > MAX_PAYLOAD_BYTES {
            return Err(ProtocolError::PayloadTooLarge(payload_len));
        }
        let payload = cursor.take(payload_len)?.to_vec();
        if !cursor.is_empty() {
            return Err(ProtocolError::TrailingBytes);
        }

        let message = Self {
            key_version,
            message_type,
            protocol_round,
            sender_id,
            recipient_id,
            task_id,
            report_id,
            batch_id,
            message_id,
            sequence_no,
            issued_at_ms,
            expires_at_ms,
            payload,
        };
        message.validate()?;
        Ok(message)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        for (name, value) in [
            ("sender_id", self.sender_id.as_str()),
            ("recipient_id", self.recipient_id.as_str()),
            ("report_id", self.report_id.as_str()),
            ("batch_id", self.batch_id.as_str()),
            ("message_id", self.message_id.as_str()),
        ] {
            validate_identifier(name, value)?;
        }
        validate_identifier("task_id", &self.task_id)?;
        if !self
            .task_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ProtocolError::InvalidIdentifier("task_id"));
        }
        if self.expires_at_ms <= self.issued_at_ms {
            return Err(ProtocolError::InvalidTimestampRange);
        }
        if self.payload.len() > MAX_PAYLOAD_BYTES {
            return Err(ProtocolError::PayloadTooLarge(self.payload.len()));
        }
        Ok(())
    }
}

fn encode_identifier(
    output: &mut Vec<u8>,
    name: &'static str,
    value: &str,
) -> Result<(), ProtocolError> {
    validate_identifier(name, value)?;
    let len: u16 = value
        .len()
        .try_into()
        .map_err(|_| ProtocolError::InvalidIdentifier(name))?;
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn validate_identifier(name: &'static str, value: &str) -> Result<(), ProtocolError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(ProtocolError::InvalidIdentifier(name));
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(ProtocolError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ProtocolError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_be_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| ProtocolError::Truncated)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ProtocolError::Truncated)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_be_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ProtocolError::Truncated)?,
        ))
    }

    fn identifier(&mut self, name: &'static str) -> Result<String, ProtocolError> {
        let len = self.u16()? as usize;
        if len == 0 || len > MAX_IDENTIFIER_BYTES {
            return Err(ProtocolError::InvalidIdentifier(name));
        }
        let bytes = self.take(len)?;
        let value = std::str::from_utf8(bytes).map_err(|_| ProtocolError::InvalidEncoding)?;
        validate_identifier(name, value)?;
        Ok(value.to_owned())
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
