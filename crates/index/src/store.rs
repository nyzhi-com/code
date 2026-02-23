use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::Result;
use rusqlite::{params, Connection};
use sha2::Digest;

use crate::chunker::Chunk;
use crate::watcher::FileEntry;

#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub file_count: usize,
    pub chunk_count: usize,
    pub vector_count: usize,
    pub db_size_bytes: u64,
}

pub struct Store {
    conn: std::sync::Mutex<Connection>,
    vectors: RwLock<VectorCache>,
    db_path: PathBuf,
}

struct VectorCache {
    ids: Vec<i64>,
    embeddings: Vec<Vec<f32>>,
    file_paths: Vec<String>,
    start_lines: Vec<usize>,
    end_lines: Vec<usize>,
    chunk_texts: Vec<String>,
}

impl Default for VectorCache {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            embeddings: Vec::new(),
            file_paths: Vec::new(),
            start_lines: Vec::new(),
            end_lines: Vec::new(),
            chunk_texts: Vec::new(),
        }
    }
}

impl Store {
    pub fn open(project_root: &Path) -> Result<Self> {
        let db_dir = index_dir(project_root);
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("index.db");

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                mtime INTEGER NOT NULL DEFAULT 0,
                chunk_count INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL REFERENCES files(path) ON DELETE CASCADE,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                chunk_text TEXT NOT NULL,
                embedding BLOB,
                model_id TEXT,
                dims INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_chunks_file ON chunks(file_path);
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;

        let store = Self {
            conn: std::sync::Mutex::new(conn),
            vectors: RwLock::new(VectorCache::default()),
            db_path,
        };

        store.load_vectors()?;
        Ok(store)
    }

    pub fn file_hashes(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path, hash FROM files")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let (path, hash) = row?;
            map.insert(path, hash);
        }
        Ok(map)
    }

    pub fn file_hash(&self, path: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT hash FROM files WHERE path = ?1")?;
        let result = stmt.query_row(params![path], |row| row.get::<_, String>(0));
        match result {
            Ok(h) => Ok(Some(h)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_file(&self, path: &str, hash: &str, chunk_count: usize) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO files (path, hash, chunk_count) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET hash = ?2, chunk_count = ?3",
            params![path, hash, chunk_count as i64],
        )?;
        Ok(())
    }

    pub fn replace_chunks(
        &self,
        file_path: &str,
        chunks: &[Chunk],
        embeddings: &[Vec<f32>],
        model_id: &str,
        dims: usize,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM chunks WHERE file_path = ?1",
            params![file_path],
        )?;

        let mut stmt = conn.prepare(
            "INSERT INTO chunks (file_path, start_line, end_line, content_hash, chunk_text, embedding, model_id, dims)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;

        for (i, chunk) in chunks.iter().enumerate() {
            let content_hash = hex::encode(sha2::Sha256::digest(chunk.text.as_bytes()));

            let embedding_blob: Option<Vec<u8>> = embeddings
                .get(i)
                .map(|emb| emb.iter().flat_map(|f| f.to_le_bytes()).collect());

            stmt.execute(params![
                file_path,
                chunk.start_line as i64,
                chunk.end_line as i64,
                content_hash,
                chunk.text,
                embedding_blob,
                model_id,
                dims as i64,
            ])?;
        }

        Ok(())
    }

    pub fn remove_deleted(&self, current_files: &[FileEntry]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path FROM files")?;
        let stored: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        let current_set: std::collections::HashSet<&str> =
            current_files.iter().map(|e| e.rel_path.as_str()).collect();

        for path in &stored {
            if !current_set.contains(path.as_str()) {
                conn.execute("DELETE FROM files WHERE path = ?1", params![path])?;
            }
        }
        Ok(())
    }

    pub fn load_vectors(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, start_line, end_line, chunk_text, embedding, dims
             FROM chunks WHERE embedding IS NOT NULL",
        )?;

        let mut cache = VectorCache::default();

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let file_path: String = row.get(1)?;
            let start_line: i64 = row.get(2)?;
            let end_line: i64 = row.get(3)?;
            let chunk_text: String = row.get(4)?;
            let blob: Vec<u8> = row.get(5)?;
            let dims: i64 = row.get(6)?;
            Ok((
                id,
                file_path,
                start_line as usize,
                end_line as usize,
                chunk_text,
                blob,
                dims as usize,
            ))
        })?;

        for row in rows {
            let (id, file_path, start, end_line, text, blob, dims) = row?;
            if dims == 0 || blob.len() != dims * 4 {
                continue;
            }
            let embedding: Vec<f32> = blob
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();

            cache.ids.push(id);
            cache.embeddings.push(embedding);
            cache.file_paths.push(file_path);
            cache.start_lines.push(start);
            cache.end_lines.push(end_line);
            cache.chunk_texts.push(text);
        }

        *self.vectors.write().unwrap() = cache;
        Ok(())
    }

    pub fn search(
        &self,
        query_vec: &[f32],
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<crate::SearchResult>> {
        let cache = self.vectors.read().unwrap();
        if cache.embeddings.is_empty() {
            return Ok(vec![]);
        }

        let query_tokens: Vec<String> = query_text
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| s.len() >= 2)
            .map(|s| s.to_lowercase())
            .collect();

        let mut scored: Vec<(usize, f32)> = cache
            .embeddings
            .iter()
            .enumerate()
            .map(|(i, emb)| {
                let mut score = cosine_similarity(query_vec, emb);

                if !query_tokens.is_empty() {
                    let text_lower = cache.chunk_texts[i].to_lowercase();
                    let hits = query_tokens
                        .iter()
                        .filter(|t| text_lower.contains(t.as_str()))
                        .count();
                    let keyword_boost = (hits as f32 / query_tokens.len() as f32) * 0.15;
                    score += keyword_boost;
                }

                (i, score)
            })
            .filter(|(_, s)| *s > 0.05)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit * 3);

        let mut results = Vec::new();
        let mut seen_files: HashMap<String, usize> = HashMap::new();

        for (idx, score) in scored {
            let file = &cache.file_paths[idx];
            let count = seen_files.entry(file.clone()).or_insert(0);
            if *count >= 3 {
                continue;
            }
            *count += 1;

            let content = &cache.chunk_texts[idx];
            let preview = if content.lines().count() > 15 {
                let lines: Vec<&str> = content.lines().take(15).collect();
                format!(
                    "{}\n  ... ({} more lines)",
                    lines.join("\n"),
                    content.lines().count() - 15
                )
            } else {
                content.clone()
            };

            results.push(crate::SearchResult {
                file: file.clone(),
                start_line: cache.start_lines[idx],
                end_line: cache.end_lines[idx],
                score,
                content: preview,
            });

            if results.len() >= limit {
                break;
            }
        }

        Ok(results)
    }

    pub fn vector_count(&self) -> usize {
        self.vectors.read().unwrap().embeddings.len()
    }

    pub fn stats(&self) -> Result<IndexStats> {
        let conn = self.conn.lock().unwrap();
        let file_count: usize = conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as usize;
        let chunk_count: usize = conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as usize;
        let vector_count = self.vector_count();

        let db_size_bytes = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(IndexStats {
            file_count,
            chunk_count,
            vector_count,
            db_size_bytes,
        })
    }
}

fn index_dir(project_root: &Path) -> PathBuf {
    let hash = hex::encode(&sha2::Sha256::digest(project_root.to_string_lossy().as_bytes())[..8]);
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nyzhi")
        .join("index")
        .join(hash)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-10 {
        0.0
    } else {
        dot / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }
}
