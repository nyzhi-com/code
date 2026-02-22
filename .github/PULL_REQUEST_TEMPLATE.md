## What

<!-- Brief description of the change. -->

## Why

<!-- Motivation, context, or link to a related issue. Use "Fixes #123" to auto-close. -->

## How

<!-- Key implementation details. Omit if obvious from the diff. -->

## Testing

<!-- How was this tested? New tests, manual steps, CI results. -->

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] Documentation updated (if behavior changed)
- [ ] No new `#[allow(clippy::...)]` without explanation
