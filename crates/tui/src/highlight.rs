use ratatui::prelude::*;
use ratatui::style::Color;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

#[derive(Debug)]
pub enum Segment<'a> {
    Prose(&'a str),
    CodeBlock {
        lang: Option<&'a str>,
        code: &'a str,
    },
    Table(Vec<&'a str>),
}

/// Split markdown text into alternating prose / fenced-code-block / table segments.
pub fn parse_segments(text: &str) -> Vec<Segment<'_>> {
    let mut raw_segments = Vec::new();
    let mut rest = text;

    loop {
        match rest.find("```") {
            None => {
                if !rest.is_empty() {
                    raw_segments.push(Segment::Prose(rest));
                }
                break;
            }
            Some(fence_start) => {
                let prose = &rest[..fence_start];
                if !prose.is_empty() {
                    raw_segments.push(Segment::Prose(prose));
                }

                let after_fence = &rest[fence_start + 3..];
                let lang_end = after_fence.find('\n').unwrap_or(after_fence.len());
                let lang_tag = after_fence[..lang_end].trim();
                let lang = if lang_tag.is_empty() {
                    None
                } else {
                    Some(lang_tag)
                };

                let code_start_offset = lang_end + 1;
                let code_body = if code_start_offset <= after_fence.len() {
                    &after_fence[code_start_offset..]
                } else {
                    ""
                };

                match code_body.find("```") {
                    Some(close) => {
                        let code = &code_body[..close];
                        let code = code.strip_suffix('\n').unwrap_or(code);
                        raw_segments.push(Segment::CodeBlock { lang, code });
                        let resume = close + 3;
                        let remaining = &code_body[resume..];
                        rest = remaining.strip_prefix('\n').unwrap_or(remaining);
                    }
                    None => {
                        raw_segments.push(Segment::CodeBlock {
                            lang,
                            code: code_body,
                        });
                        break;
                    }
                }
            }
        }
    }

    let mut segments = Vec::new();
    for seg in raw_segments {
        match seg {
            Segment::Prose(text) => extract_tables(text, &mut segments),
            other => segments.push(other),
        }
    }
    segments
}

fn is_table_line(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.ends_with('|') && t.len() > 2
}

fn extract_tables<'a>(prose: &'a str, out: &mut Vec<Segment<'a>>) {
    let lines: Vec<&str> = prose.lines().collect();
    let mut i = 0;
    let mut prose_start = 0;

    while i < lines.len() {
        if is_table_line(lines[i]) {
            let table_start = i;
            while i < lines.len() && is_table_line(lines[i]) {
                i += 1;
            }
            if i - table_start >= 2 {
                let before = &lines[prose_start..table_start];
                if !before.is_empty() {
                    let joined: String = before.join("\n");
                    if !joined.trim().is_empty() {
                        out.push(Segment::Prose(
                            &prose[byte_offset(prose, before[0])
                                ..byte_end(prose, before[before.len() - 1])],
                        ));
                    }
                }
                out.push(Segment::Table(lines[table_start..i].to_vec()));
                prose_start = i;
            }
        } else {
            i += 1;
        }
    }

    if prose_start < lines.len() {
        let remaining = &lines[prose_start..];
        if !remaining.is_empty() {
            let start = byte_offset(prose, remaining[0]);
            let end = byte_end(prose, remaining[remaining.len() - 1]);
            let slice = &prose[start..end];
            if !slice.trim().is_empty() {
                out.push(Segment::Prose(slice));
            }
        }
    } else if prose_start == 0 && lines.is_empty() && !prose.trim().is_empty() {
        out.push(Segment::Prose(prose));
    }
}

fn byte_offset(haystack: &str, needle: &str) -> usize {
    let h = haystack.as_ptr() as usize;
    let n = needle.as_ptr() as usize;
    n.saturating_sub(h)
}

fn byte_end(haystack: &str, needle: &str) -> usize {
    let start = byte_offset(haystack, needle);
    (start + needle.len()).min(haystack.len())
}

pub struct SyntaxHighlighter {
    ps: SyntaxSet,
    ts: ThemeSet,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }

    fn syntect_theme_name(dark: bool) -> &'static str {
        if dark {
            "base16-ocean.dark"
        } else {
            "base16-ocean.light"
        }
    }

    /// Highlight a code block and return ratatui Lines.
    pub fn highlight_code<'a>(
        &self,
        code: &str,
        lang: Option<&str>,
        dark: bool,
        gutter_color: Color,
        bg: Color,
    ) -> Vec<Line<'a>> {
        let syntax = lang
            .and_then(|l| self.ps.find_syntax_by_token(l))
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());

        let theme_name = Self::syntect_theme_name(dark);
        let theme = &self.ts.themes[theme_name];

        let mut h = HighlightLines::new(syntax, theme);
        let mut out = Vec::new();

        for (line_num, line_text) in LinesWithEndings::from(code).enumerate() {
            let line_no = format!(" {:>3} ", line_num + 1);
            let mut line_spans: Vec<Span<'a>> = vec![Span::styled(
                line_no,
                Style::default().fg(gutter_color).bg(bg),
            )];

            match h.highlight_line(line_text, &self.ps) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        let fg = Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        );
                        line_spans.push(Span::styled(
                            text.to_string(),
                            Style::default().fg(fg).bg(bg),
                        ));
                    }
                }
                Err(_) => {
                    line_spans.push(Span::styled(
                        line_text.to_string(),
                        Style::default().fg(gutter_color).bg(bg),
                    ));
                }
            }
            out.push(Line::from(line_spans));
        }
        out
    }
}

fn is_horizontal_rule(line: &str) -> bool {
    let t = line.trim();
    t.len() >= 3
        && (t.chars().all(|c| c == '-' || c == ' ')
            || t.chars().all(|c| c == '*' || c == ' ')
            || t.chars().all(|c| c == '_' || c == ' '))
        && t.chars().filter(|c| !c.is_whitespace()).count() >= 3
}

fn strip_numbered_list(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i >= bytes.len() {
        return None;
    }
    if bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1] == b' ' {
        Some((&line[..i + 1], &line[i + 2..]))
    } else {
        None
    }
}

/// Format a single prose line with full markdown support.
pub fn format_prose_line<'a>(raw: &str, base_fg: Color, accent: Color, code_bg: Color) -> Line<'a> {
    let trimmed = raw.trim_end();

    if trimmed.starts_with("###") {
        let heading = trimmed.trim_start_matches('#').trim();
        return Line::from(vec![Span::styled(
            format!("  {heading}"),
            Style::default().fg(accent).bold(),
        )]);
    }
    if trimmed.starts_with("##") {
        let heading = trimmed.trim_start_matches('#').trim();
        return Line::from(vec![Span::styled(
            format!("  {heading}"),
            Style::default().fg(accent).bold(),
        )]);
    }
    if trimmed.starts_with('#') {
        let heading = trimmed.trim_start_matches('#').trim();
        return Line::from(vec![Span::styled(
            format!("  {heading}"),
            Style::default().fg(accent).bold(),
        )]);
    }

    if is_horizontal_rule(trimmed) {
        let rule = crate::aesthetic::borders::THIN_H.repeat(60);
        return Line::from(Span::styled(
            format!("  {rule}"),
            Style::default().fg(Color::DarkGray),
        ));
    }

    if trimmed.starts_with("> ") || trimmed == ">" {
        let mut depth = 0u8;
        let mut rest = trimmed;
        while rest.starts_with("> ") || rest == ">" {
            depth += 1;
            rest = if rest.len() > 2 { &rest[2..] } else { "" };
        }
        let bar = "  ┃ ".repeat(depth as usize);
        let dim = Color::DarkGray;
        let mut spans = vec![Span::styled(bar, Style::default().fg(dim))];
        spans.extend(parse_inline_markdown(rest, base_fg, accent, code_bg));
        return Line::from(spans);
    }

    if let Some((num, rest)) = strip_numbered_list(trimmed) {
        let mut spans = vec![
            Span::raw("  "),
            Span::styled(
                format!("{num} "),
                Style::default().fg(accent),
            ),
        ];
        spans.extend(parse_inline_markdown(rest, base_fg, accent, code_bg));
        return Line::from(spans);
    }

    let indent = if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        "  • "
    } else {
        "  "
    };

    let text = if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        &trimmed[2..]
    } else {
        trimmed
    };

    let spans = parse_inline_markdown(text, base_fg, accent, code_bg);

    let mut result = vec![Span::raw(indent.to_string())];
    result.extend(spans);
    Line::from(result)
}

fn try_parse_link(chars: &[char], start: usize) -> Option<(String, usize)> {
    let len = chars.len();
    if start >= len || chars[start] != '[' {
        return None;
    }
    let mut i = start + 1;
    let text_start = i;
    while i < len && chars[i] != ']' {
        if chars[i] == '\n' {
            return None;
        }
        i += 1;
    }
    if i >= len {
        return None;
    }
    let text: String = chars[text_start..i].iter().collect();
    i += 1;
    if i >= len || chars[i] != '(' {
        return None;
    }
    i += 1;
    while i < len && chars[i] != ')' {
        if chars[i] == '\n' {
            return None;
        }
        i += 1;
    }
    if i >= len {
        return None;
    }
    i += 1;
    Some((text, i))
}

/// Parse inline markdown: **bold**, *italic*, `code`, [link](url)
fn parse_inline_markdown<'a>(
    text: &str,
    base_fg: Color,
    accent: Color,
    code_bg: Color,
) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut buf = String::new();

    while i < len {
        if chars[i] == '`' {
            if !buf.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut buf),
                    Style::default().fg(base_fg),
                ));
            }
            i += 1;
            let start = i;
            while i < len && chars[i] != '`' {
                i += 1;
            }
            let code: String = chars[start..i].iter().collect();
            spans.push(Span::styled(code, Style::default().fg(accent).bg(code_bg)));
            if i < len {
                i += 1;
            }
        } else if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if !buf.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut buf),
                    Style::default().fg(base_fg),
                ));
            }
            i += 2;
            let start = i;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '*') {
                i += 1;
            }
            let bold: String = chars[start..i].iter().collect();
            spans.push(Span::styled(bold, Style::default().fg(base_fg).bold()));
            if i + 1 < len {
                i += 2;
            }
        } else if chars[i] == '*' {
            if !buf.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut buf),
                    Style::default().fg(base_fg),
                ));
            }
            i += 1;
            let start = i;
            while i < len && chars[i] != '*' {
                i += 1;
            }
            let italic: String = chars[start..i].iter().collect();
            spans.push(Span::styled(italic, Style::default().fg(base_fg).italic()));
            if i < len {
                i += 1;
            }
        } else if chars[i] == '[' {
            if let Some((link_text, end_idx)) = try_parse_link(&chars, i) {
                if !buf.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut buf),
                        Style::default().fg(base_fg),
                    ));
                }
                spans.push(Span::styled(
                    link_text,
                    Style::default().fg(accent).underlined(),
                ));
                i = end_idx;
            } else {
                buf.push(chars[i]);
                i += 1;
            }
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }

    if !buf.is_empty() {
        spans.push(Span::styled(buf, Style::default().fg(base_fg)));
    }
    spans
}

fn is_separator_row(line: &str) -> bool {
    let cells: Vec<&str> = line.split('|').collect();
    cells
        .iter()
        .filter(|c| !c.trim().is_empty())
        .all(|c| c.trim().chars().all(|ch| ch == '-' || ch == ':'))
}

fn parse_table_row(line: &str) -> Vec<String> {
    line.split('|')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

pub fn format_table_lines<'a>(
    table_lines: &[&str],
    header_fg: Color,
    cell_fg: Color,
    border_fg: Color,
) -> Vec<Line<'a>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut header_count = 0;

    for (i, line) in table_lines.iter().enumerate() {
        if is_separator_row(line) {
            if i > 0 {
                header_count = i;
            }
            continue;
        }
        rows.push(parse_table_row(line));
    }

    if rows.is_empty() {
        return vec![];
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; col_count];
    for row in &rows {
        for (j, cell) in row.iter().enumerate() {
            if j < col_count {
                col_widths[j] = col_widths[j].max(cell.len());
            }
        }
    }

    let mut lines = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let is_header = i < header_count;
        let fg = if is_header { header_fg } else { cell_fg };
        let style = if is_header {
            Style::default().fg(fg).bold()
        } else {
            Style::default().fg(fg)
        };

        let mut spans = vec![Span::raw("  ")];
        for (j, width) in col_widths.iter().enumerate() {
            let cell = row.get(j).map(|s| s.as_str()).unwrap_or("");
            let padded = format!(" {:<width$} ", cell, width = width);
            spans.push(Span::styled(padded, style));
            if j + 1 < col_count {
                spans.push(Span::styled("│", Style::default().fg(border_fg)));
            }
        }
        lines.push(Line::from(spans));

        if is_header && (i + 1 == header_count || header_count == 0) {
            let mut sep_spans = vec![Span::raw("  ")];
            for (j, width) in col_widths.iter().enumerate() {
                let dash = crate::aesthetic::borders::THIN_H.repeat(width + 2);
                sep_spans.push(Span::styled(dash, Style::default().fg(border_fg)));
                if j + 1 < col_count {
                    sep_spans.push(Span::styled("┼", Style::default().fg(border_fg)));
                }
            }
            lines.push(Line::from(sep_spans));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_segments_no_code() {
        let text = "Hello world\nSecond line";
        let segs = parse_segments(text);
        assert_eq!(segs.len(), 1);
        assert!(matches!(segs[0], Segment::Prose(_)));
    }

    #[test]
    fn parse_segments_with_code_block() {
        let text = "before\n```rust\nfn main() {}\n```\nafter";
        let segs = parse_segments(text);
        assert_eq!(segs.len(), 3);
        assert!(matches!(segs[0], Segment::Prose(_)));
        assert!(matches!(
            segs[1],
            Segment::CodeBlock {
                lang: Some("rust"),
                ..
            }
        ));
        assert!(matches!(segs[2], Segment::Prose(_)));
    }

    #[test]
    fn parse_segments_no_lang() {
        let text = "```\nplain code\n```";
        let segs = parse_segments(text);
        assert_eq!(segs.len(), 1);
        assert!(matches!(segs[0], Segment::CodeBlock { lang: None, .. }));
    }

    #[test]
    fn highlighter_does_not_panic() {
        let hl = SyntaxHighlighter::new();
        let lines = hl.highlight_code(
            "fn main() {}",
            Some("rust"),
            true,
            Color::Gray,
            Color::Black,
        );
        assert!(!lines.is_empty());
    }
}
