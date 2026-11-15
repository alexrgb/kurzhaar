-- Raw events table. Present in all three databases (raw, api, dashboard).
-- {{ttl}} is empty in the raw database and a 30 day TTL in api and dashboard (ADR XET8).
-- non_replicated_deduplication_window enables insert deduplication tokens (REQ-METRICS-63R7).
CREATE TABLE IF NOT EXISTS {{database}}.events
(
    site_id      LowCardinality(String),
    ts           DateTime64(3, 'UTC'),
    type         UInt8,                          -- 0 pageview, 1 custom event, 2 identify
    name         LowCardinality(String),         -- custom event name
    path         String,
    title        String,
    referrer     String,                         -- external origin + path, or an internal path
    country      LowCardinality(String),
    session_hash String,
    uid_hash     String,                         -- HMAC of the identify id, never the id itself
    screen       LowCardinality(String),
    lang         LowCardinality(String),
    props        Map(String, String),
    INDEX idx_uid_hash uid_hash TYPE bloom_filter GRANULARITY 4
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (site_id, ts, session_hash)
{{ttl}}
SETTINGS non_replicated_deduplication_window = 1000;
