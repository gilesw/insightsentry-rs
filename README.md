# insightsentry

Unofficial async Rust client for the **direct InsightSentry API**, generated from
API **3.0.24**. Requires Rust 1.85 or newer. All 37 operations in the pinned
specification are available through the generated client.

For the older RapidAPI contract and v2 financials, use the separate
`rapidapi-insightsentry` crate. These clients have independent specifications,
response types and release versions.

This crate is being prepared for publication. Use a local dependency for now:

```toml
[dependencies]
insightsentry = { path = "../insightsentry-rs" }
```

After publication, use `insightsentry = "0.1"`.

## Authentication and quotes

```rust,no_run
use insightsentry::Client;

# async fn example() -> Result<(), insightsentry::Error> {
let client = Client::new("your-bearer-token");
if let Some(quote) = client.quote("NASDAQ:AAPL").await? {
    println!("Last price: {:?}", quote.last_price);
}
# Ok(())
# }
```

Supply the raw token without a `Bearer ` prefix. The client sends
`Authorization: Bearer …` to `https://api.insightsentry.com`. It has no RapidAPI
constructor or legacy financials endpoint. Debug output hides the token.

`quotes(&[...])` makes one batch request. Its `data` contains `QuoteResult::QuoteData`
and/or `QuoteResult::QuoteError`; successful records and errors are both retained.
`quote(symbol)` returns `Error::Quote` for a matching provider error, `None` for
an absent symbol, or a quote whose price fields can still be missing. Errors
are never converted to zero prices.

## Prices and adjustments

`quote`, `quotes`, `SeriesQuery::default()` and `HistoryQuery::new()` explicitly
request `settlement=false`. Set `settlement: Some(true)` to select futures
settlement prices. `None` omits the parameter and accepts the server default.
The old API snapshot documented true as the default; the new one documents false.

`QuoteQuery`, `SeriesQuery` and `HistoryQuery` expose split, dividend and back
adjustments. The provider documents that `split=false` overrides dividend
adjustment; `dadj` applies only with split adjustment enabled. Values are passed
through without currency conversion, rounding or application-specific defaults
for these other adjustments.

```rust,no_run
use insightsentry::{Client, SeriesQuery, GetV3SymbolsSymbolSeriesBarType};

# async fn example() -> Result<(), insightsentry::Error> {
let client = Client::new("your-bearer-token");
let response = client.series("NASDAQ:AAPL", &SeriesQuery {
    bar_type: Some(GetV3SymbolsSymbolSeriesBarType::Day),
    data_points: Some(5),
    split_adjusted: Some(true),
    dividend_adjusted: Some(false),
    ..Default::default()
}).await?;
# Ok(())
# }
```

Use `quotes_with_options` or `quote_with_options` for quote adjustments.
Series queries also support `twenty_four_hour`. The obsolete `long_poll` option
is absent. History accepts a month or full date and intraday bar types; use
series for daily bars.

Convenience series/history requests explicitly send `abbr=false` for object
rows. The upstream standard-history schemas do not correctly describe compact
arrays. Raw generated methods expose the parameter, but `abbr=true` is not
supported by their standard-history response types. This is separate from the
SIP compact schemas. See [schema notes](specs/README.md).

`history` returns archive-specific metadata. A provider-declared unavailable
period becomes `Error::HistoryUnavailable`, retaining its code, message and
earliest available period. A genuinely empty successful series stays empty.
Required bar timestamps and closing prices must be present; other bar fields
remain optional.

## Coverage

| Area | Convenience methods | Additional generated operations via `raw()` |
| --- | --- | --- |
| Quotes | `quote`, `quotes`, adjustment variants | — |
| Price history | `series`, `history` | Futures contracts, trades |
| Instrument details | `info`, `session`, `search` | Exchange catalogue |
| Fundamentals | `fundamentals`, `fundamentals_series` | Fundamentals metadata |
| Options | — | Contracts, quotes, snapshot |
| Calendars | — | Dividends, earnings, IPOs, economic events |
| News/documents | — | Newsfeed, sources, document list and content |
| Screeners | — | Parameters and queries for stock, ETF, bond and crypto |
| SIP | — | Symbols, quotes, series, history, trades, corporate actions |

`SearchQuery` includes `exchange`, independently of `country` and instrument type.
Fundamental-series points preserve named numeric lines such as RSI/MACD outputs;
they do not require every value to be called `close`. The raw document response
retains its typed unavailable-content alternative.

The generated API follows the pinned snapshot. Endpoint availability depends on
your subscription; the crate does not implement entitlement checks. There is no
WebSocket streaming client or old WebSocket-key endpoint in this direct snapshot.

## Limits and errors

Each convenience call makes one request. There is no automatic retry, batching,
pagination, background task or gateway fallback. Provider credit costs may differ
from request counts. Choose batch sizes and data-point counts explicitly.

Convenience calls have a 30-second Tokio deadline, adjustable with `with_timeout`.
`with_max_response_body_bytes` adjusts the default 8 MiB buffered-response limit.
`with_base_url` supports a proxy or local mock server. Raw operations require a
caller-supplied deadline; raw PDF streaming also requires a total byte limit.
The pinned generator does not expose a custom transport builder.

Errors distinguish HTTP status/body/`Retry-After`, per-symbol quote errors,
unavailable archive history, decoding failures, transport/body limits, timeouts
and invalid arguments. `is_unauthorized()` recognizes 401/403 (including plan
restrictions); `is_rate_limited()` recognizes 429. Success-response quota headers
are not exposed by the generated client.

Use provider symbols such as `NASDAQ:AAPL` and `LSE:LLOY`. The crate does not map
MIC codes, convert units or decide whether a session has completed. Preserve
provider dates and timestamps when selecting a Closing Price for application rules.

## Development and verification

Examples take the token from standard input:

```sh
cargo run --example quote -- NASDAQ:AAPL < /path/to/private-token-file
mise run generate
mise run generate:check
mise run check
```

Generated code is checked in. Generation pins openapi-to-rust 0.19.0; consumers
need neither the generator nor a specification download. Edit the config/overlay
and regenerate; never hand-edit `src/generated`.

Tests use synthetic responses and local servers. This direct client has not been
live-verified. No provider requests were made when splitting the crates or adapting
to API 3.0.24. Earlier successful RapidAPI checks belong to the separate client.

See [specification provenance](specs/README.md), the historical
[upgrade review](docs/spec-upgrade-review.md) and [release guide](RELEASING.md).
`mise run release` detects a matching tag and dry-runs; `release:publish` uploads
after checks. Cargo reads an inherited `CARGO_REGISTRY_TOKEN` directly.

## License

Handwritten code is MIT OR Apache-2.0. The upstream spec declares the provider's
Terms of Service; the code licence does not relicense that document. This is an
independent client. API access and market-data use remain subject to provider terms.
