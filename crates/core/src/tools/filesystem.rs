use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use super::change_tracker::FileChange;
use super::permission::ToolPermission;
use super::{Tool, ToolContext, ToolResult};

// ---------------------------------------------------------------------------
// list_dir (ReadOnly)
// ---------------------------------------------------------------------------

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List directory contents with [FILE] or [DIR] prefix and file sizes. \
         Sorted alphabetically. Hidden files (starting with '.') are omitted \
         unless the path itself targets a hidden directory."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let dir_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;

        let path = resolve_path(dir_path, &ctx.cwd);

        if !path.is_dir() {
            anyhow::bail!("Not a directory: {}", path.display());
        }

        let mut entries = Vec::new();
        let mut rd = tokio::fs::read_dir(&path).await?;

        while let Some(entry) = rd.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let ft = entry.file_type().await?;
            let meta = entry.metadata().await?;

            if name.starts_with('.') {
                continue;
            }

            if ft.is_dir() {
                entries.push(format!("[DIR]  {name}/"));
            } else {
                let size = format_size(meta.len());
                entries.push(format!("[FILE] {name} ({size})"));
            }
        }

        entries.sort();
        let count = entries.len();
        let output = if entries.is_empty() {
            format!("{} is empty", path.display())
        } else {
            entries.join("\n")
        };

        Ok(ToolResult {
            output,
            title: format!("list_dir: {dir_path}"),
            metadata: json!({ "count": count }),
        })
    }
}

// ---------------------------------------------------------------------------
// directory_tree (ReadOnly)
// ---------------------------------------------------------------------------

pub struct DirectoryTreeTool;

#[async_trait]
impl Tool for DirectoryTreeTool {
    fn name(&self) -> &str {
        "directory_tree"
    }

    fn description(&self) -> &str {
        "Show a recursive tree view of directory contents up to a given depth. \
         Returns an indented tree structure. Excludes common noisy directories \
         by default (node_modules, .git, target, __pycache__)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Root directory path"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum recursion depth (default: 3)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let dir_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;

        let max_depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let path = resolve_path(dir_path, &ctx.cwd);

        if !path.is_dir() {
            anyhow::bail!("Not a directory: {}", path.display());
        }

        let default_excludes = ["node_modules", ".git", "target", "__pycache__", ".venv"];

        let mut lines = Vec::new();
        let dir_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir_path.to_string());
        lines.push(format!("{dir_name}/"));

        build_tree(&path, "", max_depth, 0, &default_excludes, &mut lines).await?;

        let output = lines.join("\n");

        Ok(ToolResult {
            output,
            title: format!("directory_tree: {dir_path}"),
            metadata: json!({ "depth": max_depth }),
        })
    }
}

async fn build_tree(
    dir: &Path,
    prefix: &str,
    max_depth: usize,
    current_depth: usize,
    excludes: &[&str],
    lines: &mut Vec<String>,
) -> Result<()> {
    if current_depth >= max_depth {
        return Ok(());
    }

    let mut entries = Vec::new();
    let mut rd = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = rd.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || excludes.contains(&name.as_str()) {
            continue;
        }
        let ft = entry.file_type().await?;
        entries.push((name, ft.is_dir(), entry.path()));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let total = entries.len();
    for (i, (name, is_dir, entry_path)) in entries.into_iter().enumerate() {
        let is_last = i == total - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        if is_dir {
            lines.push(format!("{prefix}{connector}{name}/"));
            let new_prefix = format!("{prefix}{child_prefix}");
            Box::pin(build_tree(
                &entry_path,
                &new_prefix,
                max_depth,
                current_depth + 1,
                excludes,
                lines,
            ))
            .await?;
        } else {
            lines.push(format!("{prefix}{connector}{name}"));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// file_info (ReadOnly)
// ---------------------------------------------------------------------------

pub struct FileInfoTool;

#[async_trait]
impl Tool for FileInfoTool {
    fn name(&self) -> &str {
        "file_info"
    }

    fn description(&self) -> &str {
        "Get metadata about a file or directory: size, type, permissions, \
         and modification timestamp."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let file_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;

        let path = resolve_path(file_path, &ctx.cwd);

        let meta = tokio::fs::metadata(&path)
            .await
            .map_err(|e| anyhow::anyhow!("Cannot stat {}: {e}", path.display()))?;

        let file_type = if meta.is_dir() {
            "directory"
        } else if meta.is_symlink() {
            "symlink"
        } else {
            "file"
        };

        let size = meta.len();
        let modified = meta
            .modified()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        let created = meta
            .created()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            format!("{:o}", meta.permissions().mode())
        };
        #[cfg(not(unix))]
        let permissions = if meta.permissions().readonly() {
            "readonly".to_string()
        } else {
            "read-write".to_string()
        };

        let output = format!(
            "Path:        {}\n\
             Type:        {file_type}\n\
             Size:        {}\n\
             Permissions: {permissions}\n\
             Modified:    {modified}\n\
             Created:     {created}",
            path.display(),
            format_size(size),
        );

        Ok(ToolResult {
            output,
            title: format!("file_info: {file_path}"),
            metadata: json!({
                "type": file_type,
                "size": size,
                "permissions": permissions,
                "modified": modified,
                "created": created,
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// delete_file (NeedsApproval)
// ---------------------------------------------------------------------------

pub struct DeleteFileTool;

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Delete a file or empty directory. The operation is recorded and can \
         be undone with /undo for files (original content is preserved)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or empty directory to delete"
                }
            },
            "required": ["path"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let file_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;

        let path = resolve_path(file_path, &ctx.cwd);

        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", path.display());
        }

        let meta = tokio::fs::metadata(&path).await?;

        if meta.is_dir() {
            tokio::fs::remove_dir(&path).await.map_err(|e| {
                anyhow::anyhow!(
                    "Cannot remove directory {} (must be empty): {e}",
                    path.display()
                )
            })?;

            Ok(ToolResult {
                output: format!("Deleted empty directory: {}", path.display()),
                title: format!("delete_file: {file_path}"),
                metadata: json!({ "type": "directory" }),
            })
        } else {
            let original = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            tokio::fs::remove_file(&path).await?;

            {
                let mut tracker = ctx.change_tracker.lock().await;
                tracker.record(FileChange {
                    path: path.clone(),
                    original: Some(original),
                    new_content: String::new(),
                    tool_name: "delete_file".to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }

            Ok(ToolResult {
                output: format!("Deleted file: {}", path.display()),
                title: format!("delete_file: {file_path}"),
                metadata: json!({ "type": "file" }),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// move_file (NeedsApproval)
// ---------------------------------------------------------------------------

pub struct MoveFileTool;

#[async_trait]
impl Tool for MoveFileTool {
    fn name(&self) -> &str {
        "move_file"
    }

    fn description(&self) -> &str {
        "Move or rename a file or directory. Fails if the destination already \
         exists. Creates parent directories for the destination if needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Source path"
                },
                "destination": {
                    "type": "string",
                    "description": "Destination path"
                }
            },
            "required": ["source", "destination"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let source = args
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: source"))?;
        let destination = args
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: destination"))?;

        let src = resolve_path(source, &ctx.cwd);
        let dst = resolve_path(destination, &ctx.cwd);

        if !src.exists() {
            anyhow::bail!("Source does not exist: {}", src.display());
        }
        if dst.exists() {
            anyhow::bail!("Destination already exists: {}", dst.display());
        }

        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let is_file = src.is_file();
        if is_file {
            let content = tokio::fs::read_to_string(&src).await.unwrap_or_default();

            tokio::fs::rename(&src, &dst).await?;

            let mut tracker = ctx.change_tracker.lock().await;
            tracker.record(FileChange {
                path: dst.clone(),
                original: None,
                new_content: content.clone(),
                tool_name: "move_file".to_string(),
                timestamp: chrono::Utc::now(),
            });
            tracker.record(FileChange {
                path: src.clone(),
                original: Some(content),
                new_content: String::new(),
                tool_name: "move_file".to_string(),
                timestamp: chrono::Utc::now(),
            });
        } else {
            tokio::fs::rename(&src, &dst).await?;
        }

        Ok(ToolResult {
            output: format!("Moved {} -> {}", src.display(), dst.display()),
            title: format!("move_file: {source} -> {destination}"),
            metadata: json!({
                "source": src.display().to_string(),
                "destination": dst.display().to_string(),
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// copy_file (NeedsApproval)
// ---------------------------------------------------------------------------

pub struct CopyFileTool;

#[async_trait]
impl Tool for CopyFileTool {
    fn name(&self) -> &str {
        "copy_file"
    }

    fn description(&self) -> &str {
        "Copy a file to a new location. Fails if the destination already exists. \
         Creates parent directories for the destination if needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Source file path"
                },
                "destination": {
                    "type": "string",
                    "description": "Destination file path"
                }
            },
            "required": ["source", "destination"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let source = args
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: source"))?;
        let destination = args
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: destination"))?;

        let src = resolve_path(source, &ctx.cwd);
        let dst = resolve_path(destination, &ctx.cwd);

        if !src.is_file() {
            anyhow::bail!("Source is not a file: {}", src.display());
        }
        if dst.exists() {
            anyhow::bail!("Destination already exists: {}", dst.display());
        }

        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::copy(&src, &dst).await?;

        let content = tokio::fs::read_to_string(&dst).await.unwrap_or_default();
        let bytes = content.len();

        {
            let mut tracker = ctx.change_tracker.lock().await;
            tracker.record(FileChange {
                path: dst.clone(),
                original: None,
                new_content: content,
                tool_name: "copy_file".to_string(),
                timestamp: chrono::Utc::now(),
            });
        }

        Ok(ToolResult {
            output: format!(
                "Copied {} -> {} ({} bytes)",
                src.display(),
                dst.display(),
                bytes
            ),
            title: format!("copy_file: {source} -> {destination}"),
            metadata: json!({
                "source": src.display().to_string(),
                "destination": dst.display().to_string(),
                "bytes": bytes,
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// create_dir (NeedsApproval)
// ---------------------------------------------------------------------------

pub struct CreateDirTool;

#[async_trait]
impl Tool for CreateDirTool {
    fn name(&self) -> &str {
        "create_dir"
    }

    fn description(&self) -> &str {
        "Create a directory including any missing parent directories. \
         No-op if the directory already exists."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to create"
                }
            },
            "required": ["path"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let dir_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;

        let path = resolve_path(dir_path, &ctx.cwd);

        let already_exists = path.is_dir();
        tokio::fs::create_dir_all(&path).await?;

        let output = if already_exists {
            format!("Directory already exists: {}", path.display())
        } else {
            format!("Created directory: {}", path.display())
        };

        Ok(ToolResult {
            output,
            title: format!("create_dir: {dir_path}"),
            metadata: json!({ "already_existed": already_exists }),
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_path(file_path: &str, cwd: &Path) -> std::path::PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
