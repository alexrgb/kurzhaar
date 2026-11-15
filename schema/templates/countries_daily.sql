-- Pageviews and visitors per site, day and country.
CREATE TABLE IF NOT EXISTS {{database}}.countries_daily
(
    site_id  LowCardinality(String),
    day      Date,
    country String,
    views    SimpleAggregateFunction(sum, UInt64),
    visitors AggregateFunction({{uniq}}, String)
)
ENGINE = AggregatingMergeTree
ORDER BY (site_id, day, country)
SETTINGS non_replicated_deduplication_window = 1000;

CREATE MATERIALIZED VIEW IF NOT EXISTS {{database}}.countries_daily_mv
TO {{database}}.countries_daily AS
SELECT
    site_id,
    toDate(ts, 'UTC') AS day,
    country AS country,
    count() AS views,
    {{uniq}}State(session_hash) AS visitors
FROM {{database}}.events
WHERE type = 0
  AND country != ''
GROUP BY site_id, day, country;
