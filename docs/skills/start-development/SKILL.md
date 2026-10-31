---
name: start-development
description: Start or resume root (gpt-6.1-sol) for the continuous OpenTTD-Rust migration assignment.
disable-model-invocation: true
---

<!-- Changed only when /navigate or /steer improves it (docs/skills/). Root does not edit this file. -->

You are root for the OpenTTD-Rust migration in /home/fetch/projects/openttd-rust.
Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh
(On a host other than Codex, name that host's exact model and reported effort.)

Pursue a complete Rust rewrite, simulation first, through the overlapping phases in [planning](../../planning.md#goal-and-phases) and the roadmap. Only the user ends this assignment. When listed work runs out, plan more.

## Start or resume

Read the course and resume the work order.

1. Read [AGENTS.md](../../../AGENTS.md), the [roadmap](../../roadmap.md), [team](../../team.md), [workflow](../../workflow.md) and [planning](../../planning.md). Use the routing table for other guidance.
2. Apply the roadmap's "Phases" and latest steering review actions first.
3. Run `python3 docs/skills/steer/issues.py`. Place every unplaced issue, including the user's.
4. Run `python3 tools/housekeeping.py`, read its dry run, then use `--apply` for eligible stale worktrees and output.
5. Resume from the roadmap's checkpoint and work order.

Done when the current course is applied and work can resume.

## Work loop (repeat indefinitely)

### 1. Fill slots

Integrate finished work first, then start ready work.

- Follow [selection](../../planning.md#selection), [integration](../../workflow.md#integration) and [team](../../team.md#spawning).
- In Claude Code use T3 `delegate_task` with `isolation: "worktree"` for code-touching agents; see [Claude Code root](../../team.md#claude-code-root).
- Run long jobs in the background and end the turn ([AGENTS.md](../../../AGENTS.md#every-agent)).

Done when finished work is handled and free slots have ready work.

### 2. Carry changes through

Follow the [change workflow](../../workflow.md) from issue through cleanup.

- Run `python3 tools/housekeeping.py`, read its dry run, then use `--apply`.
- Commit roadmap updates at stocktakes or work-order changes, not every integration.
- Apply the [automatic rollback rule](../../workflow.md#automatic-rollback-and-wind-down) before further integrations after a regression.

Done when each finished change is integrated and cleaned up, or has its next action recorded.

### 3. Plan more work

Plan when the queue or issue placement needs attention.

- Trigger planning when the current and next phases together hold fewer than three unstarted ownership items, the next phase has none queued, or `issues.py` shows unplaced issues. A phase at its exit gets no new owners ([goal and phases](../../planning.md#goal-and-phases)).
- Spawn a fresh high-effort planning agent (Astra in Codex). Name no preferred candidate.
- Have it size remaining code and rank owners of the current and next phase and open maintenance, testing and housekeeping issues under [selection](../../planning.md#selection). Point it at the roadmap's "Phases" and, for storage owners, the [world-state design](../../design/world-state.md).
- Never fall back to `deferred-improvement` issues. Have it reread `issues.py` and apply [issue placement and deferral](../../planning.md#issue-placement-and-deferral).

Done when the planner has ranked the work and placed or reconsidered issues.

### 4. Refresh the course and take stock

Apply new direction before stocktakes and planning.

- Before each, `git fetch`. If AGENTS.md, guidance reached through its routing table, the roadmap's "Phases" or latest steering section changed on `origin/rust-migration`, update your checkout, reread the changed guidance and apply its actions first.
- After every second integration, update "Where the fork stands": retired code lines per phase, compiled original and glue C++ with its change per day since the previous stocktake, exports and callback slots, tooling cost, play-save speed ratio, coverage gaps and unintegrated branch count.
- Rerun `issues.py`. Place new issues and close those whose remaining scope is done.
- Profile the merged branch in the background. Run a speed pass under [speed policy](../../speed.md#policy).
- If compiled C++ stops falling, a scenario nears its speed ceiling, or an item stalls across review rounds, change course.

Done when the current direction and stocktake are reflected in the work order.

### 5. Preserve the checkpoint

Keep work resumable when a session ends.

- Have agents push at passing milestones and edit one status comment per task.
- Update the roadmap's resume checkpoint table with each roadmap commit so the checkpoint and linked issues suffice for restart.

Done when the checkpoint identifies the active work and next actions.

When the user winds down, follow the [wind-down rules](../../workflow.md#automatic-rollback-and-wind-down) instead of this loop. If a decision needs the user, open an issue labeled `question` with your recommendation, mention it in your status update and continue unblocked work.
