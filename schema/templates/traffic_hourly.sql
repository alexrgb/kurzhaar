-- Views, custom events and unique visitors per site and hour.
-- views: pageviews. custom_events: events of type 1. visitors: distinct session hashes among pageviews.
CREATE TABLE IF NOT EXISTS {{database}}.traffic_hourly
(
    site_id       LowCardinality(String),
    hour          DateTime('UTC'),
    views         SimpleAggregateFunction(sum, UInt64),
    custom_events SimpleAggregateFunction(sum, UInt64),
    visitors      AggregateFunction({{uniq}}, String)
)
ENGINE = AggregatingMergeTree
ORDER BY (site_id, hour)
SETTINGS non_replicated_deduplication_window = 1000;

CREATE MATERIALIZED VIEW IF NOT EXISTS {{database}}.traffic_hourly_mv
TO {{database}}.traffic_hourly AS
SELECT
    site_id,
    toStartOfHour(ts) AS hour,
    countIf(type = 0) AS views,
    countIf(type = 1) AS custom_events,
    {{uniq}}StateIf(session_hash, type = 0) AS visitors
FROM {{database}}.events
WHERE type IN (0, 1)
GROUP BY site_id, hour;
