//! Minimal HTTP server foundations using std::net.
//!
//! Provides lightweight, robust HTTP/1.1 parsing and serving with no external dependencies.
//! Includes explicit socket timeouts, bounded payload protection, duplicate/conflicting
//! header rejection, transfer-encoding validation, and error handling.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub const DEFAULT_SOCKET_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn ok_json(body: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".into(), "application/json".into());
        headers.insert("Connection".into(), "close".into());
        Self {
            status: 200,
            status_text: "OK".into(),
            headers,
            body: body.into_bytes(),
        }
    }

    pub fn from_status_and_body(status: u16, body: String) -> Self {
        let status_text = match status {
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            408 => "Request Timeout",
            409 => "Conflict",
            413 => "Payload Too Large",
            501 => "Not Implemented",
            503 => "Service Unavailable",
            _ => "Error",
        };
        let mut headers = HashMap::new();
        headers.insert("Content-Type".into(), "application/json".into());
        headers.insert("Connection".into(), "close".into());
        Self {
            status,
            status_text: status_text.into(),
            headers,
            body: body.into_bytes(),
        }
    }
}

/// Validates whether a header name contains only valid RFC 9110 token characters.
fn is_valid_header_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            )
    })
}

/// Validates whether a header value contains only valid visible ASCII characters and spaces/tabs (no CR/LF or control chars).
fn is_valid_header_value(val: &str) -> bool {
    val.bytes().all(|b| b == b'\t' || (0x20..=0x7E).contains(&b))
}

pub fn parse_request(stream: &mut TcpStream, max_payload_bytes: usize) -> Result<HttpRequest, (u16, String)> {
    // Configure socket timeouts to prevent Slowloris attacks
    let _ = stream.set_read_timeout(Some(DEFAULT_SOCKET_TIMEOUT));
    let _ = stream.set_write_timeout(Some(DEFAULT_SOCKET_TIMEOUT));

    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Empty HTTP request\"}".into())),
        Ok(n) => n,
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock => {
            return Err((408, "{\"error\":\"RequestTimeout\",\"detail\":\"Socket read timed out\"}".into()));
        }
        Err(e) => {
            return Err((400, format!("{{\"error\":\"ReadFailed\",\"detail\":\"{}\"}}", e)));
        }
    };

    let req_str = match std::str::from_utf8(&buffer[..bytes_read]) {
        Ok(s) => s,
        Err(_) => return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Request headers must be valid ASCII/UTF-8\"}".into())),
    };

    let parts: Vec<&str> = req_str.split("\r\n\r\n").collect();
    if parts.is_empty() {
        return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Malformed HTTP headers\"}".into()));
    }

    let header_part = parts[0];
    let body_part = if parts.len() > 1 { parts[1] } else { "" };

    let mut lines = header_part.lines();
    let request_line = lines.next().ok_or((
        400,
        "{\"error\":\"BadRequest\",\"detail\":\"Missing request line\"}".into(),
    ))?;
    let mut req_line_parts = request_line.split_whitespace();
    let method = req_line_parts.next().unwrap_or("GET").to_string();
    let path = req_line_parts.next().unwrap_or("/").to_string();

    let mut header_map: HashMap<String, String> = HashMap::new();
    let mut seen_content_length = false;
    let mut seen_transfer_encoding = false;

    for line in lines {
        let (raw_k, raw_v) = line.split_once(':').ok_or_else(|| {
            (400, "{\"error\":\"BadRequest\",\"detail\":\"Malformed header line missing colon\"}".into())
        })?;

        let k = raw_k.trim();
        let v = raw_v.trim();

        if !is_valid_header_name(k) {
            return Err((400, format!("{{\"error\":\"BadRequest\",\"detail\":\"Invalid header name: {}\"}}", k)));
        }
        if !is_valid_header_value(v) {
            return Err((400, format!("{{\"error\":\"BadRequest\",\"detail\":\"Invalid header value for: {}\"}}", k)));
        }

        let lower_k = k.to_ascii_lowercase();

        if lower_k == "content-length" {
            if seen_content_length {
                return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Duplicate Content-Length header is forbidden\"}".into()));
            }
            seen_content_length = true;
        }

        if lower_k == "transfer-encoding" {
            seen_transfer_encoding = true;
        }

        header_map.insert(lower_k, v.to_string());
    }

    // RFC 9112 Section 6.3: Reject if both Transfer-Encoding and Content-Length are present
    if seen_transfer_encoding && seen_content_length {
        return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Requests containing both Transfer-Encoding and Content-Length are forbidden\"}".into()));
    }

    // RFC 9112: If Transfer-Encoding is present, check for chunked (or reject until fully implemented)
    if seen_transfer_encoding {
        return Err((501, "{\"error\":\"NotImplemented\",\"detail\":\"Transfer-Encoding: chunked is not supported; use Content-Length\"}".into()));
    }

    let content_length: usize = if let Some(val_str) = header_map.get("content-length") {
        match val_str.parse::<usize>() {
            Ok(val) => val,
            Err(_) => return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Invalid numeric value in Content-Length header\"}".into())),
        }
    } else {
        body_part.len()
    };

    if content_length > max_payload_bytes {
        return Err((
            413,
            format!(
                "{{\"error\":\"PayloadTooLarge\",\"detail\":\"Payload length {} exceeds limit of {} bytes\"}}",
                content_length, max_payload_bytes
            ),
        ));
    }

    let mut full_body = body_part.as_bytes().to_vec();
    while full_body.len() < content_length {
        let n = match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock => {
                return Err((408, "{\"error\":\"RequestTimeout\",\"detail\":\"Timed out waiting for request body\"}".into()));
            }
            Err(e) => {
                return Err((400, format!("{{\"error\":\"BodyReadError\",\"detail\":\"{}\"}}", e)));
            }
        };
        full_body.extend_from_slice(&buffer[..n]);
        if full_body.len() > max_payload_bytes {
            return Err((
                413,
                format!(
                    "{{\"error\":\"PayloadTooLarge\",\"detail\":\"Body exceeded limit of {} bytes\"}}",
                    max_payload_bytes
                ),
            ));
        }
    }

    if full_body.len() < content_length {
        return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Incomplete request body: client closed stream before Content-Length reached\"}".into()));
    }

    Ok(HttpRequest {
        method,
        path,
        headers: header_map,
        body: full_body,
    })
}

pub fn write_response(stream: &mut TcpStream, resp: &HttpResponse) -> std::io::Result<()> {
    let _ = stream.set_write_timeout(Some(DEFAULT_SOCKET_TIMEOUT));
    let mut header_str = format!("HTTP/1.1 {} {}\r\n", resp.status, resp.status_text);
    for (k, v) in &resp.headers {
        header_str.push_str(&format!("{}: {}\r\n", k, v));
    }
    header_str.push_str(&format!("Content-Length: {}\r\n\r\n", resp.body.len()));
    stream.write_all(header_str.as_bytes())?;
    stream.write_all(&resp.body)?;
    stream.flush()?;
    Ok(())
}
