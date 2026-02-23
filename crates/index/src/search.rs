#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub score: f32,
    pub content: String,
}

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (lines {}-{}, score: {:.3})",
            self.file, self.start_line, self.end_line, self.score
        )
    }
}
