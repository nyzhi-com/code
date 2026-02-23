use std::path::{Path, PathBuf};

use regex::Regex;

const MAX_FILE_SIZE: u64 = 100 * 1024; // 100 KB
const MAX_DIR_ENTRIES: usize = 200;

#[derive(Debug, Clone)]
pub struct ContextFile {
    pub path: PathBuf,
    pub display_path: String,
    pub content: String,
    pub is_dir: bool,
    pub line_count: usize,
    pub truncated: bool,
}

/// Extract `@path` mentions from user input.
///
/// Skips email-like patterns where the character before `@` is alphanumeric.
pub fn parse_mentions(input: &str) -> Vec<String> {
    let re = Regex::new(r"@([\w./~-][\w./~-]*)").unwrap();
    let mut mentions = Vec::new();

    for cap in re.captures_iter(input) {
        let full_match = cap.get(0).unwrap();
        let start = full_match.start();

        if start > 0 {
            let prev = input[..start].chars().last().unwrap();
            if prev.is_alphanumeric() || prev == '.' {
                continue;
            }
        }

        let path = cap[1].to_string();
        if !mentions.contains(&path) {
            mentions.push(path);
        }
    }
    mentions
}

/// Resolve mention strings to actual file/directory contents.
pub fn resolve_context_files(
    mentions: &[String],
    project_root: &Path,
    cwd: &Path,
) -> Vec<ContextFile> {
    let mut files = Vec::new();

    for mention in mentions {
        let expanded: PathBuf = if let Some(rest) = mention.strip_prefix('~') {
            if let Some(home) = dirs::home_dir() {
                home.join(rest.strip_prefix('/').unwrap_or(rest))
            } else {
                continue;
            }
        } else if Path::new(mention).is_absolute() {
            PathBuf::from(mention)
        } else {
            let from_root = project_root.join(mention);
            if from_root.exists() {
                from_root
            } else {
                cwd.join(mention)
            }
        };

        let canonical: PathBuf = match expanded.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };

        if canonical.is_dir() {
            match read_directory(&canonical, mention) {
                Some(cf) => files.push(cf),
                None => continue,
            }
        } else if canonical.is_file() {
            match read_file(&canonical, mention) {
                Some(cf) => files.push(cf),
                None => continue,
            }
        }
    }
    files
}

fn read_file(path: &Path, display: &str) -> Option<ContextFile> {
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len();

    if size == 0 {
        return Some(ContextFile {
            path: path.to_path_buf(),
            display_path: display.to_string(),
            content: String::new(),
            is_dir: false,
            line_count: 0,
            truncated: false,
        });
    }

    let truncated = size > MAX_FILE_SIZE;
    let bytes = if truncated {
        let mut buf = vec![0u8; MAX_FILE_SIZE as usize];
        let mut f = std::fs::File::open(path).ok()?;
        std::io::Read::read(&mut f, &mut buf).ok()?;
        buf
    } else {
        std::fs::read(path).ok()?
    };

    let content = String::from_utf8_lossy(&bytes);
    let content = if truncated {
        let last_nl = content.rfind('\n').unwrap_or(content.len());
        format!(
            "{}\n... (truncated, file exceeds 100KB)",
            &content[..last_nl]
        )
    } else {
        content.into_owned()
    };

    let line_count = content.lines().count();
    Some(ContextFile {
        path: path.to_path_buf(),
        display_path: display.to_string(),
        content,
        is_dir: false,
        line_count,
        truncated,
    })
}

fn read_directory(path: &Path, display: &str) -> Option<ContextFile> {
    let mut entries = Vec::new();
    collect_dir_entries(path, path, &mut entries, MAX_DIR_ENTRIES);

    let content = entries.join("\n");
    let line_count = entries.len();
    let truncated = line_count >= MAX_DIR_ENTRIES;

    Some(ContextFile {
        path: path.to_path_buf(),
        display_path: display.to_string(),
        content,
        is_dir: true,
        line_count,
        truncated,
    })
}

fn collect_dir_entries(base: &Path, dir: &Path, entries: &mut Vec<String>, max: usize) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };

    let mut children: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    children.sort_by_key(|e| e.file_name());

    for entry in children {
        if entries.len() >= max {
            entries.push("... (truncated)".to_string());
            return;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') {
            continue;
        }
        if name_str == "node_modules" || name_str == "target" || name_str == "__pycache__" {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(base)
            .unwrap_or(&entry.path())
            .to_string_lossy()
            .to_string();

        let ft = entry.file_type().ok();
        if ft.as_ref().map(|f| f.is_dir()).unwrap_or(false) {
            entries.push(format!("{rel}/"));
            collect_dir_entries(base, &entry.path(), entries, max);
        } else {
            entries.push(rel);
        }
    }
}

/// Build the enriched message with XML context blocks prepended.
pub fn build_context_message(original_input: &str, files: &[ContextFile]) -> String {
    if files.is_empty() {
        return original_input.to_string();
    }

    let mut ctx = String::from("<context>\n");
    for f in files {
        if f.is_dir {
            ctx.push_str(&format!("<directory path=\"{}\">\n", f.display_path));
            ctx.push_str(&f.content);
            ctx.push_str("\n</directory>\n");
        } else {
            ctx.push_str(&format!(
                "<file path=\"{}\" lines=\"{}\">\n",
                f.display_path, f.line_count
            ));
            ctx.push_str(&f.content);
            ctx.push_str("\n</file>\n");
        }
    }
    ctx.push_str("</context>\n\n");
    ctx.push_str(original_input);
    ctx
}

/// Build a human-readable summary of attached context files.
pub fn format_attachment_summary(files: &[ContextFile]) -> String {
    let mut parts = Vec::new();
    for f in files {
        if f.is_dir {
            let suffix = if f.truncated { " (truncated)" } else { "" };
            parts.push(format!(
                "{} ({} entries{})",
                f.display_path, f.line_count, suffix
            ));
        } else {
            let suffix = if f.truncated { " (truncated)" } else { "" };
            parts.push(format!(
                "{} ({} lines{})",
                f.display_path, f.line_count, suffix
            ));
        }
    }
    format!("Attached: {}", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_mention() {
        let mentions = parse_mentions("explain @src/main.rs please");
        assert_eq!(mentions, vec!["src/main.rs"]);
    }

    #[test]
    fn parse_multiple_mentions() {
        let mentions = parse_mentions("compare @foo.rs and @bar/baz.rs");
        assert_eq!(mentions, vec!["foo.rs", "bar/baz.rs"]);
    }

    #[test]
    fn parse_deduplicates() {
        let mentions = parse_mentions("@file.rs and also @file.rs");
        assert_eq!(mentions, vec!["file.rs"]);
    }

    #[test]
    fn parse_skips_email() {
        let mentions = parse_mentions("email user@example.com");
        assert!(mentions.is_empty());
    }

    #[test]
    fn parse_mention_at_start() {
        let mentions = parse_mentions("@Cargo.toml is the manifest");
        assert_eq!(mentions, vec!["Cargo.toml"]);
    }

    #[test]
    fn parse_home_path() {
        let mentions = parse_mentions("check @~/config.toml");
        assert_eq!(mentions, vec!["~/config.toml"]);
    }

    #[test]
    fn parse_no_mentions() {
        let mentions = parse_mentions("no mentions here");
        assert!(mentions.is_empty());
    }

    #[test]
    fn build_message_no_files() {
        let msg = build_context_message("hello", &[]);
        assert_eq!(msg, "hello");
    }

    #[test]
    fn build_message_with_file() {
        let files = vec![ContextFile {
            path: PathBuf::from("/tmp/test.rs"),
            display_path: "test.rs".to_string(),
            content: "fn main() {}".to_string(),
            is_dir: false,
            line_count: 1,
            truncated: false,
        }];
        let msg = build_context_message("explain @test.rs", &files);
        assert!(msg.starts_with("<context>"));
        assert!(msg.contains("<file path=\"test.rs\" lines=\"1\">"));
        assert!(msg.contains("fn main() {}"));
        assert!(msg.contains("</file>"));
        assert!(msg.ends_with("explain @test.rs"));
    }

    #[test]
    fn build_message_with_directory() {
        let files = vec![ContextFile {
            path: PathBuf::from("/tmp/src"),
            display_path: "src/".to_string(),
            content: "main.rs\nlib.rs".to_string(),
            is_dir: true,
            line_count: 2,
            truncated: false,
        }];
        let msg = build_context_message("what is in @src/", &files);
        assert!(msg.contains("<directory path=\"src/\">"));
        assert!(msg.contains("main.rs\nlib.rs"));
        assert!(msg.contains("</directory>"));
    }

    #[test]
    fn format_summary() {
        let files = vec![
            ContextFile {
                path: PathBuf::from("/a.rs"),
                display_path: "a.rs".to_string(),
                content: String::new(),
                is_dir: false,
                line_count: 10,
                truncated: false,
            },
            ContextFile {
                path: PathBuf::from("/src"),
                display_path: "src/".to_string(),
                content: String::new(),
                is_dir: true,
                line_count: 5,
                truncated: true,
            },
        ];
        let summary = format_attachment_summary(&files);
        assert_eq!(
            summary,
            "Attached: a.rs (10 lines), src/ (5 entries (truncated))"
        );
    }
}
