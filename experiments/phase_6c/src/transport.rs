//! In-memory transport layer for inter-component communication.
//!
//! Provides a simple channel-based message bus that connects the Client,
//! Leader, Helper, and Collector without network deployment.
//!
//! # Security note
//!
//! This in-memory transport provides no confidentiality, integrity, or
//! authentication guarantees. It exists solely to demonstrate the message
//! interfaces between components. A production deployment would require
//! authenticated, encrypted channels (e.g., mutual TLS).

use crate::protocol::*;
use std::collections::VecDeque;

/// A simple in-memory mailbox for delivering messages between components.
#[derive(Debug, Default)]
pub struct Mailbox<T> {
    queue: VecDeque<T>,
}

impl<T> Mailbox<T> {
    pub fn new() -> Self {
        Mailbox {
            queue: VecDeque::new(),
        }
    }

    /// Send a message into the mailbox.
    pub fn send(&mut self, msg: T) {
        self.queue.push_back(msg);
    }

    /// Receive the next message, if any.
    pub fn recv(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    /// Check if the mailbox is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Number of pending messages.
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

/// The transport hub connecting all four components.
///
/// Each component has a dedicated inbox for receiving its messages.
/// Components send messages by writing to the recipient's inbox.
#[derive(Debug)]
pub struct TransportHub {
    /// Messages addressed to the Leader.
    pub to_leader: Mailbox<LeaderInbound>,
    /// Messages addressed to the Helper.
    pub to_helper: Mailbox<HelperInbound>,
    /// Messages addressed to the Collector.
    pub to_collector: Mailbox<CollectorInbound>,
}

impl TransportHub {
    pub fn new() -> Self {
        TransportHub {
            to_leader: Mailbox::new(),
            to_helper: Mailbox::new(),
            to_collector: Mailbox::new(),
        }
    }
}

/// Messages that the Leader can receive.
#[derive(Debug, Clone)]
pub enum LeaderInbound {
    /// A client report addressed to the Leader.
    ClientReport(ClientReport),
    /// A verifier share from the Helper.
    HelperVerifierShare(VerifierShareMessage),
    /// A collection request from the Collector.
    CollectionRequest(CollectionRequest),
    /// The Helper's aggregate share for collection.
    HelperAggregateShare(AggregateShareMessage),
}

/// Messages that the Helper can receive.
#[derive(Debug, Clone)]
pub enum HelperInbound {
    /// A client report addressed to the Helper.
    ClientReport(ClientReport),
    /// The combined verifier message from the Leader.
    VerifierMessage(VerifierMessageBroadcast),
    /// A request for the Helper's aggregate share.
    AggregateShareRequest { job_id: String },
}

/// Messages that the Collector can receive.
#[derive(Debug, Clone)]
pub enum CollectorInbound {
    /// The collection response from the Leader.
    CollectionResponse(CollectionResponse),
}

