# Model Routing

Model routing automatically selects the right model tier for each prompt based on task complexity. Simple tasks go to cheaper, faster models; complex tasks go to more capable ones.

---

## Overview

When routing is enabled, each prompt is classified into one of three tiers:

| Tier | Description | Example Tasks |
|------|-------------|---------------|
| **Low** | Simple, mechanical changes | Fix a typo, rename a variable, format code |
| **Medium** | Standard development tasks | Add a function, write a test, explain code |
| **High** | Complex, multi-step work | Architect a system, security audit, large refactor |

The provider then selects the appropriate model for the tier.

---

## Configuration

```toml
[agent.routing]
enabled = false                # disabled by default
low_keywords = []              # additional keywords for low tier
high_keywords = []             # additional keywords for high tier
```

---

## Classification Algorithm

Prompts are classified using keyword analysis and length heuristics:

### Built-In Keywords

**Low-tier keywords**: typo, rename, format, lint, fix indent, fix spacing, fix whitespace, simple, trivial, minor, small, quick, one-line, single, update comment, add comment, remove comment

**High-tier keywords**: architect, design, refactor, security, audit, migrate, complex, comprehensive, overhaul, rewrite, optimize performance, full review, system design, multi-step, parallel, large-scale

### Custom Keywords

Add project-specific keywords:

```toml
[agent.routing]
enabled = true
low_keywords = ["bump version", "update dep"]
high_keywords = ["database migration", "api redesign"]
```

### Length Heuristics

Prompt length contributes to the classification:

- **> 200 words**: +2 high score (longer prompts tend to describe complex tasks)
- **> 80 words**: +1 high score

### Scoring

The classifier counts keyword matches for low and high categories:

- If high > low → **High** tier
- If low > high → **Low** tier
- Otherwise → **Medium** tier

---

## Per-Provider Model Selection

Each provider maps tiers to specific models via `model_for_tier()`:

### OpenAI

| Tier | Model |
|------|-------|
| Low | o4-mini |
| Medium | GPT-5.2 |
| High | GPT-5.3 Codex |

### Anthropic

| Tier | Model |
|------|-------|
| Low | Claude Haiku 4.5 |
| Medium | Claude Sonnet 4.6 |
| High | Claude Opus 4.6 |

### Gemini

| Tier | Model |
|------|-------|
| Low | Gemini 3 Flash |
| Medium | Gemini 2.5 Flash |
| High | Gemini 3.1 Pro |

---

## Events

When routing selects a model, a `RoutedModel` event is emitted:

```
RoutedModel { model_name: "claude-haiku-4-5-20250301", tier: Low }
```

This is displayed in the TUI so you can see which model was selected and why.

---

## Cost Implications

Routing can significantly reduce costs by using cheaper models for simple tasks. For example:

- A typo fix routes to Haiku (~$0.001) instead of Opus (~$0.05)
- A complex refactor still gets the full power of Opus

Track actual costs with `nyz cost daily` to see the impact.

---

## When to Enable

Routing works best when your workflow mixes simple and complex tasks. If you primarily do complex work, the overhead of classification isn't worth it -- just set a high-tier default model.

Enable routing if:

- You frequently make small fixes alongside large features
- You want to minimize API costs without manually switching models
- You use a provider with clear tier differentiation (Anthropic is ideal)
