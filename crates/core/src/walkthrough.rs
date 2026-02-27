use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProjectNode {
    pub name: String,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub children: Vec<ProjectNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Directory,
    SourceFile,
    Config,
    Test,
    Doc,
}

const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "__pycache__",
    ".git",
    ".next",
    "dist",
    "build",
    ".cache",
    "vendor",
    "coverage",
    ".turbo",
];

const CONFIG_NAMES: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    "Makefile",
    "Dockerfile",
    ".env",
    "tsconfig.json",
    "docker-compose.yml",
];

pub fn scan_project(root: &Path, max_depth: u8) -> ProjectNode {
    fn walk(dir: &Path, root: &Path, depth: u8, max_depth: u8) -> Vec<ProjectNode> {
        if depth > max_depth {
            return Vec::new();
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        let mut nodes: Vec<ProjectNode> = Vec::new();
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && name != ".env" {
                continue;
            }
            let path = entry.path();
            let is_dir = path.is_dir();

            if is_dir {
                if IGNORE_DIRS.contains(&name.as_str()) {
                    continue;
                }
                dirs.push((name, path));
            } else {
                files.push((name, path));
            }
        }

        for (name, path) in &files {
            let kind = classify_file(name);
            nodes.push(ProjectNode {
                name: name.clone(),
                path: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
                kind,
                children: Vec::new(),
            });
        }

        for (name, path) in dirs {
            let children = walk(&path, root, depth + 1, max_depth);
            if !children.is_empty() || depth < 2 {
                nodes.push(ProjectNode {
                    name: name.clone(),
                    path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                    kind: NodeKind::Directory,
                    children,
                });
            }
        }

        nodes.sort_by(|a, b| {
            let ord_a = if a.kind == NodeKind::Directory { 0 } else { 1 };
            let ord_b = if b.kind == NodeKind::Directory { 0 } else { 1 };
            ord_a.cmp(&ord_b).then(a.name.cmp(&b.name))
        });

        nodes
    }

    let root_name = root
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let children = walk(root, root, 0, max_depth);
    ProjectNode {
        name: root_name,
        path: PathBuf::new(),
        kind: NodeKind::Directory,
        children,
    }
}

fn classify_file(name: &str) -> NodeKind {
    if CONFIG_NAMES.contains(&name) {
        return NodeKind::Config;
    }
    let lower = name.to_lowercase();
    if lower.starts_with("test")
        || lower.contains("_test.")
        || lower.contains(".test.")
        || lower.contains("_spec.")
        || lower.contains(".spec.")
    {
        return NodeKind::Test;
    }
    if lower.ends_with(".md") || lower.ends_with(".txt") || lower.ends_with(".rst") {
        return NodeKind::Doc;
    }
    NodeKind::SourceFile
}

pub fn generate_mermaid(tree: &ProjectNode) -> String {
    let mut out = String::from("graph TD\n");
    let mut id_map: HashMap<String, String> = HashMap::new();
    let mut counter = 0u32;

    fn get_id(counter: &mut u32, id_map: &mut HashMap<String, String>, path: &str) -> String {
        if let Some(id) = id_map.get(path) {
            return id.clone();
        }
        let id = format!("n{counter}");
        *counter += 1;
        id_map.insert(path.to_string(), id.clone());
        id
    }

    fn emit(
        node: &ProjectNode,
        parent_id: Option<&str>,
        out: &mut String,
        counter: &mut u32,
        id_map: &mut HashMap<String, String>,
        depth: u8,
    ) {
        if depth > 4 {
            return;
        }
        let path_str = node.path.to_string_lossy().to_string();
        let display_key = if path_str.is_empty() {
            node.name.clone()
        } else {
            path_str.clone()
        };
        let id = get_id(counter, id_map, &display_key);

        let label = &node.name;
        let shape = match node.kind {
            NodeKind::Directory => format!("    {id}[[\"{label}\"]]\n"),
            NodeKind::Config => format!("    {id}{{\"{label}\"}}\n"),
            NodeKind::Test => format!("    {id}([\"{label}\"])\n"),
            NodeKind::Doc => format!("    {id}>\"{label}\"]\n"),
            NodeKind::SourceFile => format!("    {id}(\"{label}\")\n"),
        };
        out.push_str(&shape);

        if let Some(pid) = parent_id {
            out.push_str(&format!("    {pid} --> {id}\n"));
        }

        for child in &node.children {
            emit(child, Some(&id), out, counter, id_map, depth + 1);
        }
    }

    emit(tree, None, &mut out, &mut counter, &mut id_map, 0);

    out.push_str("\n    classDef dir fill:#4a9eff,stroke:#333,color:#fff\n");
    out.push_str("    classDef src fill:#333,stroke:#666,color:#fff\n");
    out.push_str("    classDef cfg fill:#f5a623,stroke:#333,color:#000\n");
    out.push_str("    classDef test fill:#7ed321,stroke:#333,color:#000\n");
    out.push_str("    classDef doc fill:#bd10e0,stroke:#333,color:#fff\n");

    out
}

pub fn export_html(mermaid_code: &str, title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title} - Walkthrough</title>
<script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
<style>
  body {{ background: #1a1a2e; color: #eee; font-family: system-ui; padding: 2rem; }}
  h1 {{ color: #4a9eff; }}
  .mermaid {{ background: #16213e; border-radius: 8px; padding: 1.5rem; }}
</style>
</head>
<body>
<h1>{title}</h1>
<div class="mermaid">
{mermaid_code}
</div>
<script>mermaid.initialize({{ startOnLoad: true, theme: 'dark' }});</script>
</body>
</html>"#,
        title = title,
        mermaid_code = mermaid_code
    )
}

pub fn generate_walkthrough(root: &Path) -> (String, String) {
    let tree = scan_project(root, 3);
    let mermaid = generate_mermaid(&tree);
    let title = root
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "Project".to_string());
    let html = export_html(&mermaid, &title);
    (mermaid, html)
}
