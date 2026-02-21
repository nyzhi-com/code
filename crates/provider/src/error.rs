#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {status} - {body}")]
    HttpError { status: u16, body: String },

    #[error("SSE stream error: {0}")]
    StreamError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Rate limited. Retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Context window exceeded: {0}")]
    ContextOverflow(String),
}
