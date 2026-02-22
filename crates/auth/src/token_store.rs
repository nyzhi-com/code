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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthStore {
    #[serde(flatten)]
    pub entries: HashMap<String, StoredToken>,
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
    let store: AuthStore = serde_json::from_str(&content)?;
    Ok(store)
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
    store.entries.insert(provider.to_string(), token.clone());
    save_store(&store)
}

pub fn load_token(provider: &str) -> Result<Option<StoredToken>> {
    let store = load_store()?;
    Ok(store.entries.get(provider).cloned())
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
