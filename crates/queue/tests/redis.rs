mod support;

use kurzhaar_queue::redis::RedisQueue;
use kurzhaar_queue::Queue;
use std::time::{Duration, Instant};
use support::env;

// verifies: REQ-QUEUE-GE19.S1 (an entry with the payload bytes and the key is in the stream)
#[tokio::test]
async fn message_is_in_the_stream() {
    let Some(e) = env().await else { return };
    let q = RedisQueue::new(&e.url, &e.prefix, 1).unwrap();
    let payload = vec![0u8, 1, 2, 255, b'{', b'\n'];
    q.publish("session-1", payload.clone()).await.unwrap();
    let stream = q.stream_for("session-1");
    assert_eq!(
        e.entries(&stream).await,
        vec![(payload, "session-1".to_string())]
    );
    q.ping().await.unwrap();
}

// verifies: REQ-QUEUE-GE19.S2 (Redis cannot be reached: the write fails and readiness is red)
// verifies: REQ-QUEUE-GE19.S5 (no answer to PING: not ready)
#[tokio::test]
async fn unreachable_redis_fails_the_write_and_readiness() {
    // Nothing listens on this port.
    let q = RedisQueue::new("redis://:pw@127.0.0.1:1", "kz:events", 1).unwrap();
    let start = Instant::now();
    assert!(q.publish("k", b"x".to_vec()).await.is_err());
    assert!(q.ping().await.is_err());
    assert!(start.elapsed() < Duration::from_secs(6));
}

// verifies: REQ-QUEUE-GE19.S4 (Redis does not reply within two seconds: the write fails)
#[tokio::test]
async fn silent_redis_times_out() {
    // A server that accepts connections and never answers.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((socket, _)) = listener.accept().await {
            held.push(socket);
        }
    });
    let q = RedisQueue::new(&format!("redis://127.0.0.1:{port}"), "kz:events", 1).unwrap();
    let start = Instant::now();
    assert!(q.publish("k", b"x".to_vec()).await.is_err());
    let took = start.elapsed();
    assert!(
        took >= Duration::from_millis(1500) && took < Duration::from_secs(5),
        "{took:?}"
    );
}

// verifies: REQ-QUEUE-1PF3.S1 (one key, one stream, in the order published)
#[tokio::test]
async fn one_key_stays_in_one_stream_in_order() {
    let Some(e) = env().await else { return };
    let q = RedisQueue::new(&e.url, &e.prefix, 4).unwrap();
    for i in 0..30u8 {
        q.publish("same-session", vec![i]).await.unwrap();
    }
    let stream = q.stream_for("same-session");
    let got: Vec<u8> = e
        .entries(&stream)
        .await
        .into_iter()
        .map(|(p, _)| p[0])
        .collect();
    assert_eq!(got, (0..30).collect::<Vec<u8>>());
    let total: usize = {
        let mut n = 0;
        for shard in 0..4 {
            n += e.xlen(&format!("{}:{shard}", e.prefix)).await;
        }
        n
    };
    assert_eq!(total, 30);
}

// verifies: REQ-QUEUE-1PF3.S2 (the shard comes from the key and the shard count only)
#[tokio::test]
async fn shard_depends_only_on_the_key() {
    let a = RedisQueue::new("redis://127.0.0.1:1", "p", 8).unwrap();
    let b = RedisQueue::new("redis://127.0.0.1:2", "p", 8).unwrap();
    for key in ["a", "b", "0123456789abcdef0123456789abcdef", ""] {
        assert_eq!(a.stream_for(key), b.stream_for(key));
    }
    assert!(RedisQueue::new("redis://127.0.0.1:1", "p", 0).is_err());
}

// verifies: REQ-QUEUE-1PF3.S3 (many keys reach every shard)
#[tokio::test]
async fn keys_spread_over_all_shards() {
    let Some(e) = env().await else { return };
    let q = RedisQueue::new(&e.url, &e.prefix, 4).unwrap();
    for i in 0..200 {
        q.publish(&format!("session-{i}"), vec![0]).await.unwrap();
    }
    for shard in 0..4 {
        assert!(
            e.xlen(&format!("{}:{shard}", e.prefix)).await > 0,
            "shard {shard}"
        );
    }
}

// verifies: REQ-QUEUE-YATM.S1 (the backend never trims: it writes 500 messages and all stay)
#[tokio::test]
async fn backend_never_trims() {
    // No command in the source can remove an entry.
    let source = include_str!("../src/redis.rs");
    let code: String = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_uppercase();
    for word in [
        "XTRIM", "XDEL", "MAXLEN", "MINID", "DEL\"", "UNLINK", "FLUSH",
    ] {
        assert!(!code.contains(word), "{word}");
    }
    let Some(e) = env().await else { return };
    let q = RedisQueue::new(&e.url, &e.prefix, 1).unwrap();
    for i in 0..500u32 {
        q.publish("k", i.to_le_bytes().to_vec()).await.unwrap();
    }
    assert_eq!(e.xlen(&q.stream_for("k")).await, 500);
}

// verifies: REQ-QUEUE-754R.S2 (an account with only XADD and PING on the streams is enough)
// verifies: REQ-QUEUE-754R.S3 (an account without XADD fails visibly, without the password in the error)
#[tokio::test]
async fn least_privilege_accounts() {
    let Some(e) = env().await else { return };
    let user = format!("kz_collector_{}", e.prefix.split(':').next().unwrap());
    let pattern = format!("~{}:*", e.prefix);
    let mut admin = e.admin.clone();
    let () = redis::cmd("ACL")
        .arg("SETUSER")
        .arg(&user)
        .arg("on")
        .arg(">secret-pw-123")
        .arg(&pattern)
        .arg("-@all")
        .arg("+xadd")
        .arg("+ping")
        .query_async(&mut admin)
        .await
        .unwrap();
    let q = RedisQueue::new(&e.url_for(&user, "secret-pw-123"), &e.prefix, 1).unwrap();
    q.ping().await.unwrap();
    q.publish("k", b"x".to_vec()).await.unwrap();
    assert_eq!(e.xlen(&q.stream_for("k")).await, 1);

    // The same account without XADD.
    let () = redis::cmd("ACL")
        .arg("SETUSER")
        .arg(&user)
        .arg("-xadd")
        .query_async(&mut admin)
        .await
        .unwrap();
    let narrow = RedisQueue::new(&e.url_for(&user, "secret-pw-123"), &e.prefix, 1).unwrap();
    let err = narrow.publish("k", b"x".to_vec()).await.unwrap_err();
    assert!(!err.to_string().contains("secret-pw-123"), "{err}");
    assert_eq!(e.xlen(&narrow.stream_for("k")).await, 1);
    let _: Result<(), _> = redis::cmd("ACL")
        .arg("DELUSER")
        .arg(&user)
        .query_async(&mut admin)
        .await;
}
