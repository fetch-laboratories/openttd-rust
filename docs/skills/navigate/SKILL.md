---
name: navigate
description: Macro review of the OpenTTD-Rust migration: judges the goal, phases, rules, metrics and skills against the project's destination and resets the course. Run before /steer.
disable-model-invocation: true
---

# Navigate the OpenTTD-Rust migration

Judge the whole course against this fixed destination, which stays as written:

OpenTTD as a Rust program: every original C++ line retired, the game behaving as the original does, and root's agents getting there on their own.

Everything else, including AGENTS.md's goal, is subject to review. Work at the scale of areas, phases, thousands of lines and rates over weeks; leave transcripts, PRs, CI and individual ports to `/steer`. Judge the whole picture yourself; a subagent may count lines. Correct without waiting for the user, except an ADR marked `may change: user` ([ADRs](../../adr/README.md#who-may-change-one)).

Attribution on every issue, comment and commit body you write:
`Agent: /root (<host> navigation) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## 1. Fix the position

- Count original C++ code lines retired and still compiled, excluding comments, blank and preprocessor lines, for the whole fork and each area: simulation logic, simulation storage (object classes, pools, map accessors), save/load, script API, network, NewGRF, support, GUI and drawing, platform and vendored. Use `python3 tools/port-metrics.py --whole-fork` where it reports these; count the rest.
- State the metric's source and build-target scope. Count candidate tooling and generated C++ it omits, keeping reference-only oracle support separate. Recount past endpoints with the same metric version (`--head`) and report inventory corrections apart from retirement.
- Measure retired lines per day across the fork's history and each of the last several steering periods (`git log --format='%h %ad %s' | grep -i 'Doc: .*Steer'`).
- The last navigation is `git log --format='%h %ad %s' | grep 'Doc: Navigate'`; its figures are in that commit's roadmap "Phases".

Done when one table holds done and remaining lines by area and the rate, each with its change since the last navigation.

## 2. Read the course

Read, as a newcomer who knows only the destination and position, `AGENTS.md`, every doc its routing table names, `docs/adr/`, `docs/design/`, `.github/PULL_REQUEST_TEMPLATE.md` and every skill under `docs/skills/`.

## 3. Judge the course

Ask whether each part, followed literally, reaches the destination:

- **Goal**: AGENTS.md's goal and every restatement in the roadmap, skills and planning briefs.
- **Plan**: C++ left after every planned item and phase, and what plans the next phase before work runs dry.
- **Rules**: rules, decisions or exclusions that retain C++ indefinitely, misdirect selection or waste time; hard limits and mandatory procedures where judgment would work better.
- **Metrics**: whether every figure root and steer use measures distance.
- **Rate**: its changes and the parts of the course responsible.
- **Autonomy**: waits, parked work and decisions the destination already answers.

Done when every part has a verdict, and each failure names its text by file and line.

## 4. Reset the course

- Correct each failure at its source: the goal line in `AGENTS.md`, [goal and phases](../../planning.md#goal-and-phases), the roadmap's "Phases", owning guidance and skills. Rewrite or remove a failing rule rather than adding a counter-rule. Record a correction that is a real decision as an ADR ([ADRs](../../adr/README.md)).
- Put a tool change, such as a metric, in an issue at the top of the work order.
- Replace the roadmap's "Phases" chart with the position table, each phase's size, exit criterion and status, and root's first actions.
- Commit all edits as one docs-only commit on `rust-migration`: `Doc: Navigate toward <correction> [skip ci]`, or `Doc: Navigate, course holds [skip ci]` if only the position changed. Push once settled.

## 5. Report

Give the user the position table, verdict, each change, issue links and the commit hash in a few lines.
