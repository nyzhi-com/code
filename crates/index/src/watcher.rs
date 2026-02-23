use std::path::{Path, PathBuf};

use anyhow::Result;
use sha2::{Digest, Sha256};

const MAX_FILES: usize = 50_000;
const MAX_FILE_SIZE: u64 = 512 * 1024;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub rel_path: String,
    pub abs_path: PathBuf,
    pub hash: String,
}

pub fn walk_project(root: &Path, extra_exclude: &[String]) -> Result<Vec<FileEntry>> {
    let gitignore = load_gitignore(root);
    let mut entries = Vec::new();
    walk_dir(root, root, &gitignore, extra_exclude, &mut entries);
    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(entries)
}

pub fn hash_content(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    gitignore: &[String],
    extra_exclude: &[String],
    out: &mut Vec<FileEntry>,
) {
    if out.len() >= MAX_FILES || !dir.is_dir() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if out.len() >= MAX_FILES {
            return;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if should_skip(&name_str) {
            continue;
        }

        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        if is_ignored(&rel, gitignore, extra_exclude) {
            continue;
        }

        if path.is_dir() {
            walk_dir(root, &path, gitignore, extra_exclude, out);
        } else if path.is_file() {
            if !is_indexable_ext(&path) {
                continue;
            }
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() > MAX_FILE_SIZE {
                continue;
            }

            let content = match std::fs::read(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if content.len() > 512 && content[..512].contains(&0) {
                continue;
            }

            let hash = hash_content(&content);
            out.push(FileEntry {
                rel_path: rel,
                abs_path: path,
                hash,
            });
        }
    }
}

fn should_skip(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "node_modules"
                | "target"
                | "__pycache__"
                | "vendor"
                | "dist"
                | "build"
                | ".git"
                | ".svn"
                | ".hg"
                | "venv"
                | ".venv"
                | "env"
                | ".env"
                | "coverage"
                | ".nyc_output"
                | ".next"
                | ".nuxt"
                | ".turbo"
                | "out"
        )
}

fn is_indexable_ext(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "rs" | "py"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "rb"
            | "ex"
            | "exs"
            | "erl"
            | "hs"
            | "ml"
            | "cs"
            | "swift"
            | "kt"
            | "scala"
            | "clj"
            | "lua"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "yaml"
            | "yml"
            | "toml"
            | "json"
            | "xml"
            | "html"
            | "css"
            | "scss"
            | "sql"
            | "md"
            | "txt"
            | "proto"
            | "graphql"
            | "vue"
            | "svelte"
            | "mjs"
            | "cjs"
            | "mts"
            | "cts"
            | "tf"
            | "hcl"
    ) || path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| {
            matches!(
                n.to_lowercase().as_str(),
                "dockerfile"
                    | "makefile"
                    | "cmakelists.txt"
                    | "cargo.toml"
                    | "package.json"
                    | "go.mod"
                    | "gemfile"
                    | "rakefile"
            )
        })
        .unwrap_or(false)
}

fn load_gitignore(root: &Path) -> Vec<String> {
    let path = root.join(".gitignore");
    match std::fs::read_to_string(&path) {
        Ok(content) => content
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.trim().to_string())
            .collect(),
        Err(_) => vec![],
    }
}

fn is_ignored(rel_path: &str, gitignore: &[String], extra_exclude: &[String]) -> bool {
    for pattern in gitignore.iter().chain(extra_exclude.iter()) {
        let pat = pattern.trim_end_matches('/');
        if rel_path.starts_with(pat)
            || rel_path.contains(&format!("/{pat}/"))
            || rel_path.contains(&format!("/{pat}"))
        {
            return true;
        }
        if pat.starts_with("*.") {
            let ext = &pat[1..];
            if rel_path.ends_with(ext) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let h1 = hash_content(b"hello world");
        let h2 = hash_content(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn skip_patterns() {
        assert!(should_skip("node_modules"));
        assert!(should_skip(".git"));
        assert!(should_skip("target"));
        assert!(!should_skip("src"));
        assert!(!should_skip("main.rs"));
    }

    #[test]
    fn indexable_extensions() {
        assert!(is_indexable_ext(Path::new("main.rs")));
        assert!(is_indexable_ext(Path::new("app.py")));
        assert!(is_indexable_ext(Path::new("index.tsx")));
        assert!(!is_indexable_ext(Path::new("image.png")));
        assert!(!is_indexable_ext(Path::new("data.bin")));
    }
}
