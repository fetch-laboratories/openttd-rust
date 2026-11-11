# Team

Readers: Root and user-invoked skills that spawn agents.

## Roles and models

- Root orchestrates and delegates heavily, selects components and reads review dispositions. Root may run under Claude Code or Codex.
- For a root launch or restart, explicit user settings take precedence. Otherwise reuse the most recent development root's provider, model and model options, including effort and service tier where applicable (user, 2026-11-10). Identify that root by its assignment, then read its latest settings from host metadata; do not inherit the steering or review session's settings. If the previous settings cannot be recovered or are unavailable, ask for the missing choice instead of substituting a default. This does not authorize a launch without a user request.
- Run at most two implementation agents at once (user, 2026-11-10). On this 4-core host, 1.6 implementers were working at a cap of two and 2.0 at a cap of four (11-08/09); the rest waited on builds. Reviews, planning and research use the remaining slots and do not count; a running profile counts as one implementer. Codex's per-session limit is 12 subagents plus root and cannot be removed. Existing sessions and worktrees may retain a lower configuration; respect the actual host limit. The main checkout's uncommitted `.codex/config.toml` change belongs to the user and remains untouched. Agent slots and the six unmerged component branches in [integration](root-workflow.md#integration) are different limits: reviews, planning and measurements need no new component branch.
- Specify model and effort on every spawn. Assign effort by role, independently of root's effort (user, 2026-11-10).

| Role | Under Claude Code root | Under Codex root |
| --- | --- | --- |
| Implementation and fixes | `claude-opus-5-5`, medium; high for difficult analysis | `gpt-6.1-sol`, high |
| Planning | `claude-fable-5-2`, high | `gpt-6-astra`, high |
| Independent review | `gpt-6.1-sol`, high | `gpt-6.1-sol`, high |
| Routine runs, measurements and tracker edits | `claude-sonnet-5-5`, medium | `gpt-6.1-terra`, high |

Under a Claude Code root, Opus handles all code-touching work; Sonnet handles only non-code work. Under a Codex root, Sol handles substantive implementation and analysis; Terra handles bounded mechanical work.

## Spawning

- Point implementation briefs at the [implementer brief](implementer-brief.md) and the work order, and reviewers at the [review brief](review-brief.md); those agents read further docs only where these point.
- Children send mid-task messages to their own `parentThreadId`, read from their thread metadata; never find root by searching thread titles. A reviewer and its PR's implementer message each other directly, by the thread IDs their briefs give ([review loop](#review-loop)).
- Every agent-to-agent `t3_thread_send`, root's included, uses `mode: "queue"` unless the sender means to interrupt: the default steers a running turn, and a steer arriving during a Claude Code agent's tool call cancels the call with a notice to stop and wait for the user. On 2026-11-10 three reviewer messages within five seconds stopped an implementer mid-fix that way until root restarted it (#860). An agent whose call is cancelled as a message arrives reads the message, reruns the call and carries on unless the message says to stop.
- Spawn a fresh agent named for each task; its name is its attribution. Do not reassign a finished agent to unrelated work.
- Each review gets a fresh reviewer who reads the PR's earlier attributed review reports. A PR's review stages are in the [review loop](#review-loop).
- For an isolated code-changing task, use T3 `delegate_task` with `isolation: "worktree"`; T3 creates and binds the checkout. Follow the host's base-selection rules. The implementer names its branch and opens its PR there. Root makes worktrees by hand under `.local/worktrees` only for its own small PRs and reviews.
- Follow the host's orchestration rules: use native agents for compatible shared-checkout work completed in the same turn; use T3 delegation for background work, another provider or an isolated checkout. T3 creates and binds isolated checkouts. Follow its completion notifications and returned next action; do not poll a running delegate.
- At the host slot limit, root works on its own duties (review dispositions, CI administration, housekeeping, roadmap) until a slot frees. Do not archive threads or edit host databases to free slots.
- A quiet-host hold lasts only through timed games. At an independent wake while holding other agents, inspect the named job of a child already known to be idle; release the hold as soon as the measurement ends, before report preparation. A failed or declined peripheral host action does not cancel unrelated authorized work.
- A long-job brief links the [background rule](../AGENTS.md#every-agent). Root never runs a child's jobs for it. A child names its job ID and uses the host's background-completion mechanism before ending its turn. A child's last action before ending a turn is one check of its own job ([AGENTS.md](../AGENTS.md#every-agent)). A job that ends seconds before the turn ends never wakes the child, so a job already ended is handled at once. Root checks no child's jobs itself: on 11-10/11, job checks and process scans (`/proc`, `tail --pid`) were 568 of the two roots' 2,123 tool calls (steering 34). Missed resumes still happen (four cost about six agent-hours on 11-02). So at an independent wake root makes one `t3_thread_list` call, and if a child has sat idle on a named job for over 30 minutes (provisional), it starts the [routine agent](#routine-agent) to check each such job by its ID and resume the child through the host's supported resume action. Nobody polls a running delegate or duplicates its work. Both providers must establish a supported completion wake before ending with every child idle on a job. For T3 children, follow the delegate's returned next action and completion mechanism; do not add process watchers or poll a running delegate. Native children finish within the parent turn using the host's native wait. For root's own background jobs, Claude Code uses Bash `run_in_background`; Codex uses `exec_command` and the single final check in [AGENTS.md](../AGENTS.md#every-agent). Codex cannot use `t3_background_wait`. If the host provides no supported wake for an idle child's job, preserve its job ID and next action and report that recovery is not guaranteed; do not claim a wake was registered. This applies equally to both providers (user, 2026-11-10). A child that reports a running job without an ID has not established a resumable handoff; correct it immediately. A subagent waiting only on CI or another task reports and stops.
- Start each long job as `exec python3 tools/signal-witness.py -- <command>`. On this Linux host the wrapper records a received SIGTERM's sender and forwards it to the command's process group in the same session, then reaps the child. If the job exits 143 and the log shows no failure, read the sender record and the run's log or report, then rerun once incrementally (#724). Add no waiter sessions, pidfd waits or duplicate runs. Root adds each new kill time and sender evidence to #724.

## Claude Code root

- A user-directed session outside Codex names its exact model and host-reported effort.
- Use the [role table](#roles-and-models). Independent review runs through T3 `delegate_task` because it uses another provider.

## Delegation (user, 2026-11-08)

Under either Claude Code or Codex, these rules move routine work off root, so that root's turns go to selection, policy rulings, briefs and the roadmap (user, 2026-11-10). Where they differ from [spawning](#spawning), they replace it. Implementers queue their own PRs and wait on them in the queue; root keeps [CI administration](root-workflow.md#ci-administration) with the platform cadence, [cleanup](root-workflow.md#cleanup) and the stocktake, and neither batches, merges nor watches PRs (user, 2026-11-10 and 2026-11-11). Root counts merges from the first-parent history of `origin/rust-migration` and learns of a removal from the implementer's one-line report ([integration](root-workflow.md#integration)). Root acts on messages addressed to it and on failures; a child's routine turn end needs only its last line read. The implementer reads its own failed CI and queue runs. It checks its file set against open PRs with `python3 tools/retrospective.py readiness <PR> <open PRs>` and records the result in its status comment; root brokers no file reservation and reads no source for one. With PR watching and job scans, this routine work took about 47% of the two roots' tool calls on 11-10/11 (steering 34).

### Review loop

- Reviewers report findings and do not commit fixes. The PR's implementer fixes each finding or coherent group in its own commit, naming the finding and ending with `Found-by: review`.
- A PR has at most two review stages (steering 34). Root added a third on 11-10; in the ten hours that followed, its 11 acceptance sessions found no defect, while 16 re-checks found one.
  - The **source review**: one reviewer per function group where the PR's size calls for it.
  - One **re-check** of the fixing commits, only when a fix changed behaviour, evidence or a body's operations. It covers all fixing commits in one task and writes the PR's final disposition. Fixes limited to names, comments, types or argument structs with unchanged operations need none; the implementer says so in the status comment.
- An acceptance covers the source at the reviewed head. The implementer waits on its own `native` run in the background ([checks](workflow.md#checks)), records the passing run at that head in the status comment, then queues the PR and waits on the queue itself ([integration](root-workflow.md#integration)). No separate reviewer is started to re-verify a CI run. The queue's Full repeats the tier on the merged tree.
- The implementer starts its own reviewers and any re-check with `delegate_task` (user, 2026-11-11), using the role table's review model and the [review brief](review-brief.md)'s fixed spawn message. That message names the PR, the reviewer, its review group when the PR body splits the PR, and the implementer's thread; it says nothing about what to look for. Root starts no reviewers. On 11-10/11 it started 62.
- Reviewers rule on policy questions themselves, from the decisions recorded on the stream issues, and state each ruling in the report. Only a question no recorded decision answers goes to root, through the implementer.
- Reviewer and implementer message each other directly, and root relays nothing (user, 2026-11-10). On 11-09, relays were 88 of root's 164 outgoing actions.
  - The reviewer posts its report, then tells the implementer in one line.
  - The implementer fixes the findings. If a re-check is due, it starts one as a new delegate, as the T3 host requires, naming the fixing commits. When no re-check is due, or the re-check accepts, it queues the PR ([integration](root-workflow.md#integration)).
- The implementer also starts a fresh reviewer after a substantive rewrite or a base update that needs one ([integration](root-workflow.md#integration)). It asks root only when it and a reviewer disagree, or a reviewer raises a policy question no recorded decision answers. Root reads dispositions, not diffs.

### Ready queue and pipelining

- The roadmap's [ready queue](roadmap.md#ready-queue) holds two or three undispatched work orders, each ready once the next merge lands. Each gives its scope, its size, its file set checked against open PRs' actual edits, what it waits on, and the constraints its implementer needs. Fan-in and held-caller analysis goes on the stream issue. PR state lives on the PR, not in the queue: copied there, it goes stale within hours (steering 34).
- A planner refreshes the queue when no entry is ready for the next slot to free, and writes up to three entries at once (provisional; seven refreshes in the Codex root's ten hours cost about 610,000 uncached tokens). It edits only that roadmap section, in docs-only commits; these are the one exception to roadmap commits only at stocktakes, course changes and wind-down. An entry becomes selected when root dispatches it, and then leaves the queue: its state lives on its PR, not in the roadmap.
- Root dispatches the top ready entry as the planner wrote it, scope and file-set check included. It re-derives an entry only when a merge or open PR has changed the entry's files since the planner wrote it, or the implementer disputes it. On 11-10/11 the Codex root made 371 docs, issue and git reads, mostly re-deriving entries at xhigh (steering 34).
- An implementer whose PR waits on review or the merge queue takes the next ready entry on a new branch, within the six-branch limit. Findings on its earlier PR come first. It keeps its agent path; each PR's title names the work. The two implementer slots therefore count agents that stay busy, not PRs.

### Routine agent

Root hands fixed procedures that need no judgment to a routine agent with the role table's routine model, and keeps the decisions (steering 34: on 11-10/11 these took about 81 root tool calls at xhigh). One spawn may carry several of them:

- housekeeping: `python3 tools/housekeeping.py`, its dry run and `--apply` under [housekeeping](root-workflow.md#housekeeping), at session start and with each platform cadence;
- the platform cadence after every fifth merge: dispatch, wait in the background and report the result. Root decides whether a red result stops merges;
- a workflow-changing PR's bootstrap Full steps under [CI administration](root-workflow.md#ci-administration), after root has reviewed the workflow. Root decides whether the result lets the PR queue;
- the missed-resume check in [spawning](#spawning).

The agent changes nothing beyond its steps and reports each outcome in its last line.

## Provider handoff

For unattended handoff, establish an independently executable takeover before the source provider exhausts its allowance. A scheduled prompt is not a fallback if it needs that provider to launch its replacement. Verify the host's actual execution provider; a model name in a prompt is insufficient. If the host cannot provide independent takeover, record the limitation and preserve a checkpoint before stopping. This rule creates no schedule.

## Skills

`docs/skills/` holds four user-invoked skills: `/navigate` (macro course review of the goal and phases, before `/steer`), `/steer` (steering review), `/start-development` (root's restart prompt), and `/wind-down` (the user's message pausing root). Two automatically discoverable skills support selected work: `brief-storage-owner` prepares a pool/map ownership brief or checkpoint handoff, and `review-evidence-contract` examines production-path evidence for a port claim or changed fixture/oracle. Their descriptions define when they apply; they add no universal review gate. Steering reviews put corrections in the roadmap and edit `/start-development` only to improve it. Only steering and navigation reviews edit the four orchestration skills unless the user requests an edit; root otherwise follows them and leaves them unedited.
