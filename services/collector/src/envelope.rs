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

pub fn build(
    batch: Batch,
    site: &str,
    token: String,
    received_at: u64,
    detection: Detection,
) -> Envelope {
    let times = event_times(&batch.events, received_at);
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
        detection,
        events,
    }
}
