---
name: wind-down
description: Inspect root's live session and the project, then write a tailored wind-down message and send it directly to root's thread with T3 Code tools.
disable-model-invocation: true
---

# Write a wind-down message for root

Inspect root's session and the project in /home/fetch/projects/openttd-rust. Send a tailored message to root with `t3_thread_send` in step 5. Stay read-only apart from that send: no commits, pushes, GitHub comments, CI requests, process signals or messages to other agents. Give the user a short note and the message sent.

User input: $ARGUMENTS. It may specify a trigger or action scope, which overrides default action limits. If a PR number or name does not match, name the likely match in the note. Root is `gpt-6.1-sol` xhigh in Codex from `$start-development` (`docs/skills/start-development/SKILL.md`), with up to five subagents and component worktrees and draft PRs targeting `rust-migration`. Read AGENTS.md and docs/roadmap.md first.

Quote the first two blocks of the [rollback and wind-down rules](../../workflow.md#automatic-rollback-and-wind-down) verbatim from current base in step 4. Apply them without adding process rules. Explicit user scope changes the action allowance, not the merge checks or automatic rollback.

## 1. Read root's live session

Identify root and the actions already underway.

- In T3 Code, load tools with ToolSearch: `select:mcp__t3-code__t3_thread_list,mcp__t3-code__t3_thread_read,mcp__t3-code__t3_thread_send`. Call `t3_thread_list` with `includeSubagents: true`. Find the newest top-level development thread by its assignment and roadmap context, not model alone; do not select this wind-down thread. Subagents have root's `parentThreadId`; `running` or a recent `updatedAt` indicates activity. Read root's latest messages with `t3_thread_read`, view `messages`, near the end of `itemCount`.
- Otherwise find the most recently modified `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` whose first real user message after the AGENTS.md preamble is `$start-development`. Subagent sessions sit beside it.
- In Codex records, `type: response_item` and `payload.type: message` with role `user` or `assistant` are conversation; `function_call`, `custom_tool_call` and `function_call_output` are tools and results. Agent messages are encrypted, but `spawn_agent` names and `list_agents` outputs are plain text. An agent is done when `list_agents` shows `completed`; Codex has no close-agent tool.
- Read root's recent status and latest `list_agents`. For each running subagent, skim its session for its current step, dirty files and edit or review underway.

Done when root and each active subagent's current action are identified.

## 2. Survey the project, in parallel

Verify the live state instead of trusting the roadmap or messages.

- Run `git fetch` and `gh run list --branch rust-migration`; identify red and running jobs.
- Run `git worktree list`; check each worktree's dirty state, unpushed commits and merged status, including older leftovers.
- For each open PR, inspect head, checks, latest review report and reviewed commit, current-head `Full validation`, and last state comment.
- Read the roadmap checkpoint and count unintegrated branches. Map running cmake, ninja, cargo, openttd and tools/*.py jobs to worktrees.
- For optional cleanup of verified merged work only, root may run `python3 tools/housekeeping.py`, read its dry run and use `--apply`. List retained worktrees in the checkpoint. Leave cache and log audits and unrelated housekeeping for normal development.

Done when the live work, validation, jobs and eligible cleanup are known.

## 3. Classify everything in flight

Assign each item its next action under the wind-down rules.

- **Merge now**: review and required validation already passed for the exact combination, and strict protection permits it. Do not update base, start CI or fix a failure to qualify it.
- **Finish the current action**: finish and save an edit underway, or record findings of an active review; start no downstream round. Cancel a clean, pushed worker and record its head. If a worker is unresponsive, resend stop or cancel it once its edit is pushed.
- **Hand off**: name the next action for work needing implementation, a new review, fixes, validation or a CI result, not a chain of actions.
- **Recover**: name an identified breaking merge eligible for [automatic rollback](../../workflow.md#automatic-rollback-and-wind-down) and its restoration target. Root verifies and rolls back without asking. Hand off other failures.
- **Leave alone**: paused or older PRs nobody is working on.
- Do not demand an hours-long finish path or weaken validation to fit wind-down.

Done when every item has a classification and, where needed, a next action.

## 4. Write the message

Write one ASCII code block in the user's voice, with three parts.

1. Open with wind down now or when the user's trigger fires, plus explicit scope. Normal work continues until a trigger fires.
2. Copy the rules exactly. After `git fetch`, print them from current base:

   ```sh
   git show origin/rust-migration:docs/workflow.md | perl -00 -ne \
     'if (/^## Automatic rollback and wind-down/) { $f = 1; next } print if $f && $n++ < 2'
   ```

   Check that it printed the rollback and wind-down paragraphs. If the section changed, quote the blocks that now define them. Put them between `--- docs/workflow.md at <short SHA of
   origin/rust-migration>: Automatic rollback and wind-down ---` and `--- end ---`, without edits, summary or additions.
3. Add one or two tailored lines per item. Name PRs, agents and heads; apply the quoted rules without repeating them.
   - State each current action or handoff. Name any component, review, planner or repair round root was about to start as "do not start it; stop it if already started". Keep due speed and planning work in the checkpoint, not as a new resume gate.
   - Name tested combinations eligible to merge now, or say none. Name an eligible rollback target; root retains removed work for a replacement PR on resume. Do not demand a fix PR or CI cycle before stopping.
   - Record changed facts and preserve unfinished work.
   - Request one docs-only roadmap commit updating "Where the fork stands" and replacing the checkpoint with one opening "Paused at the user's request", each remaining draft, head, next action and running jobs or CI. Leave other sections alone; `/steer` prunes the roadmap.
   - Request a final report of integrations and rollbacks, available progress metrics, remaining work and next actions, and CI state including pending or unknown. Do not launch measurement work to fill it. Report unperformed or failed recovery honestly; do not wait for green.

Done when the message quotes current rules and accounts for every item in flight.

## 5. Send it to root

Refresh root's state, send once, and report delivery.

1. Reread root's latest few items immediately before sending. Correct any lines affected by new activity.
2. Send the message without the fence using `t3_thread_send`: root's `threadId`, `mode: "auto"` and a stable `clientRequestId`, such as `wind-down-YYYYMMDD-HHMM`, to prevent duplicate retries. Never use `create_threads` or `t3_thread_launch`. If sending fails or root is ambiguous, say so and print the message for the user to paste. If the user requested review before sending, show the message and wait for approval.
3. Give the user a short note and the sent message in a code block. Include the thread link `[title](t3-thread://v1/<threadId>)`, delivery status, state and check time, and anything they may want to change. Send corrections as a short follow-up with a new `clientRequestId`, not another full message.

Done when the message is delivered and reported, or a failure or requested review is presented to the user.
