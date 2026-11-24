//! Helpers for the tests that need a real Redis. Set KURZHAAR_TEST_REDIS_URL to a connection
//! string of an administrator (for example redis://:pw@127.0.0.1:56379). Without it the tests
//! are skipped, except in CI, where a missing variable fails them.
#![allow(dead_code)]

use redis::aio::MultiplexedConnection;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct Env {
    pub url: String,
    pub admin: MultiplexedConnection,
    /// A stream prefix no other test uses.
    pub prefix: String,
}

pub async fn env() -> Option<Env> {
    let Ok(url) = std::env::var("KURZHAAR_TEST_REDIS_URL") else {
        assert!(
            std::env::var("CI").is_err(),
            "KURZHAAR_TEST_REDIS_URL must be set in CI"
        );
        eprintln!("skipped: KURZHAAR_TEST_REDIS_URL is not set");
        return None;
    };
    let admin = redis::Client::open(url.as_str())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .expect("connect to the test Redis");
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    Some(Env {
        url,
        admin,
        prefix: format!("kz_test_{}_{n}:events", std::process::id()),
    })
}

impl Env {
    /// The URL for another account on the same server.
    pub fn url_for(&self, user: &str, password: &str) -> String {
        let (scheme, rest) = self.url.split_once("://").unwrap();
        let host = rest.rsplit_once('@').map_or(rest, |(_, h)| h);
        format!("{scheme}://{user}:{password}@{host}")
    }

    pub async fn xlen(&self, stream: &str) -> usize {
        redis::cmd("XLEN")
            .arg(stream)
            .query_async(&mut self.admin.clone())
            .await
            .unwrap()
    }

    /// All entries of a stream as (payload, key).
    pub async fn entries(&self, stream: &str) -> Vec<(Vec<u8>, String)> {
        let reply: Vec<(String, Vec<(String, redis::Value)>)> = redis::cmd("XRANGE")
            .arg(stream)
            .arg("-")
            .arg("+")
            .query_async(&mut self.admin.clone())
            .await
            .unwrap();
        reply
            .into_iter()
            .map(|(_, fields)| {
                let mut payload = Vec::new();
                let mut key = String::new();
                for (name, value) in fields {
                    match (name.as_str(), value) {
                        ("payload", redis::Value::BulkString(b)) => payload = b,
                        ("key", redis::Value::BulkString(b)) => key = String::from_utf8(b).unwrap(),
                        _ => {}
                    }
                }
                (payload, key)
            })
            .collect()
    }
}
