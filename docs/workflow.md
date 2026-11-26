# Change workflow

## Change types

- **Docs-only** means what CI and the pre-push hook treat as inert, because no build, test or check reads it:
  - `AGENTS.md`;
  - every file under `docs/`, whatever its type, except scripts (`.py`, `.sh`) and the files a build reads there: what CMake packages and the images Doxygen takes (the `builtDocs` list in `.github/workflows/ci-policy.yml`);
  - `.github/PULL_REQUEST_TEMPLATE.md` and `.github/ISSUE_TEMPLATE/`, which only GitHub's forms and agents read;
  - `.codex/`, the Codex host's settings.

  Check the whole diff, both paths of a rename included. Everything else goes through a PR that passes CI. A file joins the list once nothing in a build, test or check reads it. The list is `inert` in `tools/preflight_stamp.py` and in the three workflows that share its rule, and the push filter of `ci-build.yml` and `rust-migration.yml`; `tools/test-ci.py` holds them to one rule.
- Commit a docs-only change directly to `rust-migration`, one commit per change, and push it when done. These pushes use administrative bypass of branch protection, which covers docs-only pushes and nothing else.
- Push to `rust-migration` only while the merge queue is empty; hold the commit locally until then. Any push to the base rebuilds every queued PR's merge group and starts its Full run again, and after a queue removal the requeue check refuses a head that is behind by any commit that changes more than docs-only files. The pre-push hook reads the queue and refuses such a push while it holds an entry, naming the queued PRs.

## Records

- PRs record changes, checks and remaining limits; migration documentation records design decisions and open limits, not evidence or review history.
- A PR body follows the [PR template](../.github/PULL_REQUEST_TEMPLATE.md) and keeps the residue with reasons and the checks. Lint carries and growth or suppression proposals go in the branch's [cleanup records](rust-migration.md#quick-cleanup-checks) file, not the body.
- Evidence is exact-head CI runs and commands recorded in the PR. Do not put receipt bundles, executable hashes or `.local/` paths in comments or docs.
- Edit one status comment per task on its PR, or issue before the PR, at start, milestones and stop, with `python3 tools/ci.py comment PR --agent PATH --body-file FILE` (`--show` reads it, `--issue N` for an issue). A milestone is a pushed checkpoint whose checks pass, a blocking failure or a handoff, not each push or job start. A status comment holds the current state only: replace its text at each edit and append nothing. A tracker issue body is its current list, edited as items open or close.
- Root commits the roadmap only at a stocktake, a change of course or wind-down ([ADR 0026](adr/0026-roadmap-commit-timing.md)); the one exception is root's [ready-queue](team.md#ready-queue-and-pipelining) refill.
- Keep the roadmap forward-looking and update it in place: a completed item becomes one row with issue, PR, commit and metrics. CI run IDs, executable hashes, probe values and per-PR plans go in the PR or issue.
- Put rules every agent needs in [AGENTS.md](../AGENTS.md) and other rules in the doc its routing table names, each in one place.

## Branches and worktrees

A substantive change gets a fork issue, then a branch and worktree from `rust-migration` and a draft PR to `rust-migration` that links the issue.

## Commits and pushes

- Before pushing a change that is not docs-only, run `python3 tools/preflight.py` as a background job. Preflight compares the branch with its merge base, so a base that has moved since the branch forked is no reason for a [base update](#integration); Quick's gates compare the same way. The pre-push hook refuses a push whose tree has no preflight stamp, except a checkpoint push to a draft PR.
- A commit title starts with a prefix the pinned commit checker accepts; the `commit-msg` hook refuses any other title and says what to use.
- A fidelity or crash fix ends with `Found-by:` naming the check: `review`, `scenario NAME`, `comparison TOOL`, `unit test`, `CI JOB`, `deep check` or `user`.

## PR lifecycle

The implementer takes its own PR through review and the merge queue ([ADR 0012](adr/0012-root-decides-implementers-review-and-merge.md)); root watches no PR.

### Checks

- Scenario comparisons run on CI, not on this host ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)); run one locally, by name, only to diagnose a failure. Once the local build and Cargo checks pass, start `python3 tools/ci.py request PR --profile native --wait` with your host's [background-job](team.md#background-jobs) mechanics (exit 2 means still running, not failed: resume with `python3 tools/ci.py wait PR`), and record the passing run and its head in the status comment.
- That passing run is the PR's one `native` run. A review fix or a base update after it queues on `Quick validation` at its head: the queue's Full runs every job `native` runs ([jobs and profiles](ci.md#jobs-and-profiles)), and `enqueue` reads the PR's newest requested run on any of its commits. Request `native` again only to diagnose a failed Full: `request` refuses another while the PR's newest requested run is green, unless the queue has removed the PR since that run or the PR changes `.github/workflows/`, and `--again REASON` is for a failed Full it cannot see.
- Mark a draft PR ready with `python3 tools/ci.py ready PR --wait` as a background job: it waits for `Quick validation` at the head, which `enqueue` needs. Resume it with `python3 tools/ci.py wait PR --quick`. To wait out a run before `gh run rerun --failed`, use `wait --complete`; `wait --run RUN` waits on a run that has no request record.
- Request `--profile platform` only for a suspected macOS build failure, or once before queueing a change to `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` or `cmake/`: Full does not build macOS, and the [platform cadence](root-workflow.md#cadences) would find the break after it merges.

### Review

- The implementer starts its reviewers with T3 `delegate_task` and `isolation: "worktree"`, with the call `python3 tools/ci.py reviewer` prints ([starting reviewers](implementer-brief.md#starting-reviewers)): the [role table](team.md#roles-and-models)'s review model and the fixed message. A reviewer checks out the reviewed head, so one started in a shared checkout moves that checkout's head. Root starts no reviewers.
- Start two reviewers for the whole PR, in parallel, whatever its size: one for behaviour, which compares each changed body's operations with the original, and one for porting, which checks how each body reaches what it uses against [porting](porting.md) ([review brief](review-brief.md)). A PR that changes no game code, only tools, CI or records, gets the behaviour reviewer alone. A PR is not split into review groups ([ADR 0051](adr/0051-review-whole-pr.md)).
- Fix each finding in its own commit, or one per coherent group, ending `Found-by: review`. Where a rule, a ruling or a decision of the reviewed plan allows what a finding reports, leave the code and answer the finding in one line of the status comment, naming what allows it.
- A PR has at most two review stages ([ADR 0025](adr/0025-two-review-stages.md)): the source review, and one re-check of the fixing commits, by one reviewer for the whole PR, only when a fix changed behaviour, test evidence or a body's operations. Fixes limited to names, comments, types or argument structs with unchanged operations need none; the implementer says so in the status comment.
- The re-check is the PR's last review, whatever its disposition. Fix what it reports, or record in the PR body as residue what [porting](porting.md#design-faults) lets a PR leave, say in the status comment what the fixes change, and queue: no reviewer reads those fixes. A behaviour difference on input the game can produce goes to root first.
- Three changes made after a review still get one reviewer each, scoped to their commits and reporting on nothing else: a simulation behaviour fix that CI found (arithmetic and assertion fixes included) gets a behaviour reviewer; so does a base update with conflicts ([integration](#integration)); and a cleanup record, or an `unsafe_code` item whose code a fix changed, gets its acceptance from a porting reviewer. A fix that changes only an item's comments needs none.
- The implementer or root may commit fixes for CI warnings or platform errors, or record speed medians, without a new review.
- A review that stops before recording findings and disposition counts as no review.

### Integration

Every merge goes through GitHub's merge queue ([ADR 0019](adr/0019-merge-queue.md)): queue with `python3 tools/ci.py enqueue PR`, not `gh pr merge`, `--admin` or a hand merge. What may queue is in [root's integration rules](root-workflow.md#integration). While root has [stopped merges](root-workflow.md#merge-stops), queue nothing.

1. **Base update.** Update from base only for a conflict or a change the PR needs from base. Merge `origin/rust-migration` in a commit titled `Update: Merge rust-migration into BRANCH`; `gh pr update-branch` writes a title the commit checker rejects. Then search `rust/` and `src/rust/` for each accessor, binding and helper your PR adds: where the merged base now defines the same one, call the base's and delete yours.
2. **Review after a base update.** A reviewed PR's base update needs only CI if it has no conflicts in `src/`, `rust/`, `tools/` or `.github/`, or resolves every conflict there by keeping both sides' lines unchanged. Apply this test with `python3 tools/retrospective.py base-update [MERGE_SHA | --since REVIEWED_SHA]` and record its last line on the PR. Otherwise a fresh behaviour reviewer checks only the conflict resolution, in the files it lists.
3. **Workflow changes.** A PR that changes `.github/workflows/` queues only after root's reviewed bootstrap Full on its head ([CI administration](root-workflow.md#ci-administration)).
4. **Queue.** When the review stages due have reported and each finding is fixed, answered or recorded as residue, run `enqueue` at that head; if Quick at that head has not finished, wait for it first with `wait PR --quick`. Queue each PR alone and request no Full or further `native` for it ([checks](#checks)).
5. **Wait.** Start `python3 tools/ci.py queue-wait PR` as a background job and take your next work. When the queue merges or removes the PR, tell root in one line with the PR, its head and the outcome.
6. **Removal.** Read the failed run to its last job before recording a cause, then fix the cause under [review](#review) and merge base before queueing again. After a runner or network failure, a step before any build or test or a lost runner with no test output, queue it again once with `enqueue --retry REASON`. A `network-protocol-loopback` stall the harness already retried once (its warning names #693) is a failure to read, not one to queue again; a `content-protocol` evidence miss still counts as a network failure, and its evidence goes on #693. A removal whose cause is fixed without a commit, such as a cleanup acceptance posted after queueing, also queues again once with `--retry`, naming the run and the acceptance. `enqueue` itself refuses while any cleanup record lacks its acceptance; a PR's own runs print such a record as pending review and only the queue's run fails it.
