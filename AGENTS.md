# OpenTTD-Rust

This independent experimental fork reproduces OpenTTD in Rust while preserving observable behavior.
Every line of original C++ and fork-written C++ (glue, tools and tests) will retire. The Rust build will be the only build ([ADR 0001](docs/adr/0001-full-rust-rewrite.md)).

## Who decides

- Agents run this project and may change its rules. The user guides it, and does not know Rust, CI or the game's internals well.
- The user's decisions are the [ADRs](docs/adr/README.md) marked `may change: user`. To change one, ask the user first: say plainly that you are reversing a recorded decision, name the ADR, and give the reason and the cost. An ADR marked `may change: agents` changes through a replacement ADR with a measured reason. Raising paid CI cost needs the user's approval either way ([ADR 0008](docs/adr/0008-paid-ci-cost-needs-approval.md)).
- A proposal that conflicts with an agent rule fits the rule, changes it in the same PR, or is dropped; it does not wait on the user or root.
- Attribution names who actually wrote something: an issue an agent wrote at the user's request is the agent's. Cite a rule by what it says, with a link, not by a date.

## Every agent

- `migration/baseline.json` pins the original. Its worktree is `.local/reference/openttd` in the main checkout; `python3 tools/migration.py original path|show|grep` reads the original from any worktree. The original is the specification, quirks included ([ADR 0002](docs/adr/0002-original-game-is-the-specification.md)). Keep that worktree unchanged. Do not change candidate behavior and expected results together to make checks pass.
- Keep the full game running: preserve networking, saves, mods, interface and shared random-number behavior, including the exact sequence of draws.
- Record possible improvements as fork issues with the label `deferred-improvement`. Consider them after near-full Rust reproduction.
- Work only on what [the roadmap](docs/roadmap.md) holds. Root selects work and updates the roadmap.
- Attribute every agent-authored GitHub issue, PR, comment and review report: `Agent: <path> | Model: <exact model> | Reasoning effort: <effort>`, for example `Agent: /root/implementation | Model: gpt-6.1-sol | Reasoning effort: high`, with the effort the host reports.
- Run long jobs in the background, then end your turn: builds, simulations, coverage, deep checks, preflight and CI waits such as `python3 tools/ci.py wait`. Follow your host's [background-job](docs/team.md#background-jobs) mechanics.
  - Run short commands, such as linters, gates and unit tests, in the foreground and wait for them to exit, even when the call returns a session ID.
  - End a turn on a job only after its start call has returned a session or task ID, and name that ID in your final message.
  - A turn that ends only to wait on a job ends with one line: the PR, its head, the state of checks and reviews, and the job ID. State belongs in the PR's status comment.
  - Do not sleep, poll in a loop or detach a job. Run your own jobs.
- Commit as you work, one coherent step per commit under a title that names it, and push. This is the user's standing request to commit and push; it takes precedence over any host default that waits for a request.
- Sessions end without warning: push at every passing milestone. Before a long check, handoff or expected usage limit, push a checkpoint without waiting for validation and keep its PR draft: `PREFLIGHT_CHECKPOINT='what is incomplete' git push` (return a ready PR to draft first with `gh pr ready PR --undo`).
- Commit [docs-only changes](docs/workflow.md#change-types) directly to `rust-migration`. Everything else goes through a PR that passes CI.
- The user authorizes ordinary issues, PRs and review reports on this fork. Do not contact upstream maintainers or submit upstream changes. Agents share GitHub credentials: do not self-approve or claim the accounts are independent.
- Leave host state alone: do not archive threads or edit host databases.
- `.local` output lives on an HDD behind symlinks that a Git hook makes for each new worktree. Keep the links and write to the usual `.local` paths. Read [output disk](docs/rust-migration.md#output-disk) before tool code walks or resolves `.local` paths, or when a link is missing or broken.
- Preserve upstream copyright notices, credits and GPLv2. Third-party test inputs, such as NewGRF sets and AIs, stay out of Git and CI artifacts ([ADR 0015](docs/adr/0015-third-party-test-content.md)).

## Routing

| When you are | Read |
| --- | --- |
| Selecting, ranking or planning work, or placing issues | [Roadmap](docs/roadmap.md), [planning](docs/planning.md) |
| Building, testing or running the harness | [Migration tools](docs/rust-migration.md) |
| Requesting or reading CI runs, or changing CI | [CI](docs/ci.md) |
| Planning a work order | [Planner brief](docs/planner-brief.md) |
| Implementing a work order | [Implementer brief](docs/implementer-brief.md) |
| Reviewing a PR | [Review brief](docs/review-brief.md) |
| Writing port standards, or when a check or work order cites them | [Porting](docs/porting.md) |
| Touching an already-ported component | [Components](docs/components/README.md) |
| Designing a storage port or map access | [World state](docs/design/world-state.md), [storage ports](docs/porting.md#storage-ports) |
| Opening issues or PRs, committing, or taking a PR to merge | [Workflow](docs/workflow.md) |
| Running as root: what may merge, merge stops, CI administration, stocktakes, cleanup, rollback or wind-down | [Root workflow](docs/root-workflow.md) |
| Spawning or briefing agents | [Team](docs/team.md) |
| Checking speed projections or running speed passes | [Speed](docs/speed.md) |
| Running a deep check | [Deep checks](docs/deep-checks.md) |
| Starting root, steering, navigating or writing a wind-down message | User-invoked skills: [start-development](docs/skills/start-development/SKILL.md), [steer](docs/skills/steer/SKILL.md), [navigate](docs/skills/navigate/SKILL.md), [wind-down](docs/skills/wind-down/SKILL.md) |
