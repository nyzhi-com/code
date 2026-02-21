use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalNotifyConfig {
    #[serde(default)]
    pub webhook: Option<WebhookConfig>,
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,
    #[serde(default)]
    pub discord: Option<DiscordConfig>,
    #[serde(default)]
    pub slack: Option<SlackConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
}

pub async fn send_notifications(config: &ExternalNotifyConfig, message: &str) {
    let client = reqwest::Client::new();

    if let Some(webhook) = &config.webhook {
        let _ = client
            .post(&webhook.url)
            .json(&serde_json::json!({"text": message}))
            .send()
            .await;
    }

    if let Some(telegram) = &config.telegram {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            telegram.bot_token
        );
        let _ = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": telegram.chat_id,
                "text": message,
            }))
            .send()
            .await;
    }

    if let Some(discord) = &config.discord {
        let _ = client
            .post(&discord.webhook_url)
            .json(&serde_json::json!({"content": message}))
            .send()
            .await;
    }

    if let Some(slack) = &config.slack {
        let _ = client
            .post(&slack.webhook_url)
            .json(&serde_json::json!({"text": message}))
            .send()
            .await;
    }
}

pub fn has_any_configured(config: &ExternalNotifyConfig) -> bool {
    config.webhook.is_some()
        || config.telegram.is_some()
        || config.discord.is_some()
        || config.slack.is_some()
}
