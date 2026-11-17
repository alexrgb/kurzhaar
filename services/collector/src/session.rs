// spec: REQ-COLLECT-G8T0, REQ-PRIVACY-E4TN
//! The session hash and the daily salts it depends on (ADR-D3BB). The hash lets visits be
//! counted without a cookie; once the day's salt is gone nobody can link the hash to a visitor.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::Mutex;
use zeroize::Zeroizing;

const DAY_MS: u64 = 86_400_000;
/// A salt is deleted this long after the end of its UTC day (REQ-PRIVACY-E4TN).
const GRACE_MS: u64 = 10 * 60 * 1000;

pub type Salt = Zeroizing<[u8; 32]>;

/// Gives the salt of the UTC day of a receive time. The Postgres source will implement this
/// too, so the hash does not change when it arrives.
pub trait SaltSource: Send + Sync {
    fn salt(&self, now_ms: u64) -> Salt;
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
