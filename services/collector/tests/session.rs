use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use kurzhaar_collector::config::{Config, Site};
use kurzhaar_collector::session::{session_hash, MemorySalts, Salt, SaltSource};
use kurzhaar_collector::{public_router, AppState};
use kurzhaar_core::detect::{Detection, Detector, DeviceClass, Signals};
use kurzhaar_queue::MemoryQueue;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tower::ServiceExt;
use zeroize::Zeroizing;

const DAY: u64 = 86_400_000;
const GRACE: u64 = 10 * 60 * 1000;
const BATCH: &str = "v::1|k::abc\nts::1000|u::/";
const UA: &[u8] = b"Mozilla/5.0 (X11; Linux x86_64) Firefox/126.0";

struct Nothing;
impl Detector for Nothing {
    fn detect(&self, _: &Signals) -> Detection {
        Detection {
            browser: "Test".into(),
            browser_major: None,
            os: "Test".into(),
            os_major: None,
            device: DeviceClass::Other,
            bot: false,
        }
    }
}

struct Fixed(u8);
impl SaltSource for Fixed {
    fn salt(&self, _: u64) -> Salt {
        Zeroizing::new([self.0; 32])
    }
    fn salt_at(&self, _: u64, _: u64) -> Salt {
        Zeroizing::new([self.0; 32])
    }
}

fn salt(b: u8) -> Salt {
    Zeroizing::new([b; 32])
}

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

/// One replica with the given salts; returns the session and the partition key of one batch.
async fn session_of(
    salts: Arc<dyn SaltSource>,
    peer: &str,
    headers: &[(&str, &str)],
) -> (String, String) {
    let queue = Arc::new(MemoryQueue::new());
    let config = Config {
        sites: vec![Site {
            key: "abc".into(),
            id: "site-1".into(),
        }],
        ..Config::default()
    };
    let state = AppState::new(
        config,
        queue.clone(),
        Arc::new(Nothing),
        Arc::new(|| 1_800_000_000_000),
    )
    .unwrap()
    .with_salts(salts);
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/v1/collect")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::USER_AGENT, "Mozilla/5.0 Firefox/126.0");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let mut req = req.body(Body::from(BATCH)).unwrap();
    let addr: SocketAddr = format!("{peer}:4000").parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    let status = public_router(Arc::new(state))
        .oneshot(req)
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::ACCEPTED);
    let message = queue.messages().remove(0);
    let json: serde_json::Value = serde_json::from_slice(&message.payload).unwrap();
    (
        json["session"].as_str().unwrap().to_string(),
        message.partition_key,
    )
}

// verifies: REQ-COLLECT-G8T0.S1 (same site, address and User-Agent on one day: equal hashes, 32 hex characters)
#[test]
fn same_visitor_same_day_gives_the_same_hash() {
    let a = session_hash(&salt(1), "site-1", ip("192.0.2.7"), UA);
    let b = session_hash(&salt(1), "site-1", ip("192.0.2.7"), UA);
    assert_eq!(a, b);
    assert_eq!(a.len(), 32);
    assert!(a
        .bytes()
        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
}

// verifies: REQ-COLLECT-G8T0.S2 (a different site, address or User-Agent gives a different hash)
#[test]
fn any_changed_input_changes_the_hash() {
    let base = session_hash(&salt(1), "site-1", ip("192.0.2.7"), UA);
    assert_ne!(base, session_hash(&salt(1), "site-2", ip("192.0.2.7"), UA));
    assert_ne!(base, session_hash(&salt(1), "site-1", ip("192.0.2.8"), UA));
    assert_ne!(
        base,
        session_hash(&salt(1), "site-1", ip("2001:db8::7"), UA)
    );
    assert_ne!(
        base,
        session_hash(&salt(1), "site-1", ip("192.0.2.7"), b"Other")
    );
    // Fields cannot run into each other.
    assert_ne!(
        session_hash(&salt(1), "ab", ip("192.0.2.7"), b"c"),
        session_hash(&salt(1), "a", ip("192.0.2.7"), b"bc")
    );
}

// verifies: REQ-COLLECT-G8T0.S3 (the same request on the next UTC day gets a different hash)
// verifies: REQ-PRIVACY-E4TN.S2 (the first request of a new day differs from the day before)
#[test]
fn next_day_gives_another_hash() {
    let salts = MemorySalts::new();
    let before = session_hash(&salts.salt(DAY - 1), "site-1", ip("192.0.2.7"), UA);
    let after = session_hash(&salts.salt(DAY), "site-1", ip("192.0.2.7"), UA);
    assert_ne!(before, after);
    // And within one day the salt, and so the hash, stays.
    assert_eq!(
        after,
        session_hash(&salts.salt(2 * DAY - 1), "site-1", ip("192.0.2.7"), UA)
    );
}

// verifies: REQ-COLLECT-G8T0.S4 (no User-Agent: a hash is still derived, equal to one from an empty value)
#[tokio::test]
async fn missing_user_agent_is_an_empty_one() {
    assert_eq!(
        session_hash(&salt(1), "site-1", ip("192.0.2.7"), b"").len(),
        32
    );
    let queue = Arc::new(MemoryQueue::new());
    let config = Config {
        sites: vec![Site {
            key: "abc".into(),
            id: "site-1".into(),
        }],
        ..Config::default()
    };
    let state = AppState::new(
        config,
        queue.clone(),
        Arc::new(Nothing),
        Arc::new(|| 1_800_000_000_000),
    )
    .unwrap()
    .with_salts(Arc::new(Fixed(1)));
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/v1/collect")
        .body(Body::from(BATCH))
        .unwrap();
    let addr: SocketAddr = "192.0.2.7:4000".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    let status = public_router(Arc::new(state))
        .oneshot(req)
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::ACCEPTED);
    let message = queue.messages().remove(0);
    assert_eq!(
        message.partition_key,
        session_hash(&salt(1), "site-1", ip("192.0.2.7"), b"")
    );
}

// verifies: REQ-COLLECT-G8T0.S5 (the envelope's session is the partition key and ignores Client Hints)
// verifies: REQ-COLLECT-SD26.S2 (the partition key is the session hash)
#[tokio::test]
async fn session_is_the_partition_key_and_ignores_client_hints() {
    let salts = Arc::new(Fixed(1));
    let (session, key) = session_of(salts.clone(), "192.0.2.7", &[]).await;
    assert_eq!(session, key);
    assert_eq!(
        session,
        session_hash(
            &salt(1),
            "site-1",
            ip("192.0.2.7"),
            b"Mozilla/5.0 Firefox/126.0"
        )
    );
    let (with_hints, _) = session_of(
        salts,
        "192.0.2.7",
        &[
            ("sec-ch-ua-platform", "\"Windows\""),
            ("sec-ch-ua-mobile", "?1"),
        ],
    )
    .await;
    assert_eq!(session, with_hints);
}

// verifies: REQ-COLLECT-G8T0.S6 (replicas sharing a salt source give one visitor one hash and one partition key)
#[tokio::test]
async fn replicas_with_shared_salts_agree() {
    let shared: Arc<dyn SaltSource> = Arc::new(Fixed(7));
    let a = session_of(shared.clone(), "192.0.2.7", &[]).await;
    let b = session_of(shared, "192.0.2.7", &[]).await;
    assert_eq!(a, b);
    // Memory-only salts are not shared, so another replica differs.
    let c = session_of(Arc::new(MemorySalts::new()), "192.0.2.7", &[]).await;
    let d = session_of(Arc::new(MemorySalts::new()), "192.0.2.7", &[]).await;
    assert_ne!(c.0, d.0);
}

// verifies: REQ-PRIVACY-E4TN.S1 (a salt is gone ten minutes after its day, in the memory source)
#[test]
fn salt_is_dropped_ten_minutes_after_its_day() {
    let salts = MemorySalts::new();
    salts.salt(5);
    salts.salt(DAY + 5);
    assert_eq!(salts.held_days(), vec![0, 1]);
    // Still inside the grace period: day 0 stays for requests that are in flight.
    salts.salt(DAY + GRACE - 1);
    assert_eq!(salts.held_days(), vec![0, 1]);
    salts.salt(DAY + GRACE);
    assert_eq!(salts.held_days(), vec![1]);
}

// verifies: REQ-PRIVACY-E4TN.S4 (salts are 32 random bytes and differ between collectors)
#[test]
fn salts_are_random_and_differ_between_sources() {
    let a = MemorySalts::new().salt(0);
    let b = MemorySalts::new().salt(0);
    assert_eq!(a.len(), 32);
    assert_ne!(*a, *b);
    assert_ne!(*a, [0u8; 32]);
}

// verifies: REQ-PRIVACY-E4TN.S5 (the first millisecond of a day already has its salt)
#[test]
fn salt_exists_at_midnight() {
    let salts = MemorySalts::new();
    let first = salts.salt(3 * DAY);
    assert_eq!(salts.held_days(), vec![3]);
    assert_eq!(*first, *salts.salt(3 * DAY + 1));
}

// verifies: REQ-COLLECT-KSBB.S4 (a late batch uses the salt of its day while held, else the salt of today)
#[test]
fn late_batch_takes_the_salt_of_its_day_only_while_it_is_held() {
    let salts = MemorySalts::new();
    let yesterday = salts.salt(DAY - 1000);
    let today = salts.salt(DAY + 1000);
    assert_ne!(*yesterday, *today);
    // Five minutes after midnight the day before is still held.
    assert_eq!(*salts.salt_at(DAY - 2000, DAY + 5 * 60_000), *yesterday);
    // Twelve minutes after midnight it is gone: today's salt is used and none is created.
    assert_eq!(*salts.salt_at(DAY - 2000, DAY + 12 * 60_000), *today);
    assert_eq!(salts.held_days(), vec![1]);
    // Within one day it is the plain salt of the day.
    assert_eq!(*salts.salt_at(DAY + 100, DAY + 9000), *today);
    // A day that never had a salt does not get one made up.
    let fresh = MemorySalts::new();
    let now = fresh.salt_at(5, DAY + 1000);
    assert_eq!(*now, *fresh.salt(DAY + 1000));
    assert_eq!(fresh.held_days(), vec![1]);
}
