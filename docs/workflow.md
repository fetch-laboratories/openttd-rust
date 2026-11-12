# Change workflow

Readers: Every agent changing the repository.

## Change types

- In a user-review session, commit a settled docs-only change locally; amend each correction into that unpushed commit and retitle it for the final text. Push on user approval, a move to another subject, or before root or a delegate needs it on origin. Collect corrections to an already pushed change in one new commit the same way. Push pending docs-only commits at session start.
- Docs-only means only `docs/` or `*.md` files, including roadmap updates and skill edits under `docs/skills/`. Follow [the change policy](../AGENTS.md#every-agent), with one commit per change; an agent working alone pushes as soon as the change is done.

## Records

- Keep evidence checkable; spend effort on code and checks, not exhaustive prose. Record changes, checks and remaining limits in PRs; migration documentation records design decisions and open limits, not evidence or review history. Scaffolding does not complete a subsystem.
- The [migration issue form](../.github/ISSUE_TEMPLATE/migration.yml) covers scope, affected interfaces, evidence plan and acceptance criteria; leave design to implementation. Use the [PR template](../.github/PULL_REQUEST_TEMPLATE.md) for what moved, what stays in C++, exact commands, limits and `python3 tools/port-metrics.py` output.
- These are contents, not line counts: omit what belongs elsewhere, but retain needed facts, limits and steps (user, 2026-10-26).
- A PR body names what moved as a list of types, functions and files. It does not narrate the diff or list mechanical callers one by one; the diff and commit titles show those. Keep the review groups, the residue with reasons, the checks and the lint carries: reviewers and gates read them (user, 2026-11-12; the last 40 PR bodies ran to about 7,500 characters each, and every reviewer reads the body).
- Keep the roadmap forward-looking without a line cap. Update it in place: a completed item becomes one row with issue, PR, commit and metrics; evidence stays in the PR. Replace the resume checkpoint with a short status table, not a log. Put CI run IDs, executable hashes, probe values and per-PR plans in the PR or issue. Do not delete or fold another section to make room; each `/steer` run prunes the roadmap.
- Evidence is exact-head CI runs and commands recorded in the PR. Do not put receipt bundles, executable hashes or `.local/` paths in comments or docs. Track tooling a rule depends on, such as the speed-pass collector, in `tools/`; do not copy scripts between `.local/` runs.
- Edit one status comment per task on its PR, or issue before the PR, at start, milestones and stop. A milestone is a pushed checkpoint whose checks pass, a blocking failure or a handoff, not each push or job start. Other PR comments are review reports. A tracker issue body is its current list, edited as items open or close; a task with a PR does not also post status on the tracker.
- Docs use structured text: headings, tables and bullets; bullets may be clauses. Prose in issues, comments and commit messages uses full sentences. Use a space after every colon and between a word and a number, and "X of Y" (user, 2026-10-30).
- Put rules every agent needs in [AGENTS.md](../AGENTS.md). Put other rules in the doc named by its routing table. Keep each full rule in one place. A one-line reminder of an important rule may repeat where an agent would otherwise miss it; link to the full rule (user, 2026-10-30).

## Branches and worktrees

For substantive changes create a fork issue with a clear owner, then use an isolated branch and worktree based on `rust-migration`. The implementation agent opens a draft PR targeting `rust-migration`, links the issue and records exact validation commands and limits. See [team](team.md#spawning) for worktree creation on either provider.

## Commits and pushes

- Skip local preflight when the entire outgoing change is inert Markdown under the same rule CI uses to skip builds ([continuous integration](rust-migration.md#continuous-integration)): `AGENTS.md` and Markdown under `docs/`, excluding the docs packaged for the in-game viewer. Check the complete diff, including both paths of renames, and run `git diff --check` instead. The broader docs-only commit policy does not exempt build inputs, scripts or skill metadata; a `[skip ci]` message alone does not qualify a mixed change.
- For other changes, before pushing run `python3 tools/preflight.py --base origin/rust-migration` as a background job. A full pass on a clean tree records a stamp for that tree. The pre-push hook (`tools/git_hooks.py`, #920) runs a fast tier of a few seconds on every push (the pinned commit checker on the branch's commits, `cargo fmt --check` and Quick's Ruff lines) and refuses a push that is not inert Markdown when its tree has no stamp. A tree that differs from a stamped one only in inert Markdown counts as stamped. A checkpoint push to a draft PR, or before its PR exists, takes the logged override `PREFLIGHT_CHECKPOINT='what is incomplete' git push`; it skips the stamp, not the fast tier. A push to a ready PR always needs the stamp. The pre-commit hook runs the pinned checker's diff rules on the staged change. Preflight runs the pinned inherited commit checker and every gate of Quick's `quick` job, read from `ci-quick.yml`, and lists every failure; `--step TEXT` reruns matching steps. The commit checker rejects titles without a supported prefix such as `Add:`, `Change:`, `Fix:`, `Doc:` or `Update:` (base merges into a PR included; `Merge:` fails), non-ASCII or tab characters in messages, and non-ASCII text or space indentation in C and C++ diffs. Fix style in the introducing commit.
- A fidelity or crash fix ends with `Found-by:` naming the check: `review`, `scenario NAME`, `comparison TOOL`, `unit test`, `CI JOB`, `deep check` or `user` (#512).
- Keep an unfinished checkpoint's PR in draft, recording what is incomplete, pending or failed checks and the next action. A checkpoint is not merge evidence. A review that stops before recording findings and disposition counts as no review.

## Checks

- Iterate locally with the build and Cargo checks. Scenario comparison runs on CI, not on this host (user, 2026-11-10): once the local checks pass, push and run `python3 tools/ci.py request PR --profile native`, which runs the whole simulation tier, then wait on it in the background. A CI shard ran 186 scenarios in 3 minutes. On this host, idle, the default set was unfinished after 25 minutes and had written 55 GB to the hard disk. Run a scenario locally, by name, only to diagnose a failure. Add no local suite or pre-merge local build gate.
- A passing `native` request at the reviewed head is the scenario evidence a PR needs before it queues; the implementer waits on it in the background, records it in the status comment, and no separate reviewer re-verifies it ([review loop](team.md#review-loop)). The queue's Full repeats the tier on the merged tree.
- Pushes to a ready PR run cheap checks; a draft PR runs none until it is marked ready or a profile is requested. Mark a PR ready with `python3 tools/ci.py ready PR`, and request a profile with `ci.py request`; both need a preflight stamp for the PR head. `--unstamped REASON` requests an unstamped head, such as a seeded control, and is logged. A deliberate negative control's branch starts with `control/` (#922); `python3 tools/ci.py failures --since DATE` counts failed runs and leaves those out, and a request whose PR head has since moved or closed ends cancelled, not failed. Do not use `[skip ci]` on code pushes. An implementer requests `--profile rust` or `native` for its own PR; a new request replaces that PR's running one. Request `platform` only for a suspected macOS build failure, or once before queueing a change to `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` or `cmake/` (#642's new crate dependency turned the cadence red after it merged): it adds macOS, which Full leaves to the cadence. Docs-only pushes to `rust-migration` remain allowed. Each makes the queue rebuild the queued PRs, which reuse the earlier run of the same code ([carried validation](rust-migration.md#continuous-integration)) and cost seconds; still, push no docs only to record a merge.
- See [continuous integration](rust-migration.md#continuous-integration) for Full jobs and profiles.

## Review

- Infrastructure reviews apply [cost decisions](planning.md#cost-decisions).
- A separate reviewer examines the final commit and evidence under the [review checklist](review-checklist.md) and reports as its [report](review-checklist.md#report) says. For a port retiring more than about 1,000 original lines, assign one reviewer per function group in parallel.
- Carries and exception proposals go in the branch's [cleanup records](rust-migration.md#quick-cleanup-checks) file, not the PR body; the reviewer's attributed report accepts a proposal under the tested head. Growth awaiting that acceptance is pending review on PR runs and fails in the merge group (#921).
- The implementer or root may commit fixes for CI warnings or platform errors, or record speed medians, without a new review. CI checks those commits. A simulation behavior fix found by CI needs a fresh scoped review, including arithmetic or assertion fixes.

## Integration

When every reviewer accepts and any re-check due has accepted, the implementer queues the PR with `python3 tools/ci.py enqueue PR`, never `gh pr merge`, under [integration](root-workflow.md#integration).

## Automatic rollback and wind-down

Root follows [automatic rollback and wind-down](root-workflow.md#automatic-rollback-and-wind-down).
