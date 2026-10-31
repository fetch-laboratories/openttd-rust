# Change workflow

Readers: Every agent changing the repository.

## Change types

- In a user-review session, commit a settled docs-only change locally; amend each correction into that unpushed commit and retitle it for the final text. Push on user approval, a move to another subject, or before root or a delegate needs it on origin. Collect corrections to an already pushed change in one new commit the same way. Push pending docs-only commits at session start.
- Docs-only means only `docs/` or `*.md` files, including roadmap updates and skill edits under `docs/skills/`. Commit directly, one commit per change, and push without asking; an agent working alone pushes as soon as the change is done. Code, tools, workflows and saves go through a PR that passes CI; never use `--admin`.

## Records

- Keep evidence checkable; spend effort on code and checks, not exhaustive prose. Record changes, checks and remaining limits in PRs; migration documentation records design decisions and open limits, not evidence or review history. Scaffolding does not complete a subsystem.
- The [migration issue form](../.github/ISSUE_TEMPLATE/migration.yml) covers scope, affected interfaces, evidence plan and acceptance criteria; leave design to implementation. Use the [PR template](../.github/PULL_REQUEST_TEMPLATE.md) for what moved, what stays in C++, exact commands, limits and `python3 tools/port-metrics.py` output. Integration PRs link component PRs instead of re-describing them.
- These are contents, not line counts: omit what belongs elsewhere, but retain needed facts, limits and steps (user, 2026-10-26).
- Keep the roadmap forward-looking without a line cap. Update it in place: a completed item becomes one row with issue, PR, commit and metrics; evidence stays in the PR. Replace the resume checkpoint with a short status table, not a log. Put CI run IDs, executable hashes, probe values and per-PR plans in the PR or issue. Do not delete or fold another section to make room; each `/steer` run prunes the roadmap.
- Evidence is exact-head CI runs and commands recorded in the PR. Do not put receipt bundles, executable hashes or `.local/` paths in comments or docs. Track tooling a rule depends on, such as the speed-pass collector, in `tools/`; do not copy scripts between `.local/` runs.
- Edit one status comment per task on its PR, or issue before the PR, at start, milestones and stop. Other PR comments are review reports and root's integration note. A tracker issue body is its current list, edited as items open or close; a task with a PR does not also post status on the tracker.
- Docs use structured text: headings, tables and bullets; bullets may be clauses. Prose in issues, comments and commit messages uses full sentences. Use a space after every colon and between a word and a number, and "X of Y" (user, 2026-10-30).
- Put rules every agent needs in [AGENTS.md](../AGENTS.md). Put other rules in the doc named by its routing table. Keep each full rule in one place. A one-line reminder of an important rule may repeat where an agent would otherwise miss it; link to the full rule (user, 2026-10-30).

## Branches and worktrees

For substantive changes create a fork issue with a clear owner, then use an isolated branch and worktree based on `rust-migration`. The implementation agent opens a draft PR targeting `rust-migration`, links the issue and records exact validation commands and limits. See [team](team.md#claude-code-root) for Claude Code worktree creation.

## Commits and pushes

- The inherited commit checker examines every commit's diff and title. Fix style in the introducing commit; use supported prefixes such as `Add:`, `Change:`, `Fix:`, `Doc:` or `Update:`, including integration merges (`Merge:` is rejected).
- Commit messages, including bodies, use ASCII without tabs. A fidelity or crash fix ends with `Found-by:` naming the check: `review`, `scenario NAME`, `comparison TOOL`, `unit test`, `CI JOB`, `deep check` or `user` (#512).
- Before pushing, run `python3 tools/preflight.py --base origin/rust-migration`.
- Keep an unfinished checkpoint's PR in draft, recording what is incomplete, pending or failed checks and the next action. A checkpoint is not integration evidence. A review that stops before recording findings and disposition counts as no review.

## Checks

- Iterate locally with the build, Cargo checks and simulation scenarios touched by the change. Do not run the full default local `simulate` suite (38 minutes locally); exact-head Full runs it in about 12. Add no second full local suite or pre-merge local build gate.
- Ordinary PR pushes run cheap checks; do not use `[skip ci]` on code pushes. Use `python3 tools/ci.py request PR --profile rust|native|platform` for diagnostic subsets; request `platform` only for a suspected platform failure. Avoid docs pushes during final validation because they advance base.
- Before requesting Full for a PR removing a name, grep `tools/simulation/` and `src/tests/` for it: fixtures for scenarios not run locally compile only in Full.
- See [continuous integration](rust-migration.md#continuous-integration) for Full jobs, platform cadence and `CI_MACOS_HEAVY`.

## Review

- Keep the [reference worktree unchanged](../AGENTS.md#every-agent).
- For infrastructure work (CI, caches, evidence plumbing), fix only defects that can change a game comparison or a speed decision on this host or in CI; list hypothetical ones in the PR's limits ([cost decisions](planning.md#cost-decisions)).
- A separate reviewer examines the final commit and evidence. The reviewer checks fidelity first. Compare each replaced original body callee by callee before checking boundary rules. For a port retiring more than about 1,000 original lines, assign one reviewer per function group in parallel.
- The reviewer commits each fix to the PR branch. Rerun only checks affected by those fixes; a review without fixes reruns nothing. List each finding, its fixing commit and the check that found it. Root spot-checks the fix commits before integration. Send findings that need a scope or policy decision to root unfixed.
- For every new binding outside the rebound check's scope, the reviewer greps its callee across `src/`. This includes macros and compound callbacks, such as calls to `CheckOwnership` or `GetRoadOwner`. Report each existing wrapper or binding as a finding. See [shared accessors](porting.md#callees-and-shared-accessors).
- Use the review report skeleton in the [PR template](../.github/PULL_REQUEST_TEMPLATE.md). Start with `Reviewed commit: <sha>`. Name the functions or files compared with original bodies, duplicate callbacks, and orphaned `WITH_RUST` bodies, exports or declarations. Each orphan is a finding. Name every `WITH_RUST` body and `openttd_rust_*` export touched and its production caller. Once #564's quick-CI orphan-export check integrates, cite it for exports and name only `WITH_RUST` bodies. Add no narration when there are no findings. Put branches the harness does not reach in #156, not only in the disposition.
- The implementer or root may commit fixes for CI warnings or platform errors, or record speed medians, without a new review. CI checks those commits. A simulation behavior fix found by CI needs a fresh scoped review, including arithmetic or assertion fixes.
- A reviewed PR's base update needs no new review if CI passes and one of these conditions holds:
  - The update has no conflicts in `src/`, `rust/`, `tools/` or `.github/`.
  - It resolves every conflict by keeping both sides' lines unchanged. `git show --remerge-diff` removes only conflict markers.
  - It reruns #581's recorded `unifdef` command over every touched `src/` file.
- Root records the applicable condition on the PR. Otherwise a fresh reviewer checks and fixes only the conflict resolution.

## Integration

- Each ownership port gets one PR targeting `rust-migration` directly.
- Batch ready small maintenance items in one PR. These include cleanups, coverage, speed, tooling and fidelity fixes. List them on the tracking issue's checklist (#156, #275, #449-#452). Use one reviewer and one Full run. Move an item that fails review or CI to the next batch. A fix needed by a draft or live crash may go alone.
- Keep at most six unintegrated component branches, including drafts and WIP (#157). Integrate reviewed work before starting a component. Start from integrated `rust-migration` unless the roadmap names a dependency.
- A shared accessor or service is never such a dependency. The first integrating PR adds it to the shared layer in final typed form; other drafts adopt it when they update from base. Root builds a shared service or shared-path fix needed by multiple drafts once in its own small PR from `rust-migration`. Drafts do not build parallel copies.
- Integrate drafts in readiness order. No draft waits for another draft's services. A reviewed PR gains no new scope before integration.
- Batch two or three ready, reviewed component heads only when their diffs touch disjoint files. Integrate a lone ready PR without waiting. Integrate overlapping components one at a time at their reviewed heads.
- Assemble an integration PR from current base. Preserve component commits and link their reviews. Review only substantive conflict resolutions under [review](#review); unchanged components need no new review. Full on the combination needs no separate component Full first. A stack organizes dependent PRs but does not combine validation runs. No extra local integration build or full local simulation is required. Keep batches small: the integration merge is the rollback unit.
- Never use `--admin` or merge past CI. See [AGENTS.md](../AGENTS.md#every-agent).

For final validation and merge, root follows these steps:

1. Finish routine base-doc updates. Update the PR or integration PR from current `origin/rust-migration`. Update from base only when preparing final Full or resolving a blocking conflict, not continuously.
2. Freeze batch membership. Coordinate root sessions to avoid base pushes during the run; implementation and review continue.
3. Request `python3 tools/ci.py request PR --profile full` for the exact head and wait. Only a successful Full at that head publishes `Full validation`. Any head, membership or base change requires a fresh Full, including removal of a failing component. Strict protection also requires it after a push or base move. A clean textual merge, local build, older success or post-merge CI is no substitute.
4. Serialize integration. Merge only the passing current head with `gh pr merge --match-head-commit` and the reviewed SHA. Never bypass the gate. A failure leaves the shared branch unchanged.
5. Record completion and close included component PRs. Clean up as described below.

## Cleanup

- A worktree and local branch exist only while their PR is open, or before it while the roadmap work order holds the issue; an open PR keeps one. Once commits are pushed, root removes a review or update task's own worktree. For a merged PR use `git worktree remove PATH` and `git branch -d BRANCH`; these refuse dirty, locked or unmerged state. GitHub deletes the merged head branch.
- When work stops for good (closed, superseded or paused with no next step), close its draft PR, keep its branch on origin, record its head on the issue, and remove its worktree and local branch without archiving or copying anything out.
- `.local/` holds caches and rebuildable output. Delete ad hoc output once the PR or issue records it; establish provenance before calling a file user-owned, since untracked files may be prior agents' output. Preserve runtimes, libraries and profiles referenced by current speed anchors or active evidence until replacements are recorded.

### Housekeeping

- At session start and after integrations, root runs `python3 tools/housekeeping.py` and reads the dry run. It lists sizes and reasons. After a final live-output check, root uses `--apply` for eligible `remove` entries.
- The tool removes linked worktrees whose branch merged into `origin/rust-migration` or whose PR merged or closed. It uses `git worktree remove` without `--force`. It then deletes local branches if merged or held at the same head on origin. It keeps other no-PR branches.
- The tool never removes the main checkout, `.local/reference`, or dirty or locked trees. It keeps a live worktree's `.local`.
- The tool may remove a clean detached or no-commit T3 task worktree only if no process has its cwd inside. The roadmap and open issue and PR bodies must not name the path, including shortened T3 names. Current deep reports, speed anchors and pinned runtimes must not retain its output. If cwd inspection fails, the tool keeps the tree.
- In the main checkout's `.local`, the tool removes finished `simulation`, `verification`, `coverage` and `deep` runs when the report's `candidate_commit` is an ancestor of `origin/rust-migration`. For deep reports it uses `commit`. It keeps other finished runs as possible PR evidence and reports without a commit as unclassified.
- The tool removes unfinished runs idle for 12 hours and loose logs. It first clears read-only bits. It keeps unknown entries for a manual decision.
- The tool keeps the current deep report and its runs, and each scenario's newest candidate `callgrind.out`. It keeps shared builds, caches, toolchains, locks, the reference, retained speed profiles and CI request records. It also keeps folders containing `provenance.json` for pinned runtimes.
- The tool checks live cwd and output again at deletion. Linux proc descriptors protect same-user writable outputs and containing candidates by path and file identity during planning and immediately before deletion. If inspection is incomplete, every regular file must admit a kernel read lease. The tool retains candidates with live writers or unavailable leases. An output opened after planning can keep a listed candidate.

## Automatic rollback and wind-down

A confirmed code regression on `rust-migration` stops integrations (user, 2026-10-22). When the breaking merge has only docs commits after it or none, root automatically removes it without asking, using the rollback procedure below: preserve its work remotely, restore its first parent plus corrected docs, and report afterwards. This is authorized even during wind-down, without a fix PR or new CI wait. Runner or API failure alone is not a code regression. If later code prevents this bounded rollback, preserve the state and handle recovery before further integrations; during wind-down hand it off instead of starting repairs.

Wind-down is bounded by actions, not time estimates. Finish an edit underway and save its state even if incomplete; finish an active review by recording findings. Start no new implementation, review, repair round, build, simulation, CI request or retry; do not wait on CI. Merge only already-reviewed, already-validated combinations that still meet the exact-head Full gate; a required base update leaves the PR for resume. Perform an eligible automatic rollback, record each unfinished item's next action and running jobs, report and stop. Pending or later CI results do not restart wind-down. Preserve unfinished worktrees and branches for resume; clean up only verified merged work. The user's explicit action scope overrides these defaults.

### Rollback procedure

Read the failing result and identify its code cause; a red check does not justify rolling back an arbitrary latest commit.

1. Fetch the current remote tip; identify the breaking merge and its first parent. Check that later changes are docs-only. Preserve the old tip on an origin recovery branch and record the component PR's head on its issue before changing the shared branch. Do not discard dirty work in any checkout.
2. Prepare the replacement from that first parent in a clean checkout. Reapply later docs with completion claims corrected for removed work; keep new integration or rollback rules among those docs.
3. Push with an explicit lease: `git push --force-with-lease=refs/heads/rust-migration:OLD_REMOTE_SHA origin REPLACEMENT_SHA:refs/heads/rust-migration`. If it fails, fetch and reassess; never use bare `--force`. Root may temporarily enable force pushes if protection requires it. Preserve all other settings, restore the prior force-push setting even on failure, and read it back. Administrative access also permits documented direct docs pushes, never bypassing validation for new code integration.
4. Reopen the tracking issue if closed; record the recovery branch, head and next repair step, and report the reset afterwards. GitHub keeps the old PR marked merged; it cannot be reopened. Resume through a replacement PR linked to it. Wind-down records that next step without opening the repair PR or waiting for post-reset CI. Keep the recovery ref until replacement work is safely retained; do not restore the rejected merge from a stale local branch.

Report a failed recovery as failed, never as a reset.
