//! Fills a Redis up to its memory limit. It changes a server-wide setting, so it lives in a
//! test file of its own (cargo runs test files one after another).

mod support;

use kurzhaar_queue::redis::RedisQueue;
use kurzhaar_queue::Queue;
use support::env;

// verifies: REQ-QUEUE-GE19.S3 (a full Redis refuses the write; earlier messages stay)
#[tokio::test]
async fn full_redis_refuses_writes_and_keeps_old_messages() {
    let Some(e) = env().await else { return };
    let mut admin = e.admin.clone();
    let info: String = redis::cmd("INFO")
        .arg("memory")
        .query_async(&mut admin)
        .await
        .unwrap();
    let used: u64 = info
        .lines()
        .find_map(|l| l.strip_prefix("used_memory:"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let () = redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory-policy")
        .arg("noeviction")
        .query_async(&mut admin)
        .await
        .unwrap();

    let q = RedisQueue::new(&e.url, &e.prefix, 1).unwrap();
    q.publish("k", vec![1; 100]).await.unwrap();
    let stream = q.stream_for("k");
    let before = e.xlen(&stream).await;

    let () = redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory")
        .arg(used + 256 * 1024)
        .query_async(&mut admin)
        .await
        .unwrap();
    let mut refused = None;
    for _ in 0..200 {
        if let Err(err) = q.publish("k", vec![7; 16 * 1024]).await {
            refused = Some(err.to_string());
            break;
        }
    }
    let () = redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory")
        .arg(0)
        .query_async(&mut admin)
        .await
        .unwrap();

    let refused = refused.expect("Redis never refused a write");
    assert!(refused.contains("OOM"), "{refused}");
    // Nothing was trimmed to make room: the old message and all accepted ones are there.
    assert!(e.xlen(&stream).await >= before);
    assert_eq!(e.entries(&stream).await[0].0, vec![1; 100]);
}
