# Autopilot

Autopilot is Nyzhi's fully autonomous execution mode. Give it an idea, and it runs through five phases -- expansion, planning, execution, QA, and validation -- without requiring manual intervention between phases.

---

## Quick Start

```
/autopilot Add rate limiting to the API with per-user quotas and a Redis backend
```

The agent takes over and works through all five phases, persisting state between each phase so progress is never lost.

---

## The Five Phases

### 1. Expansion

The agent takes your initial idea and expands it into a full specification:

- Clarifies requirements and constraints
- Identifies edge cases and dependencies
- Produces a structured requirements document

The output is stored as `requirements` in the autopilot state.

### 2. Planning

Using the expanded requirements, the agent creates a detailed execution plan:

- Breaks the work into discrete, ordered steps
- Identifies files to create or modify
- Notes testing strategy and verification criteria

The plan is stored and can be reviewed later via `/plan`.

### 3. Execution

The agent executes the plan step by step:

- Creates and modifies files
- Runs commands
- Uses all available tools
- Logs each action in the execution log

This is the longest phase. The agent works through the plan items sequentially, adapting if it encounters issues.

### 4. QA

After execution, the agent reviews its own work:

- Runs verification checks (build, test, lint)
- Reviews code for correctness and quality
- Identifies remaining issues
- May make additional fixes

QA results are stored for the validation phase.

### 5. Validation

The final phase verifies everything is complete:

- Confirms all requirements are met
- Ensures tests pass
- Produces a validation report
- Marks the autopilot as `Complete`

---

## State Persistence

Autopilot state is saved to `.nyzhi/state/autopilot.json` in the project directory. This means:

- If the process is interrupted, you can resume from the last completed phase.
- State includes: the original idea, current phase, requirements, plan, execution log, QA results, and validation report.

```json
{
  "idea": "Add rate limiting...",
  "phase": "Execution",
  "requirements": "...",
  "plan": "...",
  "execution_log": ["..."],
  "qa_results": null,
  "validation_report": null
}
```

---

## Phase Transitions

```
Expansion → Planning → Execution → QA → Validation → Complete
                                                         │
                                          (or)  → Cancelled
```

Autopilot advances one phase at a time. Each phase must complete before the next begins. You can cancel at any time, which sets the phase to `Cancelled`.

---

## Cancellation

To cancel an in-progress autopilot:

- Press Ctrl+C during execution
- The state is saved with phase `Cancelled`

To clear saved state and start fresh:

```
/clear
/autopilot <new idea>
```

---

## Related Features

- **`/persist`** -- A lighter-weight autonomous mode that runs verify/fix loops until all checks pass.
- **`/qa`** -- Activates autonomous QA cycling without the full 5-phase pipeline.
- **`/plan`** -- View plans generated during the planning phase (or from manual `plan:` prefix prompts).
