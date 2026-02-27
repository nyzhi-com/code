# Routing

Source of truth:

- `crates/core/src/routing.rs`
- `crates/config/src/lib.rs` (`RoutingConfig`)

## Purpose

Routing selects a model tier based on prompt complexity when routing is enabled.

## Config

```toml
[agent.routing]
enabled = true
low_keywords = ["typo", "docs"]
high_keywords = ["security", "refactor", "performance"]
```

Fields:

- `enabled`
- `low_keywords`
- `high_keywords`

## Classification Logic

`classify_prompt(prompt, config)` computes:

- low score from built-in low keywords + configured low keywords
- high score from built-in high keywords + configured high keywords
- length boost:
  - `> 200` words: high +2
  - `> 80` words: high +1

Result:

- high score > low score -> `ModelTier::High`
- low score > high score -> `ModelTier::Low`
- tie -> `ModelTier::Medium`

## Model Selection

`select_model_for_prompt(prompt, provider, config)`:

1. classify prompt to tier
2. request provider model for that tier (`model_for_tier`)
3. fallback to provider first supported model

## Built-in Keyword Baselines

Low-signal defaults include terms such as:

- typo, rename, format, lint, simple, quick, docs

High-signal defaults include terms such as:

- architect, design, refactor, security, optimize, performance, debug, analyze

## Guidance

- enable routing when you use providers with clear low/medium/high model cost tiers
- add custom keywords aligned to your workload language
- keep keyword lists short and specific to avoid accidental misclassification
