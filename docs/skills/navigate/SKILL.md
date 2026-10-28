---
name: navigate
description: Macro review of the OpenTTD-Rust migration: judges the goal, phases, rules, metrics and skills against the project's destination and resets the course. Run before /steer.
disable-model-invocation: true
---

# Navigate the OpenTTD-Rust migration

Root runs the migration, and steer is its helmsman: it holds root to the course
the system sets (AGENTS.md, the roadmap, the design docs and the skills). You
are the navigator. You check that the course reaches the destination, and you
reset it when it does not. Agents write every part of the system and it
drifts with each edit; nothing else checks it against the destination.

## The destination

OpenTTD as a Rust program: every original C++ line retired, the game behaving
as the original does, and root's agents getting there on their own.

This is your one fixed reference, and it stays as written here. Everything
else is course, AGENTS.md's goal statement included, and is judged against it.

## Altitude

Work at the altitude of the whole project: areas and phases, thousands of
lines, the rate over weeks. Your inputs are the code, its git history and the
system's documents. Root's transcripts, PRs, CI, reviews and individual ports
are steer's; a finding smaller than an area or a phase is steer's too. Do the
judging yourself, since it needs the whole picture in one head; a subagent may
count lines for you.

The system runs without the user. Answer each question yourself from the
destination and land each correction yourself. A decision AGENTS.md records as
the user's stands as written, and you work within it.

Attribution on every issue, comment and commit body you write:
`Agent: /root (<host> navigation) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## 1. Fix the position

Measure from the code before reading the course, so the course cannot frame
the measurement:

- **Done and remaining**: original C++ code lines (no comments, blank or
  preprocessor lines), retired and still compiled, for the whole fork and by
  area: simulation logic, simulation storage (object classes, pools, map
  accessors), save/load, script API, network, NewGRF, support, GUI and
  drawing, platform, vendored. Use `python3 tools/port-metrics.py --whole-fork`
  where it reports this, and count what it does not.
- **Rate**: code lines retired per day over the fork's history and over each
  of the last several steering periods (`--head` measures a past commit;
  `git log --format='%h %ad %s' | grep -i 'Doc: .*Steer'` lists them).
- **Arrival**: the date the remaining code retires at the recent rate, and how
  far it moved since the last navigation (`git log --format='%h %ad %s' |
  grep 'Doc: Navigate'`; its figures are in the roadmap's "Phases" section).

Done when one table holds the done and remaining code lines by area, the
rate, the arrival, and each one's change since the last navigation.

## 2. Read the course

Read whole, as a newcomer who knows only the destination and the position:
AGENTS.md, `docs/roadmap.md`, `docs/rust-migration.md`, `docs/design/`, and
every skill under `docs/skills/`.

## 3. Judge the course

Test each part with one question: followed literally and perfectly, does it
reach the destination?

- **Goal**: AGENTS.md's goal statement and every restatement of it in the
  roadmap, the skills and the planning briefs they prescribe. A narrowed or
  reworded goal is the costliest drift, because every agent optimizes for it.
- **Plan**: if root finished every planned item and phase, what C++ would
  remain, and what plans the next phase before the current one runs dry?
- **Rules**: each rule, design decision or exclusion that, followed literally,
  keeps code in C++ with no end point, steers selection away from the
  remaining code, or spends root's time on work the destination does not need.
  Rigidity is the usual cause: a hard limit, fixed threshold or mandatory
  procedure where the agent's judgment toward the destination would do better.
- **Metrics**: what each figure root and steer select or report by counts, and
  whether it measures distance to the destination.
- **Rate**: why it rose or fell, and which part of the course explains it.
- **Autonomy**: each point where the system waits on the user, parks work
  until they decide, or requires handing them a decision the destination
  already answers. The open outlet for an agent to ask the user stays.

Done when every part has a verdict, and each failure names the text that
causes it, by file and line.

## 4. Reset the course

Correct each failure where it originates: the goal statement and rules in
AGENTS.md, the phases in the roadmap, the design docs and the skills' text.
Rewrite or remove the failing rule itself; a counter-rule beside it leaves
both in force. A tool change, such as a metric, goes in an issue at the top of
the work order, since code goes through root's PR process. Write each
correction as the outcome root should reach.

Rewrite the roadmap's "Phases" section as the chart root starts from: the
position table from step 1, each phase's size, exit criterion and status, and
the actions root takes first. It replaces the previous navigation's chart.

Commit every edit as one docs-only commit pushed straight to
`rust-migration`, titled `Doc: Navigate toward <correction> [skip ci]`, or
`Doc: Navigate, course holds [skip ci]` when only the position changed.
Confirm `origin/rust-migration` has it.

## 5. Report

Tell the user, in a few lines: the position table, the arrival and how far it
moved, the verdict on the course, and each change with its commit and issue
links.
