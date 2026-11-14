//! Decoder for the Kurzhaar wire format v1.
//!
//! One event per line, fields as `key::value` joined by `|`. The first line is the batch
//! header. See `spec/protocol/vectors.json` for the conformance vectors.

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Batch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
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

fn parse_line(line: &str) -> Vec<(String, String)> {
    line.split('|')
        .filter_map(|part| {
            part.find("::")
                .filter(|&i| i > 0)
                .map(|i| (part[..i].to_string(), unescape(&part[i + 2..])))
        })
        .collect()
}

fn num<T: std::str::FromStr>(v: &str) -> Result<T, DecodeError> {
    v.parse().map_err(|_| DecodeError::BadNumber(v.to_string()))
}

pub fn decode(body: &str) -> Result<Batch, DecodeError> {
    let mut lines = body.split('\n');
    let head = parse_line(lines.next().ok_or(DecodeError::Empty)?);
    let get = |k: &str| head.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
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
        t = match find("ts") {
            Some(ts) => num(ts)?,
            None => t + find("d").map(num::<u64>).transpose()?.unwrap_or(0),
        };
        let mut props = BTreeMap::new();
        for (k, v) in &fields {
            if let Some(name) = k.strip_prefix("p.") {
                props.insert(unescape(name), v.clone());
            }
        }
        events.push(Event {
            ts: t,
            kind: find("y").map(num::<u8>).transpose()?.unwrap_or(0),
            name: find("n").map(String::from),
            url: find("u").map(String::from),
            title: find("t").map(String::from),
            referrer: find("r").map(String::from),
            props,
        });
    }

    Ok(Batch {
        key: get("k"),
        screen: get("s"),
        lang: get("l"),
        identity: get("i"),
        events,
    })
}
