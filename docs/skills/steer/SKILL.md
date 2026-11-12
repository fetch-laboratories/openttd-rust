---
name: steer
description: Steering review of root's autonomous migration work since the last steering commit; audits, corrects docs/issues, refreshes /start-development, and can restart root.
disable-model-invocation: true
---

# Steer the OpenTTD-Rust migration

Review root's autonomous work since the last steering commit. Find what root cannot see and correct rules, roadmap order, issues and PR comments; leave code to root's PR process. [Navigate](../navigate/SKILL.md) owns the goal and the roadmap's "Phases"; work within that course. Root acts on corrections literally.

User focus: $ARGUMENTS (in Codex, any text sent with `$steer`). Answer a focused question first, to its root cause, and tell the user which of steps 2 and 3 still run.

Attribution on every issue, comment and commit body you write:
`Agent: /root (<host> steering review) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## While root runs

Root may keep running in the main checkout. Edit only in your own worktree under `.local/worktrees/`, using `git -C`.

- At the start, send root a notice (`t3_thread_send`, mode `steer`): steering is underway; continue in-flight work; leave the orchestration skills, the ADRs, the work order and "Latest steering actions" unchanged until release.
- Before step 4, ask root to push any docs edit in progress, hold docs pushes to `rust-migration` and reply with its last docs commit. Edit on that tip.
- After pushing, send root the commit hash and ask it to fetch and apply the new actions. Skip step 6's restart.
- Take no wall-clock timings while root builds.

## 1. Load the last steering

1. Find the last steering commit: `git log --format='%h %ad %s' --date=short | grep -i 'Doc: .*Steer'`. Read its message, which holds that review's findings, and the roadmap's "Latest steering actions".
2. `python3 docs/skills/steer/transcript.py` lists Claude Code sessions, newest first; `--codex` lists Codex sessions; `transcript.py <id>` reads one and `--tools` adds tool calls. Read root's sessions since that commit (they start with `/start-development` or `$start-development`): the user's messages and how each ended.
3. Read `AGENTS.md`, every doc its routing table names, `docs/adr/`, `docs/design/` and `.github/PULL_REQUEST_TEMPLATE.md`.
4. A user message, wind-down included, binds only its session. Check its premises against the code before carrying it forward.

Done when you can list each standing correction and the issue or ADR that tracks it.

## 2. Survey root's work since then

1. `git fetch`; read `git log` from the steering commit to `origin/rust-migration`, `git diff <steer>..origin/rust-migration -- AGENTS.md docs/ .github/` and the whole roadmap.
2. Inspect `gh run list --branch rust-migration`, cancelled runs, open PRs with their checks and latest reviews, and open issues.
3. Run `python3 tools/port-metrics.py --whole-fork`; progress counts original C++ retired, not glue removed. Re-measure the roadmap's stream inventory rows that steering owns with `python3 tools/code-standards-inventory.py`.
4. Inspect `git worktree list` and disk use. Run `python3 docs/skills/steer/issues.py`; give every unplaced or excluded issue and every exclusion line a verdict on its merits against [selection](../../planning.md#selection): selected, queued behind a named event, excluded with its return condition, or closed.
5. Post a short status line to the user and continue.
6. Judge each standing correction from the last two reviews and each rule changed outside steering: applied, slipped, reinterpreted or harmful, with its cost in agent hours, CI runs and parked PRs against what it caught. A rule loosened without an ADR is a red flag.
7. Trace each problem to its rule, roadmap action or user message. A harmful rule resting on an ADR marked `may change: user` goes back to the user with evidence and cost ([ADRs](../../adr/README.md#who-may-change-one)).

Watch for:

- **Wheel-spinning**: process and evidence outgrow retired logic.
- **Sediment**: roadmap and docs become logs.
- **Slippage**: corrections move behind other work.
- **Template drift**: one boundary pattern spreads to later ports.
- **WIP creep**: stacked or conflicting branches cause churn.
- **Finite prompts**: root stops when listed work runs out.
- **Artificial hurdles**: gates or circular deferrals park work without catching defects.
- **Shell workarounds**: `cd <worktree> &&`, hand-made agent worktrees, `sleep`, polling or hand-written environment prefixes point to a missing permanent fix.
- **Running dry**: planning selects ever smaller items while some area lacks queued work.
- **Debt recurrence**: new code lacks the original's types, named constants, comments or argument structs, or a stream inventory rises above its end state. Fix it where it got through: the implementer brief, the review brief, or a CI check.

Done when every standing correction and every issue `issues.py` reports has a verdict.

## 3. Audit independently

1. Launch fresh audits together under the host's orchestration rules, on the parent's model family (Opus under Claude Code). They leave root's worktrees, branches and `.local/` output unchanged.
2. Assign up to four fidelity audits, one per large port integrated since steering or about to integrate, largest first, and one or two targeted audits where the survey found most cost or risk: blockers, process time against port time, or speed.
3. Give every agent `AGENTS.md` and the roadmap. Give fidelity agents the worktree, commit, merge base and `.local/reference/openttd`, and have them compare original bodies line by line, including fallback bodies under `#ifndef WITH_RUST`, against [fidelity](../../porting.md#fidelity), [boundary](../../porting.md#boundary) and [code standards](../../porting.md#code-standards).
4. For speed, compare candidate and reference on play saves and `generate-tgp-256-1` with callgrind instruction counts (`.local/valgrind`), attribute the gap by port and remeasure each speed-parked PR at its head. A wall-clock median comes with the machine-wide compiler, linker and `openttd` processes from `ps -eo pid,pcpu,comm,args` and the 1-minute load average.
5. Require findings tagged REACHABLE-DIVERGENCE, UNREACHABLE, RULE-VIOLATION, COVERAGE-GAP, PERF or HURDLE, with `file:line` in Rust and original. Verify each claim you will act on.

Done when every audit has reported.

## 4. Correct course

Choose a few corrections, favoring systemic patterns.

- Prune before adding: remove each rule an agent would follow correctly without, or whose wrong action a tool already catches, and relax a harmful rule before adding a counter-rule.
- Put a standing rule once in the doc that owns it ([routing](../../../AGENTS.md#routing)), and a correction that is a real decision in an ADR ([ADRs](../../adr/README.md)).
- Give new work an issue with problem, measurements, scope and acceptance; label it `roadmap` when ordered there. Put PR findings in a PR comment that separates what blocks integration from what can follow. Put new coverage gaps on #156.
- Rewrite "Where the fork stands" and the work order. Replace "Latest steering actions" with this review's current actions and their issues, plus provisional items to judge at the next review; first move each earlier action that still stands into its owning doc or an ADR. Findings go in the commit message and issues, not the roadmap.
- Prune completed rows and superseded text, and move per-PR plans to PRs or issues.
- A gate says what happens on a miss: a profile-guided fix round, not deferral. Base a new threshold on measured noise under root's conditions and mark it provisional.
- Commit guidance, ADRs, roadmap and any step 5 edit as one docs-only commit on `rust-migration`, such as `Doc: Steer migration toward <corrections> [skip ci]`, and push once settled.

## 5. Check /start-development

Edit `docs/skills/start-development/SKILL.md` only for an early stop, a misreading or a missing standing step, keeping it open-ended: only the user ends the assignment. Include the edit in the step 4 commit.

## 6. Report and hand off

1. Report root's track, audit findings with numbers, changes with issue, ADR and commit links, any user ADR returned, and anything only the user can supply.
2. If the focus asks for a restart, wait until `origin/rust-migration` has the commit, then launch one thread with `t3_thread_launch`:
   - `modelSelection`: the previous development root's provider, exact model, options and other launch settings ([team](../../team.md#roles-and-models)), read from that root's thread, not this one, with the user's overrides;
   - `workspaceStrategy: {type: "root"}`, a title and a stable `clientRequestId`;
   - the prompt `/start-development` (Codex: `$start-development`) plus any scope from the focus.

   Confirm with `t3_thread_read` that it runs, and give its `link`.
3. Otherwise tell the user to start root with start-development. Either way, say whether the review changed the prompt.
