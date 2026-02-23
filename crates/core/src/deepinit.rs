use anyhow::Result;
use std::path::Path;

pub struct ProjectScan {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub directories: Vec<DirInfo>,
}

pub struct DirInfo {
    pub path: String,
    pub file_count: usize,
    pub primary_language: Option<String>,
}

pub fn scan_project(root: &Path) -> Result<ProjectScan> {
    let mut languages = vec![];
    let mut frameworks = vec![];
    let mut directories = vec![];

    if root.join("Cargo.toml").exists() {
        languages.push("Rust".to_string());
        frameworks.push("Cargo".to_string());
    }
    if root.join("package.json").exists() {
        languages.push("TypeScript/JavaScript".to_string());
        if root.join("next.config.js").exists() || root.join("next.config.ts").exists() {
            frameworks.push("Next.js".to_string());
        } else if root.join("vite.config.ts").exists() {
            frameworks.push("Vite".to_string());
        }
    }
    if root.join("go.mod").exists() {
        languages.push("Go".to_string());
    }
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        languages.push("Python".to_string());
        if root.join("manage.py").exists() {
            frameworks.push("Django".to_string());
        }
    }
    if root.join("Dockerfile").exists() {
        frameworks.push("Docker".to_string());
    }

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.')
                    || name == "node_modules"
                    || name == "target"
                    || name == "__pycache__"
                    || name == "vendor"
                {
                    continue;
                }
                let file_count = count_files_shallow(&path);
                let primary_lang = detect_primary_language(&path);
                directories.push(DirInfo {
                    path: name.to_string(),
                    file_count,
                    primary_language: primary_lang,
                });
            }
        }
    }

    directories.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(ProjectScan {
        languages,
        frameworks,
        directories,
    })
}

fn count_files_shallow(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| entries.flatten().filter(|e| e.path().is_file()).count())
        .unwrap_or(0)
}

fn detect_primary_language(dir: &Path) -> Option<String> {
    let mut ext_counts = std::collections::HashMap::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                *ext_counts.entry(ext.to_string()).or_insert(0usize) += 1;
            }
        }
    }
    ext_counts
        .into_iter()
        .max_by_key(|&(_, c)| c)
        .map(|(ext, _)| {
            match ext.as_str() {
                "rs" => "Rust",
                "ts" | "tsx" => "TypeScript",
                "js" | "jsx" => "JavaScript",
                "py" => "Python",
                "go" => "Go",
                "java" => "Java",
                "rb" => "Ruby",
                "md" => "Markdown",
                _ => &ext,
            }
            .to_string()
        })
}

pub fn generate_agents_md(root: &Path) -> Result<String> {
    let scan = scan_project(root)?;
    let mut content = String::from("# Project Overview\n\n");

    if !scan.languages.is_empty() {
        content.push_str(&format!("**Languages:** {}\n", scan.languages.join(", ")));
    }
    if !scan.frameworks.is_empty() {
        content.push_str(&format!("**Frameworks:** {}\n", scan.frameworks.join(", ")));
    }

    content.push_str("\n## Directory Structure\n\n");
    for dir in &scan.directories {
        let lang = dir.primary_language.as_deref().unwrap_or("mixed");
        content.push_str(&format!(
            "- **{}/** - {} files ({})\n",
            dir.path, dir.file_count, lang,
        ));
    }

    content.push_str("\n## Conventions\n\n");
    content.push_str("- Follow existing patterns in the codebase\n");
    content.push_str("- Run tests before submitting changes\n");

    Ok(content)
}

pub fn write_agents_md(root: &Path) -> Result<std::path::PathBuf> {
    let content = generate_agents_md(root)?;
    let path = root.join("AGENTS.md");
    std::fs::write(&path, &content)?;
    Ok(path)
}
