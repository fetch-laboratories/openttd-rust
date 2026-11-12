# Change workflow

Readers: every agent changing the repository; implementers and reviewers for the [PR lifecycle](#pr-lifecycle).

## Change types

- **Docs-only** means exactly what CI and the pre-push hook treat as inert: `AGENTS.md` and Markdown (`.md`) under `docs/`, except the docs CMake packages for the in-game viewer (the `builtDocs` list in `.github/workflows/ci-policy.yml`). Check the whole diff, both paths of a rename included.
- Everything else goes through a PR that passes CI, even under `docs/`: a script such as `docs/skills/steer/issues.py`, skill metadata such as `agents/openai.yaml`, images, `docs/openttd.6`, a packaged doc, and Markdown outside `docs/` other than `AGENTS.md`. A `[skip ci]` message does not make a mixed change docs-only.
- Commit a docs-only change directly to `rust-migration`, one commit per change; an agent working alone pushes as soon as the change is done. Branch protection requires PRs, so these pushes use administrative bypass, which covers documented docs pushes only, never code and never the merge queue.
- Each docs push makes the queue rebuild the queued PRs. They reuse the earlier run of the same code ([carried validation](ci.md)) and cost seconds; still, push no docs only to record a merge.
- Agent rules (see [who decides](../AGENTS.md#who-decides)) change like any docs-only change, in the doc that holds the rule. A rule change that a code proposal needs goes in that proposal's PR.
- In a user-review session, commit a settled docs-only change locally; amend each correction into that unpushed commit and retitle it for the final text. Push on user approval, a move to another subject, or before root or a delegate needs it on origin. Collect corrections to an already pushed change in one new commit the same way. Push pending docs-only commits at session start.

## Records

- Keep evidence checkable; spend effort on code and checks, not exhaustive prose. Record changes, checks and remaining limits in PRs; migration documentation records design decisions and open limits, not evidence or review history. Scaffolding does not complete a subsystem.
- The [migration issue form](../.github/ISSUE_TEMPLATE/migration.yml) covers scope, affected interfaces, evidence plan and acceptance criteria; leave design to implementation. Use the [PR template](../.github/PULL_REQUEST_TEMPLATE.md) for what moved, what stays in C++, exact commands, limits and `python3 tools/port-metrics.py` output.
- These are contents, not line counts: omit what belongs elsewhere, but retain needed facts, limits and steps.
- A PR body names what moved as a list of types, functions and files, because every reviewer reads the body. It does not narrate the diff or list mechanical callers one by one; the diff and commit titles show those. Keep the review groups, the residue (what the PR leaves unfinished) with reasons, and the checks. Lint carries and growth proposals go in the cleanup records file, not the body ([review](#review)).
- Evidence is exact-head CI runs and commands recorded in the PR. Do not put receipt bundles, executable hashes or `.local/` paths in comments or docs. Track tooling a rule depends on, such as the speed-pass collector, in `tools/`; do not copy scripts between `.local/` runs.
- Edit one status comment per task on its PR, or issue before the PR, at start, milestones and stop. A milestone is a pushed checkpoint whose checks pass, a blocking failure or a handoff, not each push or job start. Other PR comments are review reports. A tracker issue body is its current list, edited as items open or close; a task with a PR does not also post status on the tracker.
- Root commits the roadmap only at a stocktake, a change of course or wind-down ([ADR 0026](adr/0026-roadmap-commit-timing.md)); the one exception is the planner's [ready-queue](team.md#ready-queue-and-pipelining) refresh. The resume checkpoint is written only at wind-down.
- Keep the roadmap forward-looking without a line cap. Update it in place: a completed item becomes one row with issue, PR, commit and metrics; evidence stays in the PR. At wind-down, replace the resume checkpoint with a short status table, not a log. Put CI run IDs, executable hashes, probe values and per-PR plans in the PR or issue. Do not delete or fold another section to make room; each `/steer` run prunes the roadmap.
- Docs use structured text: headings, tables and bullets; bullets may be clauses. Prose in issues, comments and commit messages uses full sentences. Use a space after every colon and between a word and a number, and "X of Y".
- Put rules every agent needs in [AGENTS.md](../AGENTS.md). Put other rules in the doc named by its routing table. Keep each full rule in one place. A one-line reminder of an important rule may repeat where an agent would otherwise miss it; link to the full rule and give no numbers in it, so it cannot drift.

## Branches and worktrees

For substantive changes create a fork issue with a clear owner, then use an isolated branch and worktree based on `rust-migration`. The implementation agent opens a draft PR targeting `rust-migration`, links the issue and records exact validation commands and limits. See [team](team.md#spawning) for worktree creation on either provider.

## Commits and pushes

- **Preflight.** Before pushing a change that is not docs-only, run `python3 tools/preflight.py --base origin/rust-migration` as a background job. It runs the pinned inherited commit checker and every gate of Quick's `quick` job, read from `ci-quick.yml`, and lists every failure; `--step TEXT` reruns matching steps. A full pass on a clean tree records a **stamp** for that tree. A tree that differs from a stamped one only in docs-only files counts as stamped. A docs-only change skips preflight; run `git diff --check` instead.
- **Pre-push hook** (`tools/git_hooks.py`). Every push runs a fast tier of a few seconds: the pinned commit checker on the branch's commits, then, unless the change is docs-only, `cargo fmt --check` and Quick's Ruff lines. It refuses a push that is not docs-only when its tree has no stamp.
- **Checkpoint push.** To a draft PR, or before its PR exists, `PREFLIGHT_CHECKPOINT='what is incomplete' git push` skips the stamp, not the fast tier, and is logged.
- **Ready PR.** The hook refuses a checkpoint push to a ready PR: every push to it needs the stamp. Before a long check, handoff or expected usage limit, commit, start preflight in the background and push when it passes. If the session may end before then, return the PR to draft with `gh pr ready PR --undo` and push the checkpoint; `ci.py ready` later needs a stamp again.
- **Pre-commit hook.** It runs the pinned checker's diff rules on the staged change. If the checker cannot be fetched or is dirty, it says so and lets the commit through, and pre-push checks the same rules.
- **Commit checker.** It rejects titles without a supported prefix such as `Add:`, `Change:`, `Fix:`, `Doc:` or `Update:` (base merges into a PR included; `Merge:` fails), non-ASCII or tab characters in messages, and non-ASCII text or space indentation in C and C++ diffs. Fix style in the introducing commit.
- **Found-by.** A fidelity or crash fix ends with `Found-by:` naming the check: `review`, `scenario NAME`, `comparison TOOL`, `unit test`, `CI JOB`, `deep check` or `user`.
- Keep an unfinished checkpoint's PR in draft, recording what is incomplete, pending or failed checks and the next action. A checkpoint is not merge evidence.

## PR lifecycle

The implementer takes its own PR through every stage below ([ADR 0012](adr/0012-root-decides-implementers-review-and-merge.md)); root watches no PR.

| Stage | What happens | Rule |
| --- | --- | --- |
| Draft | Open a draft PR and push checkpoints; a draft runs no CI until it is marked ready or a profile is requested | [Commits and pushes](#commits-and-pushes) |
| Ready | `python3 tools/ci.py ready PR`, which needs a stamp | [Checks](#checks) |
| Native request | `python3 tools/ci.py request PR --profile native`, waited on in the background; the passing run at the reviewed head goes in the status comment | [Checks](#checks) |
| Review | The implementer starts its reviewers; reviewers report findings, the implementer fixes them | [Review](#review) |
| Re-check | One, only when a fix changed behaviour, test evidence or a body's operations | [Review](#review) |
| Queue | `python3 tools/ci.py enqueue PR`, then `queue-wait` in the background | [Integration](#integration) |
| Merge or removal | The queue merges the PR, or removes it and the implementer fixes the cause | [Integration](#integration) |

### Checks

- Iterate locally with the build and Cargo checks. Scenario comparisons run on CI, not on this host ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)): local runs are far slower and fill the hard disk. Once the local checks pass, push and request `--profile native`, which runs the whole simulation tier, and wait on it in the background. Run a scenario locally, by name, only to diagnose a failure. Add no local suite or pre-merge local build gate.
- A passing `native` request at the reviewed head is the scenario evidence a PR needs before it queues. The implementer records it in the status comment, and no separate reviewer re-verifies it. The queue's Full repeats the tier on the merged tree.
- Pushes to a ready PR run cheap checks; a draft PR runs none until it is marked ready or a profile is requested. `ci.py ready` and `ci.py request` both need a stamp for the PR head. `--unstamped REASON` requests an unstamped head, such as a seeded control, and is logged.
- A deliberate negative control's branch starts with `control/`. `python3 tools/ci.py failures --since DATE` counts failed runs and leaves those out; a request whose PR head has since moved or closed ends cancelled, not failed.
- Do not use `[skip ci]` on code pushes. An implementer requests `--profile rust` or `native` for its own PR; a new request replaces that PR's running one. Rerun a flaky shard with `gh run rerun RUN --failed`.
- Request `--profile platform` only for a suspected macOS build failure, or once before queueing a change to `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` or `cmake/`: dependency changes can break macOS, which Full leaves to the [platform cadence](root-workflow.md#cadences).
- See [CI](ci.md) for Full jobs and profiles.

### Review

- The implementer starts its own reviewers and any re-check with T3 `delegate_task`, using the [role table](team.md#roles-and-models)'s review model and the [review brief](review-brief.md)'s fixed spawn message. That message names the PR, the reviewer, its review group when the PR body splits the PR, and the implementer's thread; it says nothing about what to look for. Root starts no reviewers.
- Each review gets a fresh reviewer, who reads the PR's earlier attributed review reports. Reviewers report findings and do not commit fixes. Infrastructure reviews apply [cost decisions](planning.md#cost-decisions).
- A PR has at most two review stages ([ADR 0025](adr/0025-two-review-stages.md)):
  - the **source review**, of the source and evidence at the reviewed head: one reviewer, or, above about 1,000 changed lines, one reviewer per function group listed in the PR body, in parallel;
  - one **re-check** of the fixing commits, only when a fix changed behaviour, test evidence or a body's operations. Its spawn names the fixing commits; it covers them all in one task and writes the PR's final disposition. Fixes limited to names, comments, types or argument structs with unchanged operations need none; the implementer says so in the status comment.
- The implementer fixes each finding, or coherent group, in its own commit that names the finding and ends with `Found-by: review`.
- A fresh reviewer, scoped to the new work, also reviews a substantive rewrite, a base update that needs review ([integration](#integration)), and a simulation behaviour fix that CI found, arithmetic and assertion fixes included.
- The implementer or root may commit fixes for CI warnings or platform errors, or record speed medians, without a new review. CI checks those commits.
- Carries and growth or suppression proposals go in the branch's [cleanup records](rust-migration.md#quick-cleanup-checks) file (`migration/cleanup-records/<branch>.json`), not the PR body. The reviewer's attributed report accepts a proposal under the tested head. Growth still awaiting that acceptance is pending review on PR runs and fails in the merge group.
- Reviewer and implementer message each other directly, by the thread IDs their briefs give, and root relays nothing. The reviewer posts its report, then tells the implementer in one line.
- Reviewers rule on policy questions themselves ([review brief](review-brief.md)). The implementer asks root only when it and a reviewer disagree, or a reviewer raises a question no recorded decision answers. Root reads dispositions, not diffs.
- A review that stops before recording findings and disposition counts as no review.

### Integration

Every merge goes through GitHub's merge queue ([ADR 0019](adr/0019-merge-queue.md)): queue with `python3 tools/ci.py enqueue`, never `gh pr merge`, `--admin` or a hand merge. What may queue at all is in [root's integration rules](root-workflow.md#integration).

1. **Base update.** A PR that conflicts with base cannot queue. Update from base only for a conflict or a change the PR needs from base, since the queue already tests every PR on the current base. Merge `origin/rust-migration` in a commit titled `Update: Merge rust-migration into BRANCH`; `gh pr update-branch` titles it `Merge branch ...`, which the commit checker rejects.
2. **Review after a base update.** A reviewed PR's base update needs no new review if CI passes and one of these holds; record which on the PR:
   - the update has no conflicts in `src/`, `rust/`, `tools/` or `.github/`;
   - it resolves every conflict by keeping both sides' lines unchanged, so `git show --remerge-diff` removes only conflict markers.

   Such an update needs only CI: no recount, local build, proof or new evidence. Otherwise a fresh reviewer checks only the conflict resolution. Record metrics once, at the head that merges.
3. **Workflow changes.** A PR that changes `.github/workflows/` queues only after root's reviewed bootstrap Full on its head ([CI administration](root-workflow.md#ci-administration)).
4. **Queue.** When every reviewer accepts, any re-check due has accepted, and the status comment records the passing `native` run at the head, run `python3 tools/ci.py enqueue PR` at that head. It refuses, naming the run, unless the head's latest `Quick validation` and its latest requested or bootstrap run are green.
   It then reports whether the PR entered the queue or GitHub armed auto-merge, which queues it once pending required checks pass. Queue each PR alone and request no Full for it. The queue tests the base, the PRs queued ahead and this PR, and merges on a pass. A head that does not change keeps its reviews' head-bound acceptances.
5. **Wait.** Start `python3 tools/ci.py queue-wait PR` as a background job and take your next work. It exits 0 when the queue merges the PR (with the merge commit) and 3 when it removes it (with the failed merge-group run and jobs); on timeout, exit 2, start its printed resume command the same way. When the queue merges or removes the PR, tell root in one line with the PR, its head and the outcome.
6. **Removal.** A PR that fails in the queue leaves it; PRs behind it are retested automatically. Read the failed run to its last job before recording a cause, then fix the cause under [review](#review). A PR removed for a failure of its own queues again only after a base update and a preflight pass on the merged tree: `enqueue` refuses a head that is unchanged, behind base or without a stamp.
7. **Retry.** After a runner or network failure, queue it again once, without a fix, with `python3 tools/ci.py enqueue PR --retry REASON`, which is logged and refused once the same head has failed twice. Such a failure is a step before any build or test, such as package install, checkout or cache download, or a lost runner with no test output. A network-protocol scenario hang counts too; record its evidence on #693. A second failure of the same job goes to root.

While root has [stopped merges](root-workflow.md#merge-stops), queue nothing.

## Automatic rollback and wind-down

Root follows [automatic rollback and wind-down](root-workflow.md#automatic-rollback-and-wind-down).
