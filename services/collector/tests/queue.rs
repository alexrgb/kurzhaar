use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use kurzhaar_collector::config::{Config, Site};
use kurzhaar_collector::{public_router, system_clock, AppState};
use kurzhaar_core::detect::{Detection, Detector, DeviceClass, Signals};
use kurzhaar_queue::redis::RedisQueue;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt;

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

fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |k| {
        pairs
            .iter()
            .find(|(name, _)| format!("KURZHAAR_COLLECTOR_{name}") == k)
            .map(|(_, v)| v.to_string())
    }
}

// verifies: REQ-QUEUE-BMGT.S1 (redis selected with a URL: accepted, and a batch reaches the stream)
#[tokio::test]
async fn redis_selected_batches_reach_the_stream() {
    let c = Config::load(
        None,
        env_of(&[("QUEUE", "redis"), ("QUEUE_URL", "redis://127.0.0.1:1")]),
    )
    .unwrap();
    assert_eq!(c.queue, "redis");

    let Ok(url) = std::env::var("KURZHAAR_TEST_REDIS_URL") else {
        assert!(
            std::env::var("CI").is_err(),
            "KURZHAAR_TEST_REDIS_URL must be set in CI"
        );
        eprintln!("skipped: KURZHAAR_TEST_REDIS_URL is not set");
        return;
    };
    let prefix = format!("kz_test_collector_{}:events", std::process::id());
    let queue = Arc::new(RedisQueue::new(&url, &prefix, 2).unwrap());
    let config = Config {
        sites: vec![Site {
            key: "abc".into(),
            id: "site-1".into(),
        }],
        ..Config::default()
    };
    let state = AppState::new(config, queue.clone(), Arc::new(Nothing), system_clock()).unwrap();
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/v1/collect")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("v::1|k::abc\nts::1000|u::/"))
        .unwrap();
    let addr: SocketAddr = "192.0.2.7:4000".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    let status = public_router(Arc::new(state))
        .oneshot(req)
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::ACCEPTED);

    // The message is in one of the two streams, keyed by the session hash.
    let mut admin = redis::Client::open(url.as_str())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let mut found = Vec::new();
    for shard in 0..2 {
        let reply: Vec<(String, Vec<(String, redis::Value)>)> = redis::cmd("XRANGE")
            .arg(format!("{prefix}:{shard}"))
            .arg("-")
            .arg("+")
            .query_async(&mut admin)
            .await
            .unwrap();
        found.extend(reply);
    }
    assert_eq!(found.len(), 1);
    let fields = &found[0].1;
    let value = |name: &str| {
        fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| match v {
                redis::Value::BulkString(b) => b.clone(),
                _ => Vec::new(),
            })
    };
    let key = String::from_utf8(value("key").unwrap()).unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&value("payload").unwrap()).unwrap();
    assert_eq!(payload["session"], key.as_str());
    assert_eq!(key.len(), 32);
}

// verifies: REQ-QUEUE-BMGT.S2 (redis without a URL stops the start and names the setting)
#[test]
fn redis_without_a_url_is_refused() {
    let e = Config::load(None, env_of(&[("QUEUE", "redis")])).unwrap_err();
    assert!(e.contains("KURZHAAR_COLLECTOR_QUEUE_URL"), "{e}");
}

// verifies: REQ-QUEUE-BMGT.S3 (an unknown backend stops the start and lists the known names)
#[test]
fn unknown_backend_is_refused() {
    let e = Config::load(None, env_of(&[("QUEUE", "carrier-pigeon")])).unwrap_err();
    assert!(e.contains("memory") && e.contains("redis"), "{e}");
}

// verifies: REQ-QUEUE-754R.S1 (a queue URL in the file stops the start; the password is not repeated)
#[test]
fn queue_url_in_the_file_is_refused() {
    let file = "queue = \"redis\"\nqueue_url = \"redis://:secret@host:6379\"";
    let e = Config::load(Some(file), |_| None).unwrap_err();
    assert!(e.contains("KURZHAAR_COLLECTOR_QUEUE_URL"), "{e}");
    assert!(!e.contains("secret"));
    let c = Config::load(
        None,
        env_of(&[
            ("QUEUE", "redis"),
            ("QUEUE_URL", "redis://:secret@host:6379"),
        ]),
    )
    .unwrap();
    assert!(!format!("{c:?}").contains("secret"));
}
