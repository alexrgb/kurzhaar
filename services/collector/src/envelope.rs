// spec: REQ-COLLECT-X0AM, REQ-COLLECT-8J26, REQ-COLLECT-J4S5, REQ-COLLECT-SD26
//! The message the collector writes to the queue, one per accepted batch (ADR-K8N9, ADR-D3BB).
//! It carries no address, User-Agent, Client Hints or raw tracker signals.

use kurzhaar_core::detect::Detection;
use kurzhaar_protocol::{Batch, Event};
use serde::Serialize;

/// Bumped on a change consumers cannot ignore.
pub const ENVELOPE_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
pub struct Envelope {
    pub v: u32,
    pub token: String,
    pub site: String,
    pub received_at: u64,
    pub tracker_version: Option<String>,
    pub tz_offset: Option<i32>,
    pub screen: Option<String>,
    pub lang: Option<String>,
    pub identity: Option<String>,
    /// Session hash, also the queue partition key (REQ-COLLECT-G8T0).
    pub session: String,
    /// ISO 3166-1 alpha-2 code from the country database, empty when unknown (REQ-COLLECT-82NX).
    pub country: Option<String>,
    /// Browser, OS, device class and bot flag, names and major versions only.
    #[serde(flatten)]
    pub detection: Detection,
    /// Events with `ts` replaced by the server-derived time (REQ-COLLECT-8J26).
    pub events: Vec<Event>,
}

pub const MAX_TAGS: usize = 10;
pub const MAX_TAG_CHARS: usize = 32;

/// BLAKE3 over the site key and the exact body bytes, 128 bits as hex (REQ-COLLECT-X0AM).
pub fn token(site_key: &str, body: &[u8]) -> String {
    let mut h = blake3::Hasher::new();
    h.update(site_key.as_bytes());
    h.update(&[0]);
    h.update(body);
    h.finalize().as_bytes()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The longest age the collector accepts for a batch: one hour (REQ-COLLECT-KSBB).
pub const MAX_BATCH_AGE_MS: u64 = 3_600_000;

/// The batch age from the `a` query parameter. A value that is missing, is not a whole number
/// of milliseconds or is out of range counts as 0; the batch is never rejected for it.
pub fn batch_age(query: Option<&str>) -> u64 {
    query
        .into_iter()
        .flat_map(|q| q.split('&'))
        .find_map(|pair| pair.strip_prefix("a="))
        .filter(|v| !v.is_empty() && v.len() <= 7 && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|a| *a <= MAX_BATCH_AGE_MS)
        .unwrap_or(0)
}

/// Event time from the receive time and the offsets inside the batch; the client clock is not
/// used (REQ-COLLECT-8J26).
fn event_times(events: &[Event], received_at: u64) -> Vec<u64> {
    let last = events.last().map_or(0, |e| e.ts);
    events
        .iter()
        .map(|e| received_at.saturating_sub(last.saturating_sub(e.ts)))
        .collect()
}

/// At most [`MAX_TAGS`] tags of [`MAX_TAG_CHARS`] characters; the rest is dropped (REQ-COLLECT-J4S5).
fn limit_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .take(MAX_TAGS)
        .map(|t| t.chars().take(MAX_TAG_CHARS).collect())
        .collect()
}

/// When a batch was received and how old its events were already, from the `a` parameter.
#[derive(Debug, Clone, Copy)]
pub struct Arrival {
    pub received_at: u64,
    pub age_ms: u64,
}

pub fn build(
    batch: Batch,
    site: &str,
    token: String,
    arrival: Arrival,
    detection: Detection,
    country: Option<String>,
    session: String,
) -> Envelope {
    let received_at = arrival.received_at;
    let times = event_times(&batch.events, received_at.saturating_sub(arrival.age_ms));
    let events = batch
        .events
        .into_iter()
        .zip(times)
        .map(|(mut e, t)| {
            e.ts = t;
            e.tags = limit_tags(std::mem::take(&mut e.tags));
            e
        })
        .collect();
    Envelope {
        v: ENVELOPE_VERSION,
        token,
        site: site.to_string(),
        received_at,
        tracker_version: batch.tracker_version,
        tz_offset: batch.tz_offset,
        screen: batch.screen,
        lang: batch.lang,
        identity: batch.identity,
        session,
        country,
        detection,
        events,
    }
}
