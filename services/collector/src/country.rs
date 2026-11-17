// spec: REQ-COLLECT-82NX
//! Country lookup from an MMDB file the operator provides (ADR-D3BB). Only the ISO 3166-1
//! alpha-2 country code is read, whatever the database holds. A replaced file is picked up
//! by [`CountryDb::reload_if_changed`]; a replacement that cannot be opened is ignored.

use arc_swap::ArcSwapOption;
use maxminddb::{path, Reader};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

struct Loaded {
    reader: Reader<Vec<u8>>,
    stamp: Stamp,
}

/// What identifies a version of the file without reading it.
#[derive(PartialEq, Eq, Clone, Copy)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

fn stamp(path: &Path) -> Result<Stamp, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Stamp {
        modified: meta.modified().ok(),
        len: meta.len(),
    })
}

fn load(path: &Path) -> Result<Loaded, String> {
    let stamp = stamp(path)?;
    let reader = Reader::open_readfile(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Loaded { reader, stamp })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reload {
    /// No path is configured or the file has not changed.
    Unchanged,
    Reloaded,
    /// The new file could not be opened; the previous database stays in use.
    Failed(String),
}

pub struct CountryDb {
    path: Option<PathBuf>,
    loaded: ArcSwapOption<Loaded>,
}

impl CountryDb {
    /// No database: every lookup gives no country (REQ-COLLECT-82NX scenario 1).
    pub fn disabled() -> CountryDb {
        CountryDb {
            path: None,
            loaded: ArcSwapOption::empty(),
        }
    }

    /// Opens the file; an unreadable or invalid file is an error naming the path.
    pub fn open(path: impl Into<PathBuf>) -> Result<CountryDb, String> {
        let path = path.into();
        let loaded = load(&path)?;
        Ok(CountryDb {
            path: Some(path),
            loaded: ArcSwapOption::new(Some(Arc::new(loaded))),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.path.is_some()
    }

    /// The country code of the address, or `None` when the address is not in the database,
    /// the record has no country, or no database is configured.
    pub fn lookup(&self, ip: IpAddr) -> Option<String> {
        let guard = self.loaded.load();
        let db = guard.as_ref()?;
        let found = db.reader.lookup(ip).ok()?;
        let code: Option<&str> = found.decode_path(&path!["country", "iso_code"]).ok()?;
        let code = code?;
        (code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())).then(|| code.to_string())
    }

    /// Opens the file again when its modification time or size changed. Lookups running
    /// meanwhile keep the old database until the new one is ready.
    pub fn reload_if_changed(&self) -> Reload {
        let Some(path) = &self.path else {
            return Reload::Unchanged;
        };
        let now = match stamp(path) {
            Ok(s) => s,
            Err(e) => return Reload::Failed(e),
        };
        if self.loaded.load().as_ref().is_some_and(|l| l.stamp == now) {
            return Reload::Unchanged;
        }
        match load(path) {
            Ok(l) => {
                self.loaded.store(Some(Arc::new(l)));
                Reload::Reloaded
            }
            Err(e) => Reload::Failed(e),
        }
    }
}
