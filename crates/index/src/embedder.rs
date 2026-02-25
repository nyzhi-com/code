use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
    fn model_id(&self) -> &str;
}

// ---------------------------------------------------------------------------
// OpenAI-compatible API embedder
// ---------------------------------------------------------------------------

pub struct ApiEmbedder {
    api_key: String,
    model: String,
    base_url: String,
    dims: usize,
}

impl ApiEmbedder {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "text-embedding-3-small".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            dims: 1536,
        }
    }
}

#[async_trait]
impl Embedder for ApiEmbedder {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let client = reqwest::Client::new();
        let mut all_embeddings = Vec::with_capacity(texts.len());

        for batch in texts.chunks(100) {
            let input: Vec<&str> = batch.to_vec();
            let body = serde_json::json!({
                "model": self.model,
                "input": input,
            });

            let resp = client
                .post(format!("{}/embeddings", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                anyhow::bail!("Embedding API error {status}: {text}");
            }

            let json: serde_json::Value = resp.json().await?;
            if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                for item in data {
                    if let Some(emb) = item.get("embedding").and_then(|e| e.as_array()) {
                        let vec: Vec<f32> = emb
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        all_embeddings.push(vec);
                    }
                }
            }
        }

        Ok(all_embeddings)
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

// ---------------------------------------------------------------------------
// TF-IDF hash embedder (fallback, no API needed)
// Produces fixed-dimension dense vectors via feature hashing.
// ---------------------------------------------------------------------------

const TFIDF_DIMS: usize = 384;

pub struct TfIdfEmbedder;

impl TfIdfEmbedder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TfIdfEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Embedder for TfIdfEmbedder {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| hash_embed(t)).collect())
    }

    fn dimensions(&self) -> usize {
        TFIDF_DIMS
    }

    fn model_id(&self) -> &str {
        "tfidf-hash-384"
    }
}

fn hash_embed(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0f32; TFIDF_DIMS];
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return vec;
    }

    let mut tf: HashMap<String, f32> = HashMap::new();
    for tok in &tokens {
        *tf.entry(tok.clone()).or_default() += 1.0;
    }
    let total = tokens.len() as f32;
    for val in tf.values_mut() {
        *val /= total;
    }

    for (tok, weight) in &tf {
        let h = simple_hash(tok);
        let idx = (h as usize) % TFIDF_DIMS;
        let sign = if (h >> 16) & 1 == 0 { 1.0 } else { -1.0 };
        vec[idx] += sign * weight;

        let idx2 = ((h >> 8) as usize) % TFIDF_DIMS;
        let sign2 = if (h >> 24) & 1 == 0 { 1.0 } else { -1.0 };
        vec[idx2] += sign2 * weight * 0.5;
    }

    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 1e-10 {
        for v in &mut vec {
            *v /= norm;
        }
    }

    vec
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            split_camel(&current, &mut tokens);
            current.clear();
        }
    }
    if !current.is_empty() {
        split_camel(&current, &mut tokens);
    }

    tokens.retain(|t| t.len() >= 2 && !is_stop(t));
    tokens
}

fn split_camel(word: &str, out: &mut Vec<String>) {
    out.push(word.to_string());
    let chars: Vec<char> = word.chars().collect();
    let mut start = 0;
    for i in 1..chars.len() {
        if chars[i].is_uppercase() && !chars[i - 1].is_uppercase() {
            let part: String = chars[start..i].iter().collect();
            if part.len() >= 2 {
                out.push(part.to_lowercase());
            }
            start = i;
        }
    }
    if start > 0 && start < chars.len() {
        let part: String = chars[start..].iter().collect();
        if part.len() >= 2 {
            out.push(part.to_lowercase());
        }
    }
}

fn simple_hash(s: &str) -> u32 {
    let mut h: u32 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    h
}

fn is_stop(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "is"
            | "at"
            | "in"
            | "of"
            | "on"
            | "to"
            | "and"
            | "or"
            | "an"
            | "it"
            | "if"
            | "do"
            | "no"
            | "as"
            | "be"
            | "by"
            | "we"
            | "so"
            | "he"
            | "up"
            | "my"
            | "me"
            | "am"
            | "for"
            | "not"
            | "but"
            | "you"
            | "all"
            | "can"
            | "had"
            | "her"
            | "was"
            | "one"
            | "our"
            | "out"
            | "has"
            | "this"
            | "that"
            | "with"
            | "from"
            | "they"
            | "been"
            | "have"
            | "will"
            | "use"
            | "new"
            | "get"
            | "set"
            | "let"
            | "var"
            | "mut"
            | "pub"
            | "fn"
            | "mod"
            | "struct"
            | "impl"
            | "return"
            | "true"
            | "false"
            | "self"
            | "none"
            | "string"
            | "type"
            | "default"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_embed_produces_correct_dims() {
        let vec = hash_embed("fn hello_world() { println!(\"hi\"); }");
        assert_eq!(vec.len(), TFIDF_DIMS);
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01 || norm < 0.01);
    }

    #[test]
    fn similar_texts_have_higher_similarity() {
        let a = hash_embed("fn parse_json(input: &str) -> Value");
        let b = hash_embed("fn parse_json_data(s: &str) -> serde_json::Value");
        let c = hash_embed("class UserAuthenticationService implements OAuth");
        let sim_ab = cosine(&a, &b);
        let sim_ac = cosine(&a, &c);
        assert!(
            sim_ab > sim_ac,
            "similar code should score higher: {sim_ab} vs {sim_ac}"
        );
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na < 1e-10 || nb < 1e-10 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
