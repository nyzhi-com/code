use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    #[serde(default)]
    pub label: Option<String>,
    pub token: StoredToken,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub rate_limited_until: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Supports deserializing both the old single-token format and the new multi-account format.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum StoreValue {
    Multi(Vec<AccountEntry>),
    Legacy(StoredToken),
}

impl StoreValue {
    fn into_accounts(self) -> Vec<AccountEntry> {
        match self {
            StoreValue::Multi(accounts) => accounts,
            StoreValue::Legacy(token) => vec![AccountEntry {
                label: None,
                token,
                active: true,
                rate_limited_until: None,
            }],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthStore {
    #[serde(flatten)]
    pub entries: HashMap<String, Vec<AccountEntry>>,
}

fn auth_file_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nyzhi")
        .join("auth.json")
}

fn load_store() -> Result<AuthStore> {
    let path = auth_file_path();
    if !path.exists() {
        return Ok(AuthStore::default());
    }
    let content = std::fs::read_to_string(&path)?;
    if content.trim().is_empty() {
        return Ok(AuthStore::default());
    }
    let raw: HashMap<String, StoreValue> = serde_json::from_str(&content)?;
    let entries = raw
        .into_iter()
        .map(|(k, v)| (k, v.into_accounts()))
        .collect();
    Ok(AuthStore { entries })
}

fn save_store(store: &AuthStore) -> Result<()> {
    let path = auth_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(store)?;
    std::fs::write(&tmp, &content)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn store_token(provider: &str, token: &StoredToken) -> Result<()> {
    let mut store = load_store()?;
    let accounts = store.entries.entry(provider.to_string()).or_default();
    if let Some(entry) = accounts.iter_mut().find(|e| e.active) {
        entry.token = token.clone();
    } else if accounts.is_empty() {
        accounts.push(AccountEntry {
            label: None,
            token: token.clone(),
            active: true,
            rate_limited_until: None,
        });
    } else {
        accounts[0].token = token.clone();
        accounts[0].active = true;
    }
    save_store(&store)
}

pub fn store_account(provider: &str, token: &StoredToken, label: Option<&str>) -> Result<()> {
    let mut store = load_store()?;
    let accounts = store.entries.entry(provider.to_string()).or_default();

    if let Some(existing) = accounts.iter_mut().find(|e| e.label.as_deref() == label) {
        existing.token = token.clone();
        return save_store(&store);
    }

    let is_first = accounts.is_empty();
    accounts.push(AccountEntry {
        label: label.map(|s| s.to_string()),
        token: token.clone(),
        active: is_first,
        rate_limited_until: None,
    });
    save_store(&store)
}

pub fn load_token(provider: &str) -> Result<Option<StoredToken>> {
    let store = load_store()?;
    let accounts = match store.entries.get(provider) {
        Some(a) if !a.is_empty() => a,
        _ => return Ok(None),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(entry) = accounts
        .iter()
        .find(|e| e.active && e.rate_limited_until.map_or(true, |until| now >= until))
    {
        return Ok(Some(entry.token.clone()));
    }
    if let Some(entry) = accounts
        .iter()
        .find(|e| e.rate_limited_until.map_or(true, |until| now >= until))
    {
        return Ok(Some(entry.token.clone()));
    }
    Ok(accounts.first().map(|e| e.token.clone()))
}

pub fn active_account(provider: &str) -> Result<Option<AccountEntry>> {
    let store = load_store()?;
    Ok(store
        .entries
        .get(provider)
        .and_then(|accounts| accounts.iter().find(|e| e.active).cloned()))
}

pub fn list_accounts(provider: &str) -> Result<Vec<AccountEntry>> {
    let store = load_store()?;
    Ok(store.entries.get(provider).cloned().unwrap_or_default())
}

pub fn remove_account(provider: &str, label: Option<&str>) -> Result<bool> {
    let mut store = load_store()?;
    let accounts = match store.entries.get_mut(provider) {
        Some(a) => a,
        None => return Ok(false),
    };
    let before = accounts.len();
    accounts.retain(|e| e.label.as_deref() != label);
    if accounts.len() < before {
        if !accounts.is_empty() && !accounts.iter().any(|e| e.active) {
            accounts[0].active = true;
        }
        save_store(&store)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn rotate_on_rate_limit(provider: &str, wait_seconds: u64) -> Result<Option<StoredToken>> {
    let mut store = load_store()?;
    let accounts = match store.entries.get_mut(provider) {
        Some(a) if a.len() > 1 => a,
        _ => return Ok(None),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(current) = accounts.iter_mut().find(|e| e.active) {
        current.active = false;
        current.rate_limited_until = Some(now + wait_seconds);
    }

    let next = accounts
        .iter_mut()
        .find(|e| !e.active && e.rate_limited_until.map_or(true, |until| now >= until));

    if let Some(next_account) = next {
        next_account.active = true;
        let token = next_account.token.clone();
        save_store(&store)?;
        tracing::info!("Rotated to next account for {provider}");
        Ok(Some(token))
    } else {
        if let Some(first) = accounts.first_mut() {
            first.active = true;
        }
        save_store(&store)?;
        Ok(None)
    }
}

pub fn delete_token(provider: &str) -> Result<()> {
    let mut store = load_store()?;
    store.entries.remove(provider);
    save_store(&store)
}

pub fn store_api_key(provider: &str, api_key: &str) -> Result<()> {
    let token = StoredToken {
        access_token: api_key.to_string(),
        refresh_token: None,
        expires_at: None,
        provider: provider.to_string(),
    };
    store_token(provider, &token)
}

pub fn list_providers() -> Result<Vec<String>> {
    let store = load_store()?;
    Ok(store.entries.keys().cloned().collect())
}

/// Migrate tokens from keyring to auth.json (one-time, best-effort).
pub fn migrate_from_keyring() {
    let providers = ["openai", "anthropic", "gemini"];
    for prov in &providers {
        if let Ok(Some(_)) = load_token(prov) {
            continue;
        }
        if let Ok(entry) = keyring::Entry::new("nyzhi", prov) {
            if let Ok(json) = entry.get_password() {
                if let Ok(token) = serde_json::from_str::<StoredToken>(&json) {
                    let _ = store_token(prov, &token);
                    tracing::info!("Migrated {prov} credential from keyring to auth.json");
                    let _ = entry.delete_credential();
                }
            }
        }
    }
}
