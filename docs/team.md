# Team

Readers: Root and user-invoked skills that spawn agents.

## Roles and models

- Root (`gpt-6.1-sol`, xhigh) orchestrates and delegates heavily, selects components, integrates and spot-checks delegated diffs.
- Run at most four implementation agents at once (user, 2026-11-06): builds on this 8-core host are CPU-bound, so more implementers lengthen every build and test loop without adding throughput. Reviews, planning and research use the remaining slots and do not count; a running profile counts as one implementer. Codex's per-session limit is 12 subagents plus root and cannot be removed. Existing sessions and worktrees may retain a lower configuration; respect the actual host limit. The main checkout's uncommitted `.codex/config.toml` change belongs to the user and remains untouched. Agent slots and the six unintegrated component branches in [integration](root-workflow.md#integration) are different limits: reviews, planning and measurements need no new component branch.
- Specify model and effort on every spawn. Each subagent uses less effort than root's xhigh. Under a Claude Code root, root may run one role at xhigh when its work justifies it (user, 2026-11-09). No role is designated now.
- Use `gpt-6.1-sol` for most implementation and analysis and all independent review. Use high effort unless a clearly bounded task justifies lower.
- Use `gpt-6-luna` at low or medium effort for bounded mechanical work.
- Use `gpt-6-astra` at high effort for all delegated planning.

## Spawning

- Point implementation briefs at the [implementer brief](implementer-brief.md) and the work order, and reviewer briefs at the [review checklist](review-checklist.md); those agents read further docs only where these point.
- Children send mid-task messages to their own `parentThreadId`, read from their thread metadata; never find root by searching thread titles.
- Spawn a fresh agent named for each task; its name is its attribution. Do not reassign a finished agent to unrelated work.
- Each review round, including re-review, gets a fresh reviewer who reads the PR's earlier attributed review reports.
- Follow the host's orchestration rules: use native agents for compatible shared-checkout work completed in the same turn; use T3 delegation for background work, another provider or an isolated checkout. T3 creates and binds isolated checkouts. Follow its completion notifications and returned next action; do not poll a running delegate.
- At the host slot limit, root works its own queue (integration, spot-checking fix commits, roadmap) until a slot frees. Do not archive threads or edit host databases to free slots.
- A quiet-host hold lasts only through timed games. At an independent wake while holding other agents, inspect the named job of a child already known to be idle; release the hold as soon as the measurement ends, before report preparation. A failed or declined peripheral host action does not cancel unrelated authorized work.
- A long-job brief links the [background rule](../AGENTS.md#every-agent). Root never runs a child's jobs for it. A child names its job ID and uses the host's background-completion mechanism before ending its turn. At each independent wake, root handles completed results and checks jobs of children already known to be idle on them. If a job ended without waking its child, use the host's supported resume action; do not poll a running delegate or duplicate its work. This recovery remains necessary: four missed resumes cost about six agent-hours on 11-02. A child that reports a running job without an ID has not established a resumable handoff; correct it immediately. A subagent waiting only on CI or another task reports and stops.
- Start each long job as `exec python3 tools/signal-witness.py -- <command>`. On this Linux host the wrapper records a received SIGTERM's sender and forwards it to the command's process group in the same session, then reaps the child. If the job exits 143 and the log shows no failure, read the sender record and the run's log or report, then rerun once incrementally (#724). Add no waiter sessions, pidfd waits or duplicate runs. Root adds each new kill time and sender evidence to #724. Existing jobs continue unchanged.

## Claude Code root

- A user-directed session outside Codex names its exact model and host-reported effort. When it runs root, retain the same structure with its host's models: subagents at lower effort than root and a fresh reviewer per round.
- Claude Code root uses its host's models for implementation and planning, and `gpt-6.1-sol` high through T3 `delegate_task` for independent review, so reviewer and implementer differ in model family.
- Under a Claude Code root, `gpt-6.1-sol` reviewers report findings and do not commit fixes. Each finding goes in the attributed review report with its location, the original it departs from and the check that found it. The PR's Opus implementer fixes them in its own commits, one per finding or coherent group, naming the finding and ending with `Found-by: review`; the reviewer who raised each finding then verifies its fixing commits, report only ([review loop](#review-loop)). That verification is the scoped review a fix needs when it changes behaviour or evidence (user, 2026-11-08).
- Opus handles implementation, fixes and code-touching follow-ups; Sonnet handles only non-code work such as runs, measurements and tracker edits (user, 2026-10-27). Opus writes code at medium effort; use high for judgment such as planning, speed attribution and hard fidelity analysis (user, 2026-10-30).
- Spawn each code-touching agent through T3 `delegate_task` with `isolation: "worktree"`. T3 creates the worktree from root's commit, binds the shell to it and removes it when its PR closes. The agent names its branch and opens its PR there. Root makes worktrees by hand under `.local/worktrees` only for its own small PRs and reviews.

### Delegation (user, 2026-11-08)

Under a Claude Code root, these rules move routine work off root, so that root's turns go to selection, policy rulings, briefs and the roadmap. Where they differ from [spawning](#spawning), they replace it.

#### Integrator

- One integrator runs at a time, outside the implementer count. Its work is mechanical, so it runs Opus at medium effort (user, 2026-11-09). It works from the main checkout but only through worktrees under `.local/worktrees`. It never changes the main checkout's branch or tracked files, including the user's `.codex/config.toml`. It edits no docs and selects no work.
- It owns every step of [integration](root-workflow.md#integration):
  - **Readiness.** A PR is ready when every review accepts, or each finding's reviewer has verified the fix.
  - **Base updates.** Merge commits titled `Update: Merge integration N into ...`, never `gh pr update-branch`. The integrator records the base-update condition on the PR.
  - **Batching.** Overlap checks, and combined integration PRs for disjoint ready PRs.
  - **Validation and merge.** Local checks, exact-head Full requests and waits, and merges with `--match-head-commit` once the integration rule holds.
  - **Records and cleanup.** Integration notes with whole-fork metrics; closing component PRs; removing branches, worktrees and finished worktrees' `build-rust`.
  - **Platform cadence.** A run after every fifth integration.
  - **Notifying implementers** of open PRs when base moves.
- It escalates to root and does not decide:
  - a conflict in `src/`, `rust/`, `tools/` or `.github/`;
  - a failed Full or cadence run: it diagnoses and reports, and root routes the fix;
  - except a runner or network failure (a step before any build or test, such as package install, checkout or cache download, or a lost runner with no test output), which it reruns once without asking and names in the integration note. For a simulation shard, the rerun is a fresh `tools/ci.py request`, because after a failed-job rerun, `native / complete` still reads the earlier attempt's shard artifact (#693, 11-09). A second failure of the same job escalates. A network-protocol scenario hang follows #693: keep the evidence, rerun once, record it there;
  - the [rollback rule](root-workflow.md#automatic-rollback-and-wind-down);
  - anything no rule covers.
- Root sends it each newly ready PR in one line. It reports each merge or escalation to root.

#### Review loop

- Root spawns reviewers from the [review brief](review-brief.md). The spawn adds only the PR, the reviewer's name, its function group, the other groups, and the points the PR body flags.
- Reviewers rule on policy questions themselves, from the decisions recorded on the stream issues, and state each ruling in the report. Only a question no recorded decision answers waits for root.
- When a PR's reviews are all posted, root tells its implementer in one line. The implementer reads the reports on the PR.
- When the implementer reports its fixes, root asks each finding's reviewer, in one line, to verify them. Fix verification is part of that reviewer's round, not a new round; a new round after a substantive rewrite still gets a fresh reviewer. Root reads dispositions, not diffs.

#### Ready queue and pipelining

- The roadmap's [ready queue](roadmap.md#ready-queue) holds two or three work orders. Each gives its scope, its file set checked against open PRs and reservations, and the PR or reservation it waits on, if any.
- A planner (Opus, high) refreshes the queue whenever fewer than two entries are ready. It edits only that roadmap section, in docs-only commits. An entry becomes selected when root dispatches it.
- An implementer whose PR waits on review or Full takes the next ready entry on a new branch, within the six-branch limit. Findings on its earlier PR come first. It keeps its agent path; each PR's title names the work. The four implementer slots therefore count agents that stay busy, not PRs.

## Provider handoff

For unattended handoff, establish an independently executable takeover before the source provider exhausts its allowance. A scheduled prompt is not a fallback if it needs that provider to launch its replacement. Verify the host's actual execution provider; a model name in a prompt is insufficient. If the host cannot provide independent takeover, record the limitation and preserve a checkpoint before stopping. This rule creates no schedule.

## Skills

`docs/skills/` holds four user-invoked skills: `/navigate` (macro course review of the goal and phases, before `/steer`), `/steer` (steering review), `/start-development` (root's restart prompt), and `/wind-down` (the user's message pausing root). Two automatically discoverable skills support selected work: `brief-storage-owner` prepares a pool/map ownership brief or checkpoint handoff, and `review-evidence-contract` examines production-path evidence for a port claim or changed fixture/oracle. Their descriptions define when they apply; they add no universal review gate. Steering reviews put corrections in the roadmap and edit `/start-development` only to improve it. Only steering and navigation reviews edit the four orchestration skills unless the user requests an edit; root otherwise follows them and leaves them unedited.
