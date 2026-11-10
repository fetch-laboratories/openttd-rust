---
name: start-development
description: Start or resume root for the continuous OpenTTD-Rust migration assignment.
disable-model-invocation: true
---

<!-- Changed only when /navigate or /steer improves it (docs/skills/). Root does not edit this file. -->

You are root for the OpenTTD-Rust migration in /home/fetch/projects/openttd-rust.
Agent: /root | Model: <this host's exact model> | Reasoning effort: <its reported effort>
Root runs under Claude Code; Codex runs independent review only ([team](../../team.md#roles-and-models)).

Pursue a complete Rust rewrite through the stages and phases in [planning](../../planning.md#goal-and-phases), in its execution order, as the roadmap's "Phases" sizes them. Only the user ends this assignment. When listed work runs out, plan more.

## Start or resume

Read the course and resume the work order.

1. Read [AGENTS.md](../../../AGENTS.md), the [roadmap](../../roadmap.md), [team](../../team.md), [workflow](../../workflow.md) and [planning](../../planning.md). Use the routing table for other guidance.
2. Apply the roadmap's "Phases" and latest steering review actions first.
3. Run `python3 docs/skills/steer/issues.py`. Place every unplaced issue, including the user's.
4. Run `python3 tools/housekeeping.py`, read its dry run, then use `--apply` for eligible stale worktrees and output.
5. Resume from the open PRs (`gh pr list`) and their status comments, then from the roadmap's work order and checkpoint. The checkpoint is written at wind-down; where GitHub is newer, GitHub is right.

Done when the current course is applied and work can resume.

## Work loop (repeat indefinitely)

### 1. Fill slots

Integrate finished work first, then start ready work.

- Follow [selection](../../planning.md#selection), [integration](../../workflow.md#integration) and [team](../../team.md#spawning).
- Use [team's host orchestration rules](../../team.md#spawning) for native or T3 agents and isolated checkouts.
- Run long jobs in the background and end the turn ([AGENTS.md](../../../AGENTS.md#every-agent)).
- At each independent wake, handle completed child results and recover missed resumes of children already known to be idle on named jobs ([team](../../team.md#spawning)). If other agents are held for a quiet measurement, resolve that job and release the hold before waiting for its report. A peripheral host-action failure does not end the assignment.

Done when finished work is handled and free slots have ready work.

### 2. Carry changes through

Follow the [change workflow](../../workflow.md) from issue through cleanup.

- Run `python3 tools/housekeeping.py`, read its dry run, then use `--apply`.
- Commit roadmap updates at stocktakes, course changes and wind-down. Record a selection or integration on its issue or PR; alone it is not a work-order change. Each roadmap push moves base under PRs in final validation.
- Apply the [automatic rollback rule](../../workflow.md#automatic-rollback-and-wind-down) before further integrations after a regression.

Done when each finished change is integrated and cleaned up, or has its next action recorded.

### 3. Plan more work

Plan when the queue or issue placement needs attention.

- During the cleanup stage, plan when its stream issues run out of ready batches, and plan phase 3 in full once every remaining cleanup item is dispatched or in the ready queue ([goal and phases](../../planning.md#goal-and-phases)). Otherwise trigger planning at a phase exit, when the current and next phases together hold fewer than three ready unstarted ownership items (not waiting on a draft or integration), the next phase has no ready owner, or `issues.py` shows unplaced issues. At exit, account for every remaining body and candidate C++ dependency and plan the newly adjacent phase. A completed phase gets no new owners ([goal and phases](../../planning.md#goal-and-phases)).
- Spawn a fresh high-effort planning agent (Astra in Codex). Name no preferred candidate.
- Have it size remaining code and rank owners of the current and next phase and open maintenance, testing and housekeeping issues under [selection](../../planning.md#selection). Point it at the roadmap's "Phases" and, for storage owners, the [world-state design](../../design/world-state.md).
- Never fall back to `deferred-improvement` issues. Have it reread `issues.py` and apply [issue placement and deferral](../../planning.md#issue-placement-and-deferral).

Done when the planner has ranked the work and placed or reconsidered issues.

### 4. Refresh the course and take stock

Apply new direction before stocktakes and planning.

- Before each, `git fetch`. If AGENTS.md, guidance reached through its routing table, the roadmap's "Phases" or latest steering section changed on `origin/rust-migration`, update your checkout, reread the changed guidance and apply its actions first.
- After every twelfth integration (user, 2026-11-08), update "Where the fork stands": retired code lines per phase, fork C++ (original, glue, generated, and C++ under `tools/` and `src/tests`) with its change per day since the previous stocktake, exports and callback slots, rebound-callee, shared-accessor and orphan-export baseline sizes, tooling cost, play-save speed ratio, coverage gaps and unintegrated branch count. Also update the roadmap's stream inventory rows that root counts, during the cleanup stage and after it ([selection](../../planning.md#selection)).
- Rerun `issues.py`. Place new issues and close those whose remaining scope is done.
- Profile the merged branch in the background. Run a speed pass under [speed policy](../../speed.md#policy).
- Change course when fork C++ stops falling (during the cleanup stage: when a stream with ready work shows no inventory fall between two stocktakes; after it: when a stream inventory rises above its end state), a scenario nears its speed ceiling, or an item stalls across review rounds.

Done when the current direction and stocktake are reflected in the work order.

### 5. Preserve the checkpoint

Keep work resumable when a session ends.

- Have agents push at passing milestones and edit one status comment per task.
- Leave the roadmap's resume checkpoint alone between wind-downs (user, 2026-11-10). GitHub records each merge and open PR, and every roadmap push moves base under PRs in final validation.

Done when each open PR's status comment gives its state and next action.

When the user winds down, follow the [wind-down rules](../../workflow.md#automatic-rollback-and-wind-down) instead of this loop. If a decision needs the user, open an issue labeled `question` with your recommendation, mention it in your status update and continue unblocked work.
