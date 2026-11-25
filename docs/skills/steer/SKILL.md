---
name: steer
description: Steering review of root's autonomous migration work since the last steering commit; surveys, ranks problems by cost, corrects course, opens fathom cases, and can restart root.
disable-model-invocation: true
---

# Steer the OpenTTD-Rust migration

Review root's autonomous work since the last steering commit. Find what root cannot see, rank it by what it cost, and correct the roadmap's order, issues and PR comments. A problem whose fix needs its cause found goes to [fathom](../fathom/SKILL.md) as a case; code stays with root's PR process. [Navigate](../navigate/SKILL.md) owns the goal and the roadmap's "Phases"; work within that course. Root acts on corrections literally.

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
3. Run `python3 tools/port-metrics.py --whole-fork`; progress is the fall in the fork C++ census ([ADR 0022](../../adr/0022-progress-is-net-cpp-removed.md)), and original C++ retired is the share of it that ports deliver. Re-measure the roadmap's stream inventory rows that steering owns with `python3 tools/code-standards-inventory.py`.
4. Inspect `git worktree list` and disk use. Run `python3 docs/skills/steer/issues.py`: an unplaced issue, or an exclusion with no return condition, is a problem to rank.
5. Post a short status line to the user and continue.
6. Judge each standing correction from the last two reviews and each rule changed outside steering: applied, slipped, reinterpreted or harmful, with its cost in agent hours, CI runs and parked PRs against what it caught, and whether the problem it was written for still occurs. A rule loosened without an ADR is a red flag.
7. Read the fathom tracker, #1067. For each case whose fix has landed, read its number with the command the case gives: tick and close a case whose number held, and send back one whose number reached its failure value, with the new figures.
8. Trace each problem to its rule, roadmap action or user message. A harmful rule resting on an ADR marked `may change: user` goes back to the user with evidence and cost ([ADRs](../../adr/README.md#who-may-change-one)).

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
- **Debt recurrence**: new code lacks the original's types, named constants, comments or argument structs, or a stream inventory rises above its end state. Why review and CI passed it is a case (step 4).

Done when every standing correction and every case with a landed fix has a verdict.

## 3. Audit independently

1. Launch fresh audits together under the host's orchestration rules, on the parent's model family (Opus under Claude Code). They leave root's worktrees, branches and `.local/` output unchanged. Give every agent `AGENTS.md` and the roadmap.
2. Assign audits where the survey found most cost or risk: blockers, process time against port time, review findings against the code, or speed.
3. A fidelity audit compares a merged pass with the original line by line. Assign one only where one of these holds:
   - the review rules, the review model or the kind of work changed since a fidelity audit last measured what review leaves behind: one or two passes merged under the change;
   - a phase's first ports merged: each of its first three (provisional);
   - a behaviour difference reached `rust-migration` since the last steering: the passes that changed the same code.

   Give the agent the worktree, commit, merge base and `.local/reference/openttd`, and have it compare original bodies, including fallback bodies under `#ifndef WITH_RUST`, against [fidelity](../../porting.md#fidelity), [boundary](../../porting.md#boundary) and [code standards](../../porting.md#code-standards). Report what review left behind for each 1,000 changed lines, beside the last measured figure.
4. For speed, compare candidate and reference on play saves and `generate-tgp-256-1` with callgrind instruction counts (`.local/valgrind`), attribute the gap by port and remeasure each speed-parked PR at its head. A wall-clock median comes with the machine-wide compiler, linker and `openttd` processes from `ps -eo pid,pcpu,comm,args` and the 1-minute load average.
5. Require findings tagged REACHABLE-DIVERGENCE, UNREACHABLE, RULE-VIOLATION, COVERAGE-GAP, PERF or HURDLE, with `file:line` in Rust and original. Verify each claim you will act on.

Done when every audit has reported.

## 4. Correct course and open cases

Rank the problems by what each cost since the last steering, in hours, runs or changed lines.

- **Correct what needs no diagnosis.** That is the work order, a wrong figure, an issue's placement, a PR's findings and text to prune. Remove a rule an agent would follow correctly without, or whose wrong action a tool already catches, and relax or remove one whose measured cost exceeds what it caught.
- **Every other problem is a case.** Where the fix would add or rewrite a rule, a brief, a tool or a CI step, write the case ([fathom](../fathom/SKILL.md#1-write-the-case)) and start a fathom agent ([spawning](../../team.md#spawning)) for the one or two that cost most. The rest stay open on the tracker with their figures.
- Put a standing rule once in the doc that owns it ([routing](../../../AGENTS.md#routing)), and a correction that is a real decision in an ADR ([ADRs](../../adr/README.md)).
- Give new work an issue with problem, measurements, scope and acceptance; label it `roadmap` when ordered there. Put PR findings in a PR comment that separates what blocks integration from what can follow. Put new coverage gaps on #156.
- Rewrite "Where the fork stands" and the work order. Replace "Latest steering actions" with this review's current actions and their issues, plus provisional items to judge at the next review; first move each earlier action that still stands into its owning doc or an ADR. Findings go in the commit message and issues, not the roadmap.
- Prune completed rows and superseded text, and move per-PR plans to PRs or issues.
- Commit guidance, ADRs, roadmap and any step 5 edit as one docs-only commit on `rust-migration`, such as `Doc: Steer migration toward <corrections> [skip ci]`, and push once settled.

## 5. Check /start-development

Edit `docs/skills/start-development/SKILL.md` only for an early stop, a misreading or a missing standing step, keeping it open-ended: only the user ends the assignment. Include the edit in the step 4 commit.

## 6. Report and hand off

1. Report root's track, audit findings with numbers, changes with issue, ADR and commit links, each case opened, started or judged with its link, any user ADR returned, and anything only the user can supply.
2. If the focus asks for a restart, wait until `origin/rust-migration` has the commit, then launch one thread with `t3_thread_launch`:
   - `modelSelection`: the previous development root's provider, exact model, options and other launch settings ([team](../../team.md#roles-and-models)), read from that root's thread, not this one, with the user's overrides;
   - `workspaceStrategy: {type: "root"}`, a title and a stable `clientRequestId`;
   - the prompt `/start-development` (Codex: `$start-development`) plus any scope from the focus.

   Confirm with `t3_thread_read` that it runs, and give its `link`.
3. Otherwise tell the user to start root with start-development. Either way, say whether the review changed the prompt.
