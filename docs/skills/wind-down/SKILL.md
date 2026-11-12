---
name: wind-down
description: Inspect root's live session and the project, then write a tailored wind-down message and send it directly to root's thread with T3 Code tools.
disable-model-invocation: true
---

# Write a wind-down message for root

Inspect root's session and the project in /home/fetch/projects/openttd-rust, then send root a tailored message with `t3_thread_send`. Stay read-only apart from that send: no commits, pushes, GitHub comments, CI requests, process signals or messages to other agents.

User input: $ARGUMENTS. It may give a trigger or an action scope; explicit scope changes the action allowance, not the merge checks or automatic rollback. If a PR number or name does not match, name the likely match in your note. Read `AGENTS.md` and `docs/roadmap.md` first.

Attribution line for the message:
`Agent: /root (<host> wind-down) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## 1. Read root's live session

- In T3 Code, call `t3_thread_list` with `includeSubagents: true`. Root is the newest top-level thread running the start-development assignment, not this thread; its subagents carry root's `parentThreadId`. Read root's latest messages with `t3_thread_read`.
- Outside T3 Code, `python3 docs/skills/steer/transcript.py` lists Claude Code sessions and `--codex` Codex sessions; read the latest root's transcript.
- For each running subagent, find its current step, dirty files and any edit or review underway.

## 2. Survey the project

Verify live state instead of trusting the roadmap or messages.

- `git fetch` and `gh run list --branch rust-migration`: red and running jobs.
- `git worktree list`: each worktree's dirty state, unpushed commits and merged status.
- Each open PR: head, checks, latest review report and the head it reviewed, that head's `native` run ([checks](../../workflow.md#checks)), whether it is in the merge queue, and its status comment. A PR head's `Full validation` status is not a run ([merge queue](../../ci.md#merge-queue)).
- Map running cmake, ninja, cargo, openttd and `tools/*.py` jobs to worktrees.

## 3. Classify everything in flight

Give each item one of: leave queued (already in the merge queue), finish the current action, hand off with its next action, recover (an identified breaking merge eligible for [automatic rollback](../../root-workflow.md#automatic-rollback-and-wind-down), with its restoration target), or leave alone (paused or older PRs nobody is working on). A clean, pushed worker is cancelled and its head recorded.

## 4. Write the message

Write one ASCII code block addressed to root, in three parts.

1. Open with "The user asks you to wind down", now or when the user's trigger fires, plus any explicit scope, and the attribution line.
2. Quote the rules exactly. After `git fetch`, print them from current base:

   ```sh
   git show origin/rust-migration:docs/root-workflow.md | perl -00 -ne \
     'if (/^## Automatic rollback and wind-down/) { $f = 1; next } $f = 0 if /^#/; print if $f'
   ```

   Check that it printed both the rollback and the wind-down rules; if the section changed, quote the blocks that now define them. Put them between `--- docs/root-workflow.md at <short SHA of origin/rust-migration>: Automatic rollback and wind-down ---` and `--- end ---`, without edits or additions.
3. Add one or two lines per item, naming PRs, agents and heads, applying the quoted rules without repeating them:
   - each current action or handoff; anything root was about to start as "do not start it; stop it if already started";
   - the PRs in the merge queue, or none, and any rollback target;
   - one docs-only roadmap commit that updates "Where the fork stands" to current figures, with no list of merges, and replaces the checkpoint with a status table under a line "Paused at the user's request" that names no issue (`issues.py` reads a line containing "paused" as an exclusion). The table gives each remaining draft, its head, next action and running jobs or CI. Other sections stay as they are;
   - a final report of merges and rollbacks, available progress metrics, remaining work and next actions, and CI state including pending or unknown, with no measurement launched to fill it.

## 5. Send it to root

1. Reread root's latest items just before sending and correct lines that new activity affects.
2. If the user asked to review the message first, show it and wait. Otherwise send it without the fence by `t3_thread_send`: root's `threadId`, `mode: "auto"` and a stable `clientRequestId` such as `wind-down-YYYYMMDD-HHMM`. If sending fails or root is ambiguous, say so and print the message for the user to paste.
3. Give the user a short note with the sent message in a code block, the thread link `[title](t3-thread://v1/<threadId>)`, delivery status and check time, and anything they may want to change. Send corrections as a short follow-up with a new `clientRequestId`.
