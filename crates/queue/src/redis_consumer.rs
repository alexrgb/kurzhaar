// spec: REQ-INGEST-J5R5, REQ-INGEST-XS9C, REQ-QUEUE-YATM
//! The Redis Streams consumer (ADR-HB8W): one group per sink, pending entries first, takeover of
//! entries a dead instance left behind, a dead-letter stream and the retention job.

use crate::redis::stream_name;
use crate::{BoxFuture, Consumer, Delivery, QueueError};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use redis::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

/// An entry another instance has held longer than this is taken over (REQ-INGEST-J5R5.S4).
pub const CLAIM_IDLE: Duration = Duration::from_secs(60);
/// The dead-letter stream keeps this many entries.
pub const DEAD_LETTER_CAP: usize = 10_000;
const CLAIM_EVERY: Duration = Duration::from_secs(10);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    /// The same prefix and shard count the collector writes with.
    pub prefix: String,
    pub shards: u32,
    /// The group of the sink, for example `kz-raw`.
    pub group: String,
    /// The name of this instance inside the group.
    pub consumer: String,
    /// The stream for messages that are set aside, for example `kurzhaar:dead:raw`.
    pub dead_stream: String,
    pub claim_idle: Duration,
}

pub struct RedisConsumer {
    client: redis::Client,
    config: ConsumerConfig,
    streams: Vec<String>,
    /// Used for commands that return at once.
    commands: OnceCell<ConnectionManager>,
    /// Used for the blocking read, so it never holds up an acknowledgement.
    reader: OnceCell<ConnectionManager>,
    /// Set when no entry is pending for this consumer any more.
    pending_done: AtomicBool,
    last_claim: Mutex<Option<Instant>>,
}

impl RedisConsumer {
    pub fn new(url: &str, config: ConsumerConfig) -> Result<RedisConsumer, QueueError> {
        if config.shards == 0 {
            return Err(QueueError("the number of shards must be at least 1".into()));
        }
        let client = redis::Client::open(url)
            .map_err(|_| QueueError("the Redis URL is not valid".into()))?;
        let streams = (0..config.shards)
            .map(|n| stream_name(&config.prefix, n))
            .collect();
        Ok(RedisConsumer {
            client,
            config,
            streams,
            commands: OnceCell::new(),
            reader: OnceCell::new(),
            pending_done: AtomicBool::new(false),
            last_claim: Mutex::new(None),
        })
    }

    async fn connect(&self, response: Duration) -> Result<ConnectionManager, QueueError> {
        let config = ConnectionManagerConfig::new()
            .set_connection_timeout(Some(COMMAND_TIMEOUT))
            .set_response_timeout(Some(response));
        ConnectionManager::new_with_config(self.client.clone(), config)
            .await
            .map_err(err)
    }

    /// Connects and creates the group on every stream, from the start of the stream so entries
    /// written before the consumer first ran are not skipped.
    async fn commands(&self) -> Result<ConnectionManager, QueueError> {
        self.commands
            .get_or_try_init(|| async {
                let mut c = self.connect(COMMAND_TIMEOUT).await?;
                for stream in &self.streams {
                    let created: Result<String, _> = redis::cmd("XGROUP")
                        .arg("CREATE")
                        .arg(stream)
                        .arg(&self.config.group)
                        .arg("0")
                        .arg("MKSTREAM")
                        .query_async(&mut c)
                        .await;
                    match created {
                        Ok(_) => {}
                        Err(e) if e.code() == Some("BUSYGROUP") => {}
                        Err(e) => return Err(err(e)),
                    }
                }
                Ok(c)
            })
            .await
            .cloned()
    }

    async fn reader(&self, block: Duration) -> Result<ConnectionManager, QueueError> {
        self.reader
            .get_or_try_init(|| self.connect(block + COMMAND_TIMEOUT))
            .await
            .cloned()
    }

    /// `XREADGROUP` on every stream from `position` (`0` for pending, `>` for new).
    async fn xreadgroup(
        &self,
        conn: &mut ConnectionManager,
        max: usize,
        block: Option<Duration>,
        position: &str,
    ) -> Result<Vec<Delivery>, QueueError> {
        let mut cmd = redis::cmd("XREADGROUP");
        cmd.arg("GROUP")
            .arg(&self.config.group)
            .arg(&self.config.consumer)
            .arg("COUNT")
            .arg(max);
        if let Some(block) = block {
            cmd.arg("BLOCK").arg(block.as_millis() as u64);
        }
        cmd.arg("STREAMS");
        for s in &self.streams {
            cmd.arg(s);
        }
        for _ in &self.streams {
            cmd.arg(position);
        }
        let reply: Value = cmd.query_async(conn).await.map_err(err)?;
        let mut out = Vec::new();
        let Value::Array(streams) = reply else {
            return Ok(out);
        };
        for stream in streams {
            let Value::Array(pair) = stream else { continue };
            let [name, entries] = <[Value; 2]>::try_from(pair).map_err(|_| bad_reply())?;
            let name = text(&name).ok_or_else(bad_reply)?;
            out.extend(self.entries(&name, entries, conn).await?);
        }
        Ok(out)
    }

    /// Turns a list of `[id, fields]` into deliveries. An entry that was trimmed away while it
    /// was pending comes back without fields; it is acknowledged and dropped.
    async fn entries(
        &self,
        stream: &str,
        list: Value,
        conn: &mut ConnectionManager,
    ) -> Result<Vec<Delivery>, QueueError> {
        let Value::Array(list) = list else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for entry in list {
            let Value::Array(pair) = entry else { continue };
            let [id, fields] = <[Value; 2]>::try_from(pair).map_err(|_| bad_reply())?;
            let id = text(&id).ok_or_else(bad_reply)?;
            let Value::Array(fields) = fields else {
                self.ack_ids(conn, stream, std::slice::from_ref(&id))
                    .await?;
                continue;
            };
            let mut payload = Vec::new();
            let mut key = String::new();
            for pair in fields.chunks(2) {
                if let [Value::BulkString(name), Value::BulkString(value)] = pair {
                    match name.as_slice() {
                        b"payload" => payload = value.clone(),
                        b"key" => key = String::from_utf8_lossy(value).into_owned(),
                        _ => {}
                    }
                }
            }
            out.push(Delivery {
                stream: stream.to_string(),
                id,
                partition_key: key,
                payload,
                attempt: 1,
            });
        }
        Ok(out)
    }

    /// Fills in how often each message has been delivered, for messages that were read again.
    async fn count_attempts(
        &self,
        conn: &mut ConnectionManager,
        deliveries: &mut [Delivery],
    ) -> Result<(), QueueError> {
        for stream in &self.streams {
            let ids: Vec<&str> = deliveries
                .iter()
                .filter(|d| &d.stream == stream)
                .map(|d| d.id.as_str())
                .collect();
            let (Some(first), Some(last)) = (ids.iter().min(), ids.iter().max()) else {
                continue;
            };
            let rows: Vec<(String, String, u64, u32)> = redis::cmd("XPENDING")
                .arg(stream)
                .arg(&self.config.group)
                .arg(*first)
                .arg(*last)
                .arg(ids.len().max(1) * 4)
                .query_async(conn)
                .await
                .map_err(err)?;
            let counts: HashMap<String, u32> =
                rows.into_iter().map(|(id, _, _, n)| (id, n)).collect();
            for d in deliveries.iter_mut().filter(|d| &d.stream == stream) {
                if let Some(n) = counts.get(&d.id) {
                    d.attempt = *n;
                }
            }
        }
        Ok(())
    }

    /// Takes over entries an instance has left idle for longer than `claim_idle`.
    async fn claim_idle(
        &self,
        conn: &mut ConnectionManager,
        max: usize,
    ) -> Result<Vec<Delivery>, QueueError> {
        let mut out = Vec::new();
        for stream in &self.streams {
            let reply: Value = redis::cmd("XAUTOCLAIM")
                .arg(stream)
                .arg(&self.config.group)
                .arg(&self.config.consumer)
                .arg(self.config.claim_idle.as_millis() as u64)
                .arg("0-0")
                .arg("COUNT")
                .arg(max)
                .query_async(conn)
                .await
                .map_err(err)?;
            let Value::Array(parts) = reply else { continue };
            if let Some(list) = parts.into_iter().nth(1) {
                out.extend(self.entries(stream, list, conn).await?);
            }
        }
        self.count_attempts(conn, &mut out).await?;
        Ok(out)
    }

    async fn ack_ids(
        &self,
        conn: &mut ConnectionManager,
        stream: &str,
        ids: &[String],
    ) -> Result<(), QueueError> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut cmd = redis::cmd("XACK");
        cmd.arg(stream).arg(&self.config.group);
        for id in ids {
            cmd.arg(id);
        }
        cmd.query_async::<i64>(conn).await.map(|_| ()).map_err(err)
    }

    /// Removes entries older than `retention` that every group has finished with (REQ-QUEUE-YATM).
    /// It never goes past the oldest entry that is still pending for some group, nor past what a
    /// group has not read yet. Returns the number of entries removed.
    pub async fn trim(&self, retention: Duration, now_ms: u64) -> Result<usize, QueueError> {
        let mut conn = self.commands().await?;
        let cutoff = now_ms.saturating_sub(retention.as_millis() as u64);
        let mut removed = 0;
        for stream in &self.streams {
            let groups: Vec<Value> = redis::cmd("XINFO")
                .arg("GROUPS")
                .arg(stream)
                .query_async(&mut conn)
                .await
                .map_err(err)?;
            let mut limit = (cutoff, 0u64);
            for group in groups {
                let info = info_map(group);
                let name = info.get("name").and_then(text).ok_or_else(bad_reply)?;
                // Entries up to the last delivered id were handed out. With none pending
                // all of them are acknowledged; otherwise nothing from the oldest pending one on
                // may go.
                let last = info
                    .get("last-delivered-id")
                    .and_then(text)
                    .and_then(|id| parse_id(&id))
                    .ok_or_else(bad_reply)?;
                let summary: Value = redis::cmd("XPENDING")
                    .arg(stream)
                    .arg(&name)
                    .query_async(&mut conn)
                    .await
                    .map_err(err)?;
                let oldest = match &summary {
                    Value::Array(s) => s.get(1).and_then(text).and_then(|id| parse_id(&id)),
                    _ => None,
                };
                limit = limit.min(oldest.unwrap_or((last.0, last.1 + 1)));
            }
            let n: usize = redis::cmd("XTRIM")
                .arg(stream)
                .arg("MINID")
                .arg(format!("{}-{}", limit.0, limit.1))
                .query_async(&mut conn)
                .await
                .map_err(err)?;
            removed += n;
        }
        Ok(removed)
    }
}

impl Consumer for RedisConsumer {
    fn read(
        &self,
        max: usize,
        block: Duration,
    ) -> BoxFuture<'_, Result<Vec<Delivery>, QueueError>> {
        Box::pin(async move {
            let mut conn = self.commands().await?;
            // Entries this consumer already holds come first, so a restart finishes its work.
            if !self.pending_done.load(Ordering::SeqCst) {
                let mut pending = self.xreadgroup(&mut conn, max, None, "0").await?;
                if pending.is_empty() {
                    self.pending_done.store(true, Ordering::SeqCst);
                } else {
                    self.count_attempts(&mut conn, &mut pending).await?;
                    return Ok(pending);
                }
            }
            let due = {
                let mut last = self.last_claim.lock().expect("claim lock");
                let due = last.is_none_or(|t| t.elapsed() >= CLAIM_EVERY);
                if due {
                    *last = Some(Instant::now());
                }
                due
            };
            if due {
                let claimed = self.claim_idle(&mut conn, max).await?;
                if !claimed.is_empty() {
                    return Ok(claimed);
                }
            }
            let mut reader = self.reader(block).await?;
            self.xreadgroup(&mut reader, max, Some(block), ">").await
        })
    }

    fn ack<'a>(&'a self, deliveries: &'a [Delivery]) -> BoxFuture<'a, Result<(), QueueError>> {
        Box::pin(async move {
            let mut conn = self.commands().await?;
            for stream in &self.streams {
                let ids: Vec<String> = deliveries
                    .iter()
                    .filter(|d| &d.stream == stream)
                    .map(|d| d.id.clone())
                    .collect();
                self.ack_ids(&mut conn, stream, &ids).await?;
            }
            Ok(())
        })
    }

    fn dead_letter<'a>(
        &'a self,
        delivery: &'a Delivery,
        reason: &'a str,
    ) -> BoxFuture<'a, Result<(), QueueError>> {
        Box::pin(async move {
            let mut conn = self.commands().await?;
            redis::cmd("XADD")
                .arg(&self.config.dead_stream)
                .arg("MAXLEN")
                .arg(DEAD_LETTER_CAP)
                .arg("*")
                .arg("payload")
                .arg(&delivery.payload[..])
                .arg("key")
                .arg(&delivery.partition_key)
                .arg("reason")
                .arg(reason)
                .arg("stream")
                .arg(&delivery.stream)
                .arg("id")
                .arg(&delivery.id)
                .query_async::<String>(&mut conn)
                .await
                .map_err(err)?;
            self.ack_ids(
                &mut conn,
                &delivery.stream,
                std::slice::from_ref(&delivery.id),
            )
            .await
        })
    }
}

fn err(e: redis::RedisError) -> QueueError {
    QueueError(format!("redis: {e}"))
}

fn bad_reply() -> QueueError {
    QueueError("redis: unexpected reply".into())
}

fn text(v: &Value) -> Option<String> {
    match v {
        Value::BulkString(b) => String::from_utf8(b.clone()).ok(),
        Value::SimpleString(s) => Some(s.clone()),
        _ => None,
    }
}

fn parse_id(id: &str) -> Option<(u64, u64)> {
    let (ms, seq) = id.split_once('-')?;
    Some((ms.parse().ok()?, seq.parse().ok()?))
}

/// `XINFO GROUPS` rows come as a flat list of names and values (or as a map).
fn info_map(v: Value) -> HashMap<String, Value> {
    match v {
        Value::Array(items) => items
            .chunks(2)
            .filter_map(|p| match p {
                [k, v] => Some((text(k)?, v.clone())),
                _ => None,
            })
            .collect(),
        Value::Map(pairs) => pairs
            .into_iter()
            .filter_map(|(k, v)| Some((text(&k)?, v)))
            .collect(),
        _ => HashMap::new(),
    }
}
