use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

const CHUNK_SIZE: usize = 60;
const CHUNK_OVERLAP: usize = 10;
const MAX_FILE_SIZE: usize = 512 * 1024;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub score: f64,
}

#[derive(Default)]
pub struct SemanticIndex {
    chunks: Vec<Chunk>,
    file_hashes: HashMap<String, String>,
    built: bool,
}

struct Chunk {
    file: String,
    start_line: usize,
    end_line: usize,
    content: String,
    terms: HashMap<String, f64>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_built(&self) -> bool {
        self.built
    }

    pub fn build(&mut self, project_root: &Path) -> Result<()> {
        let mut files = Vec::new();
        collect_indexable_files(project_root, project_root, &mut files, 5000);

        let mut new_chunks = Vec::new();
        let mut new_hashes = HashMap::new();
        let mut df: HashMap<String, usize> = HashMap::new();

        for rel_path in &files {
            let full = project_root.join(rel_path);
            let content = match std::fs::read(&full) {
                Ok(data) => data,
                Err(_) => continue,
            };
            if content.len() > MAX_FILE_SIZE {
                continue;
            }
            if content.len() > 512 && content[..512].contains(&0) {
                continue;
            }

            let hash = hex::encode(Sha256::digest(&content));
            if self.file_hashes.get(rel_path.as_str()) == Some(&hash) {
                for chunk in &self.chunks {
                    if chunk.file == *rel_path {
                        new_chunks.push(Chunk {
                            file: chunk.file.clone(),
                            start_line: chunk.start_line,
                            end_line: chunk.end_line,
                            content: chunk.content.clone(),
                            terms: chunk.terms.clone(),
                        });
                    }
                }
                new_hashes.insert(rel_path.clone(), hash);
                continue;
            }

            let text = String::from_utf8_lossy(&content);
            let lines: Vec<&str> = text.lines().collect();

            let mut i = 0;
            while i < lines.len() {
                let end = (i + CHUNK_SIZE).min(lines.len());
                let chunk_lines = &lines[i..end];
                let chunk_text = chunk_lines.join("\n");
                let terms = compute_tf(&chunk_text);

                for term in terms.keys() {
                    *df.entry(term.clone()).or_insert(0) += 1;
                }

                new_chunks.push(Chunk {
                    file: rel_path.clone(),
                    start_line: i + 1,
                    end_line: end,
                    content: chunk_text,
                    terms,
                });

                if end >= lines.len() {
                    break;
                }
                i += CHUNK_SIZE - CHUNK_OVERLAP;
            }

            new_hashes.insert(rel_path.clone(), hash);
        }

        let n_docs = new_chunks.len().max(1) as f64;
        for chunk in &mut new_chunks {
            for (term, tf) in chunk.terms.iter_mut() {
                let doc_freq = *df.get(term).unwrap_or(&1) as f64;
                let idf = (n_docs / doc_freq).ln() + 1.0;
                *tf *= idf;
            }
        }

        self.chunks = new_chunks;
        self.file_hashes = new_hashes;
        self.built = true;
        Ok(())
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        let query_terms = compute_tf(query);
        let mut scored: Vec<(usize, f64)> = self
            .chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| {
                let score = cosine_similarity(&query_terms, &chunk.terms);
                (i, score)
            })
            .filter(|(_, score)| *score > 0.01)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_results);

        scored
            .into_iter()
            .map(|(i, score)| {
                let chunk = &self.chunks[i];
                SearchResult {
                    file: chunk.file.clone(),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    content: chunk.content.clone(),
                    score,
                }
            })
            .collect()
    }

    pub fn file_count(&self) -> usize {
        self.file_hashes.len()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

fn compute_tf(text: &str) -> HashMap<String, f64> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total = 0usize;

    for word in tokenize(text) {
        if word.len() < 2 || is_stop_word(&word) {
            continue;
        }
        *counts.entry(word).or_insert(0) += 1;
        total += 1;
    }

    let total = total.max(1) as f64;
    counts
        .into_iter()
        .map(|(term, count)| (term, count as f64 / total))
        .collect()
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
        } else {
            if !current.is_empty() {
                split_camel_case(&current, &mut tokens);
                current.clear();
            }
        }
    }
    if !current.is_empty() {
        split_camel_case(&current, &mut tokens);
    }
    tokens
}

fn split_camel_case(word: &str, out: &mut Vec<String>) {
    out.push(word.to_string());
    let chars: Vec<char> = word.chars().collect();
    let mut start = 0;
    for i in 1..chars.len() {
        if chars[i].is_uppercase() && !chars[i - 1].is_uppercase() {
            let part: String = chars[start..i].iter().collect();
            if part.len() >= 2 {
                out.push(part.to_lowercase());
            }
            start = i;
        }
    }
    if start > 0 && start < chars.len() {
        let part: String = chars[start..].iter().collect();
        if part.len() >= 2 {
            out.push(part.to_lowercase());
        }
    }
}

fn cosine_similarity(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (term, &weight) in a {
        norm_a += weight * weight;
        if let Some(&bw) = b.get(term) {
            dot += weight * bw;
        }
    }
    for (_, &weight) in b {
        norm_b += weight * weight;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-10 {
        0.0
    } else {
        dot / denom
    }
}

fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the" | "is" | "at" | "in" | "of" | "on" | "to" | "and" | "or" | "an"
            | "a" | "it" | "if" | "do" | "no" | "as" | "be" | "by" | "we" | "so"
            | "he" | "up" | "my" | "me" | "am" | "for" | "not" | "but" | "you"
            | "all" | "can" | "had" | "her" | "was" | "one" | "our" | "out" | "has"
            | "this" | "that" | "with" | "from" | "they" | "been" | "have" | "will"
            | "each" | "make" | "like" | "long" | "them" | "than" | "then" | "what"
            | "when" | "some" | "use" | "new" | "get" | "set" | "let" | "var" | "mut"
            | "pub" | "fn" | "mod" | "struct" | "impl" | "return" | "true" | "false"
            | "self" | "none" | "string" | "type" | "default"
    )
}

fn collect_indexable_files(root: &Path, dir: &Path, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit || !dir.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= limit {
            return;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.')
            || name_str == "node_modules"
            || name_str == "target"
            || name_str == "__pycache__"
            || name_str == "vendor"
            || name_str == "dist"
            || name_str == "build"
            || name_str == ".git"
        {
            continue;
        }
        if path.is_dir() {
            collect_indexable_files(root, &path, out, limit);
        } else if path.is_file() {
            if is_indexable_ext(&path) {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }
}

fn is_indexable_ext(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h"
            | "hpp" | "rb" | "ex" | "exs" | "erl" | "hs" | "ml" | "mli" | "cs" | "swift"
            | "kt" | "scala" | "clj" | "lua" | "sh" | "bash" | "zsh" | "fish" | "yaml"
            | "yml" | "toml" | "json" | "xml" | "html" | "css" | "scss" | "sql" | "md"
            | "txt" | "dockerfile" | "makefile" | "cmake" | "gradle" | "tf" | "hcl"
            | "proto" | "graphql" | "vue" | "svelte"
    ) || path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| {
            matches!(
                n.to_lowercase().as_str(),
                "dockerfile" | "makefile" | "cmakelists.txt" | "cargo.toml" | "package.json"
                    | "go.mod" | "gemfile" | "rakefile"
            )
        })
        .unwrap_or(false)
}
