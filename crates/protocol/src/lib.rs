//! Decoder for the Kurzhaar wire format v1 (revision 1.2).
//!
//! One event per line, fields as `key::value` joined by `|`. The first line is the batch
//! header. See `contracts/protocol/vectors.json` for the conformance vectors.

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub ts: u64,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    pub props: BTreeMap<String, String>,
    /// `g` tags (revision 1.2).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// `y::3` engagement: engaged time in ms (`et`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "engagedMs")]
    pub engaged_ms: Option<u32>,
    /// `y::3` engagement: maximum scroll percent (`sc`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "scrollPct")]
    pub scroll_pct: Option<u8>,
    /// `y::4` performance: Web Vitals (ms, `cls` unitless).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lcp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cls: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fcp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttfb: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Batch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    /// `tv`: version of the tracker script (revision 1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "trackerVersion")]
    pub tracker_version: Option<String>,
    /// `tz`: visitor offset from UTC in minutes, negative west of UTC (revision 1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "tzOffset")]
    pub tz_offset: Option<i32>,
    /// Device signals (revision 1.3, ADR-ZKD1): raw input for server-side detection, never stored.
    #[serde(rename = "clientBrands", skip_serializing_if = "Vec::is_empty")]
    pub client_brands: Vec<String>,
    #[serde(rename = "clientMobile", skip_serializing_if = "Option::is_none")]
    pub client_mobile: Option<u8>,
    #[serde(rename = "clientPlatform", skip_serializing_if = "Option::is_none")]
    pub client_platform: Option<String>,
    #[serde(
        rename = "clientPlatformVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub client_platform_version: Option<String>,
    #[serde(rename = "touchPoints", skip_serializing_if = "Option::is_none")]
    pub touch_points: Option<u32>,
    #[serde(rename = "coarsePointer", skip_serializing_if = "Option::is_none")]
    pub coarse_pointer: Option<u8>,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Empty,
    UnsupportedVersion(String),
    BadNumber(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Empty => write!(f, "empty batch"),
            DecodeError::UnsupportedVersion(v) => write!(f, "unsupported protocol version: {v}"),
            DecodeError::BadNumber(v) => write!(f, "invalid number: {v}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Reverses `%XX` escaping. Invalid sequences are kept as written.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Splits a line into fields. Values stay escaped, so a list field can be split before it is
/// unescaped.
fn parse_line(line: &str) -> Vec<(String, String)> {
    line.split('|')
        .filter_map(|part| {
            part.find("::")
                .filter(|&i| i > 0)
                .map(|i| (part[..i].to_string(), part[i + 2..].to_string()))
        })
        .collect()
}

/// A list field: split on commas first, then unescape each item, so `%2C` stays inside an item.
fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter(|t| !t.is_empty())
        .map(unescape)
        .collect()
}

fn num<T: std::str::FromStr>(v: &str) -> Result<T, DecodeError> {
    v.parse().map_err(|_| DecodeError::BadNumber(v.to_string()))
}

pub fn decode(body: &str) -> Result<Batch, DecodeError> {
    let mut lines = body.split('\n');
    let head = parse_line(lines.next().ok_or(DecodeError::Empty)?);
    let raw = |k: &str| head.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let get = |k: &str| raw(k).map(unescape);
    match get("v").as_deref() {
        Some("1") => {}
        other => {
            return Err(DecodeError::UnsupportedVersion(
                other.unwrap_or("").to_string(),
            ))
        }
    }

    let mut events = Vec::new();
    let mut t: u64 = 0;
    for line in lines {
        let fields = parse_line(line);
        let find = |k: &str| fields.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        let text = |k: &str| find(k).map(unescape);
        t = match find("ts") {
            Some(ts) => num(ts)?,
            None => t + find("d").map(num::<u64>).transpose()?.unwrap_or(0),
        };
        let mut props = BTreeMap::new();
        for (k, v) in &fields {
            if let Some(name) = k.strip_prefix("p.") {
                props.insert(unescape(name), unescape(v));
            }
        }
        events.push(Event {
            ts: t,
            kind: find("y").map(num::<u8>).transpose()?.unwrap_or(0),
            name: text("n"),
            url: text("u"),
            title: text("t"),
            referrer: text("r"),
            props,
            tags: find("g").map(split_list).unwrap_or_default(),
            engaged_ms: find("et").map(num::<u32>).transpose()?,
            scroll_pct: find("sc").map(num::<u8>).transpose()?,
            lcp: find("lcp").map(num::<u32>).transpose()?,
            inp: find("inp").map(num::<u32>).transpose()?,
            cls: find("cls").map(num::<f64>).transpose()?,
            fcp: find("fcp").map(num::<u32>).transpose()?,
            ttfb: find("ttfb").map(num::<u32>).transpose()?,
        });
    }

    Ok(Batch {
        key: get("k"),
        screen: get("s"),
        lang: get("l"),
        identity: get("i"),
        tracker_version: get("tv"),
        tz_offset: raw("tz").map(num::<i32>).transpose()?,
        client_brands: raw("cb").map(split_list).unwrap_or_default(),
        client_mobile: raw("cm").map(num::<u8>).transpose()?,
        client_platform: get("cp"),
        client_platform_version: get("cv"),
        touch_points: raw("tp").map(num::<u32>).transpose()?,
        coarse_pointer: raw("pc").map(num::<u8>).transpose()?,
        events,
    })
}
