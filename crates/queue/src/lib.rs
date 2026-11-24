//! Queue trait and the Redis, Kafka and RabbitMQ implementations (ADR-GTN9).
//!
//! The trait moves opaque bytes and a partition key; the envelope that goes through it belongs to
//! the collector (ADR-K8N9).

pub mod redis;

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueError(pub String);

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "queue error: {}", self.0)
    }
}

impl std::error::Error for QueueError {}

/// Where the collector writes accepted batches. A write that returns `Ok` is durable in the
/// sense of the chosen backend.
pub trait Queue: Send + Sync {
    /// Writes one message. Messages with the same partition key keep their order.
    fn publish<'a>(
        &'a self,
        partition_key: &'a str,
        payload: Vec<u8>,
    ) -> BoxFuture<'a, Result<(), QueueError>>;

    /// Reports whether the queue can take writes now, for readiness.
    fn ping(&self) -> BoxFuture<'_, Result<(), QueueError>>;
}

/// One message kept by [`MemoryQueue`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub partition_key: String,
    pub payload: Vec<u8>,
}

/// A queue in process memory, for tests and for running the collector without infrastructure.
#[derive(Default)]
pub struct MemoryQueue {
    messages: Mutex<Vec<Message>>,
    down: AtomicBool,
}

impl MemoryQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes every write and ping fail, as an unreachable queue would.
    pub fn set_down(&self, down: bool) {
        self.down.store(down, Ordering::SeqCst);
    }

    pub fn messages(&self) -> Vec<Message> {
        self.messages.lock().expect("queue lock").clone()
    }
}

impl Queue for MemoryQueue {
    fn publish<'a>(
        &'a self,
        partition_key: &'a str,
        payload: Vec<u8>,
    ) -> BoxFuture<'a, Result<(), QueueError>> {
        Box::pin(async move {
            if self.down.load(Ordering::SeqCst) {
                return Err(QueueError("memory queue is down".into()));
            }
            self.messages.lock().expect("queue lock").push(Message {
                partition_key: partition_key.to_string(),
                payload,
            });
            Ok(())
        })
    }

    fn ping(&self) -> BoxFuture<'_, Result<(), QueueError>> {
        Box::pin(async move {
            if self.down.load(Ordering::SeqCst) {
                Err(QueueError("memory queue is down".into()))
            } else {
                Ok(())
            }
        })
    }
}
