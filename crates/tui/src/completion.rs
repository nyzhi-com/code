use std::path::Path;

const MAX_CANDIDATES: usize = 50;
const SLASH_COMMANDS: &[&str] = &[
    "/accent",
    "/agents",
    "/autopilot",
    "/changes",
    "/clear",
    "/commands",
    "/compact",
    "/editor",
    "/exit",
    "/export",
    "/help",
    "/hooks",
    "/image",
    "/init",
    "/login",
    "/mcp",
    "/model",
    "/notify",
    "/learn",
    "/notepad",
    "/persist",
    "/plan",
    "/qa",
    "/quit",
    "/resume",
    "/todo",
    "/verify",
    "/retry",
    "/search",
    "/team",
    "/session delete",
    "/session rename",
    "/sessions",
    "/style",
    "/theme",
    "/think",
    "/trust",
    "/undo",
    "/undo all",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    SlashCommand,
    AtMention,
    FilePath,
}

#[derive(Debug, Clone)]
pub struct CompletionState {
    pub candidates: Vec<String>,
    pub selected: usize,
    pub prefix: String,
    pub prefix_start: usize,
    pub context: CompletionContext,
    pub scroll_offset: usize,
}

impl CompletionState {
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
        let max_visible = 8;
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + max_visible {
            self.scroll_offset = self.selected - max_visible + 1;
        }
    }
}

/// Scan backward from cursor to determine which completion context applies.
///
/// Returns `(context, prefix_text, prefix_start_byte_offset)`.
pub fn detect_context(input: &str, cursor_pos: usize) -> Option<(CompletionContext, String, usize)> {
    let before = &input[..cursor_pos.min(input.len())];

    if let Some(at_pos) = find_at_mention_start(before) {
        let prefix = before[at_pos..].to_string();
        return Some((CompletionContext::AtMention, prefix, at_pos));
    }

    let trimmed = before.trim_start();
    if trimmed.starts_with("/image ") {
        let after_cmd = before.find("/image ").unwrap() + 7;
        let path_part = &before[after_cmd..];
        return Some((CompletionContext::FilePath, path_part.to_string(), after_cmd));
    }

    if trimmed.starts_with('/') {
        let slash_pos = before.find('/').unwrap();
        let prefix = before[slash_pos..].to_string();
        return Some((CompletionContext::SlashCommand, prefix, slash_pos));
    }

    None
}

/// Find the start of an `@mention` token ending at the cursor.
/// Returns the byte offset of the `@` character, or None.
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
) -> Vec<String> {
    match ctx {
        CompletionContext::SlashCommand => generate_slash_candidates(prefix, custom_commands),
        CompletionContext::AtMention => {
            let path_part = prefix.strip_prefix('@').unwrap_or(prefix);
            let mut candidates = generate_path_candidates(path_part, cwd);
            for c in &mut candidates {
                c.insert(0, '@');
            }
            candidates
        }
        CompletionContext::FilePath => generate_path_candidates(prefix, cwd),
    }
}

fn generate_slash_candidates(prefix: &str, custom_commands: &[nyzhi_core::commands::CustomCommand]) -> Vec<String> {
    let mut all: Vec<String> = SLASH_COMMANDS.iter().map(|s| s.to_string()).collect();
    for cmd in custom_commands {
        let name = format!("/{}", cmd.name);
        if !all.contains(&name) {
            all.push(name);
        }
    }
    all.sort();
    all.into_iter()
        .filter(|cmd| cmd.starts_with(prefix) && *cmd != prefix)
        .take(MAX_CANDIDATES)
        .collect()
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

/// Split a partial path into (directory to read, filename prefix to filter).
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

/// Reconstruct the display path by replacing just the filename portion.
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

/// Apply the selected completion into the input buffer.
/// Returns true if the completed candidate ends with `/` (directory drilling).
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
        let results = generate_slash_candidates("/co", &[]);
        assert!(results.contains(&"/compact".to_string()));
        assert!(!results.contains(&"/clear".to_string()));
    }

    #[test]
    fn slash_candidates_exact_no_duplicate() {
        let results = generate_slash_candidates("/quit", &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn slash_candidates_all() {
        let results = generate_slash_candidates("/", &[]);
        assert!(results.len() > 10);
    }

    #[test]
    fn slash_candidates_include_custom() {
        let custom = vec![nyzhi_core::commands::CustomCommand {
            name: "review".to_string(),
            prompt_template: "Review $ARGUMENTS".to_string(),
            description: "Code review".to_string(),
        }];
        let results = generate_slash_candidates("/rev", &custom);
        assert!(results.contains(&"/review".to_string()));
    }

    #[test]
    fn apply_completion_basic() {
        let mut input = "/co".to_string();
        let mut cursor = 3;
        let state = CompletionState {
            candidates: vec!["/compact".to_string()],
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
            selected: 0,
            prefix: String::new(),
            prefix_start: 0,
            context: CompletionContext::SlashCommand,
            scroll_offset: 0,
        };
        state.cycle_backward();
        assert_eq!(state.selected, 2);
    }
}
