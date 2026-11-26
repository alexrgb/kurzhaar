//! Queue trait and the Redis, Kafka and RabbitMQ implementations (ADR-GTN9).
//!
//! The trait moves opaque bytes and a partition key; the envelope that goes through it belongs to
//! the collector (ADR-K8N9).

pub mod redis;
pub mod redis_consumer;

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

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

/// One message handed to a consumer, with what it needs to acknowledge or set it aside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub stream: String,
    pub id: String,
    pub partition_key: String,
    pub payload: Vec<u8>,
    /// How many times this message has been delivered to the group, this one included.
    pub attempt: u32,
}

/// The read side of the queue: one consumer group per sink (ADR-HB8W).
pub trait Consumer: Send + Sync {
    /// Returns up to `max` messages: entries still pending for this consumer first, then entries
    /// another instance left idle for too long, then new ones. Waits up to `block` for new ones.
    fn read(&self, max: usize, block: Duration)
        -> BoxFuture<'_, Result<Vec<Delivery>, QueueError>>;

    /// Acknowledges messages whose insert the sink has confirmed.
    fn ack<'a>(&'a self, deliveries: &'a [Delivery]) -> BoxFuture<'a, Result<(), QueueError>>;

    /// Moves a message that cannot be inserted to the dead-letter stream with the reason, and
    /// acknowledges it.
    fn dead_letter<'a>(
        &'a self,
        delivery: &'a Delivery,
        reason: &'a str,
    ) -> BoxFuture<'a, Result<(), QueueError>>;
}
