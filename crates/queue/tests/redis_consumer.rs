mod support;

use kurzhaar_queue::redis::RedisQueue;
use kurzhaar_queue::redis_consumer::{ConsumerConfig, RedisConsumer};
use kurzhaar_queue::{Consumer, Queue};
use std::time::Duration;
use support::{env, Env};

const NO_WAIT: Duration = Duration::from_millis(50);

fn consumer(e: &Env, group: &str, name: &str, claim_idle: Duration) -> RedisConsumer {
    RedisConsumer::new(
        &e.url,
        ConsumerConfig {
            prefix: e.prefix.clone(),
            shards: 1,
            group: group.to_string(),
            consumer: name.to_string(),
            dead_stream: format!("{}:dead:{group}", e.prefix),
            claim_idle,
        },
    )
    .unwrap()
}

async fn publish(e: &Env, n: usize) {
    let q = RedisQueue::new(&e.url, &e.prefix, 1).unwrap();
    for i in 0..n {
        q.publish("s", format!("m{i}").into_bytes()).await.unwrap();
    }
}

fn payloads(d: &[kurzhaar_queue::Delivery]) -> Vec<String> {
    d.iter()
        .map(|d| String::from_utf8(d.payload.clone()).unwrap())
        .collect()
}

fn far_future() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 48 * 3_600_000
}

// verifies: REQ-INGEST-J5R5.S1 (two sinks read the same stream and each gets every message)
#[tokio::test]
async fn every_sink_gets_every_message() {
    let Some(e) = env().await else { return };
    publish(&e, 3).await;
    let raw = consumer(&e, "kz-raw", "a", Duration::from_secs(60));
    let api = consumer(&e, "kz-api", "a", Duration::from_secs(60));
    assert_eq!(
        payloads(&raw.read(10, NO_WAIT).await.unwrap()),
        ["m0", "m1", "m2"]
    );
    assert_eq!(
        payloads(&api.read(10, NO_WAIT).await.unwrap()),
        ["m0", "m1", "m2"]
    );
}

// verifies: REQ-INGEST-J5R5.S2 (an insert not confirmed: the messages stay pending)
// verifies: REQ-INGEST-J5R5.S3 (a restart reads its pending entries first and inserts them)
#[tokio::test]
async fn unacknowledged_messages_come_back_first_after_a_restart() {
    let Some(e) = env().await else { return };
    publish(&e, 3).await;
    let first = consumer(&e, "kz-raw", "a", Duration::from_secs(60));
    let got = first.read(2, NO_WAIT).await.unwrap();
    assert_eq!(payloads(&got), ["m0", "m1"]);
    first.ack(&got[..1]).await.unwrap();
    drop(first);
    // The same instance name starts again.
    let again = consumer(&e, "kz-raw", "a", Duration::from_secs(60));
    let got = again.read(10, NO_WAIT).await.unwrap();
    assert_eq!(payloads(&got), ["m1"]);
    assert_eq!(got[0].attempt, 2);
    again.ack(&got).await.unwrap();
    // Then the new entries.
    assert_eq!(payloads(&again.read(10, NO_WAIT).await.unwrap()), ["m2"]);
}

// verifies: REQ-INGEST-J5R5.S4 (entries left idle by a dead instance are taken over)
#[tokio::test]
async fn idle_entries_of_a_dead_instance_are_taken_over() {
    let Some(e) = env().await else { return };
    publish(&e, 2).await;
    let dead = consumer(&e, "kz-raw", "dead", Duration::from_millis(200));
    assert_eq!(dead.read(10, NO_WAIT).await.unwrap().len(), 2);
    tokio::time::sleep(Duration::from_millis(400)).await;
    let live = consumer(&e, "kz-raw", "live", Duration::from_millis(200));
    let got = live.read(10, NO_WAIT).await.unwrap();
    assert_eq!(payloads(&got), ["m0", "m1"]);
    assert_eq!(got[0].attempt, 2);
}

// verifies: REQ-INGEST-J5R5.S5 (several instances: each message goes to exactly one)
#[tokio::test]
async fn instances_share_the_messages() {
    let Some(e) = env().await else { return };
    publish(&e, 6).await;
    let a = consumer(&e, "kz-raw", "a", Duration::from_secs(60));
    let b = consumer(&e, "kz-raw", "b", Duration::from_secs(60));
    let mut all = payloads(&a.read(3, NO_WAIT).await.unwrap());
    all.extend(payloads(&b.read(3, NO_WAIT).await.unwrap()));
    all.sort();
    assert_eq!(all, ["m0", "m1", "m2", "m3", "m4", "m5"]);
}

// verifies: REQ-INGEST-XS9C.S1 (a message that cannot be read goes to the dead-letter stream)
#[tokio::test]
async fn dead_letter_keeps_the_bytes_and_the_reason() {
    let Some(e) = env().await else { return };
    let q = RedisQueue::new(&e.url, &e.prefix, 1).unwrap();
    q.publish("s", vec![0xff, 0x00, b'{']).await.unwrap();
    q.publish("s", b"next".to_vec()).await.unwrap();
    let c = consumer(&e, "kz-raw", "a", Duration::from_secs(60));
    let got = c.read(10, NO_WAIT).await.unwrap();
    c.dead_letter(&got[0], "not valid JSON").await.unwrap();
    c.ack(&got[1..]).await.unwrap();
    let dead = format!("{}:dead:kz-raw", e.prefix);
    assert_eq!(
        e.entries(&dead).await,
        vec![(vec![0xff, 0x00, b'{'], "s".to_string())]
    );
    let rows: Vec<(String, Vec<(String, redis::Value)>)> = redis::cmd("XRANGE")
        .arg(&dead)
        .arg("-")
        .arg("+")
        .query_async(&mut e.admin.clone())
        .await
        .unwrap();
    assert!(rows[0].1.contains(&(
        "reason".into(),
        redis::Value::BulkString(b"not valid JSON".to_vec())
    )));
    // Nothing is left pending, and the next read finds nothing.
    assert!(c.read(10, NO_WAIT).await.unwrap().is_empty());
}

// verifies: REQ-QUEUE-YATM.S2 (a pending entry is not removed, nor anything after it)
// verifies: REQ-QUEUE-YATM.S3 (a group that has not read an entry keeps it)
// verifies: REQ-QUEUE-YATM.S4 (an old entry every group acknowledged is removed)
#[tokio::test]
async fn retention_removes_only_what_every_group_has_finished() {
    let Some(e) = env().await else { return };
    publish(&e, 4).await;
    let stream = format!("{}:0", e.prefix);
    let fast = consumer(&e, "kz-raw", "a", Duration::from_secs(60));
    let slow = consumer(&e, "kz-api", "a", Duration::from_secs(60));
    let day = Duration::from_secs(24 * 3600);

    // The slow group has read nothing: all four entries stay although they are "old".
    let got = fast.read(10, NO_WAIT).await.unwrap();
    slow.read(0, NO_WAIT).await.ok();
    fast.ack(&got).await.unwrap();
    assert_eq!(fast.trim(day, far_future()).await.unwrap(), 0);
    assert_eq!(e.xlen(&stream).await, 4);

    // The slow group takes all four and acknowledges only the first one: one entry goes.
    let slow_got = slow.read(10, NO_WAIT).await.unwrap();
    slow.ack(&slow_got[..1]).await.unwrap();
    assert_eq!(fast.trim(day, far_future()).await.unwrap(), 1);
    assert_eq!(e.xlen(&stream).await, 3);

    // The pending m1 holds back m2 and m3 even though m2 is acknowledged.
    slow.ack(&slow_got[2..]).await.unwrap();
    assert_eq!(fast.trim(day, far_future()).await.unwrap(), 0);
    assert_eq!(e.xlen(&stream).await, 3);

    // Everything acknowledged: the rest goes, but nothing newer than the retention period.
    slow.ack(&slow_got[1..2]).await.unwrap();
    assert_eq!(fast.trim(day, 0).await.unwrap(), 0);
    assert_eq!(e.xlen(&stream).await, 3);
    assert_eq!(fast.trim(day, far_future()).await.unwrap(), 3);
}
