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
pub const MAX_HEADER_BYTES: usize = 16_384;
pub const MAX_HEADER_COUNT: usize = 64;

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
    let _ = stream.set_read_timeout(Some(DEFAULT_SOCKET_TIMEOUT));
    let _ = stream.set_write_timeout(Some(DEFAULT_SOCKET_TIMEOUT));

    let mut request_bytes = Vec::new();
    let header_end = loop {
        if let Some(position) = find_header_end(&request_bytes) {
            let end = position + 4;
            if end > MAX_HEADER_BYTES {
                return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"HTTP headers exceed configured limit\"}".into()));
            }
            break end;
        }
        if request_bytes.len() >= MAX_HEADER_BYTES {
            return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"HTTP headers exceed configured limit\"}".into()));
        }

        let mut chunk = [0u8; 4096];
        let count = match stream.read(&mut chunk) {
            Ok(0) if request_bytes.is_empty() => {
                return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Empty HTTP request\"}".into()));
            }
            Ok(0) => {
                return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Incomplete HTTP headers\"}".into()));
            }
            Ok(count) => count,
            Err(ref error) if error.kind() == std::io::ErrorKind::TimedOut || error.kind() == std::io::ErrorKind::WouldBlock => {
                return Err((408, "{\"error\":\"RequestTimeout\",\"detail\":\"Socket read timed out\"}".into()));
            }
            Err(_) => {
                return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Unable to read HTTP request\"}".into()));
            }
        };
        request_bytes.extend_from_slice(&chunk[..count]);
        if request_bytes.len() > MAX_HEADER_BYTES && find_header_end(&request_bytes).is_none() {
            return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"HTTP headers exceed configured limit\"}".into()));
        }
    };

    let header_bytes = &request_bytes[..header_end - 4];
    if !header_bytes.is_ascii() {
        return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"HTTP headers must be ASCII\"}".into()));
    }
    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|_| (400, "{\"error\":\"BadRequest\",\"detail\":\"Invalid HTTP headers\"}".into()))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| {
        (400, "{\"error\":\"BadRequest\",\"detail\":\"Missing request line\"}".into())
    })?;
    let request_parts: Vec<&str> = request_line.split(' ').collect();
    if request_parts.len() != 3
        || request_parts[0].is_empty()
        || !request_parts[0].bytes().all(|byte| byte.is_ascii_uppercase())
        || request_parts[1].is_empty()
        || !request_parts[1].starts_with('/')
        || !request_parts[1].bytes().all(|byte| (0x21..=0x7e).contains(&byte))
        || request_parts[2] != "HTTP/1.1"
    {
        return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Invalid HTTP request line\"}".into()));
    }
    let method = request_parts[0].to_owned();
    let path = request_parts[1].to_owned();

    let mut header_map: HashMap<String, String> = HashMap::new();
    let mut header_count = 0usize;
    for line in lines {
        if line.is_empty() {
            return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Malformed HTTP headers\"}".into()));
        }
        header_count += 1;
        if header_count > MAX_HEADER_COUNT {
            return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Too many HTTP headers\"}".into()));
        }
        let (name, value) = line.split_once(':').ok_or_else(|| {
            (400, "{\"error\":\"BadRequest\",\"detail\":\"Malformed header line missing colon\"}".into())
        })?;
        let value = value.trim_matches([' ', '\t']);
        if !is_valid_header_name(name) || !is_valid_header_value(value) {
            return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Invalid HTTP header\"}".into()));
        }
        let lower_name = name.to_ascii_lowercase();
        if header_map.contains_key(&lower_name) {
            let detail = if lower_name == "content-length" {
                "Duplicate Content-Length header is forbidden"
            } else {
                "Duplicate HTTP header is forbidden"
            };
            return Err((400, format!("{{\"error\":\"BadRequest\",\"detail\":\"{}\"}}", detail)));
        }
        header_map.insert(lower_name, value.to_owned());
    }

    let has_content_length = header_map.contains_key("content-length");
    let has_transfer_encoding = header_map.contains_key("transfer-encoding");
    if has_transfer_encoding && has_content_length {
        return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Requests containing both Transfer-Encoding and Content-Length are forbidden\"}".into()));
    }
    if has_transfer_encoding {
        return Err((501, "{\"error\":\"NotImplemented\",\"detail\":\"Transfer-Encoding: chunked is not supported; use Content-Length\"}".into()));
    }

    let content_length = match header_map.get("content-length") {
        Some(value) if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            value.parse::<usize>().map_err(|_| {
                (400, "{\"error\":\"BadRequest\",\"detail\":\"Invalid Content-Length\"}".into())
            })?
        }
        Some(_) => {
            return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Invalid Content-Length\"}".into()));
        }
        None if method == "POST" => {
            return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Content-Length is required for request bodies\"}".into()));
        }
        None => 0,
    };
    if content_length > max_payload_bytes {
        return Err((413, "{\"error\":\"PayloadTooLarge\",\"detail\":\"Request body exceeds configured limit\"}".into()));
    }

    let prefetched_body = &request_bytes[header_end..];
    if prefetched_body.len() > content_length {
        return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Trailing bytes after request body\"}".into()));
    }
    let mut full_body = prefetched_body.to_vec();
    while full_body.len() < content_length {
        let remaining = content_length - full_body.len();
        let mut chunk = vec![0u8; remaining.min(4096)];
        let count = match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => count,
            Err(ref error) if error.kind() == std::io::ErrorKind::TimedOut || error.kind() == std::io::ErrorKind::WouldBlock => {
                return Err((408, "{\"error\":\"RequestTimeout\",\"detail\":\"Timed out waiting for request body\"}".into()));
            }
            Err(_) => {
                return Err((400, "{\"error\":\"BadRequest\",\"detail\":\"Unable to read request body\"}".into()));
            }
        };
        full_body.extend_from_slice(&chunk[..count]);
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

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
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
