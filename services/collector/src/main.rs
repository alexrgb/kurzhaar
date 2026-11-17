use kurzhaar_collector::config::Config;
use kurzhaar_collector::country::CountryDb;
use kurzhaar_collector::{metrics_router, public_router, refresh_country, system_clock, AppState};
use kurzhaar_core::detect::{CachedDetector, UaparserDetector};
use kurzhaar_queue::MemoryQueue;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();

    // The file path comes from the first argument or KURZHAAR_COLLECTOR_CONFIG.
    let path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("KURZHAAR_COLLECTOR_CONFIG").ok());
    let file = path
        .as_deref()
        .map(|p| std::fs::read_to_string(p).map_err(|e| format!("cannot read {p}: {e}")));
    let file = match file.transpose() {
        Ok(f) => f,
        Err(e) => fail(&e),
    };
    let config = match Config::load(file.as_deref(), |k| std::env::var(k).ok()) {
        Ok(c) => c,
        Err(e) => fail(&e),
    };

    let public: SocketAddr = config.listen.parse().expect("validated");
    let metrics: SocketAddr = config.metrics_listen.parse().expect("validated");
    // The queue backends arrive with their own changes; until then batches stay in memory.
    tracing::warn!("using the in-memory queue: accepted batches are not delivered anywhere");
    let detector = match UaparserDetector::builtin() {
        Ok(d) => CachedDetector::new(
            d,
            config.detector_cache_entries,
            std::time::Duration::from_secs(config.detector_cache_seconds),
        ),
        Err(e) => fail(&e),
    };
    let country = match &config.country_db {
        Some(path) => match CountryDb::open(path) {
            Ok(db) => Arc::new(db),
            Err(e) => fail(&format!("country_db: {e}")),
        },
        None => {
            tracing::warn!("no country database configured: events carry no country");
            Arc::new(CountryDb::disabled())
        }
    };
    let state = match AppState::new(
        config,
        Arc::new(MemoryQueue::new()),
        Arc::new(detector),
        system_clock(),
    ) {
        Ok(s) => Arc::new(s.with_country(country.clone())),
        Err(e) => fail(&e),
    };
    if country.is_enabled() {
        let metrics = state.metrics.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tick.tick().await;
                refresh_country(&country, &metrics);
            }
        });
    }

    let metrics_app = metrics_router(state.metrics.clone());
    let metrics_listener = tokio::net::TcpListener::bind(metrics)
        .await
        .unwrap_or_else(|e| fail(&format!("metrics_listen: {e}")));
    tokio::spawn(async move {
        let _ = axum::serve(metrics_listener, metrics_app).await;
    });

    let listener = tokio::net::TcpListener::bind(public)
        .await
        .unwrap_or_else(|e| fail(&format!("listen: {e}")));
    tracing::info!(%public, %metrics, "collector started");
    axum::serve(
        listener,
        public_router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .expect("server");
}

fn fail(message: &str) -> ! {
    eprintln!("collector: {message}");
    std::process::exit(1)
}
