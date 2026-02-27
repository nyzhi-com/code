use std::path::Path;

const MAX_CANDIDATES: usize = 50;
const MAX_VISIBLE: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Instant,
    StreamingSafe,
    Prompt,
}

pub struct SlashCommandDef {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: CommandKind,
}

pub const SLASH_COMMANDS: &[SlashCommandDef] = &[
    SlashCommandDef {
        name: "/accent",
        description: "change accent color",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/agents",
        description: "list available agent roles",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/analytics",
        description: "session analytics and friction detection",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/autopilot",
        description: "autonomous multi-step execution",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/background",
        description: "alias for /bg",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/bg",
        description: "manage background tasks",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/bug",
        description: "generate a bug report",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/checkpoint",
        description: "save/list/restore session checkpoints",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/checkpoint save",
        description: "save a named checkpoint",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/checkpoint list",
        description: "list all checkpoints",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/checkpoint restore",
        description: "restore a checkpoint by id or name",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/changes",
        description: "list file changes this session",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/clear",
        description: "clear the session",
        kind: CommandKind::StreamingSafe,
    },
    SlashCommandDef {
        name: "/clear queue",
        description: "clear the message queue",
        kind: CommandKind::StreamingSafe,
    },
    SlashCommandDef {
        name: "/commands",
        description: "list custom commands",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/compact",
        description: "compress conversation history",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/connect",
        description: "connect a provider",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/context",
        description: "show context window usage",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/deep",
        description: "deep mode: autonomous research then implement",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/diff",
        description: "show all file changes this session as diffs",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/docs",
        description: "view/manage cached documentation (librarian)",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/docs add",
        description: "cache docs: /docs add <key> <url-or-text>",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/docs get",
        description: "retrieve cached docs by key",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/docs clear",
        description: "clear all cached docs",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/doctor",
        description: "run diagnostics",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/editor",
        description: "open $EDITOR for multi-line input",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/enable_exa",
        description: "set up Exa web search",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/exit",
        description: "exit nyzhi",
        kind: CommandKind::StreamingSafe,
    },
    SlashCommandDef {
        name: "/export",
        description: "export conversation as markdown",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/handoff",
        description: "create session handoff for continuation",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/help",
        description: "show all commands and shortcuts",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/hooks",
        description: "list configured hooks",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/image",
        description: "attach an image to next prompt",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/index",
        description: "force re-index the codebase",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/index off",
        description: "disable auto-context for this session",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/index status",
        description: "show index stats (files, chunks, db size)",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/init",
        description: "initialize .nyzhi/ project config",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/init-deep",
        description: "generate AGENTS.md files across project",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/learn",
        description: "create or list learned skills",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/login",
        description: "show OAuth login status",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/mcp",
        description: "list connected MCP servers",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/model",
        description: "choose what model to use",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/notepad",
        description: "view saved notepads",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/notify",
        description: "configure notifications",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/persist",
        description: "enable verify-and-fix mode",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/plan",
        description: "view or create execution plans",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/qa",
        description: "run autonomous QA cycling",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/quit",
        description: "exit nyzhi",
        kind: CommandKind::StreamingSafe,
    },
    SlashCommandDef {
        name: "/refactor",
        description: "structured refactoring workflow",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/resume",
        description: "restore a saved session",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/review",
        description: "code review: uncommitted, HEAD~N, or pr N",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/retry",
        description: "resend the last prompt",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/search",
        description: "search session messages",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/share",
        description: "share session to share.nyzhi.com",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/voice",
        description: "toggle voice input (Whisper)",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/walkthrough",
        description: "generate codebase walkthrough diagram",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/session delete",
        description: "delete a saved session",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/session rename",
        description: "rename current session",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/sessions",
        description: "list saved sessions",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/status",
        description: "show session status and usage",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/stop",
        description: "stop all continuation mechanisms",
        kind: CommandKind::StreamingSafe,
    },
    SlashCommandDef {
        name: "/style",
        description: "change output verbosity",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/thinking toggle",
        description: "toggle thinking display",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/settings",
        description: "open settings menu",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/team",
        description: "spawn coordinated sub-agents",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/theme",
        description: "choose theme (dark/light)",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/think",
        description: "toggle extended thinking",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/thinking",
        description: "set thinking effort level",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/todo",
        description: "view todo list and progress",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/todo enforce on",
        description: "enable todo enforcer",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/todo enforce off",
        description: "pause todo enforcer",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/todo clear",
        description: "clear all todos",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/trust",
        description: "show or set trust mode",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/undo",
        description: "undo the last file change",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/undo all",
        description: "undo all changes this session",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/undo git",
        description: "restore all files from git HEAD",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/verify",
        description: "detect and list project checks",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/quick",
        description: "ad-hoc task with commit discipline",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/map",
        description: "map codebase (stack, arch, conventions, concerns)",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/init-project",
        description: "structured project initialization with research",
        kind: CommandKind::Prompt,
    },
    SlashCommandDef {
        name: "/profile",
        description: "switch model profile (quality/balanced/budget)",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/worktree",
        description: "manage git worktrees for isolated agent work",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/worktree create",
        description: "create isolated worktree workspace",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/worktree list",
        description: "list active worktrees",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/worktree merge",
        description: "merge worktree back to parent branch",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/worktree remove",
        description: "remove a worktree",
        kind: CommandKind::Instant,
    },
    SlashCommandDef {
        name: "/resume-work",
        description: "load latest handoff and resume",
        kind: CommandKind::Prompt,
    },
];

pub fn classify_command(input: &str) -> CommandKind {
    let cmd = input.split_whitespace().next().unwrap_or("");
    for def in SLASH_COMMANDS {
        if input == def.name || cmd == def.name {
            return def.kind;
        }
    }
    if cmd.starts_with('/') {
        CommandKind::Prompt
    } else {
        CommandKind::Prompt
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    SlashCommand,
    AtMention,
    FilePath,
}

#[derive(Debug, Clone)]
pub struct CompletionState {
    pub candidates: Vec<String>,
    pub descriptions: Vec<String>,
    pub selected: usize,
    pub prefix: String,
    pub prefix_start: usize,
    pub context: CompletionContext,
    pub scroll_offset: usize,
}

impl CompletionState {
    pub fn max_visible(&self) -> usize {
        MAX_VISIBLE
    }

    pub fn cycle_forward(&mut self) {
        if !self.candidates.is_empty() {
            self.selected = (self.selected + 1) % self.candidates.len();
            self.ensure_visible();
        }
    }

    pub fn cycle_backward(&mut self) {
        if !self.candidates.is_empty() {
            self.selected = if self.selected == 0 {
                self.candidates.len() - 1
            } else {
                self.selected - 1
            };
            self.ensure_visible();
        }
    }

    fn ensure_visible(&mut self) {
        let max_visible = MAX_VISIBLE;
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + max_visible {
            self.scroll_offset = self.selected - max_visible + 1;
        }
    }
}

pub fn detect_context(
    input: &str,
    cursor_pos: usize,
) -> Option<(CompletionContext, String, usize)> {
    let before = &input[..cursor_pos.min(input.len())];

    if let Some(at_pos) = find_at_mention_start(before) {
        let prefix = before[at_pos..].to_string();
        return Some((CompletionContext::AtMention, prefix, at_pos));
    }

    let trimmed = before.trim_start();
    if trimmed.starts_with("/image ") {
        let after_cmd = before.find("/image ").unwrap() + 7;
        let path_part = &before[after_cmd..];
        return Some((
            CompletionContext::FilePath,
            path_part.to_string(),
            after_cmd,
        ));
    }

    if trimmed.starts_with('/') {
        let slash_pos = before.find('/').unwrap();
        let prefix = before[slash_pos..].to_string();
        return Some((CompletionContext::SlashCommand, prefix, slash_pos));
    }

    None
}

fn find_at_mention_start(before_cursor: &str) -> Option<usize> {
    let at_pos = before_cursor.rfind('@')?;

    if at_pos > 0 {
        let prev = before_cursor.as_bytes()[at_pos - 1];
        if prev.is_ascii_alphanumeric() || prev == b'.' {
            return None;
        }
    }

    let after_at = &before_cursor[at_pos + 1..];
    let valid = after_at
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '/' || c == '~' || c == '-');
    if !valid {
        return None;
    }

    Some(at_pos)
}

pub fn generate_candidates(
    ctx: &CompletionContext,
    prefix: &str,
    cwd: &Path,
    custom_commands: &[nyzhi_core::commands::CustomCommand],
) -> (Vec<String>, Vec<String>) {
    match ctx {
        CompletionContext::SlashCommand => generate_slash_candidates(prefix, custom_commands),
        CompletionContext::AtMention => {
            let path_part = prefix.strip_prefix('@').unwrap_or(prefix);
            if path_part.contains('/') {
                let mut candidates = generate_path_candidates(path_part, cwd);
                for c in &mut candidates {
                    c.insert(0, '@');
                }
                let descs = vec![String::new(); candidates.len()];
                (candidates, descs)
            } else {
                let mut results = fuzzy_file_search(path_part, cwd, MAX_CANDIDATES);
                results.sort_by(|(_, sa), (_, sb)| sb.cmp(sa));
                let candidates: Vec<String> = results
                    .iter()
                    .map(|(path, _)| format!("@{path}"))
                    .collect();
                let descs = vec![String::new(); candidates.len()];
                (candidates, descs)
            }
        }
        CompletionContext::FilePath => {
            let candidates = generate_path_candidates(prefix, cwd);
            let descs = vec![String::new(); candidates.len()];
            (candidates, descs)
        }
    }
}

fn generate_slash_candidates(
    prefix: &str,
    custom_commands: &[nyzhi_core::commands::CustomCommand],
) -> (Vec<String>, Vec<String>) {
    let mut all: Vec<(String, String)> = SLASH_COMMANDS
        .iter()
        .map(|s| (s.name.to_string(), s.description.to_string()))
        .collect();
    for cmd in custom_commands {
        let name = format!("/{}", cmd.name);
        if !all.iter().any(|(n, _)| n == &name) {
            all.push((name, cmd.description.clone()));
        }
    }
    all.sort_by(|a, b| a.0.cmp(&b.0));
    let filtered: Vec<(String, String)> = all
        .into_iter()
        .filter(|(cmd, _)| cmd.starts_with(prefix) && *cmd != prefix)
        .take(MAX_CANDIDATES)
        .collect();
    let names = filtered.iter().map(|(n, _)| n.clone()).collect();
    let descs = filtered.iter().map(|(_, d)| d.clone()).collect();
    (names, descs)
}

fn generate_path_candidates(partial: &str, cwd: &Path) -> Vec<String> {
    let (dir_path, file_prefix) = split_path_prefix(partial, cwd);

    let read_dir = match std::fs::read_dir(&dir_path) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut candidates: Vec<String> = read_dir
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && !file_prefix.starts_with('.') {
                return None;
            }
            if !name.starts_with(&file_prefix) {
                return None;
            }
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let display = build_display_path(partial, &name, is_dir);
            Some(display)
        })
        .take(MAX_CANDIDATES)
        .collect();

    candidates.sort();
    candidates
}

fn fuzzy_file_search(query: &str, root: &Path, limit: usize) -> Vec<(String, u32)> {
    let query_lower = query.to_lowercase();
    let mut results: Vec<(String, u32)> = Vec::new();

    fn walk(dir: &Path, root: &Path, query: &str, results: &mut Vec<(String, u32)>, depth: u8) {
        if depth > 6 {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" || name == "target" || name == "__pycache__" {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let is_dir = path.is_dir();

            let score = fuzzy_score(query, &rel.to_lowercase());
            if score > 0 {
                let display = if is_dir {
                    format!("{rel}/")
                } else {
                    rel
                };
                results.push((display, score));
            }

            if is_dir && results.len() < 500 {
                walk(&path, root, query, results, depth + 1);
            }
        }
    }

    walk(root, root, &query_lower, &mut results, 0);
    results.sort_by(|a, b| b.1.cmp(&a.1));
    results.truncate(limit);
    results
}

fn fuzzy_score(query: &str, target: &str) -> u32 {
    if query.is_empty() {
        return 0;
    }
    let filename = target.rsplit('/').next().unwrap_or(target);

    if filename.contains(query) {
        return 100 + (50u32.saturating_sub(filename.len() as u32));
    }
    if target.contains(query) {
        return 50 + (50u32.saturating_sub(target.len() as u32));
    }

    let mut qi = 0;
    let query_chars: Vec<char> = query.chars().collect();
    let mut score = 0u32;
    let mut prev_match = false;

    for ch in target.chars() {
        if qi < query_chars.len() && ch == query_chars[qi] {
            score += if prev_match { 3 } else { 1 };
            qi += 1;
            prev_match = true;
        } else {
            prev_match = false;
        }
    }

    if qi == query_chars.len() {
        score + 10u32.saturating_sub(target.len() as u32 / 10)
    } else {
        0
    }
}

fn split_path_prefix(partial: &str, cwd: &Path) -> (std::path::PathBuf, String) {
    if partial.is_empty() {
        return (cwd.to_path_buf(), String::new());
    }

    if partial.ends_with('/') {
        let dir = if partial == "/" {
            std::path::PathBuf::from("/")
        } else {
            let p = Path::new(partial);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                cwd.join(p)
            }
        };
        return (dir, String::new());
    }

    let p = Path::new(partial);
    let file_prefix = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let parent = p.parent().unwrap_or(Path::new(""));

    let dir = if parent == Path::new("") {
        cwd.to_path_buf()
    } else if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        cwd.join(parent)
    };

    (dir, file_prefix)
}

fn build_display_path(partial: &str, filename: &str, is_dir: bool) -> String {
    let suffix = if is_dir {
        format!("{filename}/")
    } else {
        filename.to_string()
    };

    if let Some(last_slash) = partial.rfind('/') {
        format!("{}{suffix}", &partial[..=last_slash])
    } else {
        suffix
    }
}

pub fn apply_completion(
    input: &mut String,
    cursor_pos: &mut usize,
    state: &CompletionState,
) -> bool {
    let candidate = &state.candidates[state.selected];
    let end = (*cursor_pos).min(input.len());
    input.replace_range(state.prefix_start..end, candidate);
    *cursor_pos = state.prefix_start + candidate.len();
    candidate.ends_with('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_slash_command() {
        let (ctx, prefix, start) = detect_context("/co", 3).unwrap();
        assert_eq!(ctx, CompletionContext::SlashCommand);
        assert_eq!(prefix, "/co");
        assert_eq!(start, 0);
    }

    #[test]
    fn detect_slash_only() {
        let (ctx, prefix, _) = detect_context("/", 1).unwrap();
        assert_eq!(ctx, CompletionContext::SlashCommand);
        assert_eq!(prefix, "/");
    }

    #[test]
    fn detect_at_mention() {
        let (ctx, prefix, start) = detect_context("explain @src/ma", 15).unwrap();
        assert_eq!(ctx, CompletionContext::AtMention);
        assert_eq!(prefix, "@src/ma");
        assert_eq!(start, 8);
    }

    #[test]
    fn detect_at_mention_start_of_input() {
        let (ctx, prefix, start) = detect_context("@Cargo", 6).unwrap();
        assert_eq!(ctx, CompletionContext::AtMention);
        assert_eq!(prefix, "@Cargo");
        assert_eq!(start, 0);
    }

    #[test]
    fn skip_email_like_at() {
        assert!(detect_context("user@example.com", 16).is_none());
    }

    #[test]
    fn detect_image_path() {
        let (ctx, prefix, start) = detect_context("/image scr", 10).unwrap();
        assert_eq!(ctx, CompletionContext::FilePath);
        assert_eq!(prefix, "scr");
        assert_eq!(start, 7);
    }

    #[test]
    fn detect_empty_input() {
        assert!(detect_context("", 0).is_none());
    }

    #[test]
    fn detect_plain_text() {
        assert!(detect_context("hello world", 11).is_none());
    }

    #[test]
    fn slash_candidates_filter() {
        let (names, descs) = generate_slash_candidates("/co", &[]);
        assert!(names.contains(&"/compact".to_string()));
        assert!(!names.contains(&"/clear".to_string()));
        assert_eq!(names.len(), descs.len());
        let idx = names.iter().position(|n| n == "/compact").unwrap();
        assert!(!descs[idx].is_empty());
    }

    #[test]
    fn slash_candidates_exact_no_duplicate() {
        let (names, _) = generate_slash_candidates("/quit", &[]);
        assert!(names.is_empty());
    }

    #[test]
    fn slash_candidates_all() {
        let (names, descs) = generate_slash_candidates("/", &[]);
        assert!(names.len() > 10);
        assert_eq!(names.len(), descs.len());
    }

    #[test]
    fn slash_candidates_include_custom() {
        let custom = vec![nyzhi_core::commands::CustomCommand {
            name: "mycheck".to_string(),
            prompt_template: "Check $ARGUMENTS".to_string(),
            description: "Custom check".to_string(),
        }];
        let (names, descs) = generate_slash_candidates("/myc", &custom);
        assert!(names.contains(&"/mycheck".to_string()));
        let idx = names.iter().position(|n| n == "/mycheck").unwrap();
        assert_eq!(descs[idx], "Custom check");
    }

    #[test]
    fn apply_completion_basic() {
        let mut input = "/co".to_string();
        let mut cursor = 3;
        let state = CompletionState {
            candidates: vec!["/compact".to_string()],
            descriptions: vec!["compress history".to_string()],
            selected: 0,
            prefix: "/co".to_string(),
            prefix_start: 0,
            context: CompletionContext::SlashCommand,
            scroll_offset: 0,
        };
        let is_dir = apply_completion(&mut input, &mut cursor, &state);
        assert_eq!(input, "/compact");
        assert_eq!(cursor, 8);
        assert!(!is_dir);
    }

    #[test]
    fn apply_completion_at_mention() {
        let mut input = "explain @src/ma".to_string();
        let mut cursor = 15;
        let state = CompletionState {
            candidates: vec!["@src/main.rs".to_string()],
            descriptions: vec![String::new()],
            selected: 0,
            prefix: "@src/ma".to_string(),
            prefix_start: 8,
            context: CompletionContext::AtMention,
            scroll_offset: 0,
        };
        let is_dir = apply_completion(&mut input, &mut cursor, &state);
        assert_eq!(input, "explain @src/main.rs");
        assert_eq!(cursor, 20);
        assert!(!is_dir);
    }

    #[test]
    fn apply_completion_directory() {
        let mut input = "@sr".to_string();
        let mut cursor = 3;
        let state = CompletionState {
            candidates: vec!["@src/".to_string()],
            descriptions: vec![String::new()],
            selected: 0,
            prefix: "@sr".to_string(),
            prefix_start: 0,
            context: CompletionContext::AtMention,
            scroll_offset: 0,
        };
        let is_dir = apply_completion(&mut input, &mut cursor, &state);
        assert_eq!(input, "@src/");
        assert_eq!(cursor, 5);
        assert!(is_dir);
    }

    #[test]
    fn cycle_forward_wraps() {
        let mut state = CompletionState {
            candidates: vec!["a".into(), "b".into(), "c".into()],
            descriptions: vec![String::new(); 3],
            selected: 2,
            prefix: String::new(),
            prefix_start: 0,
            context: CompletionContext::SlashCommand,
            scroll_offset: 0,
        };
        state.cycle_forward();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn cycle_backward_wraps() {
        let mut state = CompletionState {
            candidates: vec!["a".into(), "b".into(), "c".into()],
            descriptions: vec![String::new(); 3],
            selected: 0,
            prefix: String::new(),
            prefix_start: 0,
            context: CompletionContext::SlashCommand,
            scroll_offset: 0,
        };
        state.cycle_backward();
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn slash_command_names_are_unique() {
        let mut names = std::collections::HashSet::new();
        for def in SLASH_COMMANDS {
            assert!(
                names.insert(def.name),
                "duplicate slash command definition: {}",
                def.name
            );
        }
    }

    #[test]
    fn classify_multi_word_command() {
        assert_eq!(
            classify_command("/thinking toggle"),
            CommandKind::Instant
        );
        assert_eq!(classify_command("/clear queue"), CommandKind::StreamingSafe);
    }
}
