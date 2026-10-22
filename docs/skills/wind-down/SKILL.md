---
name: wind-down
description: Inspect root's live session and the project, then write a tailored wind-down message and send it directly to root's thread with T3 Code tools.
disable-model-invocation: true
---

# Write a wind-down message for root

Root is developing OpenTTD-Rust in /home/fetch/projects/openttd-rust. Inspect
the project and root's live session, then write a message tailored to what is
in flight right now, then send it to root's thread yourself with the T3 Code
`t3_thread_send` tool (step 5).

Stay read-only apart from that one send. Make no commits, pushes, GitHub
comments, CI requests or process signals, and send no messages to other agents.
Your output is the message, sent to root, plus a short note to the user that
includes the text you sent.

User input, if any: $ARGUMENTS
This may be a trigger ("at 50% weekly usage") or scope ("only finish #218",
"don't merge #244", "stop #220"). User scope overrides the default action limits
below. If a PR number or name doesn't match anything, name the one you think
they meant in your note.

## Background

Root is gpt-6.1-sol at xhigh effort, running in Codex from `$start-development`
(docs/skills/start-development/SKILL.md). It orchestrates up to five
subagents. Each component has its own worktree and a draft PR that targets
`rust-migration`. Read AGENTS.md and docs/roadmap.md first. Root already
follows AGENTS.md, so the message says only what to finish and what to leave.
It does not restate process steps (base updates, count runs, records, review
rounds) or add any that AGENTS.md does not require: root reads a restated step
as a new rule.

The default is the action-based wind-down in AGENTS.md. There is no time
estimate or deadline. Finish only the edit or review already underway; save
unfinished implementation and record review findings without a new fix round.
Start no new work, reviews, builds, simulations, CI requests or retries, and
never wait on CI. Merge only already-reviewed combinations that already passed
Full and still meet strict protection. Anything needing a base update stays
unmerged for resume. Apply an eligible automatic rollback without asking,
record the result, and stop; no fix PR or post-reset CI wait is required.

Include these action limits in the message root receives. They replace the
normal development loop, including the reviewer's usual fix-and-recheck loop.
Do not turn an unfinished item into a requirement to reach a passing milestone.
Pending CI and later notifications are recorded for resume, not reasons to
restart work. A user's explicit alternative, such as "one CI run, no repairs",
changes only that action allowance, not the merge checks or automatic rollback.

Stopping preserves worktrees, uncommitted changes and branches for resume.
Remove only verified merged work; do not close unfinished PRs just to stop.

## 1. Read root's live session

- When root runs in T3 Code, load the T3 tools with ToolSearch
  (`select:mcp__t3-code__t3_thread_list,mcp__t3-code__t3_thread_read,mcp__t3-code__t3_thread_send`).
  `t3_thread_list` with `includeSubagents: true` shows root as the newest
  top-level development thread identified by its assignment and roadmap
  context; do not select by model alone or mistake this wind-down thread for
  root. Its
  subagents are the threads whose `parentThreadId` is root's id; `running`
  status, or a recent `updatedAt`, marks the active ones. Read root's latest
  messages with `t3_thread_read` (view `messages`, reading from near the end
  of `itemCount`).
- Otherwise, root's thread is the most recently modified
  `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` whose first real user message
  (after the AGENTS.md preamble) is `$start-development`. Subagent sessions sit
  beside it.
- Records with `type: response_item` and `payload.type: message` (role `user`
  or `assistant`) are the conversation. `function_call` and
  `custom_tool_call` records are tool calls, and `function_call_output`
  records are their results. Messages between agents are encrypted.
  `spawn_agent` task names and `list_agents` outputs are plain text. Codex has
  no close-agent tool: an agent is done when `list_agents` shows `completed`.
- Read root's recent status messages and its latest `list_agents` output. For
  each running subagent, skim its session for its current step, its dirty
  files, and the edit or review already underway.

## 2. Survey the project, in parallel

- `git fetch`; `gh run list --branch rust-migration`: anything red or running.
- `git worktree list`. For each worktree: dirty state, unpushed commits, and
  whether its branch already merged (including leftovers from earlier
  sessions).
- For each open PR: head, checks, the latest review report and the commit it
  reviewed, `Full validation` at the current head, and the last state comment.
- The roadmap's resume checkpoint and the count of unintegrated branches.
- Running local jobs (cmake, ninja, cargo, openttd, tools/*.py), mapped to
  their worktrees.
- Cleanup only for work already verified as merged. Leave cache/log audits and
  unrelated housekeeping for normal development.

Verify rather than trust the roadmap or root's messages.

## 3. Classify everything in flight

- Merge now: review and required validation already passed for the combination
  that would be merged, and strict protection allows it. Do not update its base,
  start CI, or fix a failure to make it eligible during wind-down.
- Finish the current action: complete the edit already underway and save it,
  or finish the active review with its findings recorded. No downstream round.
- Hand off: anything needing implementation, a new review, fixes, validation,
  or a CI result. Name the next action, not a chain to finish before stopping.
- Recover: an identified breaking merge eligible for AGENTS.md's automatic
  rollback. Name it and the restoration target; root verifies current state
  and performs the rollback without asking. Other failures are handed off.
- Leave alone: paused or older PRs nobody is working on.

Do not preserve an hours-long finish path and ask the user to cut it short.
Do not weaken validation to fit the wind-down: leave the change unmerged.

## 4. Write the message

Write it in the user's voice, in plain ASCII, inside one code block. Aim for
about 20 lines. It goes into a long thread close to compaction, so every line
is about this session: name the PRs, agents and heads, and choose your own
wording.

It needs to get across:

- The action limits above, plus any explicit user override. If the user gave a
  trigger, normal work continues until it; the limits apply when it fires.
- The current action or handoff for each item from step 3. Start no new
  components, planners or repair rounds. Due speed/planning work stays due in
  the checkpoint, not a new gate on resume.
- Merge only eligible tested combinations; do not wait on pending checks.
- Perform an eligible automatic rollback without asking and report afterwards;
  retain the removed work for a replacement PR on resume. Never demand that
  root fix a red tip through another PR/CI cycle before stopping.
- Record changed facts if root's state has moved on. Preserve unfinished work.
- One docs-only roadmap commit within its budget, updating "Where the fork
  stands" and a checkpoint opening "Paused at the user's request", with each
  remaining draft, its head, next action and any running jobs or CI.
- A final report: integrations and rollbacks, current progress metrics if
  available, remaining work and next actions, and CI state (including pending
  or unknown). Do not launch measurement work to fill the report. Report an
  unperformed or failed recovery honestly; do not wait for a green report.

## 5. Send it to root

Send the message text, without the code fence, with `t3_thread_send`: root's
`threadId`, `mode: "auto"` (starts root if it is idle, steers it if it is
mid-turn) and a stable `clientRequestId` such as `wind-down-YYYYMMDD-HHMM`, so
a retry does not deliver it twice. Never use `create_threads` or
`t3_thread_launch`; the message goes to root's existing thread. If the send
fails, or root's thread is ambiguous, say so and print the message so the user
can paste it. If the user's input asks for a review before sending, show the
message and wait for their approval.

Then give the user at most 10 lines, followed by the sent message in a code
block:

- that it was sent (thread link `[title](t3-thread://v1/<threadId>)` and the
  delivery status);
- the state you found, and the time you checked it;
- anything they may want to change. Send corrections to root as a short
  follow-up with a new `clientRequestId`, not as a second full message.
