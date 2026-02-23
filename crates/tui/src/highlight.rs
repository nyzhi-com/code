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
}

/// Split markdown text into alternating prose / fenced-code-block segments.
pub fn parse_segments(text: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut rest = text;

    loop {
        match rest.find("```") {
            None => {
                if !rest.is_empty() {
                    segments.push(Segment::Prose(rest));
                }
                break;
            }
            Some(fence_start) => {
                let prose = &rest[..fence_start];
                if !prose.is_empty() {
                    segments.push(Segment::Prose(prose));
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
                        segments.push(Segment::CodeBlock { lang, code });
                        let resume = close + 3;
                        let remaining = &code_body[resume..];
                        rest = remaining.strip_prefix('\n').unwrap_or(remaining);
                    }
                    None => {
                        segments.push(Segment::CodeBlock {
                            lang,
                            code: code_body,
                        });
                        break;
                    }
                }
            }
        }
    }
    segments
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
                    for segment in ranges {
                        if let Ok(span) = syntect_tui::into_span(segment) {
                            line_spans
                                .push(Span::styled(span.content.to_string(), span.style.bg(bg)));
                        }
                    }
                }
                Err(_) => {
                    line_spans.push(Span::styled(
                        line_text.to_string(),
                        Style::default().fg(gutter_color),
                    ));
                }
            }
            out.push(Line::from(line_spans));
        }
        out
    }
}

/// Format a single prose line with inline markdown:
/// **bold**, *italic*, `inline code`, # headings, - list items
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

/// Parse inline markdown: **bold**, *italic*, `code`
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
