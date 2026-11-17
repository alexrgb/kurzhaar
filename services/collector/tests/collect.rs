use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use kurzhaar_collector::config::{Config, Site};
use kurzhaar_collector::{public_router, AppState};
use kurzhaar_core::detect::{CachedDetector, UaparserDetector};
use kurzhaar_queue::MemoryQueue;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt;

const NOW: u64 = 1_800_000_000_000;

struct Rig {
    app: Router,
    queue: Arc<MemoryQueue>,
    state: Arc<AppState>,
}

fn detector() -> Arc<CachedDetector<UaparserDetector>> {
    // Parsing the rules takes a moment, so the tests share one.
    static D: std::sync::OnceLock<Arc<CachedDetector<UaparserDetector>>> =
        std::sync::OnceLock::new();
    D.get_or_init(|| {
        Arc::new(CachedDetector::new(
            UaparserDetector::builtin().unwrap(),
            100,
            std::time::Duration::from_secs(60),
        ))
    })
    .clone()
}

fn rig_with(edit: impl FnOnce(&mut Config)) -> Rig {
    let mut config = Config {
        sites: vec![Site {
            key: "abc".into(),
            id: "site-1".into(),
        }],
        ..Config::default()
    };
    edit(&mut config);
    let queue = Arc::new(MemoryQueue::new());
    let state = Arc::new(
        AppState::new(config, queue.clone(), detector(), Arc::new(|| NOW)).expect("config"),
    );
    Rig {
        app: public_router(state.clone()),
        queue,
        state,
    }
}

fn rig() -> Rig {
    rig_with(|_| {})
}

fn post(body: &str, peer: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/v1/collect")
        .header(header::CONTENT_TYPE, "text/plain;charset=UTF-8")
        .body(Body::from(body.to_string()))
        .unwrap();
    let addr: SocketAddr = format!("{peer}:4000").parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    req
}

async fn send(r: &Rig, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, String) {
    let res = r.app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8_lossy(&body).into_owned())
}

fn queued(r: &Rig) -> Vec<serde_json::Value> {
    r.queue
        .messages()
        .iter()
        .map(|m| serde_json::from_slice(&m.payload).unwrap())
        .collect()
}

const BATCH: &str = "v::1|k::abc|s::1920x1080|l::uk\nts::1000|u::/\nd::4000|u::/pricing";

// verifies: REQ-COLLECT-C50Y.S1 (a text/plain beacon post is accepted with no OPTIONS round trip)
#[tokio::test]
async fn text_plain_post_is_accepted_without_preflight() {
    let r = rig();
    let (status, _, _) = send(&r, post(BATCH, "203.0.113.5")).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(r.queue.messages().len(), 1);
}

// verifies: REQ-COLLECT-Y37S.S1 (the queue rejects the write: retryable error, not acknowledged)
#[tokio::test]
async fn queue_outage_is_a_retryable_error() {
    let r = rig();
    r.queue.set_down(true);
    let (status, headers, _) = send(&r, post(BATCH, "203.0.113.5")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(headers.contains_key(header::RETRY_AFTER));
    assert!(r.queue.messages().is_empty());
}

// verifies: REQ-COLLECT-4VTR.S1 (an unknown key is answered like an accepted batch and enqueues nothing)
#[tokio::test]
async fn unknown_site_is_dropped_silently() {
    let r = rig();
    let (known, _, known_body) = send(&r, post(BATCH, "203.0.113.5")).await;
    let (status, _, body) =
        send(&r, post(&BATCH.replace("k::abc", "k::nope"), "203.0.113.5")).await;
    assert_eq!((status, &body), (known, &known_body));
    assert_eq!(r.queue.messages().len(), 1);
}

// verifies: REQ-COLLECT-FWT0.S1 (a body over the limit gets an error and nothing is enqueued)
#[tokio::test]
async fn oversized_body_is_rejected() {
    let r = rig_with(|c| c.max_body_bytes = 64);
    let big = format!("v::1|k::abc\nts::1|u::/{}", "a".repeat(200));
    let (status, _, _) = send(&r, post(&big, "203.0.113.5")).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(r.queue.messages().is_empty());
}

// verifies: REQ-COLLECT-FWT0.S1 (too many events and malformed input are rejected, nothing is enqueued)
#[tokio::test]
async fn too_many_events_and_malformed_input_are_rejected() {
    let r = rig_with(|c| c.max_events_per_batch = 2);
    let three = "v::1|k::abc\nts::1|u::/\nd::1|u::/\nd::1|u::/";
    assert_eq!(
        send(&r, post(three, "203.0.113.5")).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(&r, post("v::9|k::abc\nts::1|u::/", "203.0.113.5"))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(&r, post("v::1|k::abc", "203.0.113.5")).await.0,
        StatusCode::BAD_REQUEST
    );
    assert!(r.queue.messages().is_empty());
}

// verifies: REQ-COLLECT-D9G1.S1 (a simple post gets the any-origin header and no cookie)
// verifies: REQ-COLLECT-D9G1.S3 (credentials are never allowed)
#[tokio::test]
async fn cross_origin_post_has_any_origin_and_no_credentials() {
    let r = rig();
    let (_, h, _) = send(&r, post(BATCH, "203.0.113.5")).await;
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert!(!h.contains_key(header::SET_COOKIE));
    assert!(!h.contains_key(header::ACCESS_CONTROL_ALLOW_CREDENTIALS));
}

// verifies: REQ-COLLECT-D9G1.S2 (a preflight is answered with 204 and the allowed method and header)
#[tokio::test]
async fn preflight_is_answered() {
    let r = rig();
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/collect")
        .body(Body::empty())
        .unwrap();
    let (status, h, _) = send(&r, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_METHODS], "POST");
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_HEADERS], "content-type");
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
}

// verifies: REQ-COLLECT-X0AM.S1 (the same bytes twice give the same token)
// verifies: REQ-COLLECT-X0AM.S2 (a body that differs by one byte gives another token)
#[tokio::test]
async fn token_is_stable_for_the_same_bytes() {
    let r = rig();
    send(&r, post(BATCH, "203.0.113.5")).await;
    send(&r, post(BATCH, "203.0.113.6")).await;
    send(
        &r,
        post(&BATCH.replace("pricing", "pricinG"), "203.0.113.5"),
    )
    .await;
    let q = queued(&r);
    assert_eq!(q[0]["token"], q[1]["token"]);
    assert_ne!(q[0]["token"], q[2]["token"]);
    assert_eq!(q[0]["token"].as_str().unwrap().len(), 32);
}

// verifies: REQ-COLLECT-X0AM.S3 (the same body under two site keys gives two tokens)
#[test]
fn token_differs_between_sites() {
    use kurzhaar_collector::envelope::token;
    assert_ne!(token("a", b"body"), token("b", b"body"));
}

// verifies: REQ-COLLECT-8J26.S1 (a client clock a day off does not move the events)
// verifies: REQ-COLLECT-8J26.S2 (deltas of 0, 1200 and 800 ms date the events from the last one)
#[tokio::test]
async fn events_are_dated_from_the_server_clock() {
    let r = rig();
    let body = "v::1|k::abc\nts::1000|u::/\nd::1200|u::/a\nd::800|u::/b";
    send(&r, post(body, "203.0.113.5")).await;
    let q = queued(&r);
    let t: Vec<u64> = q[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["ts"].as_u64().unwrap())
        .collect();
    assert_eq!(t, vec![NOW - 2000, NOW - 800, NOW]);
}

// verifies: REQ-COLLECT-8J26.S3 (a single event is dated at the receive time)
#[tokio::test]
async fn single_event_is_dated_at_receive_time() {
    let r = rig();
    send(&r, post("v::1|k::abc\nts::5|u::/", "203.0.113.5")).await;
    assert_eq!(queued(&r)[0]["events"][0]["ts"], NOW);
}

// verifies: REQ-COLLECT-J4S5.S1 (twelve tags: the batch is accepted and ten are kept)
// verifies: REQ-COLLECT-J4S5.S2 (tracker version and timezone offset travel in their own fields)
#[tokio::test]
async fn tags_are_limited_and_header_fields_are_kept() {
    let r = rig();
    let tags = (1..=12)
        .map(|i| format!("t{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let long = "x".repeat(40);
    let body = format!("v::1|k::abc|tv::1.4.2|tz::120\nts::1|u::/|g::{tags},{long}");
    let (status, _, _) = send(&r, post(&body, "203.0.113.5")).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let m = &queued(&r)[0];
    let kept = m["events"][0]["tags"].as_array().unwrap();
    assert_eq!(kept.len(), 10);
    assert_eq!(kept[9], "t10");
    assert_eq!(m["tracker_version"], "1.4.2");
    assert_eq!(m["tz_offset"], 120);

    let body = format!("v::1|k::abc\nts::1|u::/|g::{long}");
    send(&r, post(&body, "203.0.113.5")).await;
    assert_eq!(
        queued(&r)[1]["events"][0]["tags"][0]
            .as_str()
            .unwrap()
            .len(),
        32
    );
}

// verifies: REQ-COLLECT-FRDS.S1 (the queued message carries no address)
// The envelope contract (REQ-COLLECT-SD26) is verified when the consumer reads it; this checks only what it must not contain.
#[tokio::test]
async fn envelope_carries_no_address_or_client_signals() {
    let r = rig();
    let body = "v::1|k::abc|cb::Chromium/124|cm::0|cp::Windows|cv::15|tp::0|pc::0\nts::1|u::/";
    let mut req = post(body, "203.0.113.77");
    req.headers_mut().insert(
        header::USER_AGENT,
        "Mozilla/5.0 SecretAgent".parse().unwrap(),
    );
    send(&r, req).await;
    let raw = String::from_utf8(r.queue.messages()[0].payload.clone()).unwrap();
    for needle in [
        "203.0.113.77",
        "SecretAgent",
        "Chromium",
        "client_brands",
        "abc\"",
    ] {
        assert!(!raw.contains(needle), "envelope leaks {needle}: {raw}");
    }
    let m = &queued(&r)[0];
    assert_eq!(m["v"], 1);
    assert_eq!(m["site"], "site-1");
}

// verifies: REQ-COLLECT-GZF5.S1 (with no trusted proxies the header is ignored)
#[tokio::test]
async fn forwarded_header_is_ignored_without_trusted_proxies() {
    let r = rig_with(|c| {
        c.rate_limit_burst = 1;
        c.rate_limit_per_minute = 1;
    });
    for victim in ["198.51.100.1", "198.51.100.2"] {
        let mut req = post(BATCH, "203.0.113.5");
        req.headers_mut()
            .insert("x-forwarded-for", victim.parse().unwrap());
        send(&r, req).await;
    }
    // Both came from one peer, so the second hit the limit even though the header differed.
    assert_eq!(r.queue.messages().len(), 1);
}

// verifies: REQ-COLLECT-GZF5.S2 (through a trusted proxy the client is the right-most outside entry)
#[test]
fn trusted_proxy_resolves_the_client() {
    use kurzhaar_collector::clientip::resolve;
    let trusted = vec!["10.0.0.0/8".parse().unwrap()];
    let peer = "10.0.0.9".parse().unwrap();
    assert_eq!(
        resolve(peer, Some("203.0.113.9, 10.0.0.2"), &trusted),
        "203.0.113.9".parse::<std::net::IpAddr>().unwrap()
    );
    assert_eq!(resolve(peer, Some("garbage"), &trusted), peer);
    let outside = "192.0.2.1".parse().unwrap();
    assert_eq!(resolve(outside, Some("203.0.113.9"), &trusted), outside);
}

// verifies: REQ-COLLECT-51V3.S1 (below the limit everything is processed)
// verifies: REQ-COLLECT-51V3.S2 (above it: 429 with Retry-After and nothing enqueued)
#[tokio::test]
async fn rate_limit_answers_429() {
    let r = rig_with(|c| {
        c.rate_limit_burst = 2;
        c.rate_limit_per_minute = 60;
    });
    assert_eq!(
        send(&r, post(BATCH, "203.0.113.5")).await.0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        send(&r, post(BATCH, "203.0.113.5")).await.0,
        StatusCode::ACCEPTED
    );
    let (status, h, _) = send(&r, post(BATCH, "203.0.113.5")).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(h.contains_key(header::RETRY_AFTER));
    assert_eq!(r.queue.messages().len(), 2);
    // Another address is not affected.
    assert_eq!(
        send(&r, post(BATCH, "203.0.113.6")).await.0,
        StatusCode::ACCEPTED
    );
}

// verifies: REQ-COLLECT-51V3.S3 (idle addresses are removed and the table is bounded)
#[test]
fn idle_addresses_are_removed_and_the_table_is_bounded() {
    use kurzhaar_collector::ratelimit::RateLimiter;
    let l = RateLimiter::new(60, 5, 2);
    let (a, b, c) = (
        "10.0.0.1".parse().unwrap(),
        "10.0.0.2".parse().unwrap(),
        "10.0.0.3".parse().unwrap(),
    );
    l.check(a, 0).unwrap();
    l.check(b, 0).unwrap();
    // The table is full of active addresses: a third is not tracked and not blocked.
    l.check(c, 1000).unwrap();
    assert_eq!(l.len(), 2);
    l.sweep(11 * 60 * 1000);
    assert!(l.is_empty());
}

// verifies: REQ-COLLECT-X2K2.S1 (liveness does not touch the queue)
// verifies: REQ-COLLECT-X2K2.S2 (readiness is 503 with the queue down, liveness stays 200)
#[tokio::test]
async fn health_and_readiness() {
    let r = rig();
    let get = |p: &str| Request::builder().uri(p).body(Body::empty()).unwrap();
    r.queue.set_down(true);
    assert_eq!(send(&r, get("/healthz")).await.0, StatusCode::OK);
    assert_eq!(
        send(&r, get("/readyz")).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    r.queue.set_down(false);
    assert_eq!(send(&r, get("/readyz")).await.0, StatusCode::OK);
}

// verifies: REQ-COLLECT-X2K2.S3 (metrics are not on the public listener)
#[tokio::test]
async fn metrics_are_not_public() {
    let r = rig();
    let req = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&r, req).await.0, StatusCode::NOT_FOUND);
    let text = r.state.metrics.render();
    assert!(text.contains("kurzhaar_collector_batches_accepted_total"));
}

// Configuration is validated at start (ADR-K8N9).
#[test]
fn bad_configuration_stops_the_start() {
    let none = |_: &str| None;
    assert!(Config::load(None, none).is_ok());
    assert!(Config::load(Some("listen = \"nope\""), none).is_err());
    assert!(Config::load(Some("surprise = 1"), none).is_err());
    assert!(Config::load(Some("trusted_proxies = [\"x\"]"), none).is_err());
    let env = |k: &str| (k == "KURZHAAR_COLLECTOR_MAX_BODY_BYTES").then(|| "0".to_string());
    assert!(Config::load(None, env).is_err());
    let env = |k: &str| (k == "KURZHAAR_COLLECTOR_SITES").then(|| "k1=s1, k2=s2".to_string());
    assert_eq!(
        Config::load(Some("max_body_bytes = 10"), env)
            .unwrap()
            .sites
            .len(),
        2
    );
    // The environment wins over the file.
    let env = |k: &str| (k == "KURZHAAR_COLLECTOR_MAX_BODY_BYTES").then(|| "99".to_string());
    assert_eq!(
        Config::load(Some("max_body_bytes = 10"), env)
            .unwrap()
            .max_body_bytes,
        99
    );
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for LogBuffer {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuffer {
    type Writer = LogBuffer;
    fn make_writer(&'a self) -> LogBuffer {
        self.clone()
    }
}

// verifies: REQ-COLLECT-GZF5.S3 (the address appears in no log line, even when a request fails)
#[tokio::test]
async fn address_is_not_logged() {
    let logs = LogBuffer::default();
    let sub = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(sub);
    let r = rig_with(|c| c.max_body_bytes = 64);
    send(&r, post(BATCH, "203.0.113.77")).await;
    send(&r, post(&"x".repeat(500), "203.0.113.77")).await;
    r.queue.set_down(true);
    send(&r, post("v::1|k::abc\nts::1|u::/", "203.0.113.77")).await;
    let text = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(!text.contains("203.0.113.77"), "{text}");
}

fn post_ua(body: &str, ua: &str, extra: &[(&str, &str)]) -> Request<Body> {
    let mut req = post(body, "203.0.113.5");
    req.headers_mut()
        .insert(header::USER_AGENT, ua.parse().unwrap());
    for (k, v) in extra {
        req.headers_mut().insert(
            axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
            v.parse().unwrap(),
        );
    }
    req
}

const WIN_CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

// verifies: REQ-COLLECT-QZRJ.S1 (the queued event holds names and major versions only and no User-Agent)
// verifies: REQ-COLLECT-RA57.S1 (no User-Agent, client-hint or touch-point value is queued)
// verifies: REQ-COLLECT-X6WF.S1 (platform Windows with platform version 13 or higher is Windows 11)
#[tokio::test]
async fn the_envelope_holds_detected_classes_and_no_raw_signals() {
    let r = rig();
    let body = "v::1|k::abc|cb::Chromium/124,Google Chrome/124|cm::0|cp::Windows|cv::15|tp::7|pc::0\nts::1|u::/";
    send(&r, post_ua(body, WIN_CHROME, &[])).await;
    let raw = String::from_utf8(r.queue.messages()[0].payload.clone()).unwrap();
    let m = &queued(&r)[0];
    assert_eq!(
        (m["browser"].as_str(), m["browser_major"].as_u64()),
        (Some("Chrome"), Some(124))
    );
    assert_eq!(
        (m["os"].as_str(), m["os_major"].as_u64()),
        (Some("Windows"), Some(11))
    );
    assert_eq!(m["device"], "desktop");
    assert_eq!(m["bot"], false);
    for needle in [
        "Mozilla",
        "AppleWebKit",
        "Chromium",
        "124.0.0.0",
        "touch",
        "client_",
        "\"tp\"",
        "\"cv\"",
    ] {
        assert!(!raw.contains(needle), "envelope leaks {needle}: {raw}");
    }
}

// verifies: REQ-COLLECT-X6WF.S2 (Macintosh with several touch points and a coarse pointer is a tablet)
#[tokio::test]
async fn ipad_in_desktop_mode_is_a_tablet() {
    let r = rig();
    let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";
    send(&r, post_ua("v::1|k::abc|tp::5|pc::1\nts::1|u::/", ua, &[])).await;
    assert_eq!(queued(&r)[0]["device"], "tablet");
}

// verifies: REQ-COLLECT-X6WF.S3 (another detector is used without changes to the tracker or the queue format)
#[tokio::test]
async fn another_detector_can_be_plugged_in() {
    use kurzhaar_core::detect::{Detection, Detector, DeviceClass, Signals};
    struct Fixed;
    impl Detector for Fixed {
        fn detect(&self, _: &Signals) -> Detection {
            Detection {
                browser: "Mine".into(),
                browser_major: Some(1),
                os: "MyOS".into(),
                os_major: None,
                device: DeviceClass::Other,
                bot: false,
            }
        }
    }
    let queue = Arc::new(MemoryQueue::new());
    let config = Config {
        sites: vec![Site {
            key: "abc".into(),
            id: "site-1".into(),
        }],
        ..Config::default()
    };
    let state =
        Arc::new(AppState::new(config, queue.clone(), Arc::new(Fixed), Arc::new(|| NOW)).unwrap());
    let r = Rig {
        app: public_router(state.clone()),
        queue,
        state,
    };
    send(&r, post(BATCH, "203.0.113.5")).await;
    assert_eq!(queued(&r)[0]["browser"], "Mine");
}

// verifies: REQ-COLLECT-1AYR.S1 (a crawler is flagged and is not device class other)
// verifies: REQ-COLLECT-1AYR.S2 (a headless browser is flagged)
// verifies: REQ-COLLECT-1AYR.S3 (an ordinary browser is not flagged)
#[tokio::test]
async fn bots_are_flagged_and_browsers_are_not() {
    let r = rig();
    send(
        &r,
        post_ua(
            BATCH,
            "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
            &[],
        ),
    )
    .await;
    send(&r, post_ua(BATCH, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/124.0.0.0 Safari/537.36", &[])).await;
    send(&r, post_ua(BATCH, WIN_CHROME, &[])).await;
    let q = queued(&r);
    assert_eq!(q[0]["bot"], true);
    assert_ne!(q[0]["device"], "other");
    assert_eq!(q[1]["bot"], true);
    assert_eq!(q[2]["bot"], false);
}

// verifies: REQ-COLLECT-X6WF.S1 (the Client Hints headers fill what the tracker did not send)
#[tokio::test]
async fn client_hint_headers_fill_missing_signals() {
    let r = rig();
    let hints = [
        (
            "sec-ch-ua",
            "\"Chromium\";v=\"124\", \"Microsoft Edge\";v=\"124\", \"Not-A.Brand\";v=\"99\"",
        ),
        ("sec-ch-ua-mobile", "?0"),
        ("sec-ch-ua-platform", "\"Windows\""),
    ];
    send(&r, post_ua(BATCH, WIN_CHROME, &hints)).await;
    let m = &queued(&r)[0];
    assert_eq!(m["browser"], "Edge");
    assert_eq!(m["os"], "Windows");
}
