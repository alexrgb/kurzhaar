-- Pageviews and visitors per site, day and external referrer. Direct visits (empty) and internal navigation (paths starting with /) are not counted.
CREATE TABLE IF NOT EXISTS {{database}}.referrers_daily
(
    site_id  LowCardinality(String),
    day      Date,
    referrer String,
    views    SimpleAggregateFunction(sum, UInt64),
    visitors AggregateFunction({{uniq}}, String)
)
ENGINE = AggregatingMergeTree
ORDER BY (site_id, day, referrer)
SETTINGS non_replicated_deduplication_window = 1000;

CREATE MATERIALIZED VIEW IF NOT EXISTS {{database}}.referrers_daily_mv
TO {{database}}.referrers_daily AS
SELECT
    site_id,
    toDate(ts, 'UTC') AS day,
    referrer AS referrer,
    count() AS views,
    {{uniq}}State(session_hash) AS visitors
FROM {{database}}.events
WHERE type = 0
  AND referrer != ''
  AND NOT startsWith(referrer, '/')
GROUP BY site_id, day, referrer;
