# Full Wiring Audit Artifact Index

This directory contains the full audit work products for the 2026-02-25 execution.

## Directory map

- `00-baseline/`
  - Runtime file manifest, crate counts, initial git snapshot, slash command inventory.
- `01-feature-catalog/`
  - Feature catalogs for slash commands, tool registry, and CLI commands.
- `02-wiring-traces/`
  - Feature wiring matrix and per-file audit status map.
- `03-behavior-checks/`
  - Behavior verdicts, external reference baseline (Exa/Tavily), and focused issue status.
- `04-findings/`
  - Fault triage CSV and master findings report.
- `05-fix-batches/`
  - Batch plan and final fix ledger.
- `06-verification/`
  - Command output logs for required verification commands.

## Key entry files

- Baseline: `00-baseline/baseline-summary.md`
- Feature catalog: `01-feature-catalog/feature-catalog.md`
- Wiring status: `02-wiring-traces/wiring-summary.md`
- Behavior validation: `03-behavior-checks/behavior-summary.md`
- Findings: `04-findings/master-findings.md`
- Fix outcomes: `05-fix-batches/fix-ledger.md`
- Verification outcomes: `06-verification/verification-summary.md`

## Re-run verification

From repo root:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo check --workspace`
