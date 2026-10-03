/// Errors returned by the convenience client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP success containing an error for the requested quote.
    #[error("quote unavailable for {code}: {message}")]
    Quote {
        code: String,
        message: String,
        reason: Option<String>,
    },
    /// Archive history reported unavailable data; retain the provider's details.
    #[error("history unavailable for {symbol}: {code}")]
    HistoryUnavailable {
        symbol: String,
        code: String,
        message: Option<String>,
        available_start_date: Option<String>,
    },
    /// Convenience request exceeded its configured deadline.
    #[error("request exceeded timeout of {0:?}")]
    Timeout(std::time::Duration),
    /// Transport, request construction or response-body limit failure.
    #[error("transport error: {0}")]
    Transport(#[source] crate::generated::client::HttpError),
    /// A non-success HTTP response. The response body is retained for diagnosis.
    #[error("InsightSentry responded with HTTP {status}: {body}")]
    Api {
        status: u16,
        body: String,
        /// Raw Retry-After header; may be seconds or an HTTP date.
        retry_after: Option<String>,
    },
    /// A success response that could not be decoded according to the schema.
    #[error("InsightSentry response did not match the schema: {0}")]
    Decode(String),
    /// Invalid arguments rejected before making a request.
    #[error("invalid request: {0}")]
    InvalidRequest(&'static str),
}

impl Error {
    pub fn is_unauthorized(&self) -> bool {
        matches!(
            self,
            Self::Api {
                status: 401 | 403,
                ..
            }
        )
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::Api { status: 429, .. })
    }
}

impl<E: std::fmt::Debug> From<crate::generated::client::ApiOpError<E>> for Error {
    fn from(error: crate::generated::client::ApiOpError<E>) -> Self {
        use crate::generated::client::ApiOpError;
        match error {
            ApiOpError::Transport(error) => Self::Transport(error),
            ApiOpError::Api(error) => {
                if (200..300).contains(&error.status) {
                    if let Some(reason) = error.parse_error {
                        return Self::Decode(reason);
                    }
                }
                Self::Api {
                    status: error.status,
                    retry_after: error
                        .headers
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_owned),
                    body: error.body,
                }
            }
        }
    }
}
