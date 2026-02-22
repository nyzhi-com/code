use anyhow::Result;

use crate::token_store::StoredToken;

/// Login via ChatGPT Plus/Pro subscription OAuth.
/// Delegates to the OpenAI PKCE flow and re-labels the token as "chatgpt".
pub async fn login() -> Result<StoredToken> {
    let mut token = super::openai::login().await?;
    token.provider = "chatgpt".to_string();
    crate::token_store::store_token("chatgpt", &token)?;
    Ok(token)
}
