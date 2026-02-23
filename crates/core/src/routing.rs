use nyzhi_provider::{ModelInfo, ModelTier, Provider};

const LOW_KEYWORDS: &[&str] = &[
    "typo", "rename", "format", "lint", "simple", "quick", "trivial", "minor", "comment", "readme",
    "docs", "log", "print", "echo",
];

const HIGH_KEYWORDS: &[&str] = &[
    "architect",
    "design",
    "refactor",
    "security",
    "complex",
    "migrate",
    "optimize",
    "performance",
    "debug",
    "investigate",
    "analyze",
    "review",
    "concurrent",
    "parallel",
    "distributed",
    "algorithm",
];

pub fn classify_prompt(prompt: &str, config: &nyzhi_config::RoutingConfig) -> ModelTier {
    let lower = prompt.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let word_count = words.len();

    let mut low_score: i32 = 0;
    let mut high_score: i32 = 0;

    for kw in LOW_KEYWORDS {
        if lower.contains(kw) {
            low_score += 1;
        }
    }
    for kw in &config.low_keywords {
        if lower.contains(kw.as_str()) {
            low_score += 1;
        }
    }

    for kw in HIGH_KEYWORDS {
        if lower.contains(kw) {
            high_score += 1;
        }
    }
    for kw in &config.high_keywords {
        if lower.contains(kw.as_str()) {
            high_score += 1;
        }
    }

    if word_count > 200 {
        high_score += 2;
    } else if word_count > 80 {
        high_score += 1;
    }

    if high_score > low_score {
        ModelTier::High
    } else if low_score > high_score {
        ModelTier::Low
    } else {
        ModelTier::Medium
    }
}

pub fn select_model_for_prompt<'a>(
    prompt: &str,
    provider: &'a dyn Provider,
    config: &nyzhi_config::RoutingConfig,
) -> (&'a ModelInfo, ModelTier) {
    let tier = classify_prompt(prompt, config);
    let model = provider
        .model_for_tier(tier)
        .unwrap_or(&provider.supported_models()[0]);
    (model, tier)
}
