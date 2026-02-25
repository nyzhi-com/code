use anyhow::Result;
use serde_json::Value;

use crate::types::*;

/// Fetch models from a provider's API. Returns an empty vec (not error) on non-fatal failures.
pub async fn fetch_models(
    provider_id: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<ModelInfo>> {
    match provider_id {
        "openai" | "codex" => {
            fetch_openai_compat(provider_id, base_url, api_key, openai_filter).await
        }
        "anthropic" | "claude-sdk" => fetch_anthropic(provider_id, base_url, api_key).await,
        "gemini" => fetch_gemini(base_url, api_key).await,
        "openrouter" => fetch_openrouter(base_url).await,
        "ollama" => fetch_ollama(base_url).await,
        "together" => fetch_together(base_url, api_key).await,
        "deepseek" => fetch_openai_compat("deepseek", base_url, api_key, deepseek_filter).await,
        "groq" => fetch_openai_compat("groq", base_url, api_key, groq_filter).await,
        "cursor" => fetch_cursor(api_key).await,
        _ => Ok(vec![]),
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

fn openai_filter(id: &str) -> bool {
    let dominated_prefixes = ["gpt-", "o1-", "o3-", "o4-", "codex-", "chatgpt-"];
    let dominated = dominated_prefixes.iter().any(|p| id.starts_with(p));
    let excluded_contains = [
        "dall-e",
        "whisper",
        "tts",
        "embedding",
        "moderation",
        "davinci",
        "babbage",
        "curie",
        "ada",
    ];
    let excluded = excluded_contains.iter().any(|e| id.contains(e));
    dominated && !excluded
}

fn deepseek_filter(id: &str) -> bool {
    id.starts_with("deepseek")
}

fn groq_filter(id: &str) -> bool {
    !id.contains("whisper") && !id.contains("guard") && !id.contains("tool-use")
}

/// GET /v1/models (OpenAI-compatible format used by OpenAI, DeepSeek, Groq)
async fn fetch_openai_compat(
    provider_id: &str,
    base_url: &str,
    api_key: Option<&str>,
    filter: fn(&str) -> bool,
) -> Result<Vec<ModelInfo>> {
    let key = api_key.unwrap_or_default();
    if key.is_empty() {
        return Ok(vec![]);
    }

    let url = if base_url.contains("/v1") {
        format!("{}/models", base_url.trim_end_matches('/'))
    } else {
        format!("{}/v1/models", base_url.trim_end_matches('/'))
    };

    let resp = client()
        .get(&url)
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let data: Value = resp.json().await?;
    let models = data["data"].as_array().cloned().unwrap_or_default();

    Ok(models
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            if !filter(id) {
                return None;
            }
            Some(ModelInfo {
                id: id.to_string(),
                name: humanize_model_id(id),
                provider: provider_id.to_string(),
                context_window: 128_000,
                max_output_tokens: 16_384,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: false,
                input_price_per_m: 0.0,
                output_price_per_m: 0.0,
                cache_read_price_per_m: 0.0,
                cache_write_price_per_m: 0.0,
                tier: ModelTier::Medium,
                thinking: None,
            })
        })
        .collect())
}

/// GET /v1/models (Anthropic format)
async fn fetch_anthropic(
    provider_id: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<ModelInfo>> {
    let key = api_key.unwrap_or_default();
    if key.is_empty() {
        return Ok(vec![]);
    }

    let url = format!("{}/models?limit=100", base_url.trim_end_matches('/'));
    let resp = client()
        .get(&url)
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let data: Value = resp.json().await?;
    let models = data["data"].as_array().cloned().unwrap_or_default();

    Ok(models
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            let display = m["display_name"].as_str().unwrap_or(id);
            Some(ModelInfo {
                id: id.to_string(),
                name: display.to_string(),
                provider: provider_id.to_string(),
                context_window: 200_000,
                max_output_tokens: 8_192,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: true,
                input_price_per_m: 0.0,
                output_price_per_m: 0.0,
                cache_read_price_per_m: 0.0,
                cache_write_price_per_m: 0.0,
                tier: ModelTier::Medium,
                thinking: None,
            })
        })
        .collect())
}

/// GET /v1beta/models (Gemini format -- returns token limits)
async fn fetch_gemini(base_url: &str, api_key: Option<&str>) -> Result<Vec<ModelInfo>> {
    let key = api_key.unwrap_or_default();
    if key.is_empty() {
        return Ok(vec![]);
    }

    let base = if base_url.is_empty() {
        "https://generativelanguage.googleapis.com"
    } else {
        base_url.trim_end_matches('/')
    };
    let url = format!("{}/v1beta/models?key={}&pageSize=100", base, key);

    let resp = client().get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let data: Value = resp.json().await?;
    let models = data["models"].as_array().cloned().unwrap_or_default();

    Ok(models
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?;
            let id = name.strip_prefix("models/").unwrap_or(name);
            let methods = m["supportedGenerationMethods"].as_array()?;
            let supports_generate = methods
                .iter()
                .any(|v| v.as_str() == Some("generateContent"));
            if !supports_generate {
                return None;
            }
            let display = m["displayName"].as_str().unwrap_or(id);
            let input_limit = m["inputTokenLimit"].as_u64().unwrap_or(32_000) as u32;
            let output_limit = m["outputTokenLimit"].as_u64().unwrap_or(8_192) as u32;

            Some(ModelInfo {
                id: id.to_string(),
                name: display.to_string(),
                provider: "gemini".to_string(),
                context_window: input_limit,
                max_output_tokens: output_limit,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: true,
                input_price_per_m: 0.0,
                output_price_per_m: 0.0,
                cache_read_price_per_m: 0.0,
                cache_write_price_per_m: 0.0,
                tier: if id.contains("pro") {
                    ModelTier::High
                } else {
                    ModelTier::Medium
                },
                thinking: None,
            })
        })
        .collect())
}

/// GET /api/v1/models (OpenRouter -- no auth, returns full metadata)
async fn fetch_openrouter(base_url: &str) -> Result<Vec<ModelInfo>> {
    let base = if base_url.is_empty() {
        "https://openrouter.ai"
    } else {
        base_url.trim_end_matches('/')
    };
    let url = format!("{}/api/v1/models", base);

    let resp = client().get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let data: Value = resp.json().await?;
    let models = data["data"].as_array().cloned().unwrap_or_default();

    Ok(models
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            let name = m["name"].as_str().unwrap_or(id);
            let ctx = m["context_length"].as_u64().unwrap_or(4096) as u32;
            let max_out = m["top_provider"]["max_completion_tokens"]
                .as_u64()
                .unwrap_or(4096) as u32;
            let input_price = m["pricing"]["prompt"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|p| p * 1_000_000.0)
                .unwrap_or(0.0);
            let output_price = m["pricing"]["completion"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|p| p * 1_000_000.0)
                .unwrap_or(0.0);

            let modality = m["architecture"]["modality"].as_str().unwrap_or("");
            if !modality.contains("text") {
                return None;
            }

            Some(ModelInfo {
                id: id.to_string(),
                name: name.to_string(),
                provider: "openrouter".to_string(),
                context_window: ctx,
                max_output_tokens: max_out,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: modality.contains("image"),
                input_price_per_m: input_price,
                output_price_per_m: output_price,
                cache_read_price_per_m: 0.0,
                cache_write_price_per_m: 0.0,
                tier: if ctx >= 200_000 {
                    ModelTier::High
                } else {
                    ModelTier::Medium
                },
                thinking: None,
            })
        })
        .collect())
}

/// GET /api/tags (Ollama -- local models, no auth)
async fn fetch_ollama(base_url: &str) -> Result<Vec<ModelInfo>> {
    let base = if base_url.is_empty() {
        "http://localhost:11434"
    } else {
        base_url.trim_end_matches('/')
    };
    let url = format!("{}/api/tags", base);

    let resp = client().get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let data: Value = resp.json().await?;
    let models = data["models"].as_array().cloned().unwrap_or_default();

    Ok(models
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?;
            let model_id = m["model"].as_str().unwrap_or(name);
            let size_gb = m["size"].as_u64().unwrap_or(0) as f64 / 1_073_741_824.0;
            let display = format!("{} ({:.1}GB)", humanize_model_id(model_id), size_gb);

            Some(ModelInfo {
                id: model_id.to_string(),
                name: display,
                provider: "ollama".to_string(),
                context_window: 128_000,
                max_output_tokens: 32_768,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: false,
                input_price_per_m: 0.0,
                output_price_per_m: 0.0,
                cache_read_price_per_m: 0.0,
                cache_write_price_per_m: 0.0,
                tier: if size_gb > 20.0 {
                    ModelTier::High
                } else {
                    ModelTier::Medium
                },
                thinking: None,
            })
        })
        .collect())
}

/// GET /v1/models (Together -- returns context_length)
async fn fetch_together(base_url: &str, api_key: Option<&str>) -> Result<Vec<ModelInfo>> {
    let key = api_key.unwrap_or_default();
    if key.is_empty() {
        return Ok(vec![]);
    }

    let base = if base_url.is_empty() {
        "https://api.together.xyz"
    } else {
        base_url.trim_end_matches('/')
    };
    let url = format!("{}/v1/models", base);

    let resp = client()
        .get(&url)
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let data: Value = resp.json().await?;
    let models = if data.is_array() {
        data.as_array().cloned().unwrap_or_default()
    } else {
        data["data"].as_array().cloned().unwrap_or_default()
    };

    Ok(models
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            let mtype = m["type"].as_str().unwrap_or("");
            if mtype == "embedding"
                || mtype == "rerank"
                || mtype == "image"
                || mtype == "moderation"
            {
                return None;
            }
            let display = m["display_name"].as_str().unwrap_or(id);
            let ctx = m["context_length"].as_u64().unwrap_or(4096) as u32;

            Some(ModelInfo {
                id: id.to_string(),
                name: display.to_string(),
                provider: "together".to_string(),
                context_window: ctx,
                max_output_tokens: ctx.min(65_536),
                supports_tools: true,
                supports_streaming: true,
                supports_vision: false,
                input_price_per_m: 0.0,
                output_price_per_m: 0.0,
                cache_read_price_per_m: 0.0,
                cache_write_price_per_m: 0.0,
                tier: if ctx >= 100_000 {
                    ModelTier::High
                } else {
                    ModelTier::Medium
                },
                thinking: None,
            })
        })
        .collect())
}

/// GET https://api.cursor.com/v0/models (Cursor Background Agents API)
async fn fetch_cursor(api_key: Option<&str>) -> Result<Vec<ModelInfo>> {
    let key = api_key.unwrap_or_default();
    if key.is_empty() {
        return Ok(vec![]);
    }

    let url = "https://api.cursor.com/v0/models";
    let resp = client()
        .get(url)
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let data: Value = resp.json().await?;
    let models = data["models"].as_array().cloned().unwrap_or_default();

    Ok(models
        .iter()
        .filter_map(|m| {
            let id = m.as_str()?;
            Some(ModelInfo {
                id: id.to_string(),
                name: humanize_model_id(id),
                provider: "cursor".to_string(),
                context_window: 200_000,
                max_output_tokens: 64_000,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: true,
                input_price_per_m: 0.0,
                output_price_per_m: 0.0,
                cache_read_price_per_m: 0.0,
                cache_write_price_per_m: 0.0,
                tier: ModelTier::Medium,
                thinking: None,
            })
        })
        .collect())
}

fn humanize_model_id(id: &str) -> String {
    let name = id
        .rsplit('/')
        .next()
        .unwrap_or(id)
        .replace(['-', '_', ':'], " ");

    let mut result = String::with_capacity(name.len());
    let mut capitalize_next = true;
    for ch in name.chars() {
        if ch == ' ' {
            result.push(' ');
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Merge API-fetched models with hardcoded defaults.
/// Fetched models take priority; hardcoded fill gaps and supplement metadata.
pub fn merge_models(fetched: Vec<ModelInfo>, hardcoded: &[ModelInfo]) -> Vec<ModelInfo> {
    let mut merged: Vec<ModelInfo> = Vec::with_capacity(fetched.len() + hardcoded.len());
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for mut model in fetched {
        if let Some(hc) = hardcoded.iter().find(|h| h.id == model.id) {
            if model.context_window == 0 || model.context_window == 128_000 {
                model.context_window = hc.context_window;
            }
            if model.max_output_tokens == 0 || model.max_output_tokens == 16_384 {
                model.max_output_tokens = hc.max_output_tokens;
            }
            if model.input_price_per_m == 0.0 && hc.input_price_per_m > 0.0 {
                model.input_price_per_m = hc.input_price_per_m;
                model.output_price_per_m = hc.output_price_per_m;
                model.cache_read_price_per_m = hc.cache_read_price_per_m;
                model.cache_write_price_per_m = hc.cache_write_price_per_m;
            }
            if model.thinking.is_none() && hc.thinking.is_some() {
                model.thinking = hc.thinking.clone();
            }
            model.tier = hc.tier;
            model.name = hc.name.clone();
            model.supports_vision = hc.supports_vision;
        }
        seen_ids.insert(model.id.clone());
        merged.push(model);
    }

    for hc in hardcoded {
        if !seen_ids.contains(&hc.id) {
            merged.push(hc.clone());
        }
    }

    merged
}
