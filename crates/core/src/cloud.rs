use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub endpoint: String,
    pub api_token: String,
    #[serde(default = "default_image")]
    pub image: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_image() -> String {
    "ghcr.io/nyzhi/agent:latest".to_string()
}

fn default_timeout() -> u64 {
    600
}

impl CloudConfig {
    pub fn from_env() -> Option<Self> {
        let endpoint = std::env::var("NYZHI_CLOUD_ENDPOINT").ok()?;
        let token = std::env::var("NYZHI_CLOUD_TOKEN").ok()?;
        Some(Self {
            endpoint,
            api_token: token,
            image: std::env::var("NYZHI_CLOUD_IMAGE").unwrap_or_else(|_| default_image()),
            timeout_secs: std::env::var("NYZHI_CLOUD_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_timeout),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudJob {
    pub id: String,
    pub status: CloudJobStatus,
    pub prompt: String,
    pub model: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CloudJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    TimedOut,
}

impl std::fmt::Display for CloudJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudJobStatus::Queued => write!(f, "queued"),
            CloudJobStatus::Running => write!(f, "running"),
            CloudJobStatus::Completed => write!(f, "completed"),
            CloudJobStatus::Failed => write!(f, "failed"),
            CloudJobStatus::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Dispatch a prompt to the cloud agent endpoint.
pub async fn dispatch(config: &CloudConfig, prompt: &str, model: Option<&str>) -> Result<CloudJob> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "prompt": prompt,
        "model": model,
        "image": config.image,
        "timeout_secs": config.timeout_secs,
    });

    let resp = client
        .post(format!("{}/v1/jobs", config.endpoint))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .json(&body)
        .send()
        .await
        .context("Cloud dispatch failed")?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Cloud API error ({}): {}", status, text);
    }

    let job: CloudJob = resp.json().await.context("Failed to parse cloud job")?;
    Ok(job)
}

/// Check the status of a cloud job.
pub async fn poll_status(config: &CloudConfig, job_id: &str) -> Result<CloudJob> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/v1/jobs/{}", config.endpoint, job_id))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
        .context("Cloud poll failed")?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Cloud API error ({}): {}", status, text);
    }

    let job: CloudJob = resp.json().await.context("Failed to parse cloud job")?;
    Ok(job)
}

/// Cancel a running cloud job.
pub async fn cancel(config: &CloudConfig, job_id: &str) -> Result<()> {
    let client = reqwest::Client::new();

    let resp = client
        .delete(format!("{}/v1/jobs/{}", config.endpoint, job_id))
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
        .context("Cloud cancel failed")?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Cloud cancel error: {}", text);
    }

    Ok(())
}

pub fn is_configured() -> bool {
    CloudConfig::from_env().is_some()
}

pub fn status_message() -> String {
    if is_configured() {
        "Cloud agents available. Use `nyz cloud <prompt>` to dispatch.".to_string()
    } else {
        "Cloud agents require NYZHI_CLOUD_ENDPOINT and NYZHI_CLOUD_TOKEN.\n\
         Set these env vars to enable remote agent execution."
            .to_string()
    }
}
