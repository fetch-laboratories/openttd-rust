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
"don't merge #244", "stop #220"). User scope overrides the lenient default
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

The default wind-down is lenient. Root starts nothing new. Work already in
flight is finished properly: agents reach their next passing, pushed
milestone, and PRs whose implementation is complete go through their remaining
gates. Root merges what passes. Anything needing another review round, another
fix round or new scope stays a draft with its state recorded. Root then tidies
up.

Stopping work means leaving it exactly as it is: the worktree, its uncommitted
changes and its branch all stay, and a state note goes on the PR or issue.
Removing worktrees and branches is for merged work only.

## 1. Read root's live session

- When root runs in T3 Code, load the T3 tools with ToolSearch
  (`select:mcp__t3-code__t3_thread_list,mcp__t3-code__t3_thread_read,mcp__t3-code__t3_thread_send`).
  `t3_thread_list` with `includeSubagents: true` shows root as the newest
  top-level, unsettled codex `gpt-6.1-sol` thread started by the user. Its
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
  files, and how close it is to a passing, pushed milestone.

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
- `.local/` entries that are deletion candidates. Name an entry for deletion
  only after checking that no tool, workflow or skill reads it, that its PR or
  issue records what it showed, and that no running agent uses it. List the
  ones it must keep (caches, the reference, toolchains, current frozen
  candidates). Leave out anything you could not verify, and mention it in
  your note.

Verify rather than trust the roadmap or root's messages.

## 3. Classify everything in flight

- Finish and merge: the implementation is complete. Name each remaining gate
  (review, `Full validation` at the final head, any other gate AGENTS.md
  defines) as a pass/fail check. A miss whose fix needs no new review under
  AGENTS.md (formatting, CI warnings, platform errors) is fixed and rechecked;
  any other miss stops the PR there.
- Finish to a milestone: an agent mid-task. Name the milestone it reaches and
  pushes. Then it stops and its PR stays a draft.
- Stop: work the user asked to stop. Leave it as it is, as defined above.
- Leave alone: paused or older PRs nobody is working on.

When gates compete for one resource (quiet timing windows, CI capacity), give
their order. If a path could run for hours (a corpus run plus review plus full
CI), keep it in and flag it in your note so the user can cut it short.

## 4. Write the message

Write it in the user's voice, in plain ASCII, inside one code block. Aim for
about 20 lines. It goes into a long thread close to compaction, so every line
is about this session: name the PRs, agents and heads, and choose your own
wording.

It needs to get across:

- The tone: wind down calmly and leave work in flight running. Start new
  things only when the project needs them to be left in a good state. If the
  user gave a trigger, root keeps working normally until then but prefers
  finishing to starting.
- No new components, planners or issues, and the roadmap work order stays as
  it is. Work that falls due during the wind-down (a speed pass, a planning
  trigger) is recorded as due in the checkpoint; it does not become a gate on
  other work at resume.
- The finish condition for each item from step 3.
- Reviews and CI finish under the usual process. Root merges what passes.
  Anything that needs another round stays a draft with its state recorded on
  the PR.
- If root finds the state has moved on from what you describe, it records the
  actual state rather than undoing it.
- The exact cleanup from step 2, and fixing post-merge CI on `rust-migration`
  if it goes red.
- One docs-only roadmap commit, within the `AGENTS.md` roadmap budget, that
  updates "Where the fork stands" and a resume checkpoint opening "Paused at
  the user's request", with each remaining draft, its head and its next step.
- A final report: what was integrated; original C++ retired as a percent of
  all `src/` and of simulation sources (`port-metrics.py --whole-fork`, #226);
  each remaining draft and what blocks it; `rust-migration` CI state; and any
  decision only the user can make (spending, policy, scope). Engineering
  dispositions, such as closing a draft that saved nothing, are root's to make
  and record.

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
