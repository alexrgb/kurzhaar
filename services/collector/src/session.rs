// spec: REQ-COLLECT-G8T0, REQ-PRIVACY-E4TN
//! The session hash and the daily salts it depends on (ADR-D3BB). The hash lets visits be
//! counted without a cookie; once the day's salt is gone nobody can link the hash to a visitor.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

const DAY_MS: u64 = 86_400_000;
/// A salt is deleted this long after the end of its UTC day (REQ-PRIVACY-E4TN).
const GRACE_MS: u64 = 10 * 60 * 1000;

pub type Salt = Zeroizing<[u8; 32]>;

/// Gives the salt of the UTC day of a receive time. The Postgres source will implement this
/// too, so the hash does not change when it arrives.
pub trait SaltSource: Send + Sync {
    fn salt(&self, now_ms: u64) -> Salt;

    /// The salt for an event that happened at `event_ms` and arrives at `now_ms`: the salt of the
    /// event's day while it is still held, otherwise the salt of today (REQ-COLLECT-KSBB). It
    /// never creates a salt for a day that is over.
    fn salt_at(&self, event_ms: u64, now_ms: u64) -> Salt;
}

/// Random salts kept in memory, one per UTC day. Suits one replica: another replica has other
/// salts and therefore other hashes.
#[derive(Default)]
pub struct MemorySalts {
    days: Mutex<BTreeMap<u64, Salt>>,
}

impl MemorySalts {
    pub fn new() -> MemorySalts {
        MemorySalts::default()
    }

    /// The UTC days whose salt is held now, for tests.
    pub fn held_days(&self) -> Vec<u64> {
        self.days.lock().expect("salts").keys().copied().collect()
    }
}

impl SaltSource for MemorySalts {
    fn salt(&self, now_ms: u64) -> Salt {
        let mut days = self.days.lock().expect("salts");
        // Dropping a `Zeroizing` value wipes its bytes.
        days.retain(|day, _| (day + 1) * DAY_MS + GRACE_MS > now_ms);
        let day = now_ms / DAY_MS;
        days.entry(day)
            .or_insert_with(|| {
                let mut bytes = Zeroizing::new([0u8; 32]);
                getrandom::fill(&mut bytes[..]).expect("the operating system's random generator");
                bytes
            })
            .clone()
    }

    fn salt_at(&self, event_ms: u64, now_ms: u64) -> Salt {
        let earlier = event_ms / DAY_MS;
        if earlier != now_ms / DAY_MS {
            let days = self.days.lock().expect("salts");
            if let Some(salt) = days
                .get(&earlier)
                .filter(|_| (earlier + 1) * DAY_MS + GRACE_MS > now_ms)
            {
                return salt.clone();
            }
        }
        self.salt(now_ms)
    }
}

/// HMAC-SHA-256 over the site id, the address and the User-Agent, each preceded by its length
/// so the fields cannot run into each other; 128 bits as 32 lowercase hex characters
/// (REQ-COLLECT-G8T0). An IPv4 address is used in its IPv4-mapped IPv6 form.
pub fn session_hash(salt: &Salt, site: &str, address: IpAddr, user_agent: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(&salt[..]).expect("any key length");
    let ip = match address {
        IpAddr::V4(v4) => v4.to_ipv6_mapped(),
        IpAddr::V6(v6) => v6,
    }
    .octets();
    for part in [site.as_bytes(), &ip[..], user_agent] {
        mac.update(&(part.len() as u32).to_be_bytes());
        mac.update(part);
    }
    mac.finalize().into_bytes()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Salts read from Postgres, with a random in-memory salt for a day that has none
/// (REQ-COLLECT-8ERQ, ADR-DW71).
pub struct PgSalts {
    loaded: Mutex<BTreeMap<u64, Salt>>,
    fallback: MemorySalts,
    /// Counts days that had to use the fallback.
    fallbacks: Arc<AtomicU64>,
    /// The last day the fallback was reported for, so it is logged and counted once a day.
    reported_day: AtomicU64,
}

impl PgSalts {
    pub fn new(fallbacks: Arc<AtomicU64>) -> PgSalts {
        PgSalts {
            loaded: Mutex::new(BTreeMap::new()),
            fallback: MemorySalts::new(),
            fallbacks,
            reported_day: AtomicU64::new(u64::MAX),
        }
    }

    /// Replaces the held salts with what Postgres has now; a salt that is gone there is
    /// wiped here.
    pub fn replace(&self, salts: Vec<(u64, Salt)>, now_ms: u64) {
        let mut loaded = self.loaded.lock().expect("salts");
        *loaded = salts.into_iter().collect();
        loaded.retain(|day, _| (day + 1) * DAY_MS + GRACE_MS > now_ms);
    }

    /// Drops salts that are more than ten minutes past their day.
    pub fn prune(&self, now_ms: u64) {
        let mut loaded = self.loaded.lock().expect("salts");
        loaded.retain(|day, _| (day + 1) * DAY_MS + GRACE_MS > now_ms);
    }

    /// The UTC days whose Postgres salt is held now, for tests.
    pub fn held_days(&self) -> Vec<u64> {
        self.loaded.lock().expect("salts").keys().copied().collect()
    }
}

impl SaltSource for PgSalts {
    fn salt(&self, now_ms: u64) -> Salt {
        let day = now_ms / DAY_MS;
        {
            let mut loaded = self.loaded.lock().expect("salts");
            loaded.retain(|d, _| (d + 1) * DAY_MS + GRACE_MS > now_ms);
            if let Some(salt) = loaded.get(&day) {
                return salt.clone();
            }
        }
        if self.reported_day.swap(day, Ordering::Relaxed) != day {
            self.fallbacks.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                day,
                "no salt in Postgres for today, using a random one in memory"
            );
        }
        self.fallback.salt(now_ms)
    }

    fn salt_at(&self, event_ms: u64, now_ms: u64) -> Salt {
        let earlier = event_ms / DAY_MS;
        if earlier != now_ms / DAY_MS {
            let loaded = self.loaded.lock().expect("salts");
            if let Some(salt) = loaded
                .get(&earlier)
                .filter(|_| (earlier + 1) * DAY_MS + GRACE_MS > now_ms)
            {
                return salt.clone();
            }
        }
        self.salt(now_ms)
    }
}
