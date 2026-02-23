use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::types::ModelInfo;

pub type ModelCacheHandle = Arc<Mutex<ModelCache>>;

pub struct ModelCache {
    entries: HashMap<String, CachedEntry>,
    ttl: Duration,
}

struct CachedEntry {
    models: Vec<ModelInfo>,
    fetched_at: Instant,
}

impl ModelCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(300),
        }
    }

    pub fn handle() -> ModelCacheHandle {
        Arc::new(Mutex::new(Self::new()))
    }

    pub fn get(&self, provider_id: &str) -> Option<Vec<ModelInfo>> {
        let entry = self.entries.get(provider_id)?;
        if entry.fetched_at.elapsed() > self.ttl {
            return None;
        }
        Some(entry.models.clone())
    }

    pub fn set(&mut self, provider_id: &str, models: Vec<ModelInfo>) {
        self.entries.insert(
            provider_id.to_string(),
            CachedEntry {
                models,
                fetched_at: Instant::now(),
            },
        );
    }

    pub fn is_stale(&self, provider_id: &str) -> bool {
        match self.entries.get(provider_id) {
            Some(entry) => entry.fetched_at.elapsed() > self.ttl,
            None => true,
        }
    }

    pub fn invalidate(&mut self, provider_id: &str) {
        self.entries.remove(provider_id);
    }
}
