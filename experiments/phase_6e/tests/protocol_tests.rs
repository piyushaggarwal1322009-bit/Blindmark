use blindmark_phase_6e::protocol::{
    sign_frame, verify_frame, MessageType, ProtocolError, ProtocolMessage, DOMAIN_SEPARATOR,
    HMAC_SHA256_BYTES, MAX_FRAME_BYTES, MAX_PAYLOAD_BYTES, PROTOCOL_ID,
};

fn sample_message() -> ProtocolMessage {
    ProtocolMessage {
        key_version: 1,
        message_type: MessageType::LeaderToHelperRequest,
        protocol_round: 1,
        sender_id: "leader".into(),
        recipient_id: "helper".into(),
        task_id: "task-001".into(),
        report_id: "report-001".into(),
        batch_id: "batch-001".into(),
        message_id: "msg-0001".into(),
        sequence_no: 1,
        issued_at_ms: 1_700_000_000_000,
        expires_at_ms: 1_700_000_005_000,
        payload: vec![1, 2, 3, 4],
    }
}

#[test]
fn canonical_body_matches_contract_vector_and_round_trips() {
    let message = sample_message();
    let body = message.canonical_body().unwrap();
    let expected = decode_hex("424c494e444d41524b5f364632434f4e424c4d4b000100010001000100066c6561646572000668656c70657200087461736b2d303031000a7265706f72742d303031000962617463682d30303100086d73672d3030303100000000000000010000018bcfe568000000018bcfe57b880000000401020304");

    assert_eq!(DOMAIN_SEPARATOR, *b"BLINDMARK_6F2CON");
    assert_eq!(PROTOCOL_ID, *b"BLMK");
    assert_eq!(body, expected);
    assert_eq!(
        ProtocolMessage::from_canonical_body(&body).unwrap(),
        message
    );
}

#[test]
fn opaque_payload_bytes_are_not_decoded_as_utf8() {
    let mut message = sample_message();
    message.payload = vec![0x00, 0x80, 0xff];
    let body = message.canonical_body().unwrap();
    assert_eq!(
        ProtocolMessage::from_canonical_body(&body).unwrap(),
        message
    );
}

#[test]
fn rejects_invalid_identifiers_and_timestamp_ranges() {
    let mut message = sample_message();
    message.task_id = "task.with.dot".into();
    assert_eq!(
        message.canonical_body().unwrap_err(),
        ProtocolError::InvalidIdentifier("task_id")
    );

    let mut message = sample_message();
    message.sender_id = "non-ascii-é".into();
    assert_eq!(
        message.canonical_body().unwrap_err(),
        ProtocolError::InvalidIdentifier("sender_id")
    );

    let mut message = sample_message();
    message.expires_at_ms = message.issued_at_ms;
    assert_eq!(
        message.canonical_body().unwrap_err(),
        ProtocolError::InvalidTimestampRange
    );
}

#[test]
fn rejects_truncated_trailing_and_unsupported_fields() {
    let body = sample_message().canonical_body().unwrap();

    assert_eq!(
        ProtocolMessage::from_canonical_body(&body[..body.len() - 1]).unwrap_err(),
        ProtocolError::Truncated
    );

    let mut trailing = body.clone();
    trailing.push(0);
    assert_eq!(
        ProtocolMessage::from_canonical_body(&trailing).unwrap_err(),
        ProtocolError::TrailingBytes
    );

    let mut bad_version = body.clone();
    bad_version[20..22].copy_from_slice(&2u16.to_be_bytes());
    assert_eq!(
        ProtocolMessage::from_canonical_body(&bad_version).unwrap_err(),
        ProtocolError::UnsupportedVersion(2)
    );

    let mut bad_type = body;
    bad_type[24..26].copy_from_slice(&0u16.to_be_bytes());
    assert_eq!(
        ProtocolMessage::from_canonical_body(&bad_type).unwrap_err(),
        ProtocolError::UnsupportedMessageType(0)
    );
}

#[test]
fn enforces_documented_payload_and_total_frame_limits() {
    let mut message = sample_message();
    message.sender_id = "a".repeat(64);
    message.recipient_id = "b".repeat(64);
    message.task_id = "t".repeat(64);
    message.report_id = "r".repeat(64);
    message.batch_id = "c".repeat(64);
    message.message_id = "m".repeat(64);
    message.payload = vec![0xa5; MAX_PAYLOAD_BYTES];

    let body = message.canonical_body().unwrap();
    assert_eq!(body.len() + HMAC_SHA256_BYTES, MAX_FRAME_BYTES);

    message.payload.push(0);
    assert!(matches!(
        message.canonical_body(),
        Err(ProtocolError::PayloadTooLarge(_))
    ));
}

fn decode_hex(input: &str) -> Vec<u8> {
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).unwrap();
            let low = (pair[1] as char).to_digit(16).unwrap();
            ((high << 4) | low) as u8
        })
        .collect()
}

#[test]
fn hmac_matches_contract_vector_and_rejects_tampering() {
    let message = sample_message();
    let key: Vec<u8> = (0..32).collect();
    let expected_mac =
        decode_hex("593cae29ad9660a107c97e1dba5877f451684274df8e0a71ff6e95634edeb766");

    let frame = sign_frame(&message, &key).unwrap();
    assert_eq!(&frame[frame.len() - HMAC_SHA256_BYTES..], expected_mac);
    assert_eq!(verify_frame(&frame, 1, &key).unwrap(), message);

    let mut tampered = frame;
    let payload_position = tampered.len() - HMAC_SHA256_BYTES - 1;
    tampered[payload_position] ^= 1;
    assert_eq!(
        verify_frame(&tampered, 1, &key).unwrap_err(),
        ProtocolError::InvalidMac
    );
}

#[test]
fn hmac_verification_rejects_wrong_key_version_and_ambiguous_frame_lengths() {
    let message = sample_message();
    let key = [0x5a; 32];
    let frame = sign_frame(&message, &key).unwrap();

    assert_eq!(
        verify_frame(&frame, 2, &key).unwrap_err(),
        ProtocolError::KeyVersionMismatch {
            expected: 2,
            actual: 1
        }
    );
    assert_eq!(
        verify_frame(&frame[..frame.len() - 1], 1, &key).unwrap_err(),
        ProtocolError::Truncated
    );
    assert_eq!(
        verify_frame(&frame[..HMAC_SHA256_BYTES - 1], 1, &key).unwrap_err(),
        ProtocolError::Truncated
    );

    let mut trailing = frame;
    trailing.push(0);
    assert_eq!(
        verify_frame(&trailing, 1, &key).unwrap_err(),
        ProtocolError::TrailingBytes
    );
}
