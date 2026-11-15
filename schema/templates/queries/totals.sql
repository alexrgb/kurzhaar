-- Totals for a site over [from, to). Hour granularity.
-- Params: site String, from DateTime('UTC'), to DateTime('UTC')
SELECT
    sum(views)                AS views,
    sum(custom_events)        AS custom_events,
    {{uniq}}Merge(visitors)   AS visitors
FROM {{database}}.traffic_hourly
WHERE site_id = {site:String} AND hour >= {from:DateTime('UTC')} AND hour < {to:DateTime('UTC')}
