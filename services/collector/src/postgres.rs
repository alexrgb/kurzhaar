// spec: REQ-COLLECT-EWYP, REQ-COLLECT-THCX, REQ-COLLECT-Y61T, REQ-COLLECT-8ERQ
//! Reads the site registry and the daily salts from Postgres through two views and keeps the
//! collector's in-memory copies current (ADR-DW71). It only reads: `SELECT`, `LISTEN` and a
//! check of its own privileges.

use crate::config::Secret;
use crate::registry::Registry;
use crate::session::{PgSalts, Salt};
use crate::{Clock, Metrics};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio_postgres::{AsyncMessage, Client, Config, Connection};
use zeroize::Zeroizing;

/// Everything this module sends to Postgres. A test checks that none of it writes.
pub const STATEMENTS: [&str; 4] = [PRIVILEGES, LISTEN, SITES, SALTS];

const LISTEN: &str = "LISTEN kurzhaar_sites";
const SITES: &str = "SELECT key::text, site_id::text FROM collector_site_keys";
const SALTS: &str = "SELECT (day - DATE '1970-01-01')::bigint, salt FROM collector_salts";
/// Can the role write to or create anything it can see; is it a superuser.
const PRIVILEGES: &str = "SELECT \
    COALESCE((SELECT bool_or(has_table_privilege(c.oid, 'INSERT,UPDATE,DELETE,TRUNCATE')) \
        FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
        WHERE c.relkind IN ('r','p','v','m','f') \
        AND n.nspname NOT IN ('pg_catalog','information_schema') AND n.nspname !~ '^pg_'), false), \
    COALESCE((SELECT bool_or(has_schema_privilege(n.oid, 'CREATE')) FROM pg_namespace n \
        WHERE n.nspname NOT IN ('pg_catalog','information_schema') AND n.nspname !~ '^pg_'), false) \
        OR has_database_privilege(current_database(), 'CREATE'), \
    COALESCE((SELECT rolsuper FROM pg_roles WHERE rolname = current_user), false)";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const QUERY_TIMEOUT: Duration = Duration::from_secs(15);

pub struct Settings {
    pub url: Secret,
    pub tls: bool,
    /// Time between reloads when no notification arrives.
    pub refresh: Duration,
}

/// What the loader keeps current.
#[derive(Clone)]
pub struct Targets {
    pub registry: Arc<Registry>,
    pub salts: Arc<PgSalts>,
    pub metrics: Arc<Metrics>,
    pub clock: Clock,
}

enum OpenError {
    /// The role can do more than read: the collector must not run with it.
    Privileges(String),
    Unreachable(String),
}

enum Event {
    Notify,
    Closed,
}

struct Session {
    client: Client,
    events: UnboundedReceiver<Event>,
}

fn tls_config() -> Result<tokio_postgres_rustls::MakeRustlsConnect, String> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        // A certificate the library cannot parse is skipped; the others still count.
        let _ = roots.add(cert);
    }
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| format!("tls: {e}"))?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(tokio_postgres_rustls::MakeRustlsConnect::new(config))
}

/// Drives the connection and turns its messages into events.
fn drive<S, T>(mut connection: Connection<S, T>) -> UnboundedReceiver<Event>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    T: tokio_postgres::tls::TlsStream + Unpin + Send + 'static,
{
    let (tx, rx) = unbounded_channel();
    tokio::spawn(async move {
        let mut messages = futures_util::stream::poll_fn(move |cx| connection.poll_message(cx));
        while let Some(message) = messages.next().await {
            match message {
                Ok(AsyncMessage::Notification(_)) => {
                    let _ = tx.send(Event::Notify);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = tx.send(Event::Closed);
    });
    rx
}

async fn open(settings: &Settings) -> Result<Session, OpenError> {
    let unreachable = |e: tokio_postgres::Error| OpenError::Unreachable(e.to_string());
    let mut config: Config = settings
        .url
        .expose()
        .parse()
        .map_err(|e: tokio_postgres::Error| OpenError::Unreachable(e.to_string()))?;
    config.connect_timeout(CONNECT_TIMEOUT);
    let (client, events) = if settings.tls {
        let tls = tls_config().map_err(OpenError::Unreachable)?;
        let (client, connection) = config.connect(tls).await.map_err(unreachable)?;
        (client, drive(connection))
    } else {
        let (client, connection) = config
            .connect(tokio_postgres::NoTls)
            .await
            .map_err(unreachable)?;
        (client, drive(connection))
    };

    let row = client
        .query_one(PRIVILEGES, &[])
        .await
        .map_err(unreachable)?;
    let (can_write, can_create, superuser): (bool, bool, bool) =
        (row.get(0), row.get(1), row.get(2));
    for (found, what) in [
        (superuser, "is a superuser"),
        (can_write, "can write to a table or view"),
        (can_create, "can create objects"),
    ] {
        if found {
            return Err(OpenError::Privileges(format!(
                "the Postgres role {what}; use a role that can only read collector_site_keys and collector_salts"
            )));
        }
    }
    client.batch_execute(LISTEN).await.map_err(unreachable)?;
    Ok(Session { client, events })
}

impl Session {
    /// Reads both views and replaces the snapshots.
    async fn reload(&self, t: &Targets) -> Result<(), String> {
        let load = async {
            let sites = self
                .client
                .query(SITES, &[])
                .await
                .map_err(|e| e.to_string())?;
            let salts = self
                .client
                .query(SALTS, &[])
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((sites, salts))
        };
        let (sites, salts) = tokio::time::timeout(QUERY_TIMEOUT, load)
            .await
            .map_err(|_| "the query timed out".to_string())??;
        let now = (t.clock)();
        let sites: HashMap<String, String> = sites
            .iter()
            .map(|r| (r.get::<_, String>(0), r.get::<_, String>(1)))
            .collect();
        let salts: Vec<(u64, Salt)> = salts
            .iter()
            .filter_map(|r| {
                let day: i64 = r.get(0);
                let bytes: Vec<u8> = r.get(1);
                let salt: [u8; 32] = bytes.as_slice().try_into().ok().or_else(|| {
                    tracing::error!(day, "a salt in Postgres is not 32 bytes, ignored");
                    None
                })?;
                Some((u64::try_from(day).ok()?, Zeroizing::new(salt)))
            })
            .collect();
        t.registry.replace(sites, now);
        t.salts.replace(salts, now);
        t.metrics.registry_loads.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn failed(t: &Targets, what: &str) {
    t.metrics
        .registry_load_failures
        .fetch_add(1, Ordering::Relaxed);
    tracing::error!(reason = what, "cannot read the site registry from Postgres");
}

/// Connects once. A role with write privileges stops the start; an unreachable Postgres does
/// not, the collector then waits for it and answers 503 until the first load (REQ-COLLECT-THCX).
/// After that the loader keeps running in the background.
pub async fn start(settings: Settings, targets: Targets) -> Result<(), String> {
    let first = match open(&settings).await {
        Ok(session) => {
            if let Err(e) = session.reload(&targets).await {
                failed(&targets, &e);
            }
            Some(session)
        }
        Err(OpenError::Privileges(e)) => return Err(e),
        Err(OpenError::Unreachable(e)) => {
            failed(&targets, &e);
            None
        }
    };
    tokio::spawn(run(settings, targets, first));
    Ok(())
}

async fn run(settings: Settings, targets: Targets, mut session: Option<Session>) {
    let mut backoff = Duration::from_secs(1);
    loop {
        let mut current = match session.take() {
            Some(s) => s,
            None => match open(&settings).await {
                Ok(s) => {
                    backoff = Duration::from_secs(1);
                    s
                }
                Err(OpenError::Privileges(e) | OpenError::Unreachable(e)) => {
                    failed(&targets, &e);
                    targets.salts.prune((targets.clock)());
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(settings.refresh);
                    continue;
                }
            },
        };
        loop {
            if let Err(e) = current.reload(&targets).await {
                failed(&targets, &e);
                break;
            }
            tokio::select! {
                event = current.events.recv() => match event {
                    Some(Event::Notify) => while current.events.try_recv().is_ok() {},
                    Some(Event::Closed) | None => {
                        failed(&targets, "the connection closed");
                        break;
                    }
                },
                _ = tokio::time::sleep(settings.refresh) => {}
            }
            targets.salts.prune((targets.clock)());
        }
        targets.salts.prune((targets.clock)());
        tokio::time::sleep(backoff).await;
    }
}
