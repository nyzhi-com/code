use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub url: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketUpdate {
    pub status: Option<String>,
    pub comment: Option<String>,
    pub labels_add: Vec<String>,
    pub labels_remove: Vec<String>,
}

impl Default for TicketUpdate {
    fn default() -> Self {
        Self {
            status: None,
            comment: None,
            labels_add: Vec::new(),
            labels_remove: Vec::new(),
        }
    }
}

/// Linear API client.
pub struct LinearClient {
    api_key: String,
    client: reqwest::Client,
}

const LINEAR_API: &str = "https://api.linear.app/graphql";

impl LinearClient {
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("LINEAR_API_KEY").ok()?;
        Some(Self {
            api_key: key,
            client: reqwest::Client::new(),
        })
    }

    pub async fn fetch_issue(&self, issue_id: &str) -> Result<Ticket> {
        let query = format!(
            r#"{{ "query": "{{ issue(id: \"{issue_id}\") {{ id title description state {{ name }} assignee {{ name }} labels {{ nodes {{ name }} }} url }} }}" }}"#
        );

        let resp = self
            .client
            .post(LINEAR_API)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .body(query)
            .send()
            .await
            .context("Linear API request failed")?;

        let body: serde_json::Value = resp.json().await.context("Failed to parse Linear response")?;
        let issue = body
            .pointer("/data/issue")
            .context("No issue data in Linear response")?;

        Ok(Ticket {
            id: issue["id"].as_str().unwrap_or(issue_id).to_string(),
            title: issue["title"].as_str().unwrap_or("").to_string(),
            description: issue["description"].as_str().unwrap_or("").to_string(),
            status: issue
                .pointer("/state/name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            assignee: issue
                .pointer("/assignee/name")
                .and_then(|v| v.as_str())
                .map(String::from),
            labels: issue
                .pointer("/labels/nodes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|l| l["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            url: issue["url"].as_str().unwrap_or("").to_string(),
            provider: "linear".to_string(),
        })
    }

    pub async fn add_comment(&self, issue_id: &str, body: &str) -> Result<()> {
        let escaped_body = body.replace('"', "\\\"").replace('\n', "\\n");
        let query = format!(
            r#"{{ "query": "mutation {{ commentCreate(input: {{ issueId: \"{issue_id}\", body: \"{escaped_body}\" }}) {{ success }} }}" }}"#
        );

        let resp = self
            .client
            .post(LINEAR_API)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .body(query)
            .send()
            .await
            .context("Linear comment failed")?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Linear API error: {}", text);
        }

        Ok(())
    }

    pub async fn list_assigned(&self) -> Result<Vec<Ticket>> {
        let query = r#"{ "query": "{ viewer { assignedIssues(first: 20, filter: { state: { type: { nin: [\"completed\", \"canceled\"] } } }) { nodes { id title description state { name } assignee { name } labels { nodes { name } } url } } } }" }"#;

        let resp = self
            .client
            .post(LINEAR_API)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .body(query)
            .send()
            .await
            .context("Linear API request failed")?;

        let body: serde_json::Value = resp.json().await.context("Failed to parse Linear response")?;
        let nodes = body
            .pointer("/data/viewer/assignedIssues/nodes")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let tickets = nodes
            .iter()
            .map(|issue| Ticket {
                id: issue["id"].as_str().unwrap_or("").to_string(),
                title: issue["title"].as_str().unwrap_or("").to_string(),
                description: issue["description"].as_str().unwrap_or("").to_string(),
                status: issue
                    .pointer("/state/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                assignee: issue
                    .pointer("/assignee/name")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                labels: issue
                    .pointer("/labels/nodes")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|l| l["name"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                url: issue["url"].as_str().unwrap_or("").to_string(),
                provider: "linear".to_string(),
            })
            .collect();

        Ok(tickets)
    }
}

/// Jira API client (REST v3).
pub struct JiraClient {
    base_url: String,
    email: String,
    api_token: String,
    client: reqwest::Client,
}

impl JiraClient {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("JIRA_BASE_URL").ok()?;
        let email = std::env::var("JIRA_EMAIL").ok()?;
        let token = std::env::var("JIRA_API_TOKEN").ok()?;
        Some(Self {
            base_url,
            email,
            api_token: token,
            client: reqwest::Client::new(),
        })
    }

    fn auth_header(&self) -> String {
        use base64::Engine;
        let creds = format!("{}:{}", self.email, self.api_token);
        let encoded = base64::engine::general_purpose::STANDARD.encode(creds);
        format!("Basic {encoded}")
    }

    pub async fn fetch_issue(&self, issue_key: &str) -> Result<Ticket> {
        let url = format!(
            "{}/rest/api/3/issue/{}?fields=summary,description,status,assignee,labels",
            self.base_url, issue_key
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .send()
            .await
            .context("Jira API request failed")?;

        let body: serde_json::Value = resp.json().await.context("Failed to parse Jira response")?;
        let fields = body.get("fields").context("No fields in Jira issue")?;

        let desc_text = fields
            .pointer("/description/content")
            .and_then(|v| v.as_array())
            .map(|blocks| extract_jira_text(blocks))
            .unwrap_or_default();

        Ok(Ticket {
            id: body["key"].as_str().unwrap_or(issue_key).to_string(),
            title: fields["summary"].as_str().unwrap_or("").to_string(),
            description: desc_text,
            status: fields
                .pointer("/status/name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            assignee: fields
                .pointer("/assignee/displayName")
                .and_then(|v| v.as_str())
                .map(String::from),
            labels: fields["labels"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            url: format!("{}/browse/{}", self.base_url, issue_key),
            provider: "jira".to_string(),
        })
    }

    pub async fn add_comment(&self, issue_key: &str, body_text: &str) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}/comment", self.base_url, issue_key);

        let body = serde_json::json!({
            "body": {
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": body_text
                    }]
                }]
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Jira comment failed")?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Jira API error: {}", text);
        }

        Ok(())
    }
}

fn extract_jira_text(blocks: &[serde_json::Value]) -> String {
    let mut text = String::new();
    for block in blocks {
        if let Some(content) = block.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                    text.push_str(t);
                }
            }
        }
        text.push('\n');
    }
    text.trim().to_string()
}

pub fn available_integrations() -> Vec<&'static str> {
    let mut available = Vec::new();
    if std::env::var("LINEAR_API_KEY").is_ok() {
        available.push("linear");
    }
    if std::env::var("JIRA_BASE_URL").is_ok() && std::env::var("JIRA_API_TOKEN").is_ok() {
        available.push("jira");
    }
    available
}
