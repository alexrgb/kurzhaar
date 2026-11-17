// spec: REQ-COLLECT-X6WF, REQ-COLLECT-RA57
//! The detector's input, taken from the User-Agent header, the Client Hints headers the browser
//! sends by itself, and the tracker's signals in the batch header (ADR-ZKD1). The tracker's
//! values are used where both exist; the headers fill what it did not send.

use axum::http::HeaderMap;
use kurzhaar_core::detect::Signals;
use kurzhaar_protocol::Batch;

fn header<'a>(h: &'a HeaderMap, name: &str) -> Option<&'a str> {
    h.get(name).and_then(|v| v.to_str().ok())
}

/// `"Chromium";v="124", "Google Chrome";v="124"` becomes `Chromium/124`, `Google Chrome/124`.
pub fn parse_sec_ch_ua(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|part| {
            let (name, rest) = part.trim().split_once(';')?;
            let version = rest.trim().strip_prefix("v=")?;
            Some(format!(
                "{}/{}",
                name.trim().trim_matches('"'),
                version.trim().trim_matches('"')
            ))
        })
        .collect()
}

fn major(v: &str) -> Option<u32> {
    v.split('.').next().and_then(|m| m.trim().parse().ok())
}

pub fn from_request(batch: &Batch, headers: &HeaderMap) -> Signals {
    let brands = if batch.client_brands.is_empty() {
        header(headers, "sec-ch-ua")
            .map(parse_sec_ch_ua)
            .unwrap_or_default()
    } else {
        batch.client_brands.clone()
    };
    let mobile =
        batch
            .client_mobile
            .map(|m| m == 1)
            .or_else(|| match header(headers, "sec-ch-ua-mobile") {
                Some("?1") => Some(true),
                Some("?0") => Some(false),
                _ => None,
            });
    let platform = batch
        .client_platform
        .clone()
        .or_else(|| {
            header(headers, "sec-ch-ua-platform").map(|p| p.trim().trim_matches('"').to_string())
        })
        .filter(|p| !p.is_empty());
    Signals {
        user_agent: header(headers, "user-agent")
            .unwrap_or_default()
            .to_string(),
        brands,
        mobile,
        platform,
        platform_major: batch.client_platform_version.as_deref().and_then(major),
        touch_points: batch.touch_points,
        coarse_pointer: batch.coarse_pointer.map(|c| c == 1),
    }
}
