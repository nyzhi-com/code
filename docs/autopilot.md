# Autopilot

Source of truth:

- `crates/core/src/autopilot.rs`
- `crates/tui/src/input.rs` (`/autopilot` command handling)

## What Autopilot Is

Autopilot is a multi-phase execution state machine for long-running implementation workflows.

State is persisted per project and can be resumed or inspected.

## Phases

`AutopilotPhase`:

- `expansion`
- `planning`
- `execution`
- `qa`
- `validation`
- `complete`
- `cancelled`

Transition order:

`expansion -> planning -> execution -> qa -> validation -> complete`

## State Model

`AutopilotState` fields:

- `idea`
- `phase`
- `requirements`
- `plan`
- `execution_log`
- `qa_results`
- `validation_report`

## Persistence Path

Stored at:

- `<project>/.nyzhi/state/autopilot.json`

APIs:

- `save_state`
- `load_state`
- `clear_state`

## TUI Command Usage

```text
/autopilot <idea>
/autopilot
/autopilot cancel
/autopilot clear
```

Behavior:

- `/autopilot <idea>` initializes state and dispatches expansion prompt
- `/autopilot` prints current state summary
- `cancel` marks phase as `cancelled`
- `clear` removes persisted state file

## Prompt Builders

Autopilot module provides dedicated builders:

- `build_expansion_prompt`
- `build_planning_prompt`
- `build_execution_prompt`
- `build_qa_prompt`
- `build_validation_prompt`

These templates define expected output structure for each phase.

## Operational Notes

- autopilot uses standard agent/tool runtime under the hood
- it is not a separate execution engine; it is orchestrated through prompts + persisted phase state
- run verification and review outputs before finalizing production changes
