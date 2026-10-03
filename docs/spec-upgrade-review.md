# InsightSentry specification upgrade review

**Historical review.** The later decision was to split clients: the older snapshot
now lives at `../../rapidapi-insightsentry-rs/specs/openapi.json`, while this crate
uses `../specs/insightsentry2026.json` for direct access. The current README and
specification notes describe the implemented behavior and remaining schema gaps.
The one-crate proposal below is superseded by that decision.

Reviewed 2026-10-03. Compare `specs/openapi.json` (API 3.0.5) with
`specs/insightsentry2026.json` (API 3.0.24), downloaded from
<https://insightsentry.com/openapi.json>.

## Conclusion

Adopt the newer specification with targeted wrapper and compatibility changes.
It is not a drop-in regeneration. Keep one crate and explicit direct/RapidAPI
constructors, but document that the new specification describes the direct API.
RapidAPI compatibility must be supported by its existing contract and tests.

The active generator configuration and client implementation were not changed
by this review. Trial files are isolated under `target/spec-review`. No live
API requests or provider credits were used during this comparison.

## Required changes

### Authentication and gateway compatibility

The old spec declares both servers and bearer/RapidAPI-key security alternatives.
The new spec declares only the direct server and bearer security scheme. This
is a documentation/contract change, not evidence that RapidAPI stopped working:
our earlier two RapidAPI requests succeeded.

Retain `Client::rapidapi`, its host/key headers and the legacy financials overlay.
Declare the RapidAPI security scheme in that overlay: it currently refers to a
scheme supplied by the old spec, which the new spec no longer defines. Preserve
negative authentication tests. Do not promise that new endpoints or stricter
new response requirements apply to RapidAPI without verifying them.

### Quotes and per-symbol errors

`QuotesResponseData.data` becomes required and contains an `anyOf` of `QuoteData`
and `QuoteError`. Quote codes become required. The single-quote helper must
recognize a matching per-symbol error and report it as an error, rather than
turning it into an absent quote or a quote with no price. Batch callers must
retain both successes and errors.

New quote parameters are `settlement`, `badj`, `extended`, `split` and `dadj`.
Expose adjustment options without forcing callers to use raw positional arguments.
The status enum changes `HOLIDAYS` to `HOLIDAY`; retain extensible enum handling
for compatibility with both responses.

### Price semantics and defaults

The documented `settlement` default changes from **true to false**. It chooses
whether futures settlement is used as the daily close. The current wrapper omits
this parameter unless supplied: updating a schema does not itself enforce the
new default in requests. The application should choose the intended price
explicitly rather than relying on a provider default.

`split` is new and defaults true. Setting it false overrides `dadj`; dividend
adjustment applies only when split adjustment is enabled. Expose these choices
and test their serialization. Continue to leave application price-selection and
adjustment policy outside the public client.

### History and series

- Remove the obsolete `long_poll` option from the new series API.
- Add `split`, `24h` (series only) and `abbr` parameters.
- `history` now returns `ArchiveHistoryResponseData`, not `HistoricalDataResponse`.
  Preserve `error`, `message` and `available_start_date` so unavailable history
  is distinguishable from a genuinely empty result.
- Series envelopes and OHLCV now require more fields; `time` and `close` are
  required on individual bars. `_ct` is removed from the series schema.
- `start_date` explicitly allows a month or full date. The current String-based
  convenience query can preserve that distinction.

There is a specification gap worth handling deliberately: standard series/history
advertise `abbr=true` as arrays plus `series_keys`, but their response schemas
still describe object rows, and the envelopes disallow additional properties.
Only the SIP endpoints have a separate compact-response schema. Keep convenience
calls in the default object format initially; do not advertise working compact
standard history without a suitable overlay and decoding tests.

### Other wrapper and model changes

- Search gains `exchange`, shifting generated positional arguments.
- News gains source, title/content/symbol filters, date bounds, archive and limit.
- Calendars gain month selection and, where applicable, code/currency options.
- Documents gain `text` and a typed HTTP-success `no_data` alternative.
- Fundamentals, calendars, news, screeners and history have stricter required
  fields. Update realistic fixtures; do not weaken these constraints merely to
  make older tests pass. Check RapidAPI compatibility separately.
- Fundamental series values allow named numeric fields beyond `close`; preserve
  those fields instead of assuming every record has a close value.
- The existing 52-week info overlay and nullable fundamental ID override are
  still not supplied by the new schema; they are not automatically obsolete.

The new IPO schema also lists `exchange` as required without declaring that
property. This should be recorded as an upstream schema inconsistency, not used
to infer a Rust field type.

## Endpoint inventory

37 operations replace the old 29: 12 additions and 4 removals. Our legacy
RapidAPI financials overlay is separate from these counts.

Added: newsfeed source list; options contracts, quotes and snapshot; six SIP
endpoints covering symbols, quotes, series, history, trades and corporate actions;
exchange catalogue; standard symbol trades.

Removed: `/v2/websocket-key` and the old options list/expiration/strike routes.
The new options routes need new request/response models, not just renamed methods.

## Trial evidence and implementation order

The pinned openapi-to-rust 0.19.0 successfully generated from the new spec with
the existing overlays in an isolated directory. Compiling that output with our
unchanged convenience layer produced ten compiler diagnostics, stemming from
quote types and parameters, series/history parameters, history return type and
search arguments. Successful generation alone did not detect the missing
RapidAPI security-scheme reference or establish gateway compatibility.

Suggested sequence:

1. Select the new spec and update the RapidAPI compatibility overlay.
2. Adapt quote errors, query structs and the separate history return type.
3. Add tests for per-symbol errors, unavailable history/documents, adjustment
   serialization and stricter response fields, retaining both authentication tests.
4. Keep compact standard history out of the convenience API until its schema is
   accurately modelled; expose new generated endpoints without adding app policy.
5. Update provenance, coverage and examples, then run the established checks.
6. Before replacing the application's provider, do a small, explicitly bounded
   RapidAPI compatibility check for the endpoints it will actually use.
