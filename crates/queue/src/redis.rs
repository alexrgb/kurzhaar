// spec: REQ-QUEUE-GE19, REQ-QUEUE-1PF3
//! The Redis Streams backend (ADR-9FWC). It only writes: `XADD` and `PING`. It never trims
//! or deletes; that is left to the consumers' retention job.

use crate::{BoxFuture, Queue, QueueError};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use std::time::Duration;
use tokio::sync::OnceCell;

/// A write that takes longer than this fails and is retried by the client (REQ-QUEUE-GE19).
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
const PING_TIMEOUT: Duration = Duration::from_secs(1);

/// The name of one shard's stream.
pub fn stream_name(prefix: &str, shard: u32) -> String {
    format!("{prefix}:{shard}")
}

pub struct RedisQueue {
    client: redis::Client,
    prefix: String,
    shards: u32,
    /// Connected on first use, so a collector that starts before Redis is simply not ready.
    connection: OnceCell<ConnectionManager>,
}

impl RedisQueue {
    /// Checks the address and the shard count; no connection is made yet. An `rediss://` URL
    /// turns on TLS with the server's certificate verified.
    pub fn new(url: &str, prefix: &str, shards: u32) -> Result<RedisQueue, QueueError> {
        if shards == 0 {
            return Err(QueueError("the number of shards must be at least 1".into()));
        }
        // The error text of a bad URL is not passed on: it could repeat the password.
        let client = redis::Client::open(url)
            .map_err(|_| QueueError("the Redis URL is not valid".into()))?;
        Ok(RedisQueue {
            client,
            prefix: prefix.to_string(),
            shards,
            connection: OnceCell::new(),
        })
    }

    /// The stream a partition key is written to: the first four bytes of its BLAKE3 hash,
    /// modulo the number of shards (REQ-QUEUE-1PF3). Nothing but the key decides it.
    pub fn stream_for(&self, partition_key: &str) -> String {
        let hash = blake3::hash(partition_key.as_bytes());
        let n = u32::from_le_bytes(hash.as_bytes()[..4].try_into().expect("four bytes"));
        stream_name(&self.prefix, n % self.shards)
    }

    async fn connection(&self) -> Result<ConnectionManager, QueueError> {
        self.connection
            .get_or_try_init(|| async {
                let config = ConnectionManagerConfig::new()
                    .set_connection_timeout(Some(WRITE_TIMEOUT))
                    .set_response_timeout(Some(WRITE_TIMEOUT));
                ConnectionManager::new_with_config(self.client.clone(), config)
                    .await
                    .map_err(|e| QueueError(format!("redis: {e}")))
            })
            .await
            .cloned()
    }
}

impl Queue for RedisQueue {
    fn publish<'a>(
        &'a self,
        partition_key: &'a str,
        payload: Vec<u8>,
    ) -> BoxFuture<'a, Result<(), QueueError>> {
        Box::pin(async move {
            let stream = self.stream_for(partition_key);
            let write = async {
                let mut connection = self.connection().await?;
                // No MAXLEN: a full Redis refuses the write instead of dropping old messages.
                redis::cmd("XADD")
                    .arg(&stream)
                    .arg("*")
                    .arg("payload")
                    .arg(&payload[..])
                    .arg("key")
                    .arg(partition_key)
                    .query_async::<String>(&mut connection)
                    .await
                    .map(|_| ())
                    .map_err(|e| QueueError(format!("redis: {e}")))
            };
            tokio::time::timeout(WRITE_TIMEOUT, write)
                .await
                .map_err(|_| QueueError("redis: the write timed out".into()))?
        })
    }

    fn ping(&self) -> BoxFuture<'_, Result<(), QueueError>> {
        Box::pin(async move {
            let check = async {
                let mut connection = self.connection().await?;
                redis::cmd("PING")
                    .query_async::<String>(&mut connection)
                    .await
                    .map(|_| ())
                    .map_err(|e| QueueError(format!("redis: {e}")))
            };
            tokio::time::timeout(PING_TIMEOUT, check)
                .await
                .map_err(|_| QueueError("redis: no answer to PING".into()))?
        })
    }
}
