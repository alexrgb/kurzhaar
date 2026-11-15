-- Top country values for a site over the days in [from, to), most views first, ties by country.
-- Params: site String, from DateTime('UTC'), to DateTime('UTC'), limit UInt32
SELECT
    country,
    sum(views)                AS views,
    {{uniq}}Merge(visitors)   AS visitors
FROM {{database}}.countries_daily
WHERE site_id = {site:String} AND day >= toDate({from:DateTime('UTC')}) AND day < toDate({to:DateTime('UTC')})
GROUP BY country
ORDER BY views DESC, country ASC
LIMIT {limit:UInt32}
