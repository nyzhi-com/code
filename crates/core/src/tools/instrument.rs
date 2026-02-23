use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

#[derive(Debug, Clone)]
pub struct Instrumentation {
    pub file: PathBuf,
    pub line: usize,
    pub original_line: String,
    pub instrumented_line: String,
}

pub type InstrumentationStore = Arc<Mutex<HashMap<String, Vec<Instrumentation>>>>;

pub fn shared_store() -> InstrumentationStore {
    Arc::new(Mutex::new(HashMap::new()))
}

pub struct InstrumentTool {
    store: InstrumentationStore,
}

impl InstrumentTool {
    pub fn new(store: InstrumentationStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for InstrumentTool {
    fn name(&self) -> &str {
        "instrument"
    }

    fn description(&self) -> &str {
        "Add temporary debug instrumentation (logging/assertions) to a file. \
         All instrumentation is tracked and can be removed with remove_instrumentation. \
         Use during debug sessions to gather evidence."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "File to instrument"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number to insert instrumentation AFTER"
                },
                "code": {
                    "type": "string",
                    "description": "Debug code to insert (e.g. a log statement or assertion)"
                }
            },
            "required": ["file", "line", "code"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: file"))?;
        let line = args
            .get("line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: line"))?
            as usize;
        let code = args
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: code"))?;

        let file_path = resolve_path(file, &ctx.cwd);
        let content = std::fs::read_to_string(&file_path)?;
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        if line > lines.len() {
            anyhow::bail!("Line {} exceeds file length {}", line, lines.len());
        }

        let marker = format!("/* NYZHI_DEBUG */ {code}");
        lines.insert(line, marker.clone());

        std::fs::write(&file_path, lines.join("\n"))?;

        let instrumentation = Instrumentation {
            file: file_path.clone(),
            line,
            original_line: String::new(),
            instrumented_line: marker,
        };

        let mut store = self.store.lock().await;
        store
            .entry(ctx.session_id.clone())
            .or_default()
            .push(instrumentation);

        Ok(ToolResult {
            output: format!("Inserted debug instrumentation at {}:{}", file, line),
            title: format!("instrument: {}:{}", file, line),
            metadata: json!({ "file": file, "line": line }),
        })
    }
}

pub struct RemoveInstrumentationTool {
    store: InstrumentationStore,
}

impl RemoveInstrumentationTool {
    pub fn new(store: InstrumentationStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for RemoveInstrumentationTool {
    fn name(&self) -> &str {
        "remove_instrumentation"
    }

    fn description(&self) -> &str {
        "Remove all debug instrumentation added during this session. \
         Call this after debugging to clean up temporary logging."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, _args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let mut store = self.store.lock().await;
        let instrumentations = store.remove(&ctx.session_id).unwrap_or_default();

        if instrumentations.is_empty() {
            return Ok(ToolResult {
                output: "No instrumentation to remove.".to_string(),
                title: "remove_instrumentation".to_string(),
                metadata: json!({}),
            });
        }

        let mut files_cleaned: HashMap<PathBuf, usize> = HashMap::new();
        let marker = "/* NYZHI_DEBUG */";

        let unique_files: Vec<PathBuf> = instrumentations
            .iter()
            .map(|i| i.file.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for file_path in &unique_files {
            if !file_path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(file_path)?;
            let original_count = content.lines().count();
            let cleaned: Vec<&str> = content
                .lines()
                .filter(|line| !line.contains(marker))
                .collect();
            let removed = original_count - cleaned.len();
            if removed > 0 {
                std::fs::write(file_path, cleaned.join("\n"))?;
                files_cleaned.insert(file_path.clone(), removed);
            }
        }

        let total_removed: usize = files_cleaned.values().sum();
        let summary: Vec<String> = files_cleaned
            .iter()
            .map(|(path, count)| format!("  {}: {} lines removed", path.display(), count))
            .collect();

        Ok(ToolResult {
            output: format!(
                "Removed {} instrumentation lines from {} files:\n{}",
                total_removed,
                files_cleaned.len(),
                summary.join("\n")
            ),
            title: format!("remove_instrumentation: {} lines", total_removed),
            metadata: json!({ "files": files_cleaned.len(), "lines_removed": total_removed }),
        })
    }
}

fn resolve_path(file_path: &str, cwd: &Path) -> PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}
