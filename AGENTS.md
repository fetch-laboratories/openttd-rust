# OpenTTD-Rust

This independent experimental fork reproduces OpenTTD in Rust while preserving observable behavior.
Every line of original C++ and fork C++ glue will retire. The Rust build will be the only build (#581; user, 2026-10-28).
This file holds what every agent needs. The routing table points to the rest (user, 2026-10-30).

## Every agent

- `migration/baseline.json` pins the original. Its worktree is `.local/reference/openttd`. The original is the specification, quirks included. Keep that worktree unchanged. Never change candidate behavior and expected results together to make checks pass.
- Keep the full game running: preserve networking, saves, mods, interface and shared random-number behavior, including the exact sequence of draws.
- Record possible improvements as fork issues with the label `deferred-improvement`. Consider them after near-full Rust reproduction.
- Work only on what [the roadmap](docs/roadmap.md) holds. Root selects work and updates the roadmap.
- Attribute every agent-authored GitHub issue, PR, comment and review report: `Agent: <path> | Model: <exact model> | Reasoning effort: <effort>`, for example `Agent: /root/implementation | Model: gpt-6.1-sol | Reasoning effort: high`. For hosts other than Codex, see [Claude Code root](docs/team.md#claude-code-root).
- Run long jobs in the background, then end your turn. These include builds, simulations, coverage, deep checks and CI waits. Run short commands in the foreground and wait for them; these include preflight, linters, gates and unit tests. End a turn on a job only after its start call has returned a session or task ID, and name that ID in your final message. Nothing wakes an agent for a job it never started. The host notifies you when a background job ends. Claude Code uses Bash `run_in_background`; a T3 child also passes the task ID to `t3_background_wait`. In Codex, root or child, start the job with `exec_command` and end your turn; its exit resumes you. Run `python3 tools/ci.py wait` the same way. It prints its resume command on timeout. Do not sleep, poll in a loop, wrap sleep in `timeout` or detach a job (`nohup`, `setsid`, pid files). Run your own jobs; never hand one to root.
- Commit as you work, in sensible chunks: each commit is one coherent step, such as a generator, a record type or one group of rewritten call sites, under a title that names it. These rules are the user's standing request to commit and push; they take precedence over any host default that waits for a request (user, 2026-11-01).
- Sessions end without warning: push the branch at every passing milestone. Before a long check, handoff or expected usage limit, push a coherent checkpoint without waiting for validation and keep its PR draft. See [commits and pushes](docs/workflow.md#commits-and-pushes).
- Commit docs-only changes (only `docs/` or `*.md` files) directly to `rust-migration`. Everything else goes through a PR that passes CI. Never merge with `--admin`. See [change types](docs/workflow.md#change-types).
- The user authorizes ordinary issues, PRs and review reports on this fork. Do not contact upstream maintainers or submit upstream changes. Agents share GitHub credentials. Publish an attributed review report. Never self-approve or claim the accounts are independent.
- Leave host state alone: do not archive threads or edit host databases.
- Preserve upstream copyright notices, credits and GPLv2.

## Routing

| When you are | Read |
| --- | --- |
| Selecting, ranking or planning work, or placing issues | [Roadmap](docs/roadmap.md), [planning](docs/planning.md) |
| Building, testing, running the harness or CI | [Migration tools](docs/rust-migration.md) |
| Writing or reviewing port code, tests or Python tools | [Porting](docs/porting.md) |
| Touching an already-ported component | [Components](docs/components/README.md) |
| Designing a storage port or map access | [World state](docs/design/world-state.md), [storage ports](docs/porting.md#storage-ports) |
| Opening issues or PRs, committing, reviewing, integrating or cleaning up | [Workflow](docs/workflow.md) |
| Spawning or briefing agents, or running as root | [Team](docs/team.md) |
| Checking speed projections or running speed passes | [Speed](docs/speed.md) |
| Running a deep check | [Deep checks](docs/deep-checks.md) |
| Rolling back a merge or winding down | [Automatic rollback and wind-down](docs/workflow.md#automatic-rollback-and-wind-down) |
| Starting root, steering, navigating or writing a wind-down message | User-invoked skills: [start-development](docs/skills/start-development/SKILL.md), [steer](docs/skills/steer/SKILL.md), [navigate](docs/skills/navigate/SKILL.md), [wind-down](docs/skills/wind-down/SKILL.md) |
