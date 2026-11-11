---
name: steer
description: Steering review of root's autonomous migration work since the last steering commit; audits, corrects docs/issues, refreshes /start-development, and can restart root.
disable-model-invocation: true
---

# Steer the OpenTTD-Rust migration

Review root's autonomous work between sessions. Root may run under Codex or Claude Code, with settings chosen by the user ([team](../../team.md#roles-and-models)). Find what root cannot see and correct rules, roadmap order, issues and PR comments; leave code to root's PR process. [Navigate](../navigate/SKILL.md) owns the goal line in `AGENTS.md`, [goal and phases](../../planning.md#goal-and-phases), and the roadmap's "Phases". Work within that course. Root acts on corrections literally, so write them carefully.

User focus: $ARGUMENTS (in Codex, any text sent with `$steer`). Answer a focused question first, to its root cause. Read it broadly: housekeeping includes code, tests and refactoring. Tell the user which of steps 2 and 3 still run. Land the answer and root's next action in step 4. Handle restart requests in step 6.

Attribution on every issue, comment and commit body you write:
`Agent: /root (<host> steering review) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## While root runs

Root may keep running; it shares the main checkout. Steering 31 first used this protocol.

- Edit only in your own locked worktree under `.local/worktrees/`, using `git -C`. Leave the main checkout, root's worktrees and `.local` output unchanged.
- **Notice.** At the start, send root a mid-turn notice (`t3_thread_send`, mode `steer`) that steering is underway. In it:
  - Tell root to continue all in-flight work.
  - Tell root to leave the orchestration skills, the work order and the steering sections unchanged until release.
- **Freeze.** Before step 4, ask root to:
  - push any docs edit already in progress, then hold direct docs pushes to `rust-migration`;
  - reply in your thread with its last docs commit.

  Edit on that tip.
- **Release.** After pushing, send root the commit hash and ask it to fetch and apply the new steering actions.
- **Measurements and restart.** Take no wall-clock timings while root builds. Skip step 6's restart; root picks up the commit at release.

## 1. Load the last steering

Identify the standing corrections and root's subsequent sessions.

1. Find the last steering commit with `git log --format='%h %ad %s' --date=short | grep -i 'Doc: .*Steer'`. Read its message and the roadmap's steering review section.
2. Find the last steering transcript on its host: `python3 docs/skills/steer/transcript.py` lists Claude Code sessions; `transcript.py --codex` lists Codex sessions, newest first. Steering sessions invoke the skill or request a root review near the commit's time. Read the conclusion of any intervening session that changed a rule. Use `transcript.py <id>` and read final messages first.
3. Read `docs/skills/start-development/SKILL.md`, `AGENTS.md`, `docs/workflow.md`, `docs/team.md`, `docs/planning.md`, `docs/porting.md`, `docs/rust-migration.md`, `docs/speed.md`, `docs/deep-checks.md`, `docs/components/README.md` and `docs/design/`.
4. Read root's sessions since steering on each host and model. They start with `$start-development` in Codex or `/start-development` in Claude Code. Use `transcript.py <id> --tools` for tool calls. Read the user's messages and how each session ended.
5. Treat user messages, including wind-down, as binding that session only. Check their premises, such as gates, speed figures and PR state, against the code before carrying them forward.

Done when you can list each standing correction, the issue that tracks it, and the review or message that introduced it.

## 2. Survey root's work since then

Collect the state, then judge corrections and issues.

1. Run together: `git fetch`, then `git log` from the steering commit to `origin/rust-migration`, and `git diff <steer>..origin/rust-migration -- AGENTS.md docs/ .github/`; read `docs/roadmap.md` in full, including its checkpoint.
2. Inspect `gh run list --branch rust-migration`, PR checks and cancelled runs; open PRs and issues; and latest reviews of integrated and pending PRs.
3. Run `python3 tools/port-metrics.py --whole-fork`; inspect PR metrics and speed ratios. Confirm progress figures count original C++ retired, not glue removed. Re-measure the stream inventory rows that steering owns in the roadmap: missing original comments, bare integers whose concept has a named type, wrappers and named constants. Use #854's tool, or the overhaul's scripts it names until it lands. After the cleanup stage, each row holds at its end state.
4. Inspect `git worktree list` and disk use. Run `python3 docs/skills/steer/issues.py`; inspect every issue's placement, unplaced and excluded first, and every full exclusion line.
5. Post a short status line to the user and continue.
6. Judge each standing correction from the last two reviews, including yours, and each rule changed outside steering since the last review. Use the diff, commit messages and latest steering notes. Mark each applied, slipped, reinterpreted or harmful; compare its cost in agent hours, CI runs and parked PRs with what it caught. A loosened rule is a red flag unless user-directed.
7. Return a harmful user-directed change to the user with evidence; do not revert it. Copy an untested change and what to watch into the new review. Trace problems to their rule, roadmap action or user message. Relax or remove a harmful rule before adding one.

Watch for these patterns:

- **Wheel-spinning**: process and evidence outgrow retired logic.
- **Sediment**: roadmap and docs become logs.
- **Slippage**: corrections move behind other work.
- **Template drift**: one boundary pattern spreads to later ports.
- **WIP creep**: stacked or conflicting branches cause dependency churn.
- **Finite prompts**: root stops when listed work runs out.
- **Artificial hurdles**: gates or circular deferrals park work without catching defects.
- **Shell workarounds**: `cd <worktree> &&`, hand-made agent worktrees, `sleep`, polling, `timeout N bash -c 'sleep'` or hand-written environment prefixes indicate a missing permanent fix.
- **Running dry**: planning selects ever smaller items or retirement falls while other areas lack queued work; identify the area.
- **Passed over**: issues lack placement, exclusions lack issues or return conditions, or finished work stays open.
- **Debt recurrence**: new or changed code lacks the original's types, named constants, comments or argument structs; or a stream inventory rises above its end state. Fix it where it got through: the implementer brief, the review checklist, or a CI check where the rule is mechanical.

Done when every standing correction has a verdict and cost, and every issue `issues.py` reports as unplaced or excluded and every exclusion line has a verdict: selected, queued behind a named event, excluded with its return condition, or closed. Judge each issue on its merits against [selection](../../planning.md#selection) and the defects it would catch; ordering a list is not a verdict.

## 3. Audit independently

Use fresh agents to compare the source, independently of root's reviews.

1. Launch fresh independent audits together using [team's host orchestration rules](../../team.md#spawning). Audits use the parent's provider and model family (user, 2026-11-11): under Claude Code they run on Opus, as steerings 31 to 33 did. Agents leave root's worktrees, branches, profiles and `.local/` output unchanged. Background audits use the host's supported delegated completion mechanism; same-turn native audits finish before the parent turn ends. Run long measurements in the background and end the turn ([AGENTS.md](../../../AGENTS.md#every-agent)); use T3 worktree isolation for code-changing agents, not `cd` into a hand-made worktree.
2. Assign up to four fidelity audits, one per large port integrated since steering or about to integrate, largest by retired lines first. Assign one or two targeted audits where the survey found most cost or risk: blockers and return conditions; process time against port time and its causes; or speed.
3. For speed, compare candidate and reference on play saves and `generate-tgp-256-1`, attribute the gap by port, judge the speed plan and remeasure each speed-parked PR at its exact head. Use callgrind instruction counts (`.local/valgrind`). For a wall-clock median, report machine-wide compiler, linker and `openttd` processes from `ps -eo pid,pcpu,comm,args` and the 1-minute load average.
4. Give every agent `AGENTS.md` and the roadmap. Give fidelity agents the worktree, commit, merge base and `.local/reference/openttd`; have them compare original bodies line by line. Check ownership and fallback bodies under `#else` or `#ifndef WITH_RUST`; Random draw count and order, widths, wrapping, saturation, returns and side effects; added fatal paths; [boundary rules](../../porting.md#boundary) and hot allocations; glue versus retired lines; Random, crash and contention coverage; and the [code standards](../../porting.md#code-standards): the original's types and named constants, its comments at the matching branches, and argument and context structs.
5. Require findings tagged REACHABLE-DIVERGENCE, UNREACHABLE, RULE-VIOLATION, COVERAGE-GAP, PERF or HURDLE, with `file:line` in Rust and original where applicable and what was compared. Relay summaries as audits return. Verify each load-bearing claim yourself before acting.

Done when every audit has reported and every claim you will act on is verified.

## 4. Correct course

Choose a few corrections, favoring systemic patterns, and put each in one authoritative place.

- Put rule changes in the owning doc per the [routing table](../../../AGENTS.md#routing); reserve `AGENTS.md` for rules every agent needs. State target behavior.
- Give new work an issue with problem, measurements, scope, evidence and acceptance; label it `roadmap` when ordered there. Put pending PR findings in a PR comment that separates what must change before integration from what can follow. Put new coverage gaps on #156.
- Rewrite "Where the fork stands" and the work order. Add an "Nth steering review" with findings and plain actions. Fold older reviews into "still in force"; retain exclusion return conditions and give exclusions without one a verdict. Prune completed rows and superseded text; move per-PR plans to PRs or issues.
- Attach bounds to corrections. Name constraints on next actions. A gate says what happens on a miss: a profile-guided fix round, not deferral. Base new thresholds, factors and budgets on measured noise under root's conditions; record the basis and mark them provisional for the next review. Read each rule alongside interacting rules.
- In the steering section, identify any reversal of user direction and why in one line.
- Commit changed guidance, roadmap and any step 5 edit together on `rust-migration`, for example `Doc: Steer migration toward <corrections> [skip ci]`. Push once settled; the user reviews the report ([change types](../../workflow.md#change-types)).

Done when corrections have authoritative homes and the docs-only commit is settled.

## 5. Check /start-development

Improve the restart prompt only if this review shows it needs a change.

- Edit `docs/skills/start-development/SKILL.md` for an early stop, misreading or needed standing step. Keep it open-ended: completed items trigger planning and only the user ends the assignment. Include any edit in the step 4 commit.

Done when the prompt has been checked and any needed edit is included.

## 6. Report and hand off

Report the review and restart root only if the focus requested it.

1. Briefly report root's track, audit findings with numbers, changes with issue links and commit hash, reversed user direction, and anything only the user can supply.
2. If asked to restart, wait until `origin/rust-migration` has the commit. Launch one thread with `t3_thread_launch`, using the host's tool discovery (ToolSearch in Claude Code; `ALL_TOOLS` in Codex code mode). Recover the previous development root's settings under [team](../../team.md#roles-and-models), then apply the user's explicit overrides. Read the selected source thread before launching; do not use this steering session as the source.
   - `modelSelection`: the recovered provider, exact model and model options, with any explicit user overrides. Map them to the host's current schema; preserve effort and service tier where supported. Ask only when a required setting is missing or unavailable.
   - `workspaceStrategy: {type: "root"}`, a title, and a stable `clientRequestId`, so a retry after an error cannot launch twice.
   - The prompt `$start-development` in Codex or `/start-development` in Claude Code, followed by any scope from the focus. Preserve other root settings from the source thread where the host supports them, subject to the current session's permission limits.
3. Confirm with `t3_thread_read` that it is running, and link it with the `link` the launch returns.
4. Otherwise tell the user to start root with start-development. Either way, say whether the review changed the prompt; the prompt lives in the skill, not the reply.

Done when the report and requested handoff are complete.
