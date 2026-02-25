pub mod chunker;
pub mod embedder;
pub mod search;
pub mod store;
pub mod watcher;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::Result;

pub use search::SearchResult;
pub use store::IndexStats;

#[derive(Debug, Clone)]
pub struct IndexProgress {
    pub phase: &'static str,
    pub indexed: usize,
    pub total: usize,
    pub complete: bool,
}

impl Default for IndexProgress {
    fn default() -> Self {
        Self {
            phase: "idle",
            indexed: 0,
            total: 0,
            complete: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IndexOptions {
    pub embedding_mode: String,
    pub exclude: Vec<String>,
}

pub struct CodebaseIndex {
    store: store::Store,
    embedder: Arc<dyn embedder::Embedder>,
    project_root: PathBuf,
    exclude: Vec<String>,
    progress: Arc<Mutex<IndexProgress>>,
}

impl CodebaseIndex {
    /// Synchronous constructor. Opens (or creates) the SQLite index DB and
    /// selects the embedding backend. The index is NOT built yet -- call
    /// `build()` afterwards (typically in a background task).
    pub fn open_sync(project_root: &Path, api_key: Option<String>) -> Result<Self> {
        Self::open_sync_with_options(project_root, api_key, IndexOptions::default())
    }

    pub fn open_sync_with_options(
        project_root: &Path,
        api_key: Option<String>,
        options: IndexOptions,
    ) -> Result<Self> {
        let store = store::Store::open(project_root)?;

        let mode = options.embedding_mode.trim().to_ascii_lowercase();
        let embedder: Arc<dyn embedder::Embedder> = match mode.as_str() {
            "tfidf" | "local" => Arc::new(embedder::TfIdfEmbedder::new()),
            "api" | "openai" => {
                if let Some(key) = api_key {
                    Arc::new(embedder::ApiEmbedder::new(key))
                } else {
                    tracing::warn!(
                        "index.embedding={} requested but API key unavailable; falling back to tfidf",
                        mode
                    );
                    Arc::new(embedder::TfIdfEmbedder::new())
                }
            }
            "auto" | "" => {
                if let Some(key) = api_key {
                    Arc::new(embedder::ApiEmbedder::new(key))
                } else {
                    Arc::new(embedder::TfIdfEmbedder::new())
                }
            }
            other => {
                tracing::warn!(
                    "Unknown index.embedding mode '{}'; falling back to auto selection",
                    other
                );
                if let Some(key) = api_key {
                    Arc::new(embedder::ApiEmbedder::new(key))
                } else {
                    Arc::new(embedder::TfIdfEmbedder::new())
                }
            }
        };

        Ok(Self {
            store,
            embedder,
            project_root: project_root.to_path_buf(),
            exclude: options.exclude,
            progress: Arc::new(Mutex::new(IndexProgress::default())),
        })
    }

    pub async fn open(project_root: &Path, api_key: Option<String>) -> Result<Self> {
        Self::open_sync(project_root, api_key)
    }

    pub async fn open_with_options(
        project_root: &Path,
        api_key: Option<String>,
        options: IndexOptions,
    ) -> Result<Self> {
        Self::open_sync_with_options(project_root, api_key, options)
    }

    pub async fn build(&self) -> Result<IndexStats> {
        {
            let mut p = self.progress.lock().await;
            p.phase = "walking";
            p.indexed = 0;
            p.total = 0;
            p.complete = false;
        }

        let existing = self.store.file_hashes()?;
        let walk = watcher::walk_project(&self.project_root, &self.exclude)?;

        {
            let mut p = self.progress.lock().await;
            p.total = walk.len();
            p.phase = "indexing";
        }

        self.store.remove_deleted(&walk)?;

        let dims = self.embedder.dimensions();
        let mut indexed = 0usize;

        for entry in &walk {
            let rel = entry.rel_path.clone();
            if let Some(old_hash) = existing.get(&rel) {
                if *old_hash == entry.hash {
                    indexed += 1;
                    let mut p = self.progress.lock().await;
                    p.indexed = indexed;
                    continue;
                }
            }

            let content = match std::fs::read_to_string(&entry.abs_path) {
                Ok(c) => c,
                Err(_) => {
                    indexed += 1;
                    continue;
                }
            };

            let chunks = chunker::chunk_file(&rel, &content);
            if chunks.is_empty() {
                self.store.upsert_file(&rel, &entry.hash, 0)?;
                indexed += 1;
                let mut p = self.progress.lock().await;
                p.indexed = indexed;
                continue;
            }

            let texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
            let embeddings = self.embedder.embed(&texts).await?;

            let model_id = self.embedder.model_id();
            self.store.upsert_file(&rel, &entry.hash, chunks.len())?;
            self.store
                .replace_chunks(&rel, &chunks, &embeddings, model_id, dims)?;

            indexed += 1;
            let mut p = self.progress.lock().await;
            p.indexed = indexed;
        }

        self.store.load_vectors()?;

        {
            let mut p = self.progress.lock().await;
            p.phase = "complete";
            p.indexed = indexed;
            p.complete = true;
        }

        self.stats()
    }

    pub async fn update_file(&self, rel_path: &str) -> Result<()> {
        let abs = self.project_root.join(rel_path);
        let content = std::fs::read_to_string(&abs)?;
        let hash = watcher::hash_content(content.as_bytes());

        if let Some(old_hash) = self.store.file_hash(rel_path)? {
            if old_hash == hash {
                return Ok(());
            }
        }

        let chunks = chunker::chunk_file(rel_path, &content);
        let texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
        let embeddings = self.embedder.embed(&texts).await?;
        let dims = self.embedder.dimensions();
        let model_id = self.embedder.model_id();

        self.store.upsert_file(rel_path, &hash, chunks.len())?;
        self.store
            .replace_chunks(rel_path, &chunks, &embeddings, model_id, dims)?;
        self.store.load_vectors()?;

        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query_embedding = self.embedder.embed(&[query]).await?;
        if query_embedding.is_empty() {
            return Ok(vec![]);
        }
        self.store.search(&query_embedding[0], query, limit)
    }

    pub async fn auto_context(&self, query: &str, limit: usize) -> Result<String> {
        let results = self.search(query, limit).await?;
        if results.is_empty() {
            return Ok(String::new());
        }

        let mut xml = String::from("<codebase_context>\n");
        for r in &results {
            xml.push_str(&format!(
                "<chunk file=\"{}\" lines=\"{}-{}\" score=\"{:.2}\">\n{}\n</chunk>\n",
                r.file, r.start_line, r.end_line, r.score, r.content
            ));
        }
        xml.push_str("</codebase_context>");
        Ok(xml)
    }

    pub fn stats(&self) -> Result<IndexStats> {
        self.store.stats()
    }

    pub async fn progress(&self) -> IndexProgress {
        self.progress.lock().await.clone()
    }

    pub fn is_ready(&self) -> bool {
        self.store.vector_count() > 0
    }
}
