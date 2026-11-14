use blindmark_phase_6e::config::MAX_PAYLOAD_BYTES;
use blindmark_phase_6e::http::{parse_request, MAX_HEADER_BYTES, MAX_HEADER_COUNT};
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};

fn parse_request_bytes(
    request: Vec<u8>,
) -> Result<blindmark_phase_6e::http::HttpRequest, (u16, String)> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let mut client = TcpStream::connect(address).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    client.write_all(&request).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    parse_request(&mut server, MAX_PAYLOAD_BYTES)
}

fn post_prefix(content_length: Option<usize>) -> Vec<u8> {
    let mut headers = b"POST /coordination HTTP/1.1\r\nHost: localhost\r\n".to_vec();
    if let Some(length) = content_length {
        headers.extend_from_slice(format!("Content-Length: {}\r\n", length).as_bytes());
    }
    headers.extend_from_slice(b"\r\n");
    headers
}

#[test]
fn preserves_arbitrary_binary_body_bytes() {
    let mut request = post_prefix(Some(4));
    request.extend_from_slice(&[0x00, 0x80, 0xff, 0x0a]);

    let parsed = parse_request_bytes(request).unwrap();
    assert_eq!(parsed.body, [0x00, 0x80, 0xff, 0x0a]);
}

#[test]
fn reads_fragmented_body_to_exact_declared_length() {
    let body = vec![0xa5; 5000];
    let mut request = post_prefix(Some(body.len()));
    request.extend_from_slice(&body);

    let parsed = parse_request_bytes(request).unwrap();
    assert_eq!(parsed.body, body);
}

#[test]
fn rejects_missing_content_length_truncated_and_trailing_bodies() {
    let mut missing_length = post_prefix(None);
    missing_length.extend_from_slice(b"body");
    let error = parse_request_bytes(missing_length).unwrap_err();
    assert_eq!(error.0, 400);
    assert!(error.1.contains("Content-Length is required"));

    let mut truncated = post_prefix(Some(5));
    truncated.extend_from_slice(b"four");
    let error = parse_request_bytes(truncated).unwrap_err();
    assert_eq!(error.0, 400);
    assert!(error.1.contains("Incomplete request body"));

    let mut trailing = post_prefix(Some(4));
    trailing.extend_from_slice(b"bodyextra");
    let error = parse_request_bytes(trailing).unwrap_err();
    assert_eq!(error.0, 400);
    assert!(error.1.contains("Trailing bytes"));
}

#[test]
fn rejects_malformed_request_lines_and_header_count_overflow() {
    let malformed =
        b"POST /coordination HTTP/1.0\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n".to_vec();
    let error = parse_request_bytes(malformed).unwrap_err();
    assert_eq!(error.0, 400);
    assert!(error.1.contains("Invalid HTTP request line"));

    let mut too_many_headers = b"GET /health HTTP/1.1\r\n".to_vec();
    for index in 0..=MAX_HEADER_COUNT {
        too_many_headers.extend_from_slice(format!("X-Test-{}: a\r\n", index).as_bytes());
    }
    too_many_headers.extend_from_slice(b"\r\n");
    let error = parse_request_bytes(too_many_headers).unwrap_err();
    assert_eq!(error.0, 400);
    assert!(error.1.contains("Too many HTTP headers"));
}

#[test]
fn rejects_header_byte_limit_overflow() {
    let request = format!(
        "GET /health HTTP/1.1\r\nX-Large: {}\r\n\r\n",
        "a".repeat(MAX_HEADER_BYTES)
    )
    .into_bytes();
    let error = parse_request_bytes(request).unwrap_err();
    assert_eq!(error.0, 400);
    assert!(error.1.contains("headers exceed configured limit"));
}
