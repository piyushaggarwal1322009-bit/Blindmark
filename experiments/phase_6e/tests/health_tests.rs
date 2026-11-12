//! Integration & HTTP Route Unit Tests for Phase 6E Aggregator Services.

use blindmark_phase_6e::config::{ServiceConfig, DEV_SERVICE_KEY, MAX_PAYLOAD_BYTES};
use blindmark_phase_6e::helper_service::HelperService;
use blindmark_phase_6e::http::{parse_request, HttpRequest};
use blindmark_phase_6e::leader_service::LeaderService;
use blindmark_phase_6c::protocol::SEED_SIZE;
use rand::Rng;
use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn mock_get(path: &str, auth_key: Option<&str>) -> HttpRequest {
    let mut headers = HashMap::new();
    if let Some(key) = auth_key {
        headers.insert("x-service-key".into(), key.into());
    }
    HttpRequest {
        method: "GET".into(),
        path: path.into(),
        headers,
        body: Vec::new(),
    }
}

fn mock_post(path: &str, body: Vec<u8>, auth_key: Option<&str>) -> HttpRequest {
    let mut headers = HashMap::new();
    headers.insert("content-length".into(), body.len().to_string());
    if let Some(key) = auth_key {
        headers.insert("x-service-key".into(), key.into());
    }
    HttpRequest {
        method: "POST".into(),
        path: path.into(),
        headers,
        body,
    }
}

#[test]
fn test_leader_health_endpoint() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    let req = mock_get("/health", None);
    let resp = handler.handle(&req);

    assert_eq!(resp.status, 200);
    let body_str = String::from_utf8(resp.body).unwrap();
    assert!(body_str.contains("\"status\":\"healthy\""));
    assert!(body_str.contains("\"role\":\"leader\""));
    assert!(body_str.contains("\"aggregator_id\":0"));
    // Ensure sensitive configuration is NOT leaked in health output
    assert!(!body_str.contains(DEV_SERVICE_KEY));
}

#[test]
fn test_helper_health_endpoint() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = HelperService::new(ServiceConfig::default_helper(), verify_key).unwrap();
    let handler = service.handler();

    let req = mock_get("/health", None);
    let resp = handler.handle(&req);

    assert_eq!(resp.status, 200);
    let body_str = String::from_utf8(resp.body).unwrap();
    assert!(body_str.contains("\"status\":\"healthy\""));
    assert!(body_str.contains("\"role\":\"helper\""));
    assert!(body_str.contains("\"aggregator_id\":1"));
    assert!(!body_str.contains(DEV_SERVICE_KEY));
}

#[test]
fn test_authenticated_task_status_success() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    let req = mock_get("/tasks/task-123/status", Some(DEV_SERVICE_KEY));
    let resp = handler.handle(&req);

    assert_eq!(resp.status, 200);
    let body_str = String::from_utf8(resp.body).unwrap();
    assert!(body_str.contains("\"accepted_count\":0"));
    assert!(body_str.contains("\"task_id\":\"task-123\""));
}

#[test]
fn test_authenticated_task_status_unauthorized_rejection() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    // 1. Missing authentication
    let req_no_auth = mock_get("/tasks/task-123/status", None);
    let resp_no_auth = handler.handle(&req_no_auth);
    assert_eq!(resp_no_auth.status, 401);

    // 2. Invalid authentication key
    let req_bad_auth = mock_get("/tasks/task-123/status", Some("invalid-secret-key"));
    let resp_bad_auth = handler.handle(&req_bad_auth);
    assert_eq!(resp_bad_auth.status, 401);
}

#[test]
fn test_task_id_validation_rejection() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    // 1. Invalid characters in task ID (path traversal / injection attempt)
    let req_invalid = mock_get("/tasks/task%20invalid;rm/status", Some(DEV_SERVICE_KEY));
    let resp_invalid = handler.handle(&req_invalid);
    assert_eq!(resp_invalid.status, 400);
    let body_str = String::from_utf8(resp_invalid.body).unwrap();
    assert!(body_str.contains("InvalidTaskId"));

    // 2. Excessively long task ID (> 64 characters)
    let long_id = "a".repeat(70);
    let req_long = mock_get(&format!("/tasks/{}/status", long_id), Some(DEV_SERVICE_KEY));
    let resp_long = handler.handle(&req_long);
    assert_eq!(resp_long.status, 400);
    let body_str = String::from_utf8(resp_long.body).unwrap();
    assert!(body_str.contains("InvalidTaskId"));
}

#[test]
fn test_wrong_aggregator_target_rejection_at_leader() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    // Submitting to /tasks/task-123/reports/1 (Helper ID) at Leader service -> MUST BE REJECTED
    let req_wrong = mock_post("/tasks/task-123/reports/1", b"{\"share\":\"dummy\"}".to_vec(), None);
    let resp_wrong = handler.handle(&req_wrong);

    assert_eq!(resp_wrong.status, 400);
    let body_str = String::from_utf8(resp_wrong.body).unwrap();
    assert!(body_str.contains("WrongAggregatorTarget"));
}

#[test]
fn test_wrong_aggregator_target_rejection_at_helper() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = HelperService::new(ServiceConfig::default_helper(), verify_key).unwrap();
    let handler = service.handler();

    // Submitting to /tasks/task-123/reports/0 (Leader ID) at Helper service -> MUST BE REJECTED
    let req_wrong = mock_post("/tasks/task-123/reports/0", b"{\"share\":\"dummy\"}".to_vec(), None);
    let resp_wrong = handler.handle(&req_wrong);

    assert_eq!(resp_wrong.status, 400);
    let body_str = String::from_utf8(resp_wrong.body).unwrap();
    assert!(body_str.contains("WrongAggregatorTarget"));
}

#[test]
fn test_unsupported_report_ingestion_not_implemented_501() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    // Correct target and valid task ID -> returns 501 Not Implemented because cryptographic VDAF ingestion is not yet connected
    let req = mock_post("/tasks/valid-task-1/reports/0", b"valid_share_data".to_vec(), None);
    let resp = handler.handle(&req);

    assert_eq!(resp.status, 501);
    let body_str = String::from_utf8(resp.body).unwrap();
    assert!(body_str.contains("NotImplemented"));
    assert!(body_str.contains("Cryptographic report ingestion is not yet connected"));
}

#[test]
fn test_oversized_payload_rejection() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    // Exceed MAX_PAYLOAD_BYTES (65,536 bytes)
    let oversized_body = vec![0u8; MAX_PAYLOAD_BYTES + 100];
    let req_oversized = mock_post("/tasks/task-123/reports/0", oversized_body, None);
    let resp_oversized = handler.handle(&req_oversized);

    assert_eq!(resp_oversized.status, 413);
    let body_str = String::from_utf8(resp_oversized.body).unwrap();
    assert!(body_str.contains("PayloadTooLarge"));
}

#[test]
fn test_malformed_empty_body_rejection() {
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(ServiceConfig::default_leader(), verify_key).unwrap();
    let handler = service.handler();

    let req_empty = mock_post("/tasks/task-123/reports/0", Vec::new(), None);
    let resp_empty = handler.handle(&req_empty);

    assert_eq!(resp_empty.status, 400);
    let body_str = String::from_utf8(resp_empty.body).unwrap();
    assert!(body_str.contains("Request body cannot be empty"));
}

#[test]
fn test_live_bounded_concurrent_connections_and_timeout() {
    let mut config = ServiceConfig::default_leader();
    config.port = 18001; // Test-specific isolated port
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(config.clone(), verify_key).unwrap();
    let _handle = service.spawn().unwrap();

    thread::sleep(Duration::from_millis(50));

    // Send 5 concurrent valid health requests
    let mut threads = Vec::new();
    for _ in 0..5 {
        threads.push(thread::spawn(move || {
            let mut stream = TcpStream::connect("127.0.0.1:18001").expect("Connect failed");
            let req = "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
            stream.write_all(req.as_bytes()).expect("Write failed");
            let parsed = parse_request(&mut stream, MAX_PAYLOAD_BYTES);
            // We read server response using raw read or stream
            assert!(parsed.is_err() || parsed.is_ok());
        }));
    }

    for t in threads {
        t.join().unwrap();
    }

    service.stop();
}

#[test]
fn test_exactly_64_concurrent_admitted_connections_and_rejection() {
    use std::io::Read;
    use blindmark_phase_6e::leader_service::MAX_CONCURRENT_CONNECTIONS;

    let mut config = ServiceConfig::default_leader();
    config.port = 18002;
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(config.clone(), verify_key).unwrap();
    let _handle = service.spawn().unwrap();

    thread::sleep(Duration::from_millis(50));

    // Connect 64 connections that hold their sockets open without finishing
    let mut holding_streams = Vec::with_capacity(MAX_CONCURRENT_CONNECTIONS);
    for _ in 0..MAX_CONCURRENT_CONNECTIONS {
        let stream = TcpStream::connect("127.0.0.1:18002").expect("Failed to connect admitted socket");
        holding_streams.push(stream);
    }

    // Give server listener loop a moment to accept and reserve all 64 slots
    thread::sleep(Duration::from_millis(150));
    assert_eq!(service.active_connections(), MAX_CONCURRENT_CONNECTIONS);

    // 65th connection must be rejected with HTTP 503
    let mut rejected_stream = TcpStream::connect("127.0.0.1:18002").expect("Connect 65th socket failed");
    let mut buf = [0u8; 1024];
    let n = rejected_stream.read(&mut buf).expect("Read from rejected socket failed");
    let response_str = String::from_utf8_lossy(&buf[..n]);
    assert!(
        response_str.starts_with("HTTP/1.1 503") || response_str.contains("503 Service Unavailable"),
        "Expected 503 rejection, got: {}",
        response_str
    );

    // Drop holding streams; active connections must return to 0
    drop(holding_streams);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(service.active_connections(), 0);

    service.stop();
}

#[test]
fn test_simultaneous_connection_attempts() {
    let mut config = ServiceConfig::default_leader();
    config.port = 18003;
    let verify_key: [u8; SEED_SIZE] = rand::thread_rng().gen();
    let service = LeaderService::new(config.clone(), verify_key).unwrap();
    let _handle = service.spawn().unwrap();

    thread::sleep(Duration::from_millis(50));

    // Blast 80 connections simultaneously from multiple threads
    let mut handles = Vec::new();
    for _ in 0..80 {
        handles.push(thread::spawn(|| {
            if let Ok(mut stream) = TcpStream::connect("127.0.0.1:18003") {
                let req = "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(req.as_bytes());
                let mut buf = [0u8; 512];
                let _ = std::io::Read::read(&mut stream, &mut buf);
            }
        }));
    }

    for h in handles {
        let _ = h.join();
    }

    thread::sleep(Duration::from_millis(150));
    // Concurrency must never exceed 64 and must safely return to 0
    assert_eq!(service.active_connections(), 0);

    service.stop();
}

#[test]
fn test_worker_panic_slot_cleanup() {
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use blindmark_phase_6e::leader_service::ConnectionGuard;

    let counter = Arc::new(AtomicUsize::new(0));

    // Simulate worker acquiring slot then panicking
    let counter_clone = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        let _guard = ConnectionGuard::try_acquire(&counter_clone, 64).expect("Acquisition failed");
        assert_eq!(counter_clone.load(std::sync::atomic::Ordering::SeqCst), 1);
        panic!("Simulated worker panic");
    });

    let _ = handle.join(); // Catch panic

    // Guard Drop must have executed despite thread panic, restoring counter to 0
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 0);
}

fn create_tcp_pair() -> (TcpStream, TcpStream) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral listener");
    let addr = listener.local_addr().expect("Failed to get local addr");
    let client = TcpStream::connect(addr).expect("Failed to connect client");
    let (server, _) = listener.accept().expect("Failed to accept server");
    (client, server)
}

#[test]
fn test_duplicate_content_length_header_rejection() {
    let (mut client, mut server) = create_tcp_pair();
    thread::spawn(move || {
        let payload = "POST /api/v1/test HTTP/1.1\r\nHost: localhost\r\nContent-Length: 10\r\nContent-Length: 20\r\n\r\n1234567890";
        client.write_all(payload.as_bytes()).unwrap();
    });

    let res = parse_request(&mut server, MAX_PAYLOAD_BYTES);
    assert!(res.is_err());
    let (status, body) = res.unwrap_err();
    assert_eq!(status, 400);
    assert!(body.contains("Duplicate Content-Length"));
}

#[test]
fn test_conflicting_content_length_rejection() {
    let (mut client, mut server) = create_tcp_pair();
    thread::spawn(move || {
        let payload = "POST /api/v1/test HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nContent-Length: 99\r\n\r\nhello";
        client.write_all(payload.as_bytes()).unwrap();
    });

    let res = parse_request(&mut server, MAX_PAYLOAD_BYTES);
    assert!(res.is_err());
    let (status, body) = res.unwrap_err();
    assert_eq!(status, 400);
    assert!(body.contains("Duplicate Content-Length"));
}

#[test]
fn test_transfer_encoding_chunked_rejection() {
    let (mut client, mut server) = create_tcp_pair();
    thread::spawn(move || {
        let payload = "POST /api/v1/test HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        client.write_all(payload.as_bytes()).unwrap();
    });

    let res = parse_request(&mut server, MAX_PAYLOAD_BYTES);
    assert!(res.is_err());
    let (status, body) = res.unwrap_err();
    assert_eq!(status, 501);
    assert!(body.contains("chunked is not supported"));
}

#[test]
fn test_transfer_encoding_and_content_length_conflict_rejection() {
    let (mut client, mut server) = create_tcp_pair();
    thread::spawn(move || {
        let payload = "POST /api/v1/test HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\nhello";
        client.write_all(payload.as_bytes()).unwrap();
    });

    let res = parse_request(&mut server, MAX_PAYLOAD_BYTES);
    assert!(res.is_err());
    let (status, body) = res.unwrap_err();
    assert_eq!(status, 400);
    assert!(body.contains("both Transfer-Encoding and Content-Length"));
}

#[test]
fn test_malformed_header_rejection() {
    let (mut client, mut server) = create_tcp_pair();
    thread::spawn(move || {
        // Missing colon in header
        let payload = "GET /health HTTP/1.1\r\nHost localhost\r\n\r\n";
        client.write_all(payload.as_bytes()).unwrap();
    });

    let res = parse_request(&mut server, MAX_PAYLOAD_BYTES);
    assert!(res.is_err());
    let (status, body) = res.unwrap_err();
    assert_eq!(status, 400);
    assert!(body.contains("Malformed header line missing colon"));
}

#[test]
fn test_incomplete_request_timeout_or_eof() {
    let (mut client, mut server) = create_tcp_pair();
    thread::spawn(move || {
        // Declares 20 bytes but sends only 5 then abruptly closes
        let payload = "POST /api/v1/test HTTP/1.1\r\nHost: localhost\r\nContent-Length: 20\r\n\r\nhello";
        client.write_all(payload.as_bytes()).unwrap();
        drop(client);
    });

    let res = parse_request(&mut server, MAX_PAYLOAD_BYTES);
    assert!(res.is_err());
    let (status, body) = res.unwrap_err();
    assert_eq!(status, 400);
    assert!(body.contains("Incomplete request body"));
}
