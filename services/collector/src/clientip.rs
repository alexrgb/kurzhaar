//! The client address (REQ-COLLECT-GZF5). It is used in memory for one request and never written
//! anywhere (ADR-D3BB).

use ipnet::IpNet;
use std::net::IpAddr;

fn is_trusted(ip: IpAddr, trusted: &[IpNet]) -> bool {
    trusted.iter().any(|n| n.contains(&ip))
}

/// The connection peer, unless the peer is a trusted proxy; then the right-most
/// `X-Forwarded-For` entry that is not itself a trusted proxy. A header that cannot be read is
/// not believed.
pub fn resolve(peer: IpAddr, forwarded_for: Option<&str>, trusted: &[IpNet]) -> IpAddr {
    if !is_trusted(peer, trusted) {
        return peer;
    }
    let Some(header) = forwarded_for else {
        return peer;
    };
    for part in header.rsplit(',') {
        match part.trim().parse::<IpAddr>() {
            Ok(ip) if is_trusted(ip, trusted) => continue,
            Ok(ip) => return ip,
            Err(_) => return peer,
        }
    }
    peer
}
