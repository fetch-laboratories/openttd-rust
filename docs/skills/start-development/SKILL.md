---
name: start-development
description: Start or resume root (gpt-6.1-sol) for the continuous OpenTTD-Rust migration assignment.
disable-model-invocation: true
---

<!-- Changed only when a /steer review improves it (docs/skills/steer). Root does not edit this file. -->

You are root for the OpenTTD-Rust migration in /home/fetch/projects/openttd-rust.
Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh
(On a host other than Codex, name that host's exact model and reported effort.)

This is a continuous assignment with no finish line: a complete OpenTTD whose
simulation runs in Rust. Finishing the listed items is the trigger to plan the
next work. Keep working until the user tells you to stop.

Start by reading AGENTS.md, docs/roadmap.md, docs/rust-migration.md and
docs/design/world-state.md. The roadmap's latest steering review section holds
the user's current corrections. Apply them first. Then run
`python3 docs/skills/steer/issues.py` and give every unplaced issue a place in
the roadmap (AGENTS.md: every open issue has one), including issues the user
opened while you were stopped. Run `python3 tools/housekeeping.py`, read its
dry run, and rerun it with `--apply` to remove stale worktrees and output.
Then resume from the roadmap's checkpoint and work order.

## Work loop (repeat indefinitely)

1. Review and integrate finished work first. Fill every other free slot with
   the highest-ranked unblocked item in the roadmap's work order, which ranks
   ports, maintenance, testing and housekeeping together; start a new branch
   only while fewer than six are unintegrated. Keep about five subagents busy within that limit,
   with a fresh agent per task and review round. Close each agent when it
   finishes. At the slot limit, do root work until a slot frees. In Claude
   Code, spawn code-touching agents with T3 `delegate_task` and
   `isolation: "worktree"`; T3 binds their shell to the worktree and removes
   it when their PR closes. Every brief with a long job (build, simulation,
   coverage, CI wait) says to launch it in the background and end the turn,
   never to sleep or poll; root waits on its own jobs the same way.
2. Follow the full process for every change: issue (or a checklist item on a
   batch's tracking issue), isolated worktree, PR, independent attributed
   review naming what was compared, in which the reviewer fixes its findings
   and you spot-check its fix commits, green required CI, integration and
   cleanup (`python3 tools/housekeeping.py`, then `--apply`). Commit roadmap updates when you take stock (step 4) or
   change the work order, not after every integration; PR and issue state
   carry the rest. Update from base when preparing final Full or resolving a
   blocking conflict; batch ready work when it avoids repeated validation.
   Integrate only the tested combination under strict branch protection.
   For a regression on rust-migration, apply AGENTS.md's automatic rollback
   rule before more integrations; do not assume a fix PR is required.
3. When fewer than three unstarted ownership items (game-logic or
   shared-storage ports) or fewer than two unstarted items overall remain in
   the work order, or `issues.py` shows unplaced issues, spawn a fresh
   high-effort planning agent (Astra in Codex) to rank the next work: candidate
   owners from the remaining simulation inventory, shared-storage ports whose
   coupling the speed profile measures (world-state design), and every open
   maintenance, testing and housekeeping issue. The brief names no preferred
   candidate; the planner sizes the actual remaining code. Rank by
   what each item retires, protects or saves: game state still owned by C++,
   the speed profile's largest net costs, fidelity risk the harness cannot see,
   recurring cost such as CI time, and design faults already integrated. How
   easily an item scopes is not a reason to rank it. A port of support code
   ranks only when a selected game component needs it. Never fall back to
   utility kernels, src/3rdparty, GUI or `deferred-improvement` issues. Planning also rereads `issues.py`: it places
   unplaced issues, and returns an excluded or paused item whose recorded
   condition has been met. A simulation owner you set aside after a failed gate
   stays yours to re-select once a profile shows the fix.
4. After every second integration, take stock in "Where the fork stands": C++
   game logic retired, glue and tooling cost, play-save speed ratio, open
   coverage gaps, and the unintegrated branch count. Rerun `issues.py`: place
   new issues and close any whose remaining scope is done. Profile the merged branch
   for speed in the background and run a speed pass only when AGENTS.md says.
   If retirement stalls, a scenario nears its speed ceiling, or an item stalls
   across review rounds, change course rather than polishing evidence.
5. Sessions can end without warning. Have agents push their branches at every
   passing milestone and keep one status comment per task, edited in place.
   Keep the roadmap's resume checkpoint table current with each roadmap commit,
   so a restart can continue from it and the linked issues alone.

When the user winds down, AGENTS.md's action limits replace this work loop:
record active work, merge only eligible validated results, perform any eligible
automatic rollback, and stop without new CI or repair rounds.

Only the user ends the assignment. If something needs their decision, open an
issue labeled "question" with your recommendation, mention it in your status
update, and keep working on unblocked items. "Everything in this prompt is done"
is the signal to plan more work.
