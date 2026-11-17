//! Collector configuration: one TOML file, every key overridable by `KURZHAAR_COLLECTOR_<KEY>`
//! (ADR-K8N9). The environment wins. A bad value stops the start.

use ipnet::IpNet;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Site {
    pub key: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Public listener: `/v1/collect`, `/healthz`, `/readyz`.
    pub listen: String,
    /// Metrics listener, not meant to be published.
    pub metrics_listen: String,
    /// Proxy ranges whose `X-Forwarded-For` is believed (REQ-COLLECT-GZF5).
    pub trusted_proxies: Vec<String>,
    pub max_body_bytes: usize,
    pub max_events_per_batch: usize,
    /// Requests per minute per client address (REQ-COLLECT-51V3).
    pub rate_limit_per_minute: u32,
    pub rate_limit_burst: u32,
    /// Upper bound of tracked addresses.
    pub rate_limit_max_entries: usize,
    /// Detection results kept by the tuple of signals (REQ-COLLECT-4THR).
    pub detector_cache_entries: usize,
    pub detector_cache_seconds: u64,
    /// MMDB country database; unset means events carry no country (REQ-COLLECT-82NX).
    pub country_db: Option<String>,
    /// Sites known to this collector until the registry is read from Postgres.
    pub sites: Vec<Site>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen: "0.0.0.0:8080".into(),
            metrics_listen: "127.0.0.1:9090".into(),
            trusted_proxies: Vec::new(),
            max_body_bytes: 32 * 1024,
            max_events_per_batch: 50,
            rate_limit_per_minute: 300,
            rate_limit_burst: 60,
            rate_limit_max_entries: 100_000,
            detector_cache_entries: 10_000,
            detector_cache_seconds: 3600,
            country_db: None,
            sites: Vec::new(),
        }
    }
}

impl Config {
    /// Reads the optional file text, applies overrides from `env` and validates.
    pub fn load(
        file: Option<&str>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Config, String> {
        let mut c: Config = match file {
            Some(text) => toml::from_str(text).map_err(|e| format!("config file: {e}"))?,
            None => Config::default(),
        };
        let get = |k: &str| env(&format!("KURZHAAR_COLLECTOR_{k}"));
        if let Some(v) = get("LISTEN") {
            c.listen = v;
        }
        if let Some(v) = get("METRICS_LISTEN") {
            c.metrics_listen = v;
        }
        if let Some(v) = get("TRUSTED_PROXIES") {
            c.trusted_proxies = list(&v);
        }
        if let Some(v) = get("MAX_BODY_BYTES") {
            c.max_body_bytes = number("MAX_BODY_BYTES", &v)?;
        }
        if let Some(v) = get("MAX_EVENTS_PER_BATCH") {
            c.max_events_per_batch = number("MAX_EVENTS_PER_BATCH", &v)?;
        }
        if let Some(v) = get("RATE_LIMIT_PER_MINUTE") {
            c.rate_limit_per_minute = number("RATE_LIMIT_PER_MINUTE", &v)?;
        }
        if let Some(v) = get("RATE_LIMIT_BURST") {
            c.rate_limit_burst = number("RATE_LIMIT_BURST", &v)?;
        }
        if let Some(v) = get("DETECTOR_CACHE_ENTRIES") {
            c.detector_cache_entries = number("DETECTOR_CACHE_ENTRIES", &v)?;
        }
        if let Some(v) = get("DETECTOR_CACHE_SECONDS") {
            c.detector_cache_seconds = number("DETECTOR_CACHE_SECONDS", &v)?;
        }
        if let Some(v) = get("COUNTRY_DB") {
            c.country_db = Some(v).filter(|v| !v.is_empty());
        }
        if let Some(v) = get("SITES") {
            c.sites = sites(&v)?;
        }
        c.validate()?;
        Ok(c)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.listen
            .parse::<SocketAddr>()
            .map_err(|e| format!("listen `{}`: {e}", self.listen))?;
        self.metrics_listen
            .parse::<SocketAddr>()
            .map_err(|e| format!("metrics_listen `{}`: {e}", self.metrics_listen))?;
        self.trusted()?;
        for (name, value) in [
            ("max_body_bytes", self.max_body_bytes),
            ("max_events_per_batch", self.max_events_per_batch),
            ("rate_limit_per_minute", self.rate_limit_per_minute as usize),
            ("rate_limit_burst", self.rate_limit_burst as usize),
            ("rate_limit_max_entries", self.rate_limit_max_entries),
            ("detector_cache_entries", self.detector_cache_entries),
            (
                "detector_cache_seconds",
                self.detector_cache_seconds as usize,
            ),
        ] {
            if value == 0 {
                return Err(format!("{name} must be greater than zero"));
            }
        }
        if let Some(s) = self
            .sites
            .iter()
            .find(|s| s.key.is_empty() || s.id.is_empty())
        {
            return Err(format!("site `{}` needs a key and an id", s.id));
        }
        Ok(())
    }

    pub fn trusted(&self) -> Result<Vec<IpNet>, String> {
        self.trusted_proxies
            .iter()
            .map(|r| {
                r.parse::<IpNet>()
                    .or_else(|_| r.parse::<std::net::IpAddr>().map(IpNet::from))
                    .map_err(|_| {
                        format!("trusted proxy range `{r}` is not an address or a CIDR range")
                    })
            })
            .collect()
    }
}

fn list(v: &str) -> Vec<String> {
    v.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn number<T: std::str::FromStr>(name: &str, v: &str) -> Result<T, String> {
    v.trim()
        .parse()
        .map_err(|_| format!("{name} `{v}` is not a number"))
}

/// `key=id,key=id`
fn sites(v: &str) -> Result<Vec<Site>, String> {
    list(v)
        .into_iter()
        .map(|p| match p.split_once('=') {
            Some((k, i)) => Ok(Site {
                key: k.into(),
                id: i.into(),
            }),
            None => Err(format!("site `{p}` must be written key=id")),
        })
        .collect()
}
