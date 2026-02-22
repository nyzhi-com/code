use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::permission::ToolPermission;
use super::{Tool, ToolContext, ToolResult};

pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch a URL and return its content as readable text. HTML is stripped to plain text. \
         Useful for reading web pages, documentation, API responses."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "max_length": {
                    "type": "integer",
                    "description": "Maximum response length in characters (default 50000)"
                }
            },
            "required": ["url"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("web_fetch requires 'url' parameter"))?;

        let max_length = args
            .get("max_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(50_000) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("nyzhi/1.0")
            .build()?;

        let resp = client.get(url).send().await?;
        let status = resp.status();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if !status.is_success() {
            return Ok(ToolResult {
                output: format!("HTTP {status} for {url}"),
                title: format!("web_fetch {url} ({status})"),
                metadata: json!({ "url": url, "status": status.as_u16() }),
            });
        }

        let body = resp.text().await?;
        let text = if content_type.contains("html") {
            html_to_text(&body)
        } else {
            body
        };

        let truncated = if text.len() > max_length {
            format!(
                "{}...\n\n[Truncated: {} chars total, showing first {}]",
                &text[..max_length],
                text.len(),
                max_length
            )
        } else {
            text
        };

        Ok(ToolResult {
            output: truncated,
            title: format!("web_fetch {url}"),
            metadata: json!({ "url": url, "status": status.as_u16(), "content_type": content_type }),
        })
    }
}

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns a list of results with titles, URLs, and snippets. \
         Uses a simple search via DuckDuckGo HTML."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default 5, max 10)"
                }
            },
            "required": ["query"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("web_search requires 'query' parameter"))?;

        let num_results = args
            .get("num_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .min(10) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("nyzhi/1.0")
            .build()?;

        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let resp = client.get(&url).send().await?;
        let body = resp.text().await?;

        let results = parse_ddg_results(&body, num_results);

        let mut output = format!("Search results for: {query}\n\n");
        if results.is_empty() {
            output.push_str("No results found.");
        } else {
            for (i, r) in results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. {}\n   {}\n   {}\n\n",
                    i + 1,
                    r.title,
                    r.url,
                    r.snippet
                ));
            }
        }

        Ok(ToolResult {
            output,
            title: format!("web_search \"{query}\""),
            metadata: json!({
                "query": query,
                "result_count": results.len(),
            }),
        })
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn parse_ddg_results(html: &str, max: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    for segment in html.split("class=\"result__a\"") {
        if results.len() >= max {
            break;
        }
        if !segment.contains("href=\"") {
            continue;
        }

        let url = extract_between(segment, "href=\"", "\"").unwrap_or_default();
        if url.is_empty() || url.starts_with('#') {
            continue;
        }

        let after_href = &segment[segment.find("href=\"").unwrap_or(0)..];
        let title = extract_between(after_href, ">", "<")
            .unwrap_or_default()
            .trim()
            .to_string();

        let snippet = if let Some(snip_start) = html
            .find(&format!("href=\"{url}\""))
            .and_then(|pos| html[pos..].find("class=\"result__snippet\""))
            .map(|offset| {
                let base = html.find(&format!("href=\"{url}\"")).unwrap_or(0);
                base + offset
            })
        {
            let chunk = &html[snip_start..];
            extract_between(chunk, ">", "</")
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            String::new()
        };

        let clean_url = if url.starts_with("//duckduckgo.com/l/?") {
            extract_between(url, "uddg=", "&")
                .map(|u| urlencoding::decode(u).unwrap_or_default().into_owned())
                .unwrap_or_else(|| url.to_string())
        } else {
            url.to_string()
        };

        results.push(SearchResult {
            title: html_entities_decode(&strip_tags(&title)),
            url: clean_url,
            snippet: html_entities_decode(&strip_tags(&snippet)),
        });
    }

    results
}

fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_idx = s.find(start)? + start.len();
    let rest = &s[start_idx..];
    let end_idx = rest.find(end)?;
    Some(&rest[..end_idx])
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

fn html_entities_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    // Block elements -> newlines
    let block_tags = [
        "<br>", "<br/>", "<br />", "</p>", "</div>", "</li>", "</tr>",
        "</h1>", "</h2>", "</h3>", "</h4>", "</h5>", "</h6>",
        "</blockquote>", "</pre>", "</table>",
    ];
    for tag in &block_tags {
        text = text.replace(tag, &format!("\n{tag}"));
    }

    let text = strip_tags(&text);
    let text = html_entities_decode(&text);

    // Collapse whitespace
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() || lines.last().map(|l| !l.is_empty()).unwrap_or(true) {
            lines.push(trimmed.to_string());
        }
    }

    lines.join("\n")
}
