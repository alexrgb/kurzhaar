use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use kurzhaar_collector::config::{Config, Site};
use kurzhaar_collector::country::{CountryDb, Reload};
use kurzhaar_collector::{public_router, refresh_country, system_clock, AppState, Metrics};
use kurzhaar_core::detect::{Detection, Detector, DeviceClass, Signals};
use kurzhaar_queue::MemoryQueue;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tower::ServiceExt;

const COUNTRY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/country/country.mmdb"
);
const CITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/country/city.mmdb"
);
const BATCH: &str = "v::1|k::abc\nts::1000|u::/";

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

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

/// A copy of a fixture in a directory of its own, so a test can replace it.
fn work_copy(name: &str, from: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kurzhaar-country-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("db.mmdb");
    std::fs::copy(from, &path).unwrap();
    path
}

fn replace(path: &Path, from: &str) {
    // Written under another name and renamed, the way an updater would.
    let tmp = path.with_extension("new");
    std::fs::copy(from, &tmp).unwrap();
    std::fs::rename(tmp, path).unwrap();
}

async fn country_of(db: Arc<CountryDb>, peer: &str) -> (StatusCode, serde_json::Value) {
    let queue = Arc::new(MemoryQueue::new());
    let config = Config {
        sites: vec![Site {
            key: "abc".into(),
            id: "site-1".into(),
        }],
        ..Config::default()
    };
    let state = AppState::new(config, queue.clone(), Arc::new(Nothing), system_clock())
        .unwrap()
        .with_country(db);
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/v1/collect")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(BATCH))
        .unwrap();
    let addr: SocketAddr = format!("{peer}:4000").parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    let status = public_router(Arc::new(state))
        .oneshot(req)
        .await
        .unwrap()
        .status();
    let message = queue
        .messages()
        .first()
        .map_or(serde_json::Value::Null, |m| {
            serde_json::from_slice(&m.payload).unwrap()
        });
    (status, message)
}

// verifies: REQ-COLLECT-82NX.S1 (no database: lookups give no country and the batch is accepted)
#[tokio::test]
async fn without_a_database_events_carry_no_country() {
    let db = CountryDb::disabled();
    assert!(!db.is_enabled());
    assert_eq!(db.lookup(ip("192.0.2.7")), None);
    assert_eq!(db.reload_if_changed(), Reload::Unchanged);
    let (status, message) = country_of(Arc::new(db), "192.0.2.7").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(message["country"].is_null());
}

// verifies: REQ-COLLECT-82NX.S2 (a missing or invalid file is an error that names the path)
#[test]
fn missing_or_invalid_database_names_the_path() {
    let missing = "/nonexistent/kurzhaar/db.mmdb";
    let Err(e) = CountryDb::open(missing) else {
        panic!("opened a missing file")
    };
    assert!(e.contains(missing), "{e}");

    let bad = work_copy("invalid", COUNTRY);
    std::fs::write(&bad, b"this is not a database").unwrap();
    let Err(e) = CountryDb::open(&bad) else {
        panic!("opened an invalid file")
    };
    assert!(e.contains(&bad.display().to_string()), "{e}");
}

// verifies: REQ-COLLECT-82NX.S3 (a replaced file is used by the next check, lookups keep working)
#[test]
fn replaced_file_is_picked_up() {
    let path = work_copy("replace", COUNTRY);
    let db = CountryDb::open(&path).unwrap();
    assert_eq!(db.lookup(ip("192.0.2.7")).as_deref(), Some("DE"));
    assert_eq!(db.reload_if_changed(), Reload::Unchanged);

    replace(&path, CITY);
    assert_eq!(db.reload_if_changed(), Reload::Reloaded);
    assert_eq!(db.lookup(ip("203.0.113.9")).as_deref(), Some("PL"));
    assert_eq!(db.lookup(ip("192.0.2.7")), None);
}

// verifies: REQ-COLLECT-82NX.S4 (a city database gives only the country code)
#[test]
fn city_database_gives_only_the_country_code() {
    let db = CountryDb::open(CITY).unwrap();
    assert_eq!(db.lookup(ip("203.0.113.9")).as_deref(), Some("PL"));
}

// verifies: REQ-COLLECT-82NX.S5 (an address with no entry: empty country, batch accepted)
#[tokio::test]
async fn address_not_in_the_database_has_no_country() {
    let db = Arc::new(CountryDb::open(COUNTRY).unwrap());
    assert_eq!(db.lookup(ip("10.1.2.3")), None);
    let (status, message) = country_of(db.clone(), "10.1.2.3").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(message["country"].is_null());

    // And a known one reaches the queue, for both address families.
    let (_, message) = country_of(db.clone(), "198.51.100.4").await;
    assert_eq!(message["country"], "UA");
    assert_eq!(db.lookup(ip("2001:db8::1")).as_deref(), Some("FR"));
}

// verifies: REQ-COLLECT-82NX.S6 (an invalid replacement is ignored, logged and counted)
#[test]
fn invalid_replacement_keeps_the_previous_database() {
    let path = work_copy("invalid-replacement", COUNTRY);
    let db = CountryDb::open(&path).unwrap();
    let metrics = Metrics::default();

    std::fs::write(&path, b"truncated").unwrap();
    assert!(matches!(refresh_country(&db, &metrics), Reload::Failed(_)));
    assert_eq!(
        metrics.country_db_reload_failures.load(Ordering::Relaxed),
        1
    );
    assert!(metrics
        .render()
        .contains("kurzhaar_collector_country_db_reload_failures_total 1"));
    assert_eq!(db.lookup(ip("192.0.2.7")).as_deref(), Some("DE"));

    // A good file afterwards is picked up again.
    replace(&path, CITY);
    assert_eq!(refresh_country(&db, &metrics), Reload::Reloaded);
    assert_eq!(db.lookup(ip("203.0.113.9")).as_deref(), Some("PL"));
}
