#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {status} - {body}")]
    HttpError { status: u16, body: String },

    #[error("Server error: {status} - {body}")]
    ServerError { status: u16, body: String },

    #[error("SSE stream error: {0}")]
    StreamError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Rate limited. Retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Context window exceeded: {0}")]
    ContextOverflow(String),
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::ServerError { .. } | Self::StreamError(_)
        )
    }

    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after_ms } => Some(*retry_after_ms),
            _ => None,
        }
    }

    pub fn from_http(status: u16, body: String, retry_after: Option<&str>) -> Self {
        if status == 429 {
            let retry_ms = retry_after
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| s * 1000)
                .unwrap_or(1000);
            return Self::RateLimited {
                retry_after_ms: retry_ms,
            };
        }
        if status >= 500 {
            return Self::ServerError { status, body };
        }
        Self::HttpError { status, body }
    }
}
