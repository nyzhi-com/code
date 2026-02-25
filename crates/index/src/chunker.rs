use std::path::Path;

const MAX_CHUNK_LINES: usize = 80;
const MIN_CHUNK_LINES: usize = 5;
const TARGET_CHUNK_LINES: usize = 60;
const OVERLAP_LINES: usize = 5;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
    pub kind: ChunkKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkKind {
    Function,
    Class,
    Module,
    Block,
}

pub fn chunk_file(rel_path: &str, content: &str) -> Vec<Chunk> {
    if content.is_empty() {
        return vec![];
    }

    let lang = detect_language(rel_path);
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    let boundaries = find_boundaries(&lines, lang);
    if boundaries.is_empty() {
        return sliding_window_chunks(rel_path, &lines);
    }

    structural_chunks(rel_path, &lines, &boundaries)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Ruby,
    Unknown,
}

fn detect_language(path: &str) -> Language {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "rs" => Language::Rust,
        "py" | "pyi" => Language::Python,
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
        "go" => Language::Go,
        "java" => Language::Java,
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" => Language::C,
        "rb" => Language::Ruby,
        _ => Language::Unknown,
    }
}

struct Boundary {
    line: usize,
    kind: ChunkKind,
}

fn find_boundaries(lines: &[&str], lang: Language) -> Vec<Boundary> {
    let mut boundaries = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        let kind = match lang {
            Language::Rust => detect_rust_boundary(trimmed),
            Language::Python => detect_python_boundary(trimmed),
            Language::JavaScript | Language::TypeScript => detect_js_boundary(trimmed),
            Language::Go => detect_go_boundary(trimmed),
            Language::Java => detect_java_boundary(trimmed),
            Language::C => detect_c_boundary(trimmed, i, lines),
            Language::Ruby => detect_ruby_boundary(trimmed),
            Language::Unknown => None,
        };

        if let Some(k) = kind {
            boundaries.push(Boundary { line: i, kind: k });
        }
    }

    boundaries
}

fn detect_rust_boundary(line: &str) -> Option<ChunkKind> {
    let s = line
        .trim_start_matches("pub(crate) ")
        .trim_start_matches("pub ");
    if s.starts_with("fn ")
        || s.starts_with("async fn ")
        || s.starts_with("unsafe fn ")
        || s.starts_with("const fn ")
    {
        return Some(ChunkKind::Function);
    }
    if s.starts_with("struct ") || s.starts_with("enum ") || s.starts_with("trait ") {
        return Some(ChunkKind::Class);
    }
    if s.starts_with("impl ") || s.starts_with("impl<") {
        return Some(ChunkKind::Class);
    }
    if s.starts_with("mod ") {
        return Some(ChunkKind::Module);
    }
    None
}

fn detect_python_boundary(line: &str) -> Option<ChunkKind> {
    if line.starts_with("def ") || line.starts_with("async def ") {
        return Some(ChunkKind::Function);
    }
    if line.starts_with("class ") {
        return Some(ChunkKind::Class);
    }
    if line.starts_with("    def ") || line.starts_with("    async def ") {
        return Some(ChunkKind::Function);
    }
    None
}

fn detect_js_boundary(line: &str) -> Option<ChunkKind> {
    if line.starts_with("function ")
        || line.starts_with("async function ")
        || line.starts_with("export function ")
        || line.starts_with("export async function ")
        || line.starts_with("export default function ")
    {
        return Some(ChunkKind::Function);
    }
    if (line.contains("=> {") || line.contains("=> ("))
        && (line.starts_with("const ")
            || line.starts_with("export const ")
            || line.starts_with("let "))
    {
        return Some(ChunkKind::Function);
    }
    if line.starts_with("class ")
        || line.starts_with("export class ")
        || line.starts_with("export default class ")
    {
        return Some(ChunkKind::Class);
    }
    None
}

fn detect_go_boundary(line: &str) -> Option<ChunkKind> {
    if line.starts_with("func ") {
        return Some(ChunkKind::Function);
    }
    if line.starts_with("type ") && (line.contains(" struct ") || line.contains(" interface ")) {
        return Some(ChunkKind::Class);
    }
    None
}

fn detect_java_boundary(line: &str) -> Option<ChunkKind> {
    let s = line
        .trim_start_matches("public ")
        .trim_start_matches("private ")
        .trim_start_matches("protected ")
        .trim_start_matches("static ")
        .trim_start_matches("final ")
        .trim_start_matches("abstract ")
        .trim_start_matches("synchronized ");
    if s.starts_with("class ") || s.starts_with("interface ") || s.starts_with("enum ") {
        return Some(ChunkKind::Class);
    }
    if s.contains('(')
        && !s.starts_with("if ")
        && !s.starts_with("for ")
        && !s.starts_with("while ")
    {
        if let Some(paren) = s.find('(') {
            let before = &s[..paren];
            let parts: Vec<&str> = before.split_whitespace().collect();
            if parts.len() >= 2 {
                return Some(ChunkKind::Function);
            }
        }
    }
    None
}

fn detect_c_boundary(line: &str, idx: usize, lines: &[&str]) -> Option<ChunkKind> {
    if line.starts_with("typedef struct") || line.starts_with("struct ") {
        return Some(ChunkKind::Class);
    }
    if line.contains('(')
        && !line.starts_with("if ")
        && !line.starts_with("for ")
        && !line.starts_with("while ")
        && !line.starts_with('#')
        && !line.starts_with("//")
    {
        let has_brace =
            line.ends_with('{') || (idx + 1 < lines.len() && lines[idx + 1].trim() == "{");
        if has_brace {
            return Some(ChunkKind::Function);
        }
    }
    None
}

fn detect_ruby_boundary(line: &str) -> Option<ChunkKind> {
    if line.starts_with("def ") || line.starts_with("  def ") {
        return Some(ChunkKind::Function);
    }
    if line.starts_with("class ") || line.starts_with("module ") {
        return Some(ChunkKind::Class);
    }
    None
}

fn structural_chunks(file: &str, lines: &[&str], boundaries: &[Boundary]) -> Vec<Chunk> {
    let mut chunks = Vec::new();

    for (i, boundary) in boundaries.iter().enumerate() {
        let start = if boundary.line >= OVERLAP_LINES {
            boundary.line - OVERLAP_LINES
        } else {
            boundary.line
        };

        let end_limit = if i + 1 < boundaries.len() {
            boundaries[i + 1].line
        } else {
            lines.len()
        };
        let end = end_limit.min(start + MAX_CHUNK_LINES);

        if end <= start || end - start < MIN_CHUNK_LINES {
            continue;
        }

        let text = lines[start..end].join("\n");
        chunks.push(Chunk {
            file: file.to_string(),
            start_line: start + 1,
            end_line: end,
            text,
            kind: boundary.kind.clone(),
        });
    }

    if chunks.is_empty() {
        return sliding_window_chunks(file, lines);
    }

    chunks
}

fn sliding_window_chunks(file: &str, lines: &[&str]) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut pos = 0;

    while pos < lines.len() {
        let end = (pos + TARGET_CHUNK_LINES).min(lines.len());
        let text = lines[pos..end].join("\n");

        if !text.trim().is_empty() {
            chunks.push(Chunk {
                file: file.to_string(),
                start_line: pos + 1,
                end_line: end,
                text,
                kind: ChunkKind::Block,
            });
        }

        if end >= lines.len() {
            break;
        }
        pos = end.saturating_sub(OVERLAP_LINES);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_function_detected() {
        let code = "use std::io;\n\npub fn hello() {\n    println!(\"hi\");\n}\n\nfn world() {\n    println!(\"world\");\n}\n";
        let chunks = chunk_file("main.rs", code);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Function));
    }

    #[test]
    fn python_class_detected() {
        let code = "import os\n\nclass Foo:\n    def bar(self):\n        pass\n\ndef standalone():\n    pass\n";
        let chunks = chunk_file("app.py", code);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn unknown_language_uses_sliding_window() {
        let code = (0..100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_file("data.xyz", &code);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| c.kind == ChunkKind::Block));
    }
}
