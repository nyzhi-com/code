use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    pub timestamp: u64,
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
}

fn analytics_path() -> PathBuf {
    nyzhi_config::Config::data_dir().join("analytics.jsonl")
}

pub fn log_usage(entry: &UsageEntry) -> Result<()> {
    use std::io::Write;
    let path = analytics_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn load_entries() -> Result<Vec<UsageEntry>> {
    let path = analytics_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    let entries: Vec<UsageEntry> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(entries)
}

pub struct CostReport {
    pub period: String,
    pub total_cost: f64,
    pub total_input: u64,
    pub total_output: u64,
    pub by_provider: Vec<(String, f64)>,
    pub by_model: Vec<(String, f64)>,
}

impl CostReport {
    pub fn display(&self) -> String {
        let mut lines = vec![];
        lines.push(format!("=== {} Cost Report ===", self.period));
        lines.push(format!("Total cost:   ${:.4}", self.total_cost));
        lines.push(format!("Total input:  {} tokens", self.total_input));
        lines.push(format!("Total output: {} tokens", self.total_output));
        if !self.by_provider.is_empty() {
            lines.push(String::new());
            lines.push("By Provider:".to_string());
            for (name, cost) in &self.by_provider {
                lines.push(format!("  {name:<20} ${cost:.4}"));
            }
        }
        if !self.by_model.is_empty() {
            lines.push(String::new());
            lines.push("By Model:".to_string());
            for (name, cost) in &self.by_model {
                lines.push(format!("  {name:<30} ${cost:.4}"));
            }
        }
        lines.join("\n")
    }
}

pub fn generate_report(entries: &[UsageEntry], period: &str, since_ts: u64) -> CostReport {
    use std::collections::HashMap;

    let filtered: Vec<&UsageEntry> = entries.iter().filter(|e| e.timestamp >= since_ts).collect();

    let total_cost: f64 = filtered.iter().map(|e| e.cost_usd).sum();
    let total_input: u64 = filtered.iter().map(|e| e.input_tokens).sum();
    let total_output: u64 = filtered.iter().map(|e| e.output_tokens).sum();

    let mut by_provider: HashMap<String, f64> = HashMap::new();
    let mut by_model: HashMap<String, f64> = HashMap::new();

    for e in &filtered {
        *by_provider.entry(e.provider.clone()).or_default() += e.cost_usd;
        *by_model.entry(e.model.clone()).or_default() += e.cost_usd;
    }

    let mut providers: Vec<_> = by_provider.into_iter().collect();
    providers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut models: Vec<_> = by_model.into_iter().collect();
    models.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    CostReport {
        period: period.to_string(),
        total_cost,
        total_input,
        total_output,
        by_provider: providers,
        by_model: models,
    }
}

pub fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
