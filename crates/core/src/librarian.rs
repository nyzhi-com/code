use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntry {
    pub source: String,
    pub content: String,
    pub fetched_at: u64,
    pub ttl_secs: u64,
}

impl DocEntry {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.fetched_at + self.ttl_secs
    }
}

pub struct Librarian {
    cache_dir: PathBuf,
    index: HashMap<String, DocEntry>,
}

const DEFAULT_TTL: u64 = 86400 * 7; // 7 days

impl Librarian {
    pub fn new(data_dir: &Path) -> Self {
        let cache_dir = data_dir.join("librarian");
        let _ = std::fs::create_dir_all(&cache_dir);
        let index = load_index(&cache_dir);
        Self { cache_dir, index }
    }

    pub fn get(&self, key: &str) -> Option<&DocEntry> {
        let entry = self.index.get(key)?;
        if entry.is_expired() {
            return None;
        }
        Some(entry)
    }

    pub fn get_content(&self, key: &str) -> Option<String> {
        let entry = self.get(key)?;
        let content_file = self.cache_dir.join(slug(key));
        std::fs::read_to_string(content_file).ok().or_else(|| Some(entry.content.clone()))
    }

    pub fn put(&mut self, key: &str, source: &str, content: &str, ttl_secs: Option<u64>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let ttl = ttl_secs.unwrap_or(DEFAULT_TTL);

        let entry = DocEntry {
            source: source.to_string(),
            content: if content.len() > 512 {
                content[..512].to_string()
            } else {
                content.to_string()
            },
            fetched_at: now,
            ttl_secs: ttl,
        };

        let content_path = self.cache_dir.join(slug(key));
        let _ = std::fs::write(&content_path, content);

        self.index.insert(key.to_string(), entry);
        save_index(&self.cache_dir, &self.index);
    }

    pub fn list_keys(&self) -> Vec<(&str, bool)> {
        self.index
            .iter()
            .map(|(k, v)| (k.as_str(), v.is_expired()))
            .collect()
    }

    pub fn evict_expired(&mut self) -> usize {
        let expired: Vec<String> = self
            .index
            .iter()
            .filter(|(_, v)| v.is_expired())
            .map(|(k, _)| k.clone())
            .collect();
        let count = expired.len();
        for key in &expired {
            let _ = std::fs::remove_file(self.cache_dir.join(slug(key)));
            self.index.remove(key);
        }
        if count > 0 {
            save_index(&self.cache_dir, &self.index);
        }
        count
    }

    pub fn clear(&mut self) {
        for key in self.index.keys() {
            let _ = std::fs::remove_file(self.cache_dir.join(slug(key)));
        }
        self.index.clear();
        save_index(&self.cache_dir, &self.index);
    }

    pub fn stats(&self) -> (usize, usize) {
        let total = self.index.len();
        let expired = self.index.values().filter(|v| v.is_expired()).count();
        (total, expired)
    }

    /// Build a context string for injecting cached docs into a prompt.
    pub fn build_context(&self, keys: &[&str]) -> String {
        let mut parts = Vec::new();
        for &key in keys {
            if let Some(content) = self.get_content(key) {
                let truncated = if content.len() > 4000 {
                    format!("{}...\n(truncated)", &content[..4000])
                } else {
                    content
                };
                parts.push(format!("## {key}\n{truncated}"));
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("# Cached Documentation\n\n{}", parts.join("\n\n"))
        }
    }
}

fn slug(key: &str) -> String {
    let mut s = String::with_capacity(key.len());
    for c in key.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            s.push(c);
        } else {
            s.push('_');
        }
    }
    if s.len() > 100 {
        s.truncate(100);
    }
    format!("{s}.md")
}

fn index_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("_index.json")
}

fn load_index(cache_dir: &Path) -> HashMap<String, DocEntry> {
    let path = index_path(cache_dir);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_index(cache_dir: &Path, index: &HashMap<String, DocEntry>) {
    let path = index_path(cache_dir);
    if let Ok(json) = serde_json::to_string_pretty(index) {
        let _ = std::fs::write(path, json);
    }
}
