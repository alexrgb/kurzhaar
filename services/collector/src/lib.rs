//! The collector: accepts tracker batches, checks them and writes them to the queue (ADR-K8N9).

pub mod clientip;
pub mod config;
pub mod country;
pub mod envelope;
pub mod ratelimit;
pub mod signals;

use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, State};
use axum::http::{header, HeaderValue, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use config::Config;
use country::{CountryDb, Reload};
use ipnet::IpNet;
use kurzhaar_core::detect::Detector;
use kurzhaar_queue::Queue;
use ratelimit::RateLimiter;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Counters shown on the metrics listener. They carry no addresses or keys.
#[derive(Default)]
pub struct Metrics {
    pub accepted: AtomicU64,
    pub dropped: AtomicU64,
    pub malformed: AtomicU64,
    pub oversized: AtomicU64,
    pub rate_limited: AtomicU64,
    pub queue_errors: AtomicU64,
    /// Country database replacements that could not be opened (REQ-COLLECT-82NX).
    pub country_db_reload_failures: AtomicU64,
}

impl Metrics {
    pub fn render(&self) -> String {
        let rows = [
            ("accepted", &self.accepted),
            ("dropped", &self.dropped),
            ("malformed", &self.malformed),
            ("oversized", &self.oversized),
            ("rate_limited", &self.rate_limited),
            ("queue_errors", &self.queue_errors),
        ];
        let mut out = String::new();
        for (name, v) in rows {
            out.push_str(&format!(
                "# TYPE kurzhaar_collector_batches_{name}_total counter\nkurzhaar_collector_batches_{name}_total {}\n",
                v.load(Ordering::Relaxed)
            ));
        }
        out.push_str(&format!(
            "# TYPE kurzhaar_collector_country_db_reload_failures_total counter\nkurzhaar_collector_country_db_reload_failures_total {}\n",
            self.country_db_reload_failures.load(Ordering::Relaxed)
        ));
        out
    }
}

pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

pub fn system_clock() -> Clock {
    Arc::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    })
}

pub struct AppState {
    pub config: Config,
    pub sites: HashMap<String, String>,
    pub trusted: Vec<IpNet>,
    pub queue: Arc<dyn Queue>,
    pub detector: Arc<dyn Detector>,
    pub country: Arc<CountryDb>,
    pub limiter: RateLimiter,
    pub metrics: Arc<Metrics>,
    pub clock: Clock,
}

impl AppState {
    pub fn new(
        config: Config,
        queue: Arc<dyn Queue>,
        detector: Arc<dyn Detector>,
        clock: Clock,
    ) -> Result<AppState, String> {
        config.validate()?;
        Ok(AppState {
            sites: config
                .sites
                .iter()
                .map(|s| (s.key.clone(), s.id.clone()))
                .collect(),
            trusted: config.trusted()?,
            limiter: RateLimiter::new(
                config.rate_limit_per_minute,
                config.rate_limit_burst,
                config.rate_limit_max_entries,
            ),
            metrics: Arc::new(Metrics::default()),
            config,
            queue,
            detector,
            country: Arc::new(CountryDb::disabled()),
            clock,
        })
    }

    /// Uses this country database instead of none (REQ-COLLECT-82NX).
    pub fn with_country(mut self, country: Arc<CountryDb>) -> AppState {
        self.country = country;
        self
    }
}

/// Checks the country file once and counts a failed replacement; the caller runs this on a
/// timer (REQ-COLLECT-82NX).
pub fn refresh_country(db: &CountryDb, metrics: &Metrics) -> Reload {
    let result = db.reload_if_changed();
    if let Reload::Failed(reason) = &result {
        metrics
            .country_db_reload_failures
            .fetch_add(1, Ordering::Relaxed);
        tracing::error!(%reason, "country database not replaced, keeping the previous one");
    }
    result
}

/// The public listener: `/v1/collect`, `/healthz`, `/readyz`. `/metrics` is not here
/// (REQ-COLLECT-X2K2).
pub fn public_router(state: Arc<AppState>) -> Router {
    let collect = Router::new()
        .route("/v1/collect", post(collect).options(preflight))
        .layer(middleware::from_fn(allow_any_origin));
    Router::new()
        .merge(collect)
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/readyz", get(ready))
        .with_state(state)
}

/// The metrics listener, meant for the operator's network only.
pub fn metrics_router(metrics: Arc<Metrics>) -> Router {
    Router::new().route("/metrics", get(move || async move { metrics.render() }))
}

/// `Access-Control-Allow-Origin: *`, never credentials, never a cookie (REQ-COLLECT-D9G1).
// spec: REQ-COLLECT-D9G1
async fn allow_any_origin(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    res
}

async fn preflight() -> Response {
    (
        StatusCode::NO_CONTENT,
        [
            (header::ACCESS_CONTROL_ALLOW_METHODS, "POST"),
            (header::ACCESS_CONTROL_ALLOW_HEADERS, "content-type"),
            (header::ACCESS_CONTROL_MAX_AGE, "86400"),
        ],
    )
        .into_response()
}

// spec: REQ-COLLECT-X2K2
async fn ready(State(s): State<Arc<AppState>>) -> StatusCode {
    match s.queue.ping().await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

fn retry_after(status: StatusCode, seconds: u64) -> Response {
    (status, [(header::RETRY_AFTER, seconds.to_string())]).into_response()
}

// spec: REQ-COLLECT-C50Y, REQ-COLLECT-Y37S, REQ-COLLECT-4VTR, REQ-COLLECT-FWT0, REQ-COLLECT-FRDS
async fn collect(State(s): State<Arc<AppState>>, req: Request<Body>) -> Response {
    let now = (s.clock)();
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or(IpAddr::from([0, 0, 0, 0]), |c| c.0.ip());
    let forwarded = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok());
    let client = clientip::resolve(peer, forwarded, &s.trusted);
    let headers = req.headers().clone();

    if let Err(wait) = s.limiter.check(client, now) {
        s.metrics.rate_limited.fetch_add(1, Ordering::Relaxed);
        return retry_after(StatusCode::TOO_MANY_REQUESTS, wait);
    }

    let body = match read_body(req.into_body(), s.config.max_body_bytes).await {
        Ok(b) => b,
        Err(status) => {
            s.metrics.oversized.fetch_add(1, Ordering::Relaxed);
            return status.into_response();
        }
    };
    let malformed = || {
        s.metrics.malformed.fetch_add(1, Ordering::Relaxed);
        StatusCode::BAD_REQUEST.into_response()
    };
    let Ok(text) = std::str::from_utf8(&body) else {
        return malformed();
    };
    let Ok(batch) = kurzhaar_protocol::decode(text) else {
        return malformed();
    };
    if batch.events.is_empty() || batch.events.len() > s.config.max_events_per_batch {
        return malformed();
    }

    // An unknown or disabled site is answered like an accepted batch, so the answer never
    // reveals which keys exist (REQ-COLLECT-4VTR).
    let key = batch.key.clone().unwrap_or_default();
    let Some(site) = s.sites.get(&key) else {
        s.metrics.dropped.fetch_add(1, Ordering::Relaxed);
        return StatusCode::ACCEPTED.into_response();
    };

    // The User-Agent and the signals are used here and dropped with the request (REQ-COLLECT-RA57).
    let detection = s.detector.detect(&signals::from_request(&batch, &headers));
    let country = s.country.lookup(client);
    let token = envelope::token(&key, &body);
    let message = envelope::build(batch, site, token, now, detection, country);
    let payload = match serde_json::to_vec(&message) {
        Ok(p) => p,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    // Acknowledge only after the queue has the batch (REQ-COLLECT-Y37S).
    match s.queue.publish(site, payload).await {
        Ok(()) => {
            s.metrics.accepted.fetch_add(1, Ordering::Relaxed);
            StatusCode::ACCEPTED.into_response()
        }
        Err(_) => {
            s.metrics.queue_errors.fetch_add(1, Ordering::Relaxed);
            retry_after(StatusCode::SERVICE_UNAVAILABLE, 5)
        }
    }
}

async fn read_body(body: Body, limit: usize) -> Result<axum::body::Bytes, StatusCode> {
    to_bytes(body, limit)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)
}
