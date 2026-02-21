use crate::app::{DisplayItem, ToolStatus};
use nyzhi_core::agent::SessionUsage;

pub struct ExportMeta {
    pub provider: String,
    pub model: String,
    pub usage: SessionUsage,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn export_session_markdown(items: &[DisplayItem], meta: &ExportMeta) -> String {
    let mut out = String::with_capacity(4096);

    out.push_str("---\n");
    out.push_str(&format!(
        "date: {}\n",
        meta.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    out.push_str(&format!("provider: {}\n", meta.provider));
    out.push_str(&format!("model: {}\n", meta.model));
    out.push_str(&format!(
        "tokens: {} input / {} output\n",
        meta.usage.total_input_tokens, meta.usage.total_output_tokens
    ));
    if meta.usage.total_cost_usd > 0.0 {
        out.push_str(&format!("cost: ${:.4}\n", meta.usage.total_cost_usd));
    }
    out.push_str("---\n\n");

    for item in items {
        match item {
            DisplayItem::Message { role, content } => {
                match role.as_str() {
                    "user" => {
                        out.push_str("## You\n\n");
                        out.push_str(content);
                        out.push_str("\n\n");
                    }
                    "assistant" => {
                        out.push_str("## Assistant\n\n");
                        out.push_str(content);
                        out.push_str("\n\n");
                    }
                    _ => {
                        for line in content.lines() {
                            out.push_str("> ");
                            out.push_str(line);
                            out.push('\n');
                        }
                        out.push('\n');
                    }
                }
            }
            DisplayItem::ToolCall {
                name,
                args_summary,
                output,
                status,
                elapsed_ms,
            } => {
                let icon = match status {
                    ToolStatus::Running => "*",
                    ToolStatus::WaitingApproval => "?",
                    ToolStatus::Completed => "+",
                    ToolStatus::Denied => "x",
                };
                let elapsed = elapsed_ms
                    .map(|ms| format!(" ({ms}ms)"))
                    .unwrap_or_default();
                out.push_str(&format!("### [{icon}] `{name}`{elapsed}\n\n"));

                if !args_summary.is_empty() {
                    out.push_str("```\n");
                    out.push_str(args_summary);
                    if !args_summary.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("```\n\n");
                }

                if let Some(o) = output {
                    if !o.is_empty() {
                        out.push_str("<details><summary>Output</summary>\n\n");
                        out.push_str("```\n");
                        out.push_str(o);
                        if !o.ends_with('\n') {
                            out.push('\n');
                        }
                        out.push_str("```\n\n");
                        out.push_str("</details>\n\n");
                    }
                }
            }
        }
    }

    out
}

pub fn default_export_path() -> String {
    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    format!("nyzhi-export-{ts}.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_empty() {
        let meta = ExportMeta {
            provider: "test".into(),
            model: "test-model".into(),
            usage: SessionUsage::default(),
            timestamp: chrono::Utc::now(),
        };
        let md = export_session_markdown(&[], &meta);
        assert!(md.contains("provider: test"));
        assert!(md.contains("model: test-model"));
    }

    #[test]
    fn test_export_messages() {
        let items = vec![
            DisplayItem::Message {
                role: "user".into(),
                content: "Hello".into(),
            },
            DisplayItem::Message {
                role: "assistant".into(),
                content: "Hi there!".into(),
            },
            DisplayItem::Message {
                role: "system".into(),
                content: "Session started".into(),
            },
        ];
        let meta = ExportMeta {
            provider: "anthropic".into(),
            model: "claude".into(),
            usage: SessionUsage::default(),
            timestamp: chrono::Utc::now(),
        };
        let md = export_session_markdown(&items, &meta);
        assert!(md.contains("## You\n\nHello"));
        assert!(md.contains("## Assistant\n\nHi there!"));
        assert!(md.contains("> Session started"));
    }

    #[test]
    fn test_export_tool_call() {
        let items = vec![DisplayItem::ToolCall {
            name: "bash".into(),
            args_summary: "ls -la".into(),
            output: Some("file1.rs\nfile2.rs".into()),
            status: ToolStatus::Completed,
            elapsed_ms: Some(150),
        }];
        let meta = ExportMeta {
            provider: "openai".into(),
            model: "gpt-4".into(),
            usage: SessionUsage::default(),
            timestamp: chrono::Utc::now(),
        };
        let md = export_session_markdown(&items, &meta);
        assert!(md.contains("[+] `bash` (150ms)"));
        assert!(md.contains("ls -la"));
        assert!(md.contains("file1.rs"));
    }

    #[test]
    fn test_default_export_path() {
        let path = default_export_path();
        assert!(path.starts_with("nyzhi-export-"));
        assert!(path.ends_with(".md"));
    }
}
