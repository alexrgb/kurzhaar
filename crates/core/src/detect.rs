// spec: REQ-COLLECT-X6WF, REQ-COLLECT-QZRJ, REQ-COLLECT-4THR, REQ-COLLECT-1AYR
//! Browser, operating system, device class and bot detection (ADR-ZKD1, ADR-1X7Z).
//!
//! The collector calls a [`Detector`] with the raw signals and keeps only the result. Client-hint
//! signals win over the User-Agent where both exist.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uaparser::{Parser, UserAgentParser};

/// Everything the detector may look at. It is the cache key, so it holds nothing else.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Signals {
    pub user_agent: String,
    /// Client-hint brands written `Brand/major`.
    pub brands: Vec<String>,
    pub mobile: Option<bool>,
    pub platform: Option<String>,
    /// Major version of the platform version only.
    pub platform_major: Option<u32>,
    pub touch_points: Option<u32>,
    pub coarse_pointer: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceClass {
    Desktop,
    Mobile,
    Tablet,
    Other,
}

/// Names and major versions only (REQ-COLLECT-QZRJ).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detection {
    pub browser: String,
    pub browser_major: Option<u32>,
    pub os: String,
    pub os_major: Option<u32>,
    pub device: DeviceClass,
    pub bot: bool,
}

pub trait Detector: Send + Sync {
    fn detect(&self, signals: &Signals) -> Detection;
}

/// The default detector: uap-core rules for the User-Agent, our own logic over the signals.
pub struct UaparserDetector {
    parser: UserAgentParser,
}

const UAP_CORE_RULES: &[u8] = include_bytes!("../data/uap-core/regexes.yaml");

impl UaparserDetector {
    pub fn builtin() -> Result<Self, String> {
        let parser = UserAgentParser::builder()
            .build_from_bytes(UAP_CORE_RULES)
            .map_err(|e| format!("cannot load the uap-core rules: {e}"))?;
        Ok(UaparserDetector { parser })
    }
}

fn major(v: Option<&str>) -> Option<u32> {
    v.and_then(|s| s.trim().parse().ok())
}

/// A brand written `Brand/major`: the major version follows the last slash.
fn split_brand(entry: &str) -> (&str, Option<u32>) {
    match entry.rsplit_once('/') {
        Some((name, v)) => (name, v.parse().ok()),
        None => (entry, None),
    }
}

/// Brands that name the product, in order of preference. `Chromium` and the random "Not A Brand"
/// entry say nothing the User-Agent does not.
fn hinted_browser(brands: &[String]) -> Option<(String, Option<u32>)> {
    const KNOWN: [(&str, &str); 4] = [
        ("Microsoft Edge", "Edge"),
        ("Opera", "Opera"),
        ("Samsung Internet", "Samsung Internet"),
        ("Google Chrome", "Chrome"),
    ];
    for (hint, name) in KNOWN {
        if let Some(b) = brands
            .iter()
            .map(|b| split_brand(b))
            .find(|(n, _)| *n == hint)
        {
            return Some((name.to_string(), b.1));
        }
    }
    None
}

fn normalize_browser(family: &str) -> String {
    match family {
        "Mobile Safari" | "Mobile Safari UI/WKWebView" | "Safari" => "Safari".into(),
        "Chrome Mobile" | "Chrome Mobile iOS" | "Chrome Mobile WebView" | "Chrome" => {
            "Chrome".into()
        }
        "Firefox Mobile" | "Firefox iOS" | "Firefox" => "Firefox".into(),
        "Edge Mobile" | "Edge" => "Edge".into(),
        "Opera Mobile" | "Opera" => "Opera".into(),
        "Other" => "Other".into(),
        other => other.into(),
    }
}

fn normalize_os(family: &str) -> String {
    match family {
        "Mac OS X" | "macOS" => "macOS".into(),
        "Windows" | "Windows NT" => "Windows".into(),
        "Android" => "Android".into(),
        "iOS" => "iOS".into(),
        "Chrome OS" | "Chromium OS" => "ChromeOS".into(),
        "Linux" | "Ubuntu" | "Fedora" | "Debian" | "Arch Linux" | "Mint" | "openSUSE"
        | "Gentoo" => "Linux".into(),
        "Other" => "Other".into(),
        other => other.into(),
    }
}

fn normalize_platform(platform: &str) -> String {
    match platform {
        "macOS" => "macOS".into(),
        "Windows" => "Windows".into(),
        "Android" => "Android".into(),
        "iOS" => "iOS".into(),
        "Chrome OS" | "Chromium OS" => "ChromeOS".into(),
        "Linux" => "Linux".into(),
        other => other.into(),
    }
}

/// Crawlers, headless browsers and command-line clients. Spiders known to uap-core are caught
/// separately by their device family.
fn looks_like_bot(ua: &str) -> bool {
    let l = ua.to_ascii_lowercase();
    if l.trim().is_empty() {
        return true;
    }
    const WORDS: [&str; 12] = [
        "crawler",
        "spider",
        "headless",
        "lighthouse",
        "pingdom",
        "uptimerobot",
        "curl/",
        "wget/",
        "python-requests",
        "go-http-client",
        "libwww",
        "phantomjs",
    ];
    if WORDS.iter().any(|w| l.contains(w)) {
        return true;
    }
    // "bot" as a word end ("Googlebot/2.1", "bingbot;", "SomeBot)"), not inside "Cubot".
    let b = l.as_bytes();
    l.match_indices("bot").any(|(i, _)| {
        let next = b.get(i + 3).copied();
        let ends = next.is_none_or(|c| !c.is_ascii_alphanumeric());
        ends && !l[..i + 3].ends_with("cubot")
    })
}

/// Televisions, consoles and similar: not a computer, a phone or a tablet.
fn is_other_device(l: &str) -> bool {
    [
        "smart-tv",
        "smarttv",
        "tizen",
        "webos",
        "playstation",
        "xbox",
        "nintendo",
        "appletv",
        "crkey",
        "hbbtv",
    ]
    .iter()
    .any(|w| l.contains(w))
}

impl Detector for UaparserDetector {
    fn detect(&self, s: &Signals) -> Detection {
        let client = self.parser.parse(&s.user_agent);
        let ua = s.user_agent.to_ascii_lowercase();
        let bot = client.device.family == "Spider" || looks_like_bot(&s.user_agent);

        // Browser: a product brand from the hints, else the User-Agent.
        let (browser, browser_major) = match hinted_browser(&s.brands) {
            Some(found) => found,
            None => (
                normalize_browser(&client.user_agent.family),
                major(client.user_agent.major.as_deref()),
            ),
        };

        // Operating system: the platform hint, else the User-Agent.
        let ua_os = normalize_os(&client.os.family);
        let mac_ua = ua.contains("macintosh");
        let ipad_desktop_mode =
            mac_ua && s.touch_points.unwrap_or(0) > 1 && s.coarse_pointer != Some(false);
        let mut os = s
            .platform
            .as_deref()
            .map_or(ua_os.clone(), normalize_platform);
        let mut os_major = major(client.os.major.as_deref());
        if ipad_desktop_mode {
            // iPadOS in desktop mode says Macintosh; its Safari version tracks the OS version.
            os = "iOS".into();
            os_major = major(client.user_agent.major.as_deref());
        } else if os == "Windows" {
            os_major = match s.platform_major {
                Some(v) if v >= 13 => Some(11),
                Some(_) => Some(10),
                None => os_major,
            };
        } else if os == "macOS" {
            // Safari freezes the version in the User-Agent at 10.15.
            os_major = s.platform_major.filter(|v| *v > 10);
        } else if s.platform_major.is_some() && s.platform.is_some() {
            os_major = s.platform_major;
        }

        // Device class.
        let family = client.device.family.to_ascii_lowercase();
        let touch = s.touch_points.unwrap_or(0);
        let device = if is_other_device(&ua) {
            DeviceClass::Other
        } else if ipad_desktop_mode
            || ua.contains("ipad")
            || family.contains("ipad")
            || ua.contains("tablet")
        {
            DeviceClass::Tablet
        } else if os == "Android" {
            match s.mobile {
                Some(true) => DeviceClass::Mobile,
                Some(false) if touch > 0 => DeviceClass::Tablet,
                _ if ua.contains("mobile") => DeviceClass::Mobile,
                _ => DeviceClass::Tablet,
            }
        } else if ua.contains("iphone")
            || ua.contains("ipod")
            || s.mobile == Some(true)
            || ua.contains("mobile")
        {
            DeviceClass::Mobile
        } else {
            DeviceClass::Desktop
        };

        Detection {
            browser,
            browser_major,
            os,
            os_major,
            device,
            bot,
        }
    }
}

/// Remembers results by the tuple of signals, so a seen tuple never reaches the detector again
/// (REQ-COLLECT-4THR). When full it forgets everything, which keeps the memory bound simple.
pub struct CachedDetector<D> {
    inner: D,
    max_entries: usize,
    ttl: Duration,
    cache: Mutex<HashMap<Signals, (Detection, Instant)>>,
}

impl<D: Detector> CachedDetector<D> {
    pub fn new(inner: D, max_entries: usize, ttl: Duration) -> Self {
        CachedDetector {
            inner,
            max_entries,
            ttl,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.cache.lock().expect("detector cache lock").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<D: Detector> Detector for CachedDetector<D> {
    fn detect(&self, signals: &Signals) -> Detection {
        if let Some((found, at)) = self.cache.lock().expect("detector cache lock").get(signals) {
            if at.elapsed() < self.ttl {
                return found.clone();
            }
        }
        let found = self.inner.detect(signals);
        let mut cache = self.cache.lock().expect("detector cache lock");
        if cache.len() >= self.max_entries {
            cache.clear();
        }
        cache.insert(signals.clone(), (found.clone(), Instant::now()));
        found
    }
}
