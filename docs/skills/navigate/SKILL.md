---
name: navigate
description: Macro review of the OpenTTD-Rust migration: judges the goal, phases, rules, metrics and skills against the project's destination and resets the course. Run before /steer.
disable-model-invocation: true
---

# Navigate the OpenTTD-Rust migration

Judge the whole course against this fixed destination, which stays as written:

OpenTTD as a Rust program: every original C++ line retired, the game behaving as the original does, and root's agents getting there on their own.

Everything else, including AGENTS.md's goal, is subject to review. Work at the scale of areas, phases, thousands of lines and rates over weeks. Use code, history and system documents. Steer handles root transcripts, PRs, CI, reviews, individual ports and smaller findings. Judge the whole picture yourself; a subagent may count lines. Answer and correct without waiting for the user, but respect decisions recorded as the user's.

Attribution on every issue, comment and commit body you write:
`Agent: /root (<host> navigation) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## 1. Fix the position

Measure the code before reading the course.

- Count original C++ code lines retired and still compiled, excluding comments, blank and preprocessor lines. Count the whole fork and each area: simulation logic, simulation storage (object classes, pools, map accessors), save/load, script API, network, NewGRF, support, GUI and drawing, platform and vendored.
- Use `python3 tools/port-metrics.py --whole-fork` where it reports these; count the rest.
- Measure retired code lines per day across the fork's history and each of the last several steering periods. Use `--head` for a past commit and `git log --format='%h %ad %s' | grep -i 'Doc: .*Steer'` to list periods.
- Find the last navigation with `git log --format='%h %ad %s' | grep 'Doc: Navigate'`. Its figures are in the roadmap's "Phases".

Done when one table holds done and remaining code lines by area and the rate, each with its change since the last navigation.

## 2. Read the course

Read the whole system as a newcomer who knows only the destination and position.

- Read `AGENTS.md`, `docs/roadmap.md`, `docs/workflow.md`, `docs/team.md`, `docs/planning.md`, `docs/porting.md`, `docs/rust-migration.md`, `docs/speed.md`, `docs/deep-checks.md`, `docs/components/`, `docs/design/` and every skill under `docs/skills/`.

Done when you have read the course in full.

## 3. Judge the course

Ask whether each part, followed literally and perfectly, reaches the destination.

- **Goal**: Check AGENTS.md's goal and every restatement in the roadmap, skills and their planning briefs. A narrowed goal redirects every agent.
- **Plan**: Identify C++ left after every planned item and phase, and what plans the next phase before work runs dry.
- **Rules**: Find rules, design decisions or exclusions that retain C++ indefinitely, misdirect selection or waste time. Examine hard limits, thresholds and mandatory procedures where judgment may work better.
- **Metrics**: Check what every figure root and steer use counts and whether it measures distance.
- **Rate**: Explain its changes and the parts of the course responsible.
- **Autonomy**: Find waits, parked work and decisions the destination already answers. Keep the outlet for agents to ask the user.

Done when every part has a verdict, and each failure names the text that causes it, by file and line.

## 4. Reset the course

Correct each failure at its source.

- Correct the goal line in `AGENTS.md`, [goal and phases](../../planning.md#goal-and-phases), the roadmap's "Phases", owning guidance and skills as needed. Rewrite or remove a failing rule rather than adding a counter-rule. State the outcome root should reach.
- Put a tool change, such as a metric, in an issue at the top of the work order; code follows root's PR process.
- Replace the roadmap's "Phases" chart with the position table, each phase's size, exit criterion and status, and root's first actions.
- Commit all edits as one docs-only commit on `rust-migration`: `Doc: Navigate toward <correction> [skip ci]`, or `Doc: Navigate, course holds [skip ci]` if only position changed. Push once settled; the user reviews the report ([change types](../../workflow.md#change-types)).

Done when each failure is corrected or assigned to its tool issue and the chart is updated.

## 5. Report

Give the user the course verdict and changes.

- Report the position table, verdict, each change, issue links and commit hash in a few lines.

Done when the user has the position, verdict and changes.
