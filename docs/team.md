# Team

Readers: root, the planner and the routine agent, and the user-invoked skills that spawn agents; every agent for the [background-job](#background-jobs) mechanics of its host.

## Roles and models

- Root is the top agent: it orchestrates, delegates heavily, chooses work and settles questions ([ADR 0012](adr/0012-root-decides-implementers-review-and-merge.md)). Root may run under Claude Code or Codex.
- Each role has a fixed model and effort, in the table below ([ADR 0011](adr/0011-model-and-effort-per-role.md)). Specify model and effort on every spawn. Effort follows the role, not root's own effort.
- For a root launch or restart, explicit user settings take precedence. Otherwise reuse the most recent development root's provider, model and model options, including effort and service tier where applicable. Identify that root by its assignment, then read its latest settings from host metadata; do not inherit the steering or review session's settings. If the previous settings cannot be recovered or are unavailable, ask the user for the missing choice instead of substituting a default. This does not authorize a launch without a user request.

| Role | Under Claude Code root | Under Codex root |
| --- | --- | --- |
| Implementation and fixes | `claude-opus-5-5`, medium; high for difficult analysis | `gpt-6.1-sol`, high |
| Planning | `claude-fable-5-2`, medium | `gpt-6-astra`, high |
| Independent review | `gpt-6.1-sol`, high | `gpt-6.1-sol`, high |
| Routine runs, measurements and tracker edits | `claude-sonnet-5-5`, medium | `gpt-6.1-terra`, high |

Under a Claude Code root, Opus handles all code-touching work; Sonnet handles only non-code work. Under a Codex root, Sol handles substantive implementation and analysis; Terra handles bounded mechanical work.

## Claude Code root

- A user-directed session outside Codex names its exact model and host-reported effort in its attribution.
- Use the [role table](#roles-and-models). Independent review runs through T3 `delegate_task` because it uses another provider.

## Slots

- At most two implementers work at once ([ADR 0010](adr/0010-two-implementers.md)). An implementer whose PRs all wait on CI, review or the merge queue does not count. A running profile counts as one implementer. Reviews, planning and research use the remaining slots and do not count.
- The host's agent limit also applies. For Codex, `.codex/config.toml` sets it (`max_concurrent_threads_per_session`: eight subagents plus root). Existing sessions may keep a lower limit their host supplied; respect the actual host limit.
- Agent slots and the six unmerged component branches in [integration](root-workflow.md#integration) are different limits.
- At the host slot limit, root works on its own duties (review dispositions, CI administration, housekeeping, roadmap) until a slot frees. Do not archive threads or edit host databases to free slots.

## Spawning

- Point implementation briefs at the [implementer brief](implementer-brief.md) and the work order, and reviewers at the [review brief](review-brief.md); those agents read further docs only where these point.
- Every message root sends an implementer, the spawn brief included, contains the phrase "complete the whole task".
- Spawn a fresh agent named for each task; its name is its attribution. Do not reassign a finished agent to unrelated work.
- For an isolated code-changing task, use T3 `delegate_task` with `isolation: "worktree"`; T3 creates and binds the checkout. Follow the host's base-selection rules. The implementer names its branch and opens its PR there. Root makes worktrees by hand under `.local/worktrees` only for its own small PRs.
- Follow the host's orchestration rules: use native agents for compatible shared-checkout work completed in the same turn; use T3 delegation for background work, another provider or an isolated checkout. Follow its completion notifications and returned next action; do not poll a running delegate.
- Children send mid-task messages to their own `parentThreadId`, read from their thread metadata; never find root by searching thread titles. A reviewer and its PR's implementer message each other directly ([review](workflow.md#review)).
- Every agent-to-agent `t3_thread_send`, root's included, uses `mode: "queue"` unless the sender means to interrupt: the default steers a running turn, and a steer arriving during a Claude Code agent's tool call cancels the call with a notice to stop and wait for the user. An agent whose call is cancelled as a message arrives reads the message, reruns the call and carries on unless the message says to stop.
- The host hands root a child's whole last message at every turn end, and root acts on its last line ([AGENTS.md](../AGENTS.md#every-agent)). So a brief never asks for a report at every turn end, and root writes no summary of a waiting notice.
- A quiet-host hold ([speed passes](speed.md#speed-passes)) ends when the timed games end, before report preparation. A failed or declined peripheral host action does not cancel unrelated authorized work.

## Background jobs

The rule every agent follows is in [AGENTS.md](../AGENTS.md#every-agent). A long-job brief links it. These are the host mechanics.

| Host | Start the job | Make sure its end wakes you |
| --- | --- | --- |
| Claude Code, root or child | Bash `run_in_background` | The host notifies you; a T3 child also passes the task ID to `t3_background_wait` |
| Codex, root or child | `exec_command` | Check the job once, as your last action before the final message; if it has already exited, read its output now. Codex cannot use `t3_background_wait`. |

- Start each long job as `exec python3 tools/signal-witness.py -- <command>`. On this Linux host the wrapper records a received SIGTERM's sender, forwards it to the command's process group in the same session, then reaps the child. If the job exits 143 and the log shows no failure, read the sender record and the run's log or report, then rerun once incrementally. Add no waiter sessions, pidfd waits or duplicate runs. Root adds each new kill time and sender evidence to #724.
- A child's last action before ending a turn on a job is one check of that job. A job that ends seconds before the turn ends never wakes its agent, so a job already ended is handled at once.
- Native children finish within the parent turn using the host's native wait. For T3 children, follow the delegate's returned next action and completion mechanism; add no process watchers.
- Root never runs a child's jobs for it. A child that reports a running job without an ID has not established a resumable handoff; correct it immediately. A subagent waiting only on CI or another task reports and stops.
- Both providers must establish a supported completion wake before ending with every child idle on a job. If the host provides no supported wake for an idle child's job, preserve its job ID and next action and report that recovery is not guaranteed; do not claim a wake was registered.

### Missed resumes

A job's end usually resumes its agent, but not always. Root checks no child's jobs itself: no job checks and no process scans (`/proc`, `tail --pid`).

- At each **independent wake**, a turn root starts on something other than a child's report (a user message, its own job's end or a schedule), root makes one `t3_thread_list` call.
- If a child has sat idle on a named job for over 30 minutes (provisional), root starts the [routine agent](#routine-agent) to check each such job by its ID and resume the child through the host's supported resume action.
- Nobody polls a running delegate or duplicates its work.

## Delegation

Root's turns go to choosing work, settling questions, briefs and the roadmap ([ADR 0012](adr/0012-root-decides-implementers-review-and-merge.md)). These rules apply under either provider; where they differ from [spawning](#spawning), they replace it.

- Implementers take their own PRs through review and the queue ([PR lifecycle](workflow.md#pr-lifecycle)). Root keeps [CI administration](root-workflow.md#ci-administration), the [cadences](root-workflow.md#cadences) and [cleanup](root-workflow.md#cleanup), and neither batches, merges nor watches PRs.
- Root counts merges from the first-parent history of `origin/rust-migration` and learns of a queue removal from the implementer's one-line report.
- Root acts on messages addressed to it and on failures. A child's routine turn end, one that only waits on a job, needs only its last line read.
- The implementer reads its own failed CI and queue runs.

### Review loop

The review loop is in the [PR lifecycle](workflow.md#review). Root starts no reviewers and reads dispositions, not diffs; it answers only a disagreement between implementer and reviewer, or a policy question no recorded decision answers.

### Ready queue and pipelining

- The roadmap's [ready queue](roadmap.md#ready-queue) holds two or three undispatched work orders, each ready once the next merge lands. Each gives its scope, its size, its file set checked against open PRs' actual edits, what it waits on, and the constraints its implementer needs. Fan-in (which other files call into the entry's files) and held-caller analysis (which callers keep an export alive) go on the stream issue. PR state lives on the PR, not in the queue: copied there, it goes stale within hours.
- A planner refreshes the queue when no entry is ready for the next slot to free, and writes up to three entries at once (provisional, to bound each refresh's token cost). It edits only that roadmap section, in docs-only commits; these are the one exception to [roadmap commit timing](workflow.md#records). An entry becomes selected when root dispatches it, and then leaves the queue.
- Root dispatches the top ready entry as the planner wrote it, scope and file-set check included. It re-derives an entry only when a merge or open PR has changed the entry's files since the planner wrote it, or the implementer disputes it.
- An implementer whose PRs all wait holds no slot ([slots](#slots)). Root spawns a fresh implementer for the next ready entry, within the six-branch limit, and gives the waiting agent no new work order. The waiting agent finishes its own PRs: findings, base updates, queueing and the merge.
- **File sets.** Before starting, the implementer checks its file set against open PRs' actual edits with `python3 tools/retrospective.py readiness <PR> <open PRs>` and records the result in its status comment; root brokers no file reservation and reads no source for one.
  - A #734/#735 pass owns the files it finishes whole. Another stream's PR may add separate test modules or one-line hooks to those files; it names the hook, and a conflict is resolved by base update.
  - Files whose edits merge mechanically are exempt, including `lib.rs` module lists, `migration/*-debt.json`, `migration/harness-inventory.json` and generated registries; resolve those edits when updating from base.

### Routine agent

Root hands fixed procedures that need no judgment to a routine agent with the role table's routine model, and keeps the decisions. One spawn may carry several of them:

- [housekeeping](root-workflow.md#housekeeping), on its [cadence](root-workflow.md#cadences);
- the [platform cadence](root-workflow.md#cadences): dispatch, wait in the background and report the result. Root decides whether a red result stops merges;
- a workflow-changing PR's bootstrap Full steps under [CI administration](root-workflow.md#ci-administration), after root has reviewed the workflow. Root decides whether the result lets the PR queue;
- the [missed-resume](#missed-resumes) check.

The agent changes nothing beyond its steps. A turn that ends to wait on a job ends with one line; the last turn reports each outcome.

## Provider handoff

For unattended handoff, establish an independently executable takeover before the source provider exhausts its allowance. A scheduled prompt is not a fallback if it needs that provider to launch its replacement. Verify the host's actual execution provider; a model name in a prompt is insufficient. If the host cannot provide independent takeover, record the limitation and preserve a checkpoint before stopping. This rule creates no schedule.

## Skills

- `docs/skills/` holds four user-invoked skills: `/navigate` (macro course review of the goal and phases, before `/steer`), `/steer` (steering review), `/start-development` (root's restart prompt) and `/wind-down` (the user's message pausing root).
- Two automatically discoverable skills support selected work: `brief-storage-owner` prepares a pool/map ownership brief or checkpoint handoff, and `review-evidence-contract` examines production-path evidence for a port claim or changed fixture/oracle. Their descriptions define when they apply; they add no universal review gate.
- Steering reviews put corrections in the roadmap and edit `/start-development` only to improve it. Only steering and navigation reviews edit the four orchestration skills unless the user requests an edit; root otherwise follows them and leaves them unedited.
