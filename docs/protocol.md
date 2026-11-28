# Wire format reference

<!-- Generated from contracts/protocol/fields.json by packages/protocol/scripts/reference.ts. Do not edit. -->

Wire format version 1, revision 1.4. Revisions inside a version only add optional fields; decoders ignore keys they do not know. The decisions behind the format are in ADR-T7N1 and ADR-VJGJ.

A batch is plain text. Line 1 is the header, every following line is one event. A line is a list of `key::value` fields joined by `|`. In values, `%`, `|`, CR and LF are escaped as `%XX`.

## Event types

| `y` | Name | Status | Since | Description |
|---|---|---|---|---|
| 0 | pageview | active | 1.0 | A page was viewed. `y` is omitted on the wire. |
| 1 | custom event | active | 1.0 | A named event with optional properties. |
| 2 | identify | active | 1.0 | Links the visitor to a site-supplied id. |
| 3 | engagement | active | 1.1 | Sent when a page is left: engaged time and scroll depth. |
| 4 | performance | active | 1.1 | Web Vitals of one page. |
| 5 | error | reserved | 1.1 | Reserved for JavaScript errors; not used yet. |

## Header fields

| Key | Name | Decoded as | Type | Unit | Required | Event types | Since | Description |
|---|---|---|---|---|---|---|---|---|
| `v` | version | - | int | - | yes | - | 1.0 | Protocol version. Always the first field of the header line; a decoder rejects versions it does not support. |
| `k` | project key | `key` | string | - | no | - | 1.0 | Public project (site) key that routes the batch to a site. |
| `s` | screen | `screen` | string | - | no | - | 1.0 | Viewport size as WIDTHxHEIGHT in CSS pixels. |
| `l` | language | `lang` | string | - | no | - | 1.0 | Browser language tag. |
| `i` | identity | `identity` | string | - | no | - | 1.0 | Identify id supplied by the site; the collector stores only a keyed hash of it. |
| `tv` | tracker version | `trackerVersion` | string | - | no | - | 1.2 | Version of the tracker script that built the batch. Not the protocol version `v`. Stored per event for diagnosis. |
| `tz` | timezone offset | `tzOffset` | int | min | no | - | 1.2 | Visitor offset from UTC in minutes at the time of the batch, negative west of UTC (120 for Kyiv in summer, -300 for New York in winter). The zone name is not collected. Used for local-time reports, not for geography. |
| `cb` | client brands | `clientBrands` | list | - | no | - | 1.3 | Client-hint brands, each written `Brand/major`; the major version follows the last slash. Raw signal for server-side device detection (ADR-ZKD1); the collector discards it after detection. |
| `cm` | client mobile flag | `clientMobile` | int | - | no | - | 1.3 | Client-hint mobile flag, 1 or 0. Raw signal, discarded after detection. |
| `cp` | client platform | `clientPlatform` | string | - | no | - | 1.3 | Client-hint platform name. Raw signal, discarded after detection. |
| `cv` | client platform major version | `clientPlatformVersion` | string | - | no | - | 1.3 | Major version of the client-hint platform version only. Tells Windows 11 (13 and above) from Windows 10. Raw signal, discarded after detection. |
| `tp` | maximum touch points | `touchPoints` | int | - | no | - | 1.3 | `navigator.maxTouchPoints`. Raw signal, discarded after detection. |
| `pc` | coarse primary pointer | `coarsePointer` | int | - | no | - | 1.3 | 1 when the primary pointer is coarse (finger), else 0. Raw signal, discarded after detection. |

## Event fields

| Key | Name | Decoded as | Type | Unit | Required | Event types | Since | Description |
|---|---|---|---|---|---|---|---|---|
| `ts` | timestamp | `ts` | int | ms | yes | all | 1.0 | Absolute event time, epoch milliseconds. Present on the first event of a batch only. |
| `d` | delta | - | int | ms | no | all | 1.0 | Milliseconds since the previous event (may be negative). Used on every event after the first; decoders turn it into an absolute `ts`. |
| `y` | event type | `type` | int | - | no | all | 1.0 | Event type, see the event type table. Omitted for a pageview (0). |
| `n` | event name | `name` | string | - | no | 1 | 1.0 | Name of a custom event. Names starting with `$` are reserved for automatic events. |
| `u` | url | `url` | string | - | no | all | 1.0 | Page path without query string, except campaign parameters and click ids. |
| `t` | title | `title` | string | - | no | 0 | 1.0 | Page title. |
| `r` | referrer | `referrer` | string | - | no | 0 | 1.0 | External referrer as host and path, or an internal path. |
| `g` | tags | `tags` | list | - | no | all | 1.2 | Short labels on an event, separated by commas. A comma inside a tag is written `%2C`. At most 10 tags of 32 characters; the tracker enforces it and the collector drops the excess. |
| `p.<name>` | property | `props` | string | - | no | 1, 2 | 1.0 | Custom property. The property name follows `p.` and is escaped (`:` as `%3A`). Values are strings; booleans are 1 and 0. |
| `et` | engaged time | `engagedMs` | int | ms | no | 3 | 1.1 | Time the page was visible and active before it was left. |
| `sc` | scroll depth | `scrollPct` | int | percent | no | 3 | 1.1 | Maximum scroll depth reached, 0 to 100. |
| `lcp` | largest contentful paint | `lcp` | int | ms | no | 4 | 1.1 | Web Vitals LCP. |
| `inp` | interaction to next paint | `inp` | int | ms | no | 4 | 1.1 | Web Vitals INP. |
| `cls` | cumulative layout shift | `cls` | decimal | - | no | 4 | 1.1 | Web Vitals CLS, a unitless decimal. |
| `fcp` | first contentful paint | `fcp` | int | ms | no | 4 | 1.1 | Web Vitals FCP. |
| `ttfb` | time to first byte | `ttfb` | int | ms | no | 4 | 1.1 | Web Vitals TTFB. |

## Query parameters

Parameters of the collect request URL, not keys of the body. They never change the body, so the batch token stays the same.

| Key | Name | Type | Unit | Range | Since | Description |
|---|---|---|---|---|---|---|
| `a` | batch age | int | ms | 0 to 3600000 | 1.4 | Milliseconds between the moment the tracker built the batch and the moment this attempt is sent, a query parameter of the collect request and not part of the body. A missing or invalid value counts as 0. |

## Example

```
v::1|k::abc|s::1920x1080|l::uk-UA|tv::1.4.2|tz::120
ts::1731150000000|u::/
d::4000|y::1|n::signup|g::paid,trial|p.plan::pro
d::8000|y::3|u::/|et::7500|sc::80
```
