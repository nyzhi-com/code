use std::path::Path;
use std::process::Command;

pub struct FormatResult {
    pub file: String,
    pub formatter: String,
    pub success: bool,
    pub output: String,
}

struct FormatterDef {
    name: &'static str,
    extensions: &'static [&'static str],
    command: &'static str,
    args_template: &'static str,
    detect_files: &'static [&'static str],
}

const FORMATTERS: &[FormatterDef] = &[
    FormatterDef {
        name: "rustfmt",
        extensions: &["rs"],
        command: "rustfmt",
        args_template: "{file}",
        detect_files: &["Cargo.toml"],
    },
    FormatterDef {
        name: "prettier",
        extensions: &[
            "js", "ts", "jsx", "tsx", "json", "css", "scss", "html", "md", "yaml", "yml",
            "graphql",
        ],
        command: "npx",
        args_template: "prettier --write {file}",
        detect_files: &["package.json", ".prettierrc", ".prettierrc.json", ".prettierrc.js"],
    },
    FormatterDef {
        name: "black",
        extensions: &["py"],
        command: "black",
        args_template: "--quiet {file}",
        detect_files: &["pyproject.toml", "setup.py", "setup.cfg"],
    },
    FormatterDef {
        name: "gofmt",
        extensions: &["go"],
        command: "gofmt",
        args_template: "-w {file}",
        detect_files: &["go.mod"],
    },
    FormatterDef {
        name: "clang-format",
        extensions: &["c", "cpp", "cc", "h", "hpp"],
        command: "clang-format",
        args_template: "-i {file}",
        detect_files: &[".clang-format"],
    },
    FormatterDef {
        name: "shfmt",
        extensions: &["sh", "bash"],
        command: "shfmt",
        args_template: "-w {file}",
        detect_files: &[],
    },
];

fn detect_formatter(file_path: &str, project_root: &Path) -> Option<&'static FormatterDef> {
    let ext = file_path.rsplit('.').next()?;

    for def in FORMATTERS {
        if !def.extensions.contains(&ext) {
            continue;
        }
        if def.detect_files.is_empty() {
            if which_exists(def.command) {
                return Some(def);
            }
            continue;
        }
        for detect_file in def.detect_files {
            if project_root.join(detect_file).exists() {
                return Some(def);
            }
        }
    }
    None
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn format_file(file_path: &str, project_root: &Path) -> Option<FormatResult> {
    let def = detect_formatter(file_path, project_root)?;

    let abs_path = if Path::new(file_path).is_absolute() {
        file_path.to_string()
    } else {
        project_root.join(file_path).display().to_string()
    };

    let args_str = def.args_template.replace("{file}", &abs_path);
    let args: Vec<&str> = args_str.split_whitespace().collect();

    let result = Command::new(def.command)
        .args(&args)
        .current_dir(project_root)
        .output();

    match result {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Some(FormatResult {
                file: file_path.to_string(),
                formatter: def.name.to_string(),
                success: output.status.success(),
                output: if stderr.is_empty() {
                    "formatted".to_string()
                } else {
                    stderr
                },
            })
        }
        Err(_) => None,
    }
}

pub async fn format_file_async(file_path: String, project_root: std::path::PathBuf) -> Option<FormatResult> {
    tokio::task::spawn_blocking(move || format_file(&file_path, &project_root))
        .await
        .ok()?
}
