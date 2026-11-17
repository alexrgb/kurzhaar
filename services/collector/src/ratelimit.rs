//! Token bucket per client address, in memory only (REQ-COLLECT-51V3).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;

const IDLE_MS: u64 = 10 * 60 * 1000;

struct Bucket {
    tokens: f64,
    last_ms: u64,
}

pub struct RateLimiter {
    per_ms: f64,
    burst: f64,
    max_entries: usize,
    buckets: Mutex<HashMap<IpAddr, Bucket>>,
}

impl RateLimiter {
    pub fn new(per_minute: u32, burst: u32, max_entries: usize) -> Self {
        RateLimiter {
            per_ms: f64::from(per_minute) / 60_000.0,
            burst: f64::from(burst),
            max_entries,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// `Ok` when the request may go on, otherwise the seconds to wait.
    pub fn check(&self, ip: IpAddr, now_ms: u64) -> Result<(), u64> {
        let mut map = self.buckets.lock().expect("limiter lock");
        if map.len() >= self.max_entries && !map.contains_key(&ip) {
            Self::sweep_locked(&mut map, now_ms);
            if map.len() >= self.max_entries {
                // Full of active addresses: do not track more, and do not turn the table into a
                // way to block everyone else.
                return Ok(());
            }
        }
        let b = map.entry(ip).or_insert(Bucket {
            tokens: self.burst,
            last_ms: now_ms,
        });
        let elapsed = now_ms.saturating_sub(b.last_ms) as f64;
        b.tokens = (b.tokens + elapsed * self.per_ms).min(self.burst);
        b.last_ms = now_ms;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            Ok(())
        } else {
            let wait_ms = (1.0 - b.tokens) / self.per_ms;
            Err((wait_ms / 1000.0).ceil().max(1.0) as u64)
        }
    }

    /// Removes addresses that have been idle for ten minutes.
    pub fn sweep(&self, now_ms: u64) {
        Self::sweep_locked(&mut self.buckets.lock().expect("limiter lock"), now_ms);
    }

    fn sweep_locked(map: &mut HashMap<IpAddr, Bucket>, now_ms: u64) {
        map.retain(|_, b| now_ms.saturating_sub(b.last_ms) < IDLE_MS);
    }

    pub fn len(&self) -> usize {
        self.buckets.lock().expect("limiter lock").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
