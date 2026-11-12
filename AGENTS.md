# OpenTTD-Rust

This independent experimental fork reproduces OpenTTD in Rust while preserving observable behavior.
Every line of original C++ and fork-written C++ (glue, tools and tests) will retire. The Rust build will be the only build ([ADR 0001](docs/adr/0001-full-rust-rewrite.md)).
This file holds what every agent needs. The routing table points to the rest.

## Who decides

- Agents run this project. The user guides it, and does not know Rust, CI or the game's internals well.
- A rule in these docs is an agent rule unless an ADR in `docs/adr/` marks it `may change: user`. Agents may change agent rules.
- The user's decisions are the ADRs marked `may change: user`. To change one, ask the user first: say plainly that you are reversing a recorded decision, name the ADR, and give the reason and the cost. An ADR marked `may change: agents` changes through a replacement ADR with a measured reason. A change that raises paid CI cost needs the user's approval either way ([ADR 0008](docs/adr/0008-paid-ci-cost-needs-approval.md)). See [ADRs](docs/adr/README.md).
- When a proposal seems to conflict with a rule, the agent making the proposal settles it before filing: the proposal fits the rule, or the rule changes in the same PR, or the proposal is dropped. No issue or PR waits on the user, or on root's ruling, over an agent rule.
- Attribution names who actually wrote something. An issue an agent wrote at the user's request is that agent's issue, not the user's. Refer to a rule or decision by what it says, with a link, never by a date.

## Every agent

- `migration/baseline.json` pins the original. Its worktree is `.local/reference/openttd`. The original is the specification, quirks included ([ADR 0002](docs/adr/0002-original-game-is-the-specification.md)). Keep that worktree unchanged. Never change candidate behavior and expected results together to make checks pass.
- Keep the full game running: preserve networking, saves, mods, interface and shared random-number behavior, including the exact sequence of draws.
- Record possible improvements as fork issues with the label `deferred-improvement`. Consider them after near-full Rust reproduction.
- Work only on what [the roadmap](docs/roadmap.md) holds. Root selects work and updates the roadmap.
- Attribute every agent-authored GitHub issue, PR, comment and review report: `Agent: <path> | Model: <exact model> | Reasoning effort: <effort>`, for example `Agent: /root/implementation | Model: gpt-6.1-sol | Reasoning effort: high`. For hosts other than Codex, see [Claude Code root](docs/team.md#claude-code-root).
- Run long jobs in the background, then end your turn: builds, simulations, coverage, deep checks, preflight and CI waits, including `python3 tools/ci.py wait`, which prints its resume command on timeout. [Background jobs](docs/team.md#background-jobs) gives each host's mechanics; follow them for yours.
  - Run short commands in the foreground and wait for them: linters (the Clippy and Ruff lint ratchets too), gates and unit tests. A short command that returns a session ID is still short: wait on that session until it exits, and never end a turn on it, since a job that ends after your turn wakes no one.
  - End a turn on a job only after its start call has returned a session or task ID, and name that ID in your final message. Nothing wakes an agent for a job it never started.
  - The host hands a child's whole last message to its parent at every turn end, and the parent acts only on its last line. So a turn that ends only to wait on a job writes that line and nothing else: the PR, its head, the state of checks and reviews, and the job ID. State belongs in the PR's status comment.
  - Do not sleep, poll in a loop, wrap sleep in `timeout` or detach a job (`nohup`, `setsid`, pid files). Run your own jobs; never hand one to root.
- Commit as you work, in sensible chunks: each commit is one coherent step, such as a generator, a record type or one group of rewritten call sites, under a title that names it. These rules are the user's standing request to commit and push; they take precedence over any host default that waits for a request.
- Sessions end without warning: push the branch at every passing milestone. Before a long check, handoff or expected usage limit, push a coherent checkpoint without waiting for validation and keep its PR draft: `PREFLIGHT_CHECKPOINT='what is incomplete' git push` skips the pre-push hook's preflight stamp, runs only its few-second fast tier and is logged. Quick does not run on a draft, so a checkpoint is untested, never red. The hook refuses a checkpoint push to a ready PR; see [commits and pushes](docs/workflow.md#commits-and-pushes) for that case.
- Commit docs-only changes (`AGENTS.md` and Markdown under `docs/`, except the docs packaged for the in-game viewer) directly to `rust-migration`. Everything else goes through a PR that passes CI, scripts and other files under `docs/` included. Never merge with `--admin`. See [change types](docs/workflow.md#change-types).
- The user authorizes ordinary issues, PRs and review reports on this fork. Do not contact upstream maintainers or submit upstream changes. Agents share GitHub credentials. Publish an attributed review report. Never self-approve or claim the accounts are independent.
- Leave host state alone: do not archive threads or edit host databases.
- This host's SSD is small, so `.local` output lives on an HDD behind symlinks that a Git hook makes for each new worktree. Keep the links and write to the usual `.local` paths. Read [output disk](docs/rust-migration.md#output-disk) before tool code walks or resolves `.local` paths, or when a link is missing or broken.
- Preserve upstream copyright notices, credits and GPLv2. Third-party test inputs, such as NewGRF sets and AIs, stay out of Git and CI artifacts, so the fork redistributes nothing ([ADR 0015](docs/adr/0015-third-party-test-content.md), [evidence](docs/porting.md#evidence)).

## Routing

| When you are | Read |
| --- | --- |
| Changing a recorded decision, or checking who may change one | [ADRs](docs/adr/README.md) |
| Selecting, ranking or planning work, or placing issues | [Roadmap](docs/roadmap.md), [planning](docs/planning.md) |
| Building, testing or running the harness | [Migration tools](docs/rust-migration.md) |
| Requesting or reading CI runs, or changing CI | [CI](docs/ci.md) |
| Implementing a work order | [Implementer brief](docs/implementer-brief.md) |
| Reviewing a PR | [Review brief](docs/review-brief.md), [review](docs/workflow.md#review) |
| Writing port standards, or when a check or work order cites them | [Porting](docs/porting.md) |
| Touching an already-ported component | [Components](docs/components/README.md) |
| Designing a storage port or map access | [World state](docs/design/world-state.md), [storage ports](docs/porting.md#storage-ports) |
| Opening issues or PRs, committing, or taking a PR through review and the merge queue | [Workflow](docs/workflow.md) |
| Deciding what may merge, stopping merges, administering CI, taking stock or reclaiming output | [Root workflow](docs/root-workflow.md) |
| Spawning or briefing agents, running as root, or registering a background job's wake | [Team](docs/team.md) |
| Checking speed projections or running speed passes | [Speed](docs/speed.md) |
| Running a deep check | [Deep checks](docs/deep-checks.md) |
| Rolling back a merge or winding down | [Automatic rollback and wind-down](docs/root-workflow.md#automatic-rollback-and-wind-down) |
| Starting root, steering, navigating or writing a wind-down message | User-invoked skills: [start-development](docs/skills/start-development/SKILL.md), [steer](docs/skills/steer/SKILL.md), [navigate](docs/skills/navigate/SKILL.md), [wind-down](docs/skills/wind-down/SKILL.md) |
