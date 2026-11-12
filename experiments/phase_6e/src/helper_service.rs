//! Helper Service runner with bounded concurrent worker handling.

use crate::config::ServiceConfig;
use crate::http::{parse_request, write_response};
use crate::routes::RouteHandler;
use crate::state::HelperState;
use blindmark_phase_6c::protocol::SEED_SIZE;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const MAX_CONCURRENT_CONNECTIONS: usize = 64;

/// RAII Guard that automatically decrements the active connection counter upon drop,
/// guaranteeing that panics, early returns, or errors never leak reserved slots.
pub struct ConnectionGuard {
    counter: Arc<AtomicUsize>,
}

impl ConnectionGuard {
    /// Attempts to atomically reserve a connection slot without TOCTOU race conditions.
    pub fn try_acquire(counter: &Arc<AtomicUsize>, max_limit: usize) -> Option<Self> {
        let mut current = counter.load(Ordering::SeqCst);
        loop {
            if current >= max_limit {
                return None;
            }
            match counter.compare_exchange_weak(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Some(Self { counter: Arc::clone(counter) }),
                Err(actual) => current = actual,
            }
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct HelperService {
    config: ServiceConfig,
    state: Arc<HelperState>,
    handler: Arc<RouteHandler>,
    is_running: Arc<AtomicBool>,
    active_connections: Arc<AtomicUsize>,
}

impl HelperService {
    pub fn new(config: ServiceConfig, verify_key: [u8; SEED_SIZE]) -> Result<Self, blindmark_phase_6c::protocol::ProtocolError> {
        let state = Arc::new(HelperState::new(verify_key)?);
        let handler = Arc::new(RouteHandler::for_helper(config.clone(), Arc::clone(&state)));
        Ok(Self {
            config,
            state,
            handler,
            is_running: Arc::new(AtomicBool::new(false)),
            active_connections: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn state(&self) -> Arc<HelperState> {
        Arc::clone(&self.state)
    }

    pub fn handler(&self) -> Arc<RouteHandler> {
        Arc::clone(&self.handler)
    }

    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Spawns the HTTP listener on a background thread with bounded concurrent connection dispatch.
    pub fn spawn(&self) -> std::io::Result<thread::JoinHandle<()>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr)?;
        listener.set_nonblocking(true)?;

        let handler = Arc::clone(&self.handler);
        let is_running = Arc::clone(&self.is_running);
        let active_connections = Arc::clone(&self.active_connections);
        let max_payload = self.config.max_payload_bytes;
        is_running.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            while is_running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Atomically reserve a connection slot before spawning any worker thread
                        let guard = match ConnectionGuard::try_acquire(&active_connections, MAX_CONCURRENT_CONNECTIONS) {
                            Some(g) => g,
                            None => {
                                // Reject connection if 64-connection limit is reached
                                let resp = crate::http::HttpResponse::from_status_and_body(
                                    503,
                                    "{\"error\":\"ServiceUnavailable\",\"detail\":\"Server busy: connection limit reached\"}".into(),
                                );
                                let _ = write_response(&mut stream, &resp);
                                continue;
                            }
                        };

                        let handler_clone = Arc::clone(&handler);

                        // Spawn worker; if thread spawning fails, guard drops immediately releasing slot
                        let spawn_res = thread::Builder::new()
                            .name("helper-worker".into())
                            .spawn(move || {
                                let _guard = guard; // Moved into thread; drops when thread finishes or panics
                                let _ = stream.set_nonblocking(false);
                                match parse_request(&mut stream, max_payload) {
                                    Ok(req) => {
                                        let resp = handler_clone.handle(&req);
                                        let _ = write_response(&mut stream, &resp);
                                    }
                                    Err((status, body)) => {
                                        let resp = crate::http::HttpResponse::from_status_and_body(status, body);
                                        let _ = write_response(&mut stream, &resp);
                                    }
                                }
                            });

                        if let Err(e) = spawn_res {
                            eprintln!("Failed to spawn worker thread: {}", e);
                            // guard will drop here if not moved, releasing slot
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(handle)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}
