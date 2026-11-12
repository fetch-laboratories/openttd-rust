# Team

## Roles and models

Each role has a fixed model and effort, in the table below ([ADR 0011](adr/0011-model-and-effort-per-role.md)). Specify model and effort on every spawn. Effort follows the role, not root's own effort. A root launch or restart reuses the previous root's settings as the ADR says.

| Role | Under Claude Code root | Under Codex root |
| --- | --- | --- |
| Implementation and fixes | `claude-opus-5-5`, medium; high for difficult analysis | `gpt-6.1-sol`, high |
| Planning | `claude-fable-5-2`, medium | `gpt-6-astra`, high |
| Independent review | `gpt-6.1-sol`, high | `gpt-6.1-sol`, high |
| Routine runs, measurements and tracker edits | `claude-sonnet-5-5`, medium | `gpt-6.1-terra`, high |

Under a Claude Code root, Opus handles all code-touching work; Sonnet handles only non-code work. Under a Codex root, Sol handles substantive implementation and analysis; Terra handles bounded mechanical work.

## Slots

At most two implementers work at once ([ADR 0010](adr/0010-two-implementers.md)). An implementer whose PRs all wait on CI, review or the merge queue does not count; a running profile counts as one. Reviews, planning and research do not count.

## Spawning

- Root starts each planner with this fixed message:
  > Plan the roadmap's ready-queue entry <entry name> (<issue URL>) under `docs/planner-brief.md`. You are <name>. Root's thread is <thread ID>.
- Root starts each implementer with this fixed message, and adds only facts the plan and the docs do not hold, such as a base change since the plan was written:
  > Implement the roadmap's ready-queue entry <entry name> (<issue URL>) under `docs/implementer-brief.md`. You are <name>. Complete the whole task. Root's thread is <thread ID>.
- Spawn a fresh agent named for each task; its name is its attribution.
- Root writes no summary of a child's waiting notice.
- An agent's own thread ID is `$T3CODE_THREAD_ID`, or in its thread metadata when that is unset. Children send mid-task messages to the `parentThreadId` in their thread metadata.
- Every agent-to-agent `t3_thread_send` uses `mode: "queue"` unless the sender means to interrupt: a steer arriving during a Claude Code agent's tool call cancels the call. An agent whose call is cancelled as a message arrives reads the message, reruns the call and carries on unless the message says to stop.

## Background jobs

- Claude Code: start the job with Bash `run_in_background`; a T3 child also passes its ID to `t3_background_wait` as `{taskIds: ["<id>"]}`.
- Codex, root or child: start the job with `exec_command` and check it once, as your last action before the final message; if it has already exited, read its output now.
- Start each long job as `exec python3 tools/signal-witness.py -- <command>`. If the job exits 143 and the log shows no failure, read the sender record and the run's log or report, then rerun once incrementally. Add each new kill time and sender evidence to #724.

### Missed resumes

A job's end usually resumes its agent, but not always. Root checks no child's jobs itself.

- At each independent wake, a turn root starts on something other than a child's report, root makes one `t3_thread_list` call.
- If a child has sat idle on a named job for over 30 minutes (provisional), root starts the [routine agent](#routine-agent) to check each such job by its ID and resume the child through the host's resume action.

## Delegation

### Review loop

The review loop is in [review](workflow.md#review). Root reads dispositions, not diffs, and answers only a disagreement between implementer and reviewer, or a policy question no recorded decision answers.

### Ready queue and pipelining

- Root selects work and writes the roadmap's [ready queue](roadmap.md#ready-queue): two or three undispatched work orders. Each work order has its own issue, whose body root opens with one line saying what is finished when the work is done. An entry is that issue and the PR or issue it waits on. Size, files and method are the planner's to settle.
  - Work under a tracker that spans many PRs, such as #734, gets a sub-issue of it: `gh api repos/{owner}/{repo}/issues/<tracker>/sub_issues -F sub_issue_id=$(gh api repos/{owner}/{repo}/issues/<new issue> -q .id)`.
- Root refills the queue when it holds no entry for the next slot to free, up to three entries at once (provisional), in a docs-only commit that edits only that section.
- **Plans.** Each work order gets an implementation plan before it gets an implementer. Once an entry waits on nothing, root starts a [planner](planner-brief.md) for it, whether or not a slot is free. The planner writes the plan into the issue's body, below root's line. Root settles the plan's open questions, then dispatches the entry to the next free slot.
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
