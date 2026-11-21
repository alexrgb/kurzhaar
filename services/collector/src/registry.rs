// spec: REQ-COLLECT-EWYP, REQ-COLLECT-THCX
//! The site registry the collector checks keys against (ADR-DW71). It is an immutable snapshot
//! replaced as a whole, so a request never waits for Postgres and never sees half a reload.

use arc_swap::ArcSwapOption;
use std::collections::HashMap;
use std::sync::Arc;

struct Snapshot {
    /// Site key to site id.
    sites: HashMap<String, String>,
    /// Receive time of the last successful load, in milliseconds.
    loaded_at: u64,
}

/// What a key lookup found.
#[derive(Debug, PartialEq, Eq)]
pub enum Lookup {
    Site(String),
    /// The registry is loaded and has no such key (unknown or disabled).
    Unknown,
    /// Nothing has been loaded yet; the batch must be retried, not dropped.
    Unavailable,
}

pub struct Registry {
    snapshot: ArcSwapOption<Snapshot>,
    /// `None`: a fixed list that never goes stale. `Some(ms)`: how long a snapshot counts as
    /// fresh after its last successful load.
    grace_ms: Option<u64>,
}

impl Registry {
    /// A fixed list, for development and single-node installs without Postgres.
    pub fn fixed(sites: HashMap<String, String>) -> Registry {
        Registry {
            snapshot: ArcSwapOption::new(Some(Arc::new(Snapshot {
                sites,
                loaded_at: 0,
            }))),
            grace_ms: None,
        }
    }

    /// An empty registry that a loader fills from Postgres.
    pub fn loaded_from_postgres(grace_ms: u64) -> Registry {
        Registry {
            snapshot: ArcSwapOption::empty(),
            grace_ms: Some(grace_ms),
        }
    }

    /// Replaces the snapshot with a freshly loaded one.
    pub fn replace(&self, sites: HashMap<String, String>, now_ms: u64) {
        self.snapshot.store(Some(Arc::new(Snapshot {
            sites,
            loaded_at: now_ms,
        })));
    }

    pub fn lookup(&self, key: &str) -> Lookup {
        match self.snapshot.load().as_ref() {
            None => Lookup::Unavailable,
            Some(s) => s
                .sites
                .get(key)
                .map_or(Lookup::Unknown, |id| Lookup::Site(id.clone())),
        }
    }

    /// Whether the collector may call itself ready: a snapshot exists and, for a registry
    /// read from Postgres, it was loaded within the grace period. A stale registry is still
    /// used for lookups (REQ-COLLECT-THCX).
    pub fn is_fresh(&self, now_ms: u64) -> bool {
        match (self.snapshot.load().as_ref(), self.grace_ms) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(s), Some(grace)) => now_ms.saturating_sub(s.loaded_at) <= grace,
        }
    }
}
