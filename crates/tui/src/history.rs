use std::path::PathBuf;

const MAX_ENTRIES: usize = 1000;

pub struct InputHistory {
    entries: Vec<String>,
    cursor: usize,
    draft: String,
    navigating: bool,
    file_path: PathBuf,
}

impl InputHistory {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            draft: String::new(),
            navigating: false,
            file_path,
        }
    }

    pub fn load(&mut self) {
        let Ok(contents) = std::fs::read_to_string(&self.file_path) else {
            return;
        };
        self.entries = contents
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.replace("\\n", "\n"))
            .collect();
        if self.entries.len() > MAX_ENTRIES {
            let drain = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(..drain);
        }
        self.cursor = self.entries.len();
    }

    pub fn save(&self) {
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let lines: Vec<String> = self
            .entries
            .iter()
            .map(|e| e.replace('\n', "\\n"))
            .collect();
        let _ = std::fs::write(&self.file_path, lines.join("\n") + "\n");
    }

    pub fn push(&mut self, entry: String) {
        let trimmed = entry.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        if self.entries.last().map(|e| e.as_str()) == Some(&trimmed) {
            self.reset_cursor();
            return;
        }
        self.entries.push(trimmed);
        if self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.reset_cursor();
        self.save();
    }

    /// Move backward through history. On first call, saves current input as draft.
    /// Returns the history entry to display, or None if at the beginning.
    pub fn navigate_up(&mut self, current_input: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        if !self.navigating {
            self.draft = current_input.to_string();
            self.navigating = true;
            self.cursor = self.entries.len();
        }
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        Some(self.entries[self.cursor].clone())
    }

    /// Move forward through history. Returns draft when reaching the end.
    /// Returns None if not navigating or already past the end.
    pub fn navigate_down(&mut self) -> Option<String> {
        if !self.navigating {
            return None;
        }
        if self.cursor >= self.entries.len() {
            return None;
        }
        self.cursor += 1;
        if self.cursor >= self.entries.len() {
            self.navigating = false;
            Some(self.draft.clone())
        } else {
            Some(self.entries[self.cursor].clone())
        }
    }

    pub fn reset_cursor(&mut self) {
        self.cursor = self.entries.len();
        self.navigating = false;
        self.draft.clear();
    }

    /// Search entries matching query (case-insensitive), most recent first.
    /// Returns (original_index, entry) pairs.
    pub fn search(&self, query: &str) -> Vec<(usize, &str)> {
        if query.is_empty() {
            return self
                .entries
                .iter()
                .enumerate()
                .rev()
                .map(|(i, e)| (i, e.as_str()))
                .collect();
        }
        let q = query.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, e)| e.to_lowercase().contains(&q))
            .map(|(i, e)| (i, e.as_str()))
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct HistorySearch {
    pub query: String,
    pub selected: usize,
}

impl HistorySearch {
    pub fn new() -> Self {
        Self::default()
    }
}
