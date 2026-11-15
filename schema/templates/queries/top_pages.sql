-- Top path values for a site over the days in [from, to), most views first, ties by path.
-- Params: site String, from DateTime('UTC'), to DateTime('UTC'), limit UInt32
SELECT
    path,
    sum(views)                AS views,
    {{uniq}}Merge(visitors)   AS visitors
FROM {{database}}.pages_daily
WHERE site_id = {site:String} AND day >= toDate({from:DateTime('UTC')}) AND day < toDate({to:DateTime('UTC')})
GROUP BY path
ORDER BY views DESC, path ASC
LIMIT {limit:UInt32}
