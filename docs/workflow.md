# Change workflow

Readers: Every agent changing the repository.

## Change types

- In a user-review session, commit a settled docs-only change locally; amend each correction into that unpushed commit and retitle it for the final text. Push on user approval, a move to another subject, or before root or a delegate needs it on origin. Collect corrections to an already pushed change in one new commit the same way. Push pending docs-only commits at session start.
- Docs-only means only `docs/` or `*.md` files, including roadmap updates and skill edits under `docs/skills/`. Follow [the change policy](../AGENTS.md#every-agent), with one commit per change; an agent working alone pushes as soon as the change is done.

## Records

- Keep evidence checkable; spend effort on code and checks, not exhaustive prose. Record changes, checks and remaining limits in PRs; migration documentation records design decisions and open limits, not evidence or review history. Scaffolding does not complete a subsystem.
- The [migration issue form](../.github/ISSUE_TEMPLATE/migration.yml) covers scope, affected interfaces, evidence plan and acceptance criteria; leave design to implementation. Use the [PR template](../.github/PULL_REQUEST_TEMPLATE.md) for what moved, what stays in C++, exact commands, limits and `python3 tools/port-metrics.py` output. Integration PRs link component PRs instead of re-describing them.
- These are contents, not line counts: omit what belongs elsewhere, but retain needed facts, limits and steps (user, 2026-10-26).
- Keep the roadmap forward-looking without a line cap. Update it in place: a completed item becomes one row with issue, PR, commit and metrics; evidence stays in the PR. Replace the resume checkpoint with a short status table, not a log. Put CI run IDs, executable hashes, probe values and per-PR plans in the PR or issue. Do not delete or fold another section to make room; each `/steer` run prunes the roadmap.
- Evidence is exact-head CI runs and commands recorded in the PR. Do not put receipt bundles, executable hashes or `.local/` paths in comments or docs. Track tooling a rule depends on, such as the speed-pass collector, in `tools/`; do not copy scripts between `.local/` runs.
- Edit one status comment per task on its PR, or issue before the PR, at start, milestones and stop. A milestone is a pushed checkpoint whose checks pass, a blocking failure or a handoff, not each push or job start. Other PR comments are review reports and root's integration note. A tracker issue body is its current list, edited as items open or close; a task with a PR does not also post status on the tracker.
- Docs use structured text: headings, tables and bullets; bullets may be clauses. Prose in issues, comments and commit messages uses full sentences. Use a space after every colon and between a word and a number, and "X of Y" (user, 2026-10-30).
- Put rules every agent needs in [AGENTS.md](../AGENTS.md). Put other rules in the doc named by its routing table. Keep each full rule in one place. A one-line reminder of an important rule may repeat where an agent would otherwise miss it; link to the full rule (user, 2026-10-30).

## Branches and worktrees

For substantive changes create a fork issue with a clear owner, then use an isolated branch and worktree based on `rust-migration`. The implementation agent opens a draft PR targeting `rust-migration`, links the issue and records exact validation commands and limits. See [team](team.md#claude-code-root) for Claude Code worktree creation.

## Commits and pushes

- Before pushing, run `python3 tools/preflight.py --base origin/rust-migration`. It runs the pinned inherited commit checker and every gate of Quick's `quick` job, read from `ci-quick.yml`, and lists every failure; `--step TEXT` reruns matching steps. The commit checker rejects titles without a supported prefix such as `Add:`, `Change:`, `Fix:`, `Doc:` or `Update:` (integration merges included; `Merge:` fails), non-ASCII or tab characters in messages, and non-ASCII text or space indentation in C and C++ diffs. Fix style in the introducing commit.
- A fidelity or crash fix ends with `Found-by:` naming the check: `review`, `scenario NAME`, `comparison TOOL`, `unit test`, `CI JOB`, `deep check` or `user` (#512).
- Keep an unfinished checkpoint's PR in draft, recording what is incomplete, pending or failed checks and the next action. A checkpoint is not integration evidence. A review that stops before recording findings and disposition counts as no review.

## Checks

- Iterate locally with the build and Cargo checks. Scenario comparison runs on CI, not on this host (user, 2026-11-10): once the local checks pass, push and run `python3 tools/ci.py request PR --profile native`, which runs the whole simulation tier, then wait on it in the background. A CI shard ran 186 scenarios in 3 minutes. On this host, idle, the default set was unfinished after 25 minutes and had written 55 GB to the hard disk. Run a scenario locally, by name, only to diagnose a failure. Add no local suite or pre-merge local build gate.
- A passing `native` request at the reviewed head is the scenario evidence a review needs. Final exact-head Full repeats the tier on the merged tree.
- Ordinary PR pushes run cheap checks; do not use `[skip ci]` on code pushes. An implementer requests `--profile rust` or `native` for its own PR; a new request replaces that PR's running one. Request `platform` only for a suspected macOS failure or once before Full for a change to `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` or `cmake/` (#642's new crate dependency turned the cadence red after integration): it adds macOS, the one platform Full leaves to the cadence. Docs pushes to base cost no Full: the PR's update from base carries its status ([carried validation](rust-migration.md#continuous-integration)).
- Before requesting Full for a PR removing a name, grep `tools/simulation/` and `src/tests/` for it: fixtures for scenarios not run locally compile only in Full.
- See [continuous integration](rust-migration.md#continuous-integration) for Full jobs and profiles.

## Review

- Infrastructure reviews apply [cost decisions](planning.md#cost-decisions).
- A separate reviewer examines the final commit and evidence under the [review checklist](review-checklist.md). For a port retiring more than about 1,000 original lines, assign one reviewer per function group in parallel.
- For each new scenario, fixture or checker, the reviewer names the behavior it distinguishes that existing checks cannot ([cost decisions](planning.md#cost-decisions)). One that names none, or compares an operation with itself, is a finding; the fix deletes it. Scenarios that exercise the component stay ([evidence](porting.md#evidence)).
- The reviewer commits each fix to the PR branch; under a Claude Code root the reviewer reports findings and the implementer commits the fixes ([team](team.md#claude-code-root)). Rerun only checks affected by those fixes; a review without fixes reruns nothing. List each finding, its fixing commit and the check that found it. Send findings that need a scope or policy decision to root unfixed.
- Use the review report skeleton in the [PR template](../.github/PULL_REQUEST_TEMPLATE.md), starting with `Reviewed commit: <sha>`. Name the functions or files compared with original bodies and duplicate callbacks; cite the quick-CI orphan-export check for exports and declarations. Name touched `WITH_RUST` bodies and their production callers. Each orphan is a finding. Add no narration when there are no findings. Put branches the harness does not reach in #156, not only in the disposition.
- The implementer or root may commit fixes for CI warnings or platform errors, or record speed medians, without a new review. CI checks those commits. A simulation behavior fix found by CI needs a fresh scoped review, including arithmetic or assertion fixes.
- A reviewed PR's base update that meets a condition under [integration](root-workflow.md#integration) needs only CI.

## Integration

Root follows [integration](root-workflow.md#integration), including the exact-head Full gate.

## Cleanup

Root follows [cleanup and housekeeping](root-workflow.md#cleanup).

## Automatic rollback and wind-down

Root follows [automatic rollback and wind-down](root-workflow.md#automatic-rollback-and-wind-down).
