fn extract_error_message(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = v["error"]["message"].as_str() {
            return msg.to_string();
        }
        if let Some(msg) = v["message"].as_str() {
            return msg.to_string();
        }
        if let Some(msg) = v["error"].as_str() {
            return msg.to_string();
        }
    }
    let trimmed = body.trim();
    if trimmed.len() > 200 {
        format!("{}...", &trimmed[..200])
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("{status}: {}", extract_error_message(body))]
    HttpError { status: u16, body: String },

    #[error("{status}: {}", extract_error_message(body))]
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
