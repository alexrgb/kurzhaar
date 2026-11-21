//! Tests against a real Postgres. Set KURZHAAR_TEST_POSTGRES_URL to a superuser connection
//! string (for example postgres://postgres:pw@127.0.0.1:55432/postgres). Without it the tests
//! are skipped, except in CI, where a missing variable fails them.

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use kurzhaar_collector::config::{Config, Secret};
use kurzhaar_collector::postgres::{self, Settings, Targets, STATEMENTS};
use kurzhaar_collector::registry::Registry;
use kurzhaar_collector::session::{PgSalts, SaltSource};
use kurzhaar_collector::{public_router, AppState, Clock};
use kurzhaar_core::detect::{Detection, Detector, DeviceClass, Signals};
use kurzhaar_queue::MemoryQueue;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::{Client, NoTls};
use tower::ServiceExt;

const NOW: u64 = 1_800_000_000_000;
const DAY: u64 = 86_400_000;
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

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A database of its own with the two views, and a role that can only read them.
struct Db {
    admin: Client,
    admin_url: String,
    name: String,
    role: String,
}

async fn connect(url: &str) -> Client {
    let (client, connection) = tokio_postgres::connect(url, NoTls).await.expect("connect");
    tokio::spawn(connection);
    client
}

fn url_for(admin_url: &str, user: &str, database: &str) -> String {
    let c: tokio_postgres::Config = admin_url.parse().unwrap();
    let host = match &c.get_hosts()[0] {
        tokio_postgres::config::Host::Tcp(h) => h.clone(),
        _ => panic!("tcp host expected"),
    };
    format!(
        "postgres://{user}:pw@{host}:{}/{database}",
        c.get_ports()[0]
    )
}

impl Db {
    async fn setup() -> Option<Db> {
        let Ok(admin_url) = std::env::var("KURZHAAR_TEST_POSTGRES_URL") else {
            assert!(
                std::env::var("CI").is_err(),
                "KURZHAAR_TEST_POSTGRES_URL must be set in CI"
            );
            eprintln!("skipped: KURZHAAR_TEST_POSTGRES_URL is not set");
            return None;
        };
        let n = format!(
            "{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let name = format!("kz_test_{n}");
        let role = format!("kz_reader_{n}");
        let server = connect(&admin_url).await;
        // CREATE DATABASE cannot share a transaction with other statements.
        server
            .batch_execute(&format!("CREATE DATABASE {name}"))
            .await
            .unwrap();
        server
            .batch_execute(&format!(
                "CREATE ROLE {role} LOGIN PASSWORD 'pw';
                 GRANT CONNECT ON DATABASE {name} TO {role};"
            ))
            .await
            .unwrap();
        let admin = connect(&url_for(&admin_url, "postgres", &name)).await;
        admin
            .batch_execute(&format!(
                "CREATE TABLE sites (id text PRIMARY KEY, enabled boolean NOT NULL DEFAULT true);
                 CREATE TABLE site_keys (key text PRIMARY KEY, site_id text NOT NULL REFERENCES sites(id));
                 CREATE UNLOGGED TABLE session_salts (day date PRIMARY KEY, salt bytea NOT NULL);
                 CREATE VIEW collector_site_keys AS
                   SELECT k.key, k.site_id FROM site_keys k JOIN sites s ON s.id = k.site_id WHERE s.enabled;
                 CREATE VIEW collector_salts AS SELECT day, salt FROM session_salts;
                 REVOKE CREATE ON SCHEMA public FROM PUBLIC;
                 GRANT USAGE ON SCHEMA public TO {role};
                 GRANT SELECT ON collector_site_keys, collector_salts TO {role};"
            ))
            .await
            .unwrap();
        let db = Db {
            admin,
            admin_url,
            name,
            role,
        };
        db.add_site("site-1", "abc").await;
        Some(db)
    }

    fn url(&self) -> String {
        url_for(&self.admin_url, &self.role, &self.name)
    }

    async fn add_site(&self, id: &str, key: &str) {
        self.admin
            .execute(
                "INSERT INTO sites (id) VALUES ($1) ON CONFLICT DO NOTHING",
                &[&id],
            )
            .await
            .unwrap();
        self.admin
            .execute("INSERT INTO site_keys VALUES ($1, $2)", &[&key, &id])
            .await
            .unwrap();
    }

    async fn notify(&self) {
        self.admin
            .batch_execute("NOTIFY kurzhaar_sites")
            .await
            .unwrap();
    }

    async fn set_salt(&self, day: u64, byte: u8) {
        self.admin
            .execute(
                "INSERT INTO session_salts VALUES (DATE '1970-01-01' + $1::int, $2)
                 ON CONFLICT (day) DO UPDATE SET salt = EXCLUDED.salt",
                &[&(day as i32), &vec![byte; 32]],
            )
            .await
            .unwrap();
    }

    /// Cuts the reader off: ends its connections and refuses new ones.
    async fn outage(&self) {
        let (name, role) = (&self.name, &self.role);
        let server = connect(&url_for(&self.admin_url, "postgres", "postgres")).await;
        server
            .batch_execute(&format!(
                "REVOKE CONNECT ON DATABASE {name} FROM {role};
                 SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE usename = '{role}';"
            ))
            .await
            .unwrap();
    }

    async fn recover(&self) {
        let server = connect(&url_for(&self.admin_url, "postgres", "postgres")).await;
        server
            .batch_execute(&format!(
                "GRANT CONNECT ON DATABASE {} TO {}",
                self.name, self.role
            ))
            .await
            .unwrap();
    }
}

struct Rig {
    app: axum::Router,
    state: Arc<AppState>,
    queue: Arc<MemoryQueue>,
    salts: Arc<PgSalts>,
    now: Arc<AtomicU64>,
    targets: Targets,
}

fn rig(grace_seconds: u64) -> Rig {
    let now = Arc::new(AtomicU64::new(NOW));
    let clock: Clock = {
        let now = now.clone();
        Arc::new(move || now.load(Ordering::Relaxed))
    };
    let queue = Arc::new(MemoryQueue::new());
    let state = AppState::new(
        Config::default(),
        queue.clone(),
        Arc::new(Nothing),
        clock.clone(),
    )
    .unwrap();
    let registry = Arc::new(Registry::loaded_from_postgres(grace_seconds * 1000));
    let salts = Arc::new(PgSalts::new(state.metrics.salt_fallbacks.clone()));
    let targets = Targets {
        registry: registry.clone(),
        salts: salts.clone(),
        metrics: state.metrics.clone(),
        clock,
    };
    let state = Arc::new(
        state
            .with_registry(registry.clone())
            .with_salts(salts.clone()),
    );
    Rig {
        app: public_router(state.clone()),
        state,
        queue,
        salts,
        now,
        targets,
    }
}

fn settings(url: String, refresh_ms: u64) -> Settings {
    Settings {
        url: Secret::new(url),
        tls: false,
        refresh: Duration::from_millis(refresh_ms),
    }
}

async fn post(r: &Rig, key: &str) -> (StatusCode, bool) {
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/v1/collect")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(BATCH.replace("k::abc", &format!("k::{key}"))))
        .unwrap();
    let addr: SocketAddr = "192.0.2.7:4000".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    let res = r.app.clone().oneshot(req).await.unwrap();
    (
        res.status(),
        res.headers().contains_key(header::RETRY_AFTER),
    )
}

async fn readyz(r: &Rig) -> StatusCode {
    let req = Request::builder()
        .uri("/readyz")
        .body(Body::empty())
        .unwrap();
    r.app.clone().oneshot(req).await.unwrap().status()
}

async fn eventually<F: std::future::Future<Output = bool>>(mut check: impl FnMut() -> F) {
    for _ in 0..100 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the condition did not hold within five seconds");
}

// verifies: REQ-COLLECT-EWYP.S1 (a key in the loaded snapshot is accepted, an unknown one is dropped)
#[tokio::test]
async fn known_key_is_accepted_and_unknown_is_dropped() {
    let Some(db) = Db::setup().await else { return };
    let r = rig(3600);
    postgres::start(settings(db.url(), 60_000), r.targets.clone())
        .await
        .unwrap();
    assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
    assert_eq!(r.queue.messages().len(), 1);
    assert_eq!(post(&r, "nope").await.0, StatusCode::ACCEPTED);
    assert_eq!(r.queue.messages().len(), 1);
}

// verifies: REQ-COLLECT-EWYP.S2 (batches cause no load from Postgres)
#[tokio::test]
async fn requests_do_not_query_postgres() {
    let Some(db) = Db::setup().await else { return };
    let r = rig(3600);
    postgres::start(settings(db.url(), 60_000), r.targets.clone())
        .await
        .unwrap();
    let loads = r.state.metrics.registry_loads.load(Ordering::Relaxed);
    assert!(loads >= 1);
    for _ in 0..30 {
        assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
        assert_eq!(post(&r, "unknown").await.0, StatusCode::ACCEPTED);
    }
    assert_eq!(
        r.state.metrics.registry_loads.load(Ordering::Relaxed),
        loads
    );
}

// verifies: REQ-COLLECT-EWYP.S3 (a new site is accepted after a notification, with no polling in between)
#[tokio::test]
async fn notification_makes_a_new_site_known() {
    let Some(db) = Db::setup().await else { return };
    let r = rig(3600);
    postgres::start(settings(db.url(), 60_000), r.targets.clone())
        .await
        .unwrap();
    assert_eq!(post(&r, "fresh").await.0, StatusCode::ACCEPTED);
    let before = r.queue.messages().len();
    db.add_site("site-2", "fresh").await;
    db.notify().await;
    eventually(|| async {
        post(&r, "fresh").await;
        r.queue.messages().len() > before
    })
    .await;
}

// verifies: REQ-COLLECT-EWYP.S4 (without a notification the next periodic load drops a disabled site)
#[tokio::test]
async fn periodic_load_covers_a_lost_notification() {
    let Some(db) = Db::setup().await else { return };
    let r = rig(3600);
    postgres::start(settings(db.url(), 200), r.targets.clone())
        .await
        .unwrap();
    assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
    let before = r.queue.messages().len();
    db.admin
        .execute("UPDATE sites SET enabled = false WHERE id = 'site-1'", &[])
        .await
        .unwrap();
    eventually(|| async {
        let n = r.queue.messages().len();
        post(&r, "abc").await;
        r.queue.messages().len() == n && n >= before
    })
    .await;
    // The configured interval can never exceed a minute.
    let env =
        |k: &str| (k == "KURZHAAR_COLLECTOR_REGISTRY_REFRESH_SECONDS").then(|| "61".to_string());
    assert!(Config::load(None, env).is_err());
    let env =
        |k: &str| (k == "KURZHAAR_COLLECTOR_REGISTRY_REFRESH_SECONDS").then(|| "60".to_string());
    assert!(Config::load(None, env).is_ok());
}

// verifies: REQ-COLLECT-EWYP.S5 (batches during constant reloads are all accepted, none fails)
#[tokio::test]
async fn reload_never_fails_a_batch() {
    let Some(db) = Db::setup().await else { return };
    let r = rig(3600);
    postgres::start(settings(db.url(), 20), r.targets.clone())
        .await
        .unwrap();
    let loads = r.state.metrics.registry_loads.load(Ordering::Relaxed);
    let mut seen = 0;
    while r.state.metrics.registry_loads.load(Ordering::Relaxed) < loads + 10 {
        assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
        seen += 1;
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert_eq!(r.queue.messages().len(), seen);
}

// verifies: REQ-COLLECT-THCX.S1 (Postgres goes away: known keys still accepted, readiness green)
// verifies: REQ-COLLECT-THCX.S2 (past the grace period readiness is red, the stale snapshot still serves)
// verifies: REQ-COLLECT-THCX.S4 (Postgres returns: the next load makes the collector ready again)
#[tokio::test]
async fn outage_grace_period_and_recovery() {
    let Some(db) = Db::setup().await else { return };
    let r = rig(3600);
    postgres::start(settings(db.url(), 100), r.targets.clone())
        .await
        .unwrap();
    assert_eq!(readyz(&r).await, StatusCode::OK);

    db.outage().await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    r.now.fetch_add(10 * 60 * 1000, Ordering::Relaxed);
    assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
    assert_eq!(r.queue.messages().len(), 1);
    assert_eq!(readyz(&r).await, StatusCode::OK);

    r.now.fetch_add(60 * 60 * 1000, Ordering::Relaxed);
    assert_eq!(readyz(&r).await, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
    assert_eq!(r.queue.messages().len(), 2);
    assert!(
        r.state
            .metrics
            .registry_load_failures
            .load(Ordering::Relaxed)
            >= 1
    );

    db.recover().await;
    eventually(|| async { readyz(&r).await == StatusCode::OK }).await;
}

// verifies: REQ-COLLECT-THCX.S3 (started without Postgres: not ready, 503 with Retry-After, nothing enqueued)
#[tokio::test]
async fn start_without_postgres_answers_503() {
    let r = rig(3600);
    // Nothing listens on this port.
    postgres::start(
        settings("postgres://reader:pw@127.0.0.1:1/none".into(), 60_000),
        r.targets.clone(),
    )
    .await
    .unwrap();
    assert_eq!(readyz(&r).await, StatusCode::SERVICE_UNAVAILABLE);
    let (status, retry) = post(&r, "abc").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(retry);
    assert!(r.queue.messages().is_empty());
    assert!(r.state.metrics.registry_unavailable.load(Ordering::Relaxed) >= 1);
}

// verifies: REQ-COLLECT-Y61T.S1 (a role that can write, create or is a superuser stops the start)
#[tokio::test]
async fn role_with_more_than_read_is_refused() {
    let Some(db) = Db::setup().await else { return };
    let start = |url: String| {
        let r = rig(3600);
        async move { postgres::start(settings(url, 60_000), r.targets.clone()).await }
    };
    assert!(start(db.url()).await.is_ok());

    let role = &db.role;
    db.admin
        .batch_execute(&format!("GRANT INSERT ON sites TO {role}"))
        .await
        .unwrap();
    let e = start(db.url()).await.unwrap_err();
    assert!(e.contains("write"), "{e}");
    db.admin
        .batch_execute(&format!("REVOKE INSERT ON sites FROM {role}"))
        .await
        .unwrap();

    db.admin
        .batch_execute(&format!("GRANT CREATE ON SCHEMA public TO {role}"))
        .await
        .unwrap();
    let e = start(db.url()).await.unwrap_err();
    assert!(e.contains("create"), "{e}");
    db.admin
        .batch_execute(&format!("REVOKE CREATE ON SCHEMA public FROM {role}"))
        .await
        .unwrap();

    let e = start(url_for(&db.admin_url, "postgres", &db.name))
        .await
        .unwrap_err();
    assert!(e.contains("superuser"), "{e}");
}

// verifies: REQ-COLLECT-Y61T.S2 (every statement the collector sends is a read or a LISTEN)
#[test]
fn collector_sends_only_reads() {
    for sql in STATEMENTS {
        let first = sql.split_whitespace().next().unwrap().to_ascii_uppercase();
        assert!(first == "SELECT" || first == "LISTEN", "{sql}");
        for word in [
            "INSERT ",
            "UPDATE ",
            "DELETE ",
            "CREATE ",
            "DROP ",
            "ALTER ",
            "TRUNCATE ",
        ] {
            assert!(!sql.to_ascii_uppercase().contains(word), "{sql}");
        }
    }
}

// verifies: REQ-COLLECT-Y61T.S3 (postgres_url in the file stops the start; the environment is accepted and hidden)
#[test]
fn connection_string_belongs_in_the_environment() {
    let e = Config::load(Some("postgres_url = \"postgres://u:secret@h/db\""), |_| {
        None
    })
    .unwrap_err();
    assert!(e.contains("KURZHAAR_COLLECTOR_POSTGRES_URL"), "{e}");
    assert!(!e.contains("secret"));
    let env = |k: &str| {
        (k == "KURZHAAR_COLLECTOR_POSTGRES_URL").then(|| "postgres://u:secret@h/db".to_string())
    };
    let c = Config::load(None, env).unwrap();
    assert!(c.postgres_url.is_some());
    assert!(!format!("{c:?}").contains("secret"));
}

// verifies: REQ-COLLECT-8ERQ.S1 (two collectors reading the same salts give one visitor one session)
// verifies: REQ-COLLECT-8ERQ.S2 (the next day's salt, already loaded, is used from midnight)
#[tokio::test]
async fn replicas_share_salts_and_the_next_day_is_ready() {
    let Some(db) = Db::setup().await else { return };
    let today = NOW / DAY;
    db.set_salt(today, 1).await;
    db.set_salt(today + 1, 2).await;
    let a = rig(3600);
    let b = rig(3600);
    postgres::start(settings(db.url(), 60_000), a.targets.clone())
        .await
        .unwrap();
    postgres::start(settings(db.url(), 60_000), b.targets.clone())
        .await
        .unwrap();
    let t = today * DAY + 5;
    assert_eq!(*a.salts.salt(t), [1; 32]);
    assert_eq!(*a.salts.salt(t), *b.salts.salt(t));
    let midnight = (today + 1) * DAY;
    assert_eq!(*a.salts.salt(midnight), [2; 32]);
    assert_eq!(a.state.metrics.salt_fallbacks.load(Ordering::Relaxed), 0);
}

// verifies: REQ-COLLECT-8ERQ.S3 (no salt for today: a random one, logged once, counted, batches accepted)
#[tokio::test]
async fn missing_salt_falls_back_to_a_random_one() {
    let Some(db) = Db::setup().await else { return };
    let r = rig(3600);
    postgres::start(settings(db.url(), 60_000), r.targets.clone())
        .await
        .unwrap();
    assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
    assert_eq!(post(&r, "abc").await.0, StatusCode::ACCEPTED);
    assert_eq!(r.state.metrics.salt_fallbacks.load(Ordering::Relaxed), 1);
    assert_eq!(
        r.queue.messages()[0].partition_key,
        r.queue.messages()[1].partition_key
    );
    assert!(r
        .state
        .metrics
        .render()
        .contains("kurzhaar_collector_salt_fallback_total 1"));
}

// verifies: REQ-COLLECT-8ERQ.S4 (a salt deleted in Postgres goes from memory; so does one past its grace)
#[tokio::test]
async fn deleted_salt_leaves_memory() {
    let Some(db) = Db::setup().await else { return };
    let today = NOW / DAY;
    db.set_salt(today, 1).await;
    let r = rig(3600);
    postgres::start(settings(db.url(), 100), r.targets.clone())
        .await
        .unwrap();
    assert_eq!(r.salts.held_days(), vec![today]);
    db.admin
        .execute("DELETE FROM session_salts", &[])
        .await
        .unwrap();
    eventually(|| async { r.salts.held_days().is_empty() }).await;

    // And a salt that stays in Postgres is still dropped ten minutes after its day.
    db.set_salt(today, 1).await;
    eventually(|| async { r.salts.held_days() == vec![today] }).await;
    r.salts.prune((today + 1) * DAY + 10 * 60 * 1000);
    assert!(r.salts.held_days().is_empty());
}
