# Country database fixtures

Two tiny MMDB files for the collector tests (REQ-COLLECT-82NX). They hold only documentation address ranges (RFC 5737 and RFC 3849), no real data, and were generated with the `mmdb-writer` crate (0.1.0).

| File | Contents |
| --- | --- |
| `country.mmdb` | `192.0.2.0/24` → DE, `198.51.100.0/24` → UA, `2001:db8::/32` → FR, record `{country: {iso_code}}` |
| `city.mmdb` | `203.0.113.0/24` → PL, record `{country: {iso_code}, city: {names: {en}}}`, to check that only the country is read |

To regenerate them, write a small program with `mmdb_writer::Writer::new(name)`, `insert(net, &record)` and `to_bytes()`.
