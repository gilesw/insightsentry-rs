use std::time::Duration;

use crate::{
    ArchiveHistoryResponseData, Error, FundamentalsResponse, FundamentalsSeriesResponse,
    GetV3SymbolsSearchResponse, GetV3SymbolsSearchType, GetV3SymbolsSymbolHistoryBarType,
    GetV3SymbolsSymbolInfoResponse, GetV3SymbolsSymbolSeriesBarType,
    GetV3SymbolsSymbolSeriesCurrency, GetV3SymbolsSymbolSessionResponse, HistoricalDataResponse,
    QuoteData, QuotesResponseData, QuotesResponseDataDataItemUnion, generated::HttpClient,
};

pub const DIRECT_BASE_URL: &str = "https://api.insightsentry.com";

/// Recent bars. Settlement is explicitly false by default; other omitted
/// options use provider defaults.
#[derive(Debug, Clone)]
pub struct SeriesQuery {
    pub bar_type: Option<GetV3SymbolsSymbolSeriesBarType>,
    pub bar_interval: Option<i64>,
    pub extended: Option<bool>,
    pub dividend_adjusted: Option<bool>,
    pub back_adjusted: Option<bool>,
    pub data_points: Option<i64>,
    pub twenty_four_hour: Option<bool>,
    pub split_adjusted: Option<bool>,
    pub currency: Option<GetV3SymbolsSymbolSeriesCurrency>,
    pub settlement: Option<bool>,
}

impl Default for SeriesQuery {
    fn default() -> Self {
        Self {
            bar_type: None,
            bar_interval: None,
            extended: None,
            dividend_adjusted: None,
            back_adjusted: None,
            data_points: None,
            twenty_four_hour: None,
            split_adjusted: None,
            currency: None,
            settlement: Some(false),
        }
    }
}

/// Quote adjustments. Settlement pricing is explicitly off by default.
#[derive(Debug, Clone)]
pub struct QuoteQuery {
    pub settlement: Option<bool>,
    pub back_adjusted: Option<bool>,
    pub extended: Option<bool>,
    pub split_adjusted: Option<bool>,
    pub dividend_adjusted: Option<bool>,
}
impl Default for QuoteQuery {
    fn default() -> Self {
        Self {
            settlement: Some(false),
            back_adjusted: None,
            extended: None,
            split_adjusted: None,
            dividend_adjusted: None,
        }
    }
}

/// Historical bars starting at a provider-formatted date (normally YYYY-MM-DD).
#[derive(Debug, Clone)]
pub struct HistoryQuery {
    pub bar_type: GetV3SymbolsSymbolHistoryBarType,
    pub start_date: String,
    pub split_adjusted: Option<bool>,
    pub bar_interval: Option<i64>,
    pub extended: Option<bool>,
    pub dividend_adjusted: Option<bool>,
    pub back_adjusted: Option<bool>,
    pub settlement: Option<bool>,
}

impl HistoryQuery {
    pub fn new(bar_type: GetV3SymbolsSymbolHistoryBarType, start_date: impl Into<String>) -> Self {
        Self {
            bar_type,
            start_date: start_date.into(),
            split_adjusted: None,
            bar_interval: None,
            extended: None,
            dividend_adjusted: None,
            back_adjusted: None,
            settlement: Some(false),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub exchange: Option<String>,
    pub query: Option<String>,
    pub kind: Option<GetV3SymbolsSearchType>,
    pub country: Option<String>,
    pub page: Option<i64>,
}

/// Direct InsightSentry client. No automatic retries or gateway fallback.
#[derive(Clone)]
pub struct Client {
    inner: HttpClient,
    timeout: Duration,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Supply the raw bearer token without the `Bearer ` prefix.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            inner: HttpClient::new().with_api_key(token),
            timeout: Duration::from_secs(30),
        }
    }

    /// Deadline for convenience methods, including reading and decoding the response.
    /// Raw generated operations do not inherit this deadline.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn request<T, E: std::fmt::Debug>(
        &self,
        request: impl std::future::Future<Output = Result<T, crate::generated::client::ApiOpError<E>>>,
    ) -> Result<T, Error> {
        tokio::time::timeout(self.timeout, request)
            .await
            .map_err(|_| Error::Timeout(self.timeout))?
            .map_err(Error::from)
    }

    /// Override the direct API root for a proxy or local tests.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner = self
            .inner
            .with_base_url(base_url.into().trim_end_matches('/'));
        self
    }

    pub fn with_max_response_body_bytes(mut self, limit: usize) -> Self {
        self.inner = self.inner.with_max_response_body_bytes(limit);
        self
    }

    /// All 37 operations from the direct API 3.0.24 specification.
    pub fn raw(&self) -> &HttpClient {
        &self.inner
    }

    /// One request using explicit default adjustments. Batch results retain both
    /// quote records and per-symbol errors; no symbol is silently discarded.
    pub async fn quotes(&self, symbols: &[&str]) -> Result<QuotesResponseData, Error> {
        self.quotes_with_options(symbols, &QuoteQuery::default())
            .await
    }

    pub async fn quotes_with_options(
        &self,
        symbols: &[&str],
        query: &QuoteQuery,
    ) -> Result<QuotesResponseData, Error> {
        if symbols.is_empty() || symbols.iter().any(|symbol| symbol.trim().is_empty()) {
            return Err(Error::InvalidRequest(
                "At least one non-empty symbol is required",
            ));
        }
        self.request(self.inner.get_v3_symbols_quotes(
            symbols.join(","),
            query.settlement,
            query.back_adjusted,
            query.extended,
            query.split_adjusted,
            query.dividend_adjusted,
        ))
        .await
    }

    /// A matching provider error is Err; a valid response without the requested
    /// symbol is None. A quote can still have a missing last price.
    pub async fn quote(&self, symbol: &str) -> Result<Option<QuoteData>, Error> {
        self.quote_with_options(symbol, &QuoteQuery::default())
            .await
    }

    pub async fn quote_with_options(
        &self,
        symbol: &str,
        query: &QuoteQuery,
    ) -> Result<Option<QuoteData>, Error> {
        let mut found = None;
        for item in self.quotes_with_options(&[symbol], query).await?.data {
            match item {
                QuotesResponseDataDataItemUnion::QuoteData(quote) if quote.code == symbol => {
                    found = Some(quote)
                }
                QuotesResponseDataDataItemUnion::QuoteError(error) if error.code == symbol => {
                    return Err(Error::Quote {
                        code: error.code,
                        message: error.message,
                        reason: error.error.map(|reason| reason.to_string()),
                    });
                }
                _ => {}
            }
        }
        Ok(found)
    }

    pub async fn series(
        &self,
        symbol: &str,
        query: &SeriesQuery,
    ) -> Result<HistoricalDataResponse, Error> {
        if query.data_points.is_some_and(|value| value < 1)
            || query.bar_interval.is_some_and(|value| value < 1)
        {
            return Err(Error::InvalidRequest(
                "Data points and bar interval must be positive",
            ));
        }
        self.request(self.inner.get_v3_symbols_symbol_series(
            symbol,
            query.bar_type,
            query.bar_interval,
            query.extended,
            query.twenty_four_hour,
            query.split_adjusted,
            query.dividend_adjusted,
            query.back_adjusted,
            query.data_points,
            query.currency,
            query.settlement,
            Some(false), // Keep object rows: compact standard-history schema is incomplete.
        ))
        .await
    }

    pub async fn history(
        &self,
        symbol: &str,
        query: &HistoryQuery,
    ) -> Result<ArchiveHistoryResponseData, Error> {
        if query.start_date.trim().is_empty() || query.bar_interval.is_some_and(|value| value < 1) {
            return Err(Error::InvalidRequest(
                "Start date is required and bar interval must be positive",
            ));
        }
        let response = self
            .request(self.inner.get_v3_symbols_symbol_history(
                symbol,
                query.bar_type,
                query.bar_interval,
                &query.start_date,
                query.extended,
                query.split_adjusted,
                query.dividend_adjusted,
                query.back_adjusted,
                query.settlement,
                Some(false),
            ))
            .await?;
        if let Some(code) = &response.error {
            return Err(Error::HistoryUnavailable {
                symbol: response.code,
                code: code.clone(),
                message: response.message,
                available_start_date: response.available_start_date,
            });
        }
        Ok(response)
    }

    pub async fn info(&self, symbol: &str) -> Result<GetV3SymbolsSymbolInfoResponse, Error> {
        self.request(self.inner.get_v3_symbols_symbol_info(symbol))
            .await
    }

    pub async fn session(&self, symbol: &str) -> Result<GetV3SymbolsSymbolSessionResponse, Error> {
        self.request(self.inner.get_v3_symbols_symbol_session(symbol))
            .await
    }

    /// v3 fundamentals through the direct API.
    pub async fn fundamentals(&self, symbol: &str) -> Result<FundamentalsResponse, Error> {
        self.request(self.inner.get_v3_symbols_symbol_fundamentals(symbol))
            .await
    }

    pub async fn fundamentals_series(
        &self,
        symbol: &str,
        ids: &[&str],
    ) -> Result<FundamentalsSeriesResponse, Error> {
        if ids.is_empty() || ids.iter().any(|id| id.trim().is_empty()) {
            return Err(Error::InvalidRequest(
                "At least one non-empty fundamental ID is required",
            ));
        }
        self.request(
            self.inner
                .get_v3_symbols_symbol_fundamentals_series(symbol, ids.join(",")),
        )
        .await
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<GetV3SymbolsSearchResponse, Error> {
        self.request(self.inner.get_v3_symbols_search(
            query.query.as_deref(),
            query.exchange.as_deref(),
            query.kind,
            query.country.as_deref(),
            query.page,
        ))
        .await
    }
}
