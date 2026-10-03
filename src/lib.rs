//! Async client for the direct InsightSentry API.
//!
//! ```no_run
//! use insightsentry::Client;
//!
//! # async fn example() -> Result<(), insightsentry::Error> {
//! let client = Client::new("your-bearer-token");
//! if let Some(quote) = client.quote("NASDAQ:AAPL").await? {
//!     println!("Last price: {:?}", quote.last_price);
//! }
//! # Ok(())
//! # }
//! ```

mod client;
mod error;

#[allow(clippy::all, dead_code, unused_imports)]
#[rustfmt::skip]
pub mod generated;

pub use client::{Client, DIRECT_BASE_URL, HistoryQuery, QuoteQuery, SearchQuery, SeriesQuery};
pub use error::Error;
pub use generated::client::{
    GetV3SymbolsSearchType, GetV3SymbolsSymbolHistoryBarType, GetV3SymbolsSymbolSeriesBarType,
    GetV3SymbolsSymbolSeriesCurrency,
};
pub use generated::types::*;

/// A batch entry is a quote or a provider error for a particular symbol.
pub use generated::types::QuotesResponseDataDataItemUnion as QuoteResult;
