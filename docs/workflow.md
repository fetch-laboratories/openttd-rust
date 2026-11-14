# Change workflow

## Change types

- **Docs-only** means what CI and the pre-push hook treat as inert: `AGENTS.md` and Markdown (`.md`) under `docs/`, except the docs CMake packages for the in-game viewer (the `builtDocs` list in `.github/workflows/ci-policy.yml`). Check the whole diff, both paths of a rename included. Everything else goes through a PR that passes CI.
- Commit a docs-only change directly to `rust-migration`, one commit per change, and push it when done. These pushes use administrative bypass of branch protection, which covers docs-only pushes and nothing else.
- Push to `rust-migration` only while the merge queue is empty; hold the commit locally until then. Any push to the base rebuilds every queued PR's merge group and starts its Full run again, and after a queue removal the requeue check refuses a head that is behind by any commit. `gh pr list --search "is:queued"` lists the queue.

## Records

- PRs record changes, checks and remaining limits; migration documentation records design decisions and open limits, not evidence or review history.
- A PR body follows the [PR template](../.github/PULL_REQUEST_TEMPLATE.md) and keeps the review groups, the residue with reasons, and the checks. Lint carries and growth or suppression proposals go in the branch's [cleanup records](rust-migration.md#quick-cleanup-checks) file, not the body.
- Evidence is exact-head CI runs and commands recorded in the PR. Do not put receipt bundles, executable hashes or `.local/` paths in comments or docs.
- Edit one status comment per task on its PR, or issue before the PR, at start, milestones and stop, with `python3 tools/ci.py comment PR --agent PATH --body-file FILE` (`--show` reads it, `--issue N` for an issue). A milestone is a pushed checkpoint whose checks pass, a blocking failure or a handoff, not each push or job start. A tracker issue body is its current list, edited as items open or close.
- Root commits the roadmap only at a stocktake, a change of course or wind-down ([ADR 0026](adr/0026-roadmap-commit-timing.md)); the one exception is root's [ready-queue](team.md#ready-queue-and-pipelining) refill.
- Keep the roadmap forward-looking and update it in place: a completed item becomes one row with issue, PR, commit and metrics. CI run IDs, executable hashes, probe values and per-PR plans go in the PR or issue.
- Put rules every agent needs in [AGENTS.md](../AGENTS.md) and other rules in the doc its routing table names, each in one place.

## Branches and worktrees

A substantive change gets a fork issue, then a branch and worktree from `rust-migration` and a draft PR to `rust-migration` that links the issue.

## Commits and pushes

- Before pushing a change that is not docs-only, run `python3 tools/preflight.py --base "$(git merge-base origin/rust-migration HEAD)"` as a background job. Give it the merge base, not the branch's tip: preflight compares the two trees, so against a tip that has moved it reads each newer merge as lines your branch removed, and that is no reason for a [base update](#integration). The pre-push hook refuses a push whose tree has no preflight stamp, except a checkpoint push to a draft PR.
- A commit title starts with a prefix the pinned commit checker accepts; the `commit-msg` hook refuses any other title and says what to use.
- A fidelity or crash fix ends with `Found-by:` naming the check: `review`, `scenario NAME`, `comparison TOOL`, `unit test`, `CI JOB`, `deep check` or `user`.

## PR lifecycle

The implementer takes its own PR through review and the merge queue ([ADR 0012](adr/0012-root-decides-implementers-review-and-merge.md)); root watches no PR.

### Checks

- Scenario comparisons run on CI, not on this host ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)); run one locally, by name, only to diagnose a failure. Once the local build and Cargo checks pass, start `python3 tools/ci.py request PR --profile native --wait` with your host's [background-job](team.md#background-jobs) mechanics (exit 2 means still running, not failed: resume with `python3 tools/ci.py wait PR`), and record the passing run at the reviewed head in the status comment.
- Request `--profile platform` only for a suspected macOS build failure, or once before queueing a change to `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` or `cmake/`: Full does not build macOS, and the [platform cadence](root-workflow.md#cadences) would find the break after it merges.

### Review

- The implementer starts its reviewers with T3 `delegate_task` and `isolation: "worktree"`, using the [role table](team.md#roles-and-models)'s review model and the fixed messages in [starting reviewers](implementer-brief.md#starting-reviewers). A reviewer checks out the reviewed head, so one started in a shared checkout moves that checkout's head. Root starts no reviewers.
- Start two reviewers for the whole PR, in parallel: one for behaviour, which compares each changed body's operations with the original, and one for porting, which checks how each body reaches what it uses against [porting](porting.md) ([review brief](review-brief.md)). Above about 1,000 changed lines, list review groups in the PR body and start the two for each group, all in parallel.
- Fix each finding in its own commit, or one per coherent group, ending `Found-by: review`.
- A PR has at most two review stages ([ADR 0025](adr/0025-two-review-stages.md)): the source review, and one re-check of the fixing commits only when a fix changed behaviour, test evidence or a body's operations. Fixes limited to names, comments, types or argument structs with unchanged operations need none; the implementer says so in the status comment.
- A substantive rewrite gets fresh behaviour and porting reviewers scoped to it; a simulation behaviour fix that CI found (arithmetic and assertion fixes included) gets a fresh behaviour reviewer. That is a review of new work, not a third stage.
- The implementer or root may commit fixes for CI warnings or platform errors, or record speed medians, without a new review.
- A review that stops before recording findings and disposition counts as no review.

### Integration

Every merge goes through GitHub's merge queue ([ADR 0019](adr/0019-merge-queue.md)): queue with `python3 tools/ci.py enqueue PR`, not `gh pr merge`, `--admin` or a hand merge. What may queue is in [root's integration rules](root-workflow.md#integration). While root has [stopped merges](root-workflow.md#merge-stops), queue nothing.

1. **Base update.** Update from base only for a conflict or a change the PR needs from base. Merge `origin/rust-migration` in a commit titled `Update: Merge rust-migration into BRANCH`; `gh pr update-branch` writes a title the commit checker rejects.
2. **Review after a base update.** A reviewed PR's base update needs only CI if it has no conflicts in `src/`, `rust/`, `tools/` or `.github/`, or resolves every conflict there by keeping both sides' lines unchanged. Apply this test with `python3 tools/retrospective.py base-update [MERGE_SHA | --since REVIEWED_SHA]` and record its last line on the PR. Otherwise a fresh behaviour reviewer checks only the conflict resolution, in the files it lists.
3. **Workflow changes.** A PR that changes `.github/workflows/` queues only after root's reviewed bootstrap Full on its head ([CI administration](root-workflow.md#ci-administration)).
4. **Queue.** When every reviewer accepts and any re-check due has accepted, run `enqueue` at that head. Queue each PR alone and request no Full for it.
5. **Wait.** Start `python3 tools/ci.py queue-wait PR` as a background job and take your next work. When the queue merges or removes the PR, tell root in one line with the PR, its head and the outcome.
6. **Removal.** Read the failed run to its last job before recording a cause, then fix the cause under [review](#review) and merge base before queueing again. After a runner or network failure, a step before any build or test or a lost runner with no test output, queue it again once with `enqueue --retry REASON`; a network-protocol scenario hang counts too, and its evidence goes on #693. A removal whose cause is fixed without a commit, such as a growth or lint acceptance missing at the queued head, also queues again once with `--retry`, naming the run and the acceptance: a base merge there would move the head and void the acceptance again. Before queueing a head that review fixes or a CI fix moved, check that every acceptance the cleanup records need is posted at that head; a PR's own runs print growth as pending review and only the queue's run fails it.
