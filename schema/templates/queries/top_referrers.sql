-- Top referrer values for a site over the days in [from, to), most views first, ties by referrer.
-- Params: site String, from DateTime('UTC'), to DateTime('UTC'), limit UInt32
SELECT
    referrer,
    sum(views)                AS views,
    {{uniq}}Merge(visitors)   AS visitors
FROM {{database}}.referrers_daily
WHERE site_id = {site:String} AND day >= toDate({from:DateTime('UTC')}) AND day < toDate({to:DateTime('UTC')})
GROUP BY referrer
ORDER BY views DESC, referrer ASC
LIMIT {limit:UInt32}
