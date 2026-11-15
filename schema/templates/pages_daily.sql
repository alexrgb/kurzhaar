-- Pageviews and visitors per site, day and page path.
CREATE TABLE IF NOT EXISTS {{database}}.pages_daily
(
    site_id  LowCardinality(String),
    day      Date,
    path String,
    views    SimpleAggregateFunction(sum, UInt64),
    visitors AggregateFunction({{uniq}}, String)
)
ENGINE = AggregatingMergeTree
ORDER BY (site_id, day, path)
SETTINGS non_replicated_deduplication_window = 1000;

CREATE MATERIALIZED VIEW IF NOT EXISTS {{database}}.pages_daily_mv
TO {{database}}.pages_daily AS
SELECT
    site_id,
    toDate(ts, 'UTC') AS day,
    path AS path,
    count() AS views,
    {{uniq}}State(session_hash) AS visitors
FROM {{database}}.events
WHERE type = 0
GROUP BY site_id, day, path;
