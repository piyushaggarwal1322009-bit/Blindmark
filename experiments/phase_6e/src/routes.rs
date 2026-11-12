//! HTTP Route Handlers for Leader and Helper Aggregators.

use crate::config::ServiceConfig;
use crate::error::ServiceError;
use crate::http::{HttpRequest, HttpResponse};
use crate::state::{HelperState, LeaderState};
use blindmark_phase_6c::protocol::*;
use std::sync::Arc;

pub struct RouteHandler {
    config: ServiceConfig,
    leader_state: Option<Arc<LeaderState>>,
    helper_state: Option<Arc<HelperState>>,
}

impl RouteHandler {
    pub fn for_leader(config: ServiceConfig, state: Arc<LeaderState>) -> Self {
        Self {
            config,
            leader_state: Some(state),
            helper_state: None,
        }
    }

    pub fn for_helper(config: ServiceConfig, state: Arc<HelperState>) -> Self {
        Self {
            config,
            leader_state: None,
            helper_state: Some(state),
        }
    }

    pub fn handle(&self, req: &HttpRequest) -> HttpResponse {
        // Enforce maximum payload size bounds before processing
        if req.body.len() > self.config.max_payload_bytes {
            let (status, body) = ServiceError::PayloadTooLarge {
                max_bytes: self.config.max_payload_bytes,
                received_bytes: req.body.len(),
            }
            .to_http_status_and_body();
            return HttpResponse::from_status_and_body(status, body);
        }

        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/health") => self.handle_health(),
            ("GET", path) if path.starts_with("/tasks/") && path.ends_with("/status") => {
                self.handle_task_status(req, path)
            }
            ("POST", path) if path.starts_with("/tasks/") && path.contains("/reports/") => {
                self.handle_report_ingestion(req, path)
            }
            _ => {
                let (status, body) = ServiceError::NotFound(format!("Route {} not found", req.path))
                    .to_http_status_and_body();
                HttpResponse::from_status_and_body(status, body)
            }
        }
    }

    fn handle_health(&self) -> HttpResponse {
        let role = match self.config.aggregator_id {
            AggregatorId::Leader => "leader",
            AggregatorId::Helper => "helper",
        };
        // Operational log: minimal, zero sensitive key exposure
        let body = format!(
            "{{\"status\":\"healthy\",\"role\":\"{}\",\"aggregator_id\":{}}}",
            role,
            self.config.aggregator_id.index()
        );
        HttpResponse::ok_json(body)
    }

    fn handle_task_status(&self, req: &HttpRequest, path: &str) -> HttpResponse {
        // 1. Task ID Extraction and Validation
        let parts: Vec<&str> = path.split('/').collect();
        let task_id = if parts.len() >= 4 { parts[2] } else { "" };
        if let Err(err) = self.validate_task_id(task_id) {
            let (s, b) = err.to_http_status_and_body();
            return HttpResponse::from_status_and_body(s, b);
        }

        // 2. Authentication Verification
        if let Err(err) = self.verify_auth(req) {
            let (status, body) = err.to_http_status_and_body();
            return HttpResponse::from_status_and_body(status, body);
        }

        let accepted_count = if let Some(ref leader) = self.leader_state {
            leader.aggregator.lock().unwrap().accepted_count()
        } else if let Some(ref helper) = self.helper_state {
            helper.aggregator.lock().unwrap().accepted_count()
        } else {
            0
        };

        let body = format!(
            "{{\"task_id\":\"{}\",\"aggregator_id\":{},\"accepted_count\":{}}}",
            task_id,
            self.config.aggregator_id.index(),
            accepted_count
        );
        HttpResponse::ok_json(body)
    }

    fn handle_report_ingestion(&self, req: &HttpRequest, path: &str) -> HttpResponse {
        // 1. Task ID and Aggregator ID extraction from path: /tasks/{task_id}/reports/{agg_id}
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 5 {
            let (s, b) = ServiceError::BadRequest("Malformed route path format".into()).to_http_status_and_body();
            return HttpResponse::from_status_and_body(s, b);
        }

        let task_id = parts[2];
        if let Err(err) = self.validate_task_id(task_id) {
            let (s, b) = err.to_http_status_and_body();
            return HttpResponse::from_status_and_body(s, b);
        }

        let target_id_str = parts[4];
        let target_id: u8 = match target_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                let (s, b) = ServiceError::BadRequest("Invalid aggregator target ID in path".into())
                    .to_http_status_and_body();
                return HttpResponse::from_status_and_body(s, b);
            }
        };

        // 2. Strict Aggregator Identity Binding
        let expected_id = self.config.aggregator_id.index() as u8;
        if target_id != expected_id {
            let (s, b) = ServiceError::WrongAggregator {
                expected: expected_id,
                got: target_id,
            }
            .to_http_status_and_body();
            return HttpResponse::from_status_and_body(s, b);
        }

        // 3. Reject empty request bodies
        if req.body.is_empty() {
            let (s, b) = ServiceError::BadRequest("Request body cannot be empty".into())
                .to_http_status_and_body();
            return HttpResponse::from_status_and_body(s, b);
        }

        // 4. Honest Status Reporting: Phase 6E provides HTTP service scaffolding only.
        // Full cryptographic VDAF deserialization and multi-round verification are scheduled for Phase 6F.
        // Returning HTTP 501 Not Implemented prevents clients from falsely believing their share was aggregated.
        let (status, body) = ServiceError::NotImplemented(format!(
            "Cryptographic report ingestion is not yet connected in Phase 6E scaffolding. Submission for task '{}' at aggregator {} was not processed into aggregate state.",
            task_id, expected_id
        ))
        .to_http_status_and_body();

        HttpResponse::from_status_and_body(status, body)
    }

    fn validate_task_id(&self, task_id: &str) -> Result<(), ServiceError> {
        if task_id.is_empty() {
            return Err(ServiceError::InvalidTaskId("Task ID cannot be empty".into()));
        }
        if task_id.len() > 64 {
            return Err(ServiceError::InvalidTaskId("Task ID exceeds 64 characters limit".into()));
        }
        // Task ID must contain only alphanumeric, hyphen, or underscore characters
        if !task_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(ServiceError::InvalidTaskId(
                "Task ID must contain only ASCII alphanumeric characters, hyphens, or underscores".into(),
            ));
        }
        Ok(())
    }

    fn verify_auth(&self, req: &HttpRequest) -> Result<(), ServiceError> {
        let auth_hdr = req.headers.get("x-service-key").or_else(|| req.headers.get("authorization"));
        match auth_hdr {
            Some(key) if key == &self.config.auth_key || key == &format!("Bearer {}", self.config.auth_key) => Ok(()),
            _ => Err(ServiceError::Unauthorized("Invalid or missing X-Service-Key credential".into())),
        }
    }
}
