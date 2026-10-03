# Direct API specification

`insightsentry2026.json` is the unchanged API **3.0.24** snapshot from
<https://insightsentry.com/openapi.json>. It declares the direct API server and
bearer authentication. All 37 operations are generated. No RapidAPI security or
legacy financials overlay is applied here.

The older API 3.0.5 contract lives in the separate `rapidapi-insightsentry-rs` repo.

## Local configuration

- `compatibility.overlay.yaml` retains optional 52-week high/low info fields from
  the previous integration. These additive fields are not in the new snapshot
  and have not been verified live against the direct API.
- Quote/info statuses, fundamental types and quote error codes retain unknown
  string values through extensible enums.
- Required fields from the new specification are respected, including non-null
  fundamental IDs and required quote codes, bar times and closing prices.
- `src/generated/effective.json` records the effective specification.

The generator is pinned to openapi-to-rust 0.19.0. It does not enforce a request
timeout: the convenience client uses Tokio deadlines. Raw operations require a
caller-supplied deadline and PDF streams require a caller-supplied byte limit.

## Known upstream schema inconsistencies

Standard series/history advertise `abbr=true` but their response schemas model
object rows rather than compact arrays and `series_keys`. Convenience methods
explicitly request `abbr=false`. Raw standard-history compact responses remain
unsupported; only SIP has a separate compact schema.

`IpoData` lists `exchange` as required without defining its property type. No
invented property type or required-field relaxation is applied locally.

The spec identifies its licence as the provider's Terms of Service. The licence
of handwritten code does not replace that declaration.

Snapshot SHA-256:
`ecb7bd27eba04cb2943923c5284bf0ec27cf91f031f58461b8bff463a3e1861c`.
