# Team

## Roles and models

Each role has a fixed model and effort, in the table below ([ADR 0011](adr/0011-model-and-effort-per-role.md)). Specify model and effort on every spawn. Effort follows the role, not root's own effort. A root launch or restart reuses the previous root's settings as the ADR says.

| Role | Under Claude Code root | Under Codex root |
| --- | --- | --- |
| Implementation and fixes | `claude-opus-5-5`, medium; high for difficult analysis | `gpt-6.1-sol`, high |
| Planning | `claude-fable-5-2`, medium | `gpt-6-astra`, high |
| Independent review | `gpt-6.2-sol`, high | `gpt-6.2-sol`, high |
| Routine runs, measurements and tracker edits | `claude-sonnet-5-5`, medium | `gpt-6.1-terra`, high |
| [Fathom](skills/fathom/SKILL.md) | `claude-opus-5-5`, high | `claude-opus-5-5`, high |

Under a Claude Code root, Opus handles all code-touching work; Sonnet handles only non-code work. Under a Codex root, Sol handles substantive implementation and analysis; Terra handles bounded mechanical work.

## Slots

At most two implementers exist at once ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). An implementer holds its slot from dispatch until its PR merges or root stops it, including while it waits on CI, review or the merge queue, and root gives it no new work meanwhile. A running profile counts as one. Reviews, planning and research do not count, and neither does a fathom agent or the PR it opens ([ADR 0053](adr/0053-a-fathom-takes-no-slot.md)).

## Spawning

- Root starts each planner with this fixed message, and adds only facts the entry and the docs do not hold, never repeating rules from the docs:
  > Plan the roadmap's ready-queue entry <entry name> (<issue URL>) under `docs/planner-brief.md`. You are <name>, on <model> at <effort> effort. Root's thread is <thread ID>.
- Root starts each implementer with this fixed message, and adds only facts the plan and the docs do not hold, such as a base change since the plan was written, never repeating rules from the docs:
  > Implement the roadmap's ready-queue entry <entry name> (<issue URL>) under `docs/implementer-brief.md`. You are <name>, on <model> at <effort> effort. Complete the whole task. Root's thread is <thread ID>.
- Root, a steering review or a navigation review starts a [fathom](skills/fathom/SKILL.md) agent for a case as a thread of its own and does not wait on it: `t3_thread_launch` with the role table's model and effort as `modelSelection`, `workspaceStrategy: {type: "root"}`, a title that names the case, a stable `clientRequestId` and the prompt `/fathom <case issue URL>`.
- Spawn a fresh agent named for each task; its name is its attribution.
- The host does not report a child's model or effort to it, so the fixed message that starts a child names the child's own, and the child signs with them. A planner or implementer starts each reviewer on the review row of the [role table](#roles-and-models), never on its own model, and that reviewer's message names the review row's model and effort. `delegate_task` starts a child on its parent's model unless the call names another.
- The host wakes a parent when a child's turn ends with no job held by `t3_background_wait`, and not while one is held: a child that waits on its jobs as [background jobs](#background-jobs) says sends its parent nothing until it messages. Root writes no summary of a waiting notice it does get.
- An agent's own thread ID is `$T3CODE_THREAD_ID`, or in its thread metadata when that is unset. Children send mid-task messages to the `parentThreadId` in their thread metadata.
- Every agent-to-agent `t3_thread_send` uses `mode: "queue"` unless the sender means to interrupt: a steer arriving during a Claude Code agent's tool call cancels the call. An agent whose call is cancelled as a message arrives reads the message, reruns the call and carries on unless the message says to stop.

## Background jobs

- Claude Code: start the job with Bash `run_in_background`; a T3 child also passes its ID to `t3_background_wait` as `{taskIds: ["<id>"]}`. A background job is stopped after 30 minutes unless the call sets `timeout`: give a build, profile, verify or CI wait `timeout: 7200000`, the two-hour maximum.
- Stop only your own job, by the task ID its start returned or the process ID you started. Never stop one by a name pattern (`pgrep -f`, `pkill`, `killall`): several agents run the same tools on this host, and a pattern stops theirs too.
- Codex, root or child: start the job with `exec_command` and check it once, as your last action before the final message; if it has already exited, read its output now.
- Start each long job as `exec python3 tools/signal-witness.py -- <command>`. If the job exits 143 and the log shows no failure, read the sender record and the run's log or report, then rerun once incrementally. Add each new kill time and sender evidence to #724.

### Missed resumes

A job's end usually resumes its agent, but not always. Root checks no child's jobs itself.

- At each independent wake, a turn root starts on something other than a child's report, root makes one `t3_thread_list` call.
- If a child has sat idle on a named job for over 30 minutes (provisional), root starts the [routine agent](#routine-agent) to check each such job by its ID and resume the child through the host's resume action.

## Delegation

### Review loop

The review loop is in [review](workflow.md#review). Root reads dispositions, not diffs, and answers only a disagreement between implementer and reviewer, a policy question no recorded decision answers, or a scoped review `ci.py reviewer` sent to it: one more, or what it would review recorded as residue.

### Ready queue and pipelining

- Root selects work and writes the roadmap's [ready queue](roadmap.md#ready-queue): the undispatched work orders, the next entry of each slot's chain. Each work order has its own issue, whose body root opens with one line saying what is finished when the work is done. An entry is that issue and the PR or issue it waits on. Size, files and method are the planner's to settle.
  - Work under a tracker that spans many PRs, such as #734, gets a sub-issue of it: `gh api repos/{owner}/{repo}/issues/<tracker>/sub_issues -F sub_issue_id=$(gh api repos/{owner}/{repo}/issues/<new issue> -q .id)`.
- Root refills the queue when no entry could go to the slot that frees next: an entry counts for a slot only if it waits on nothing but that slot's own PR (provisional). The refill is a docs-only commit that edits only that section. While the [merge queue is not empty](workflow.md#change-types) the refill waits as a local commit, and root's status comment on #3 carries the queue's state.
- **Plans.** Each work order gets an implementation plan before it gets an implementer. Once an entry waits on nothing, root starts a [planner](planner-brief.md) for it, whether or not a slot is free. The planner writes the plan into the issue's body, below root's line, and has it [reviewed](planner-brief.md#plan-review) by a reviewer of its own; that reviewer does not review the PR later. Root settles the plan's open questions, then dispatches the entry to the next free slot.
- **File sets.** Before starting, the implementer checks the plan's file set against open PRs' actual edits with `python3 tools/retrospective.py readiness <PR> <open PRs>` and records the result in its status comment; root brokers no file reservation.
  - A #734/#735 pass owns the files it finishes whole. Another stream's PR may add separate test modules or one-line hooks to those files; it names the hook, and a conflict is resolved by base update.
  - Files whose edits merge mechanically are exempt, including `lib.rs` module lists, `migration/*-debt.json`, `migration/harness-inventory.json` and generated registries.

### Routine agent

Root hands fixed procedures that need no judgment to a routine agent with the role table's routine model, and keeps the decisions. One spawn may carry several of them:

- [housekeeping](root-workflow.md#housekeeping);
- the [platform cadence](root-workflow.md#cadences): dispatch, wait in the background and report the result;
- a workflow-changing PR's bootstrap Full steps under [CI administration](root-workflow.md#ci-administration), after root has reviewed the workflow;
- the [missed-resume](#missed-resumes) check.

The agent changes nothing beyond its steps.

## Skills

Only steering and navigation reviews edit the four user-invoked skills, unless the user requests an edit; root follows them and leaves them unedited.
