# Root change workflow

Readers: Root integrating changes, administering CI, taking stock, reclaiming output or winding down.
For issue, PR, commit and review procedures, see [change workflow](workflow.md).

## Integration

Under a Claude Code root, the [integrator](team.md#integrator) carries out this section, and root keeps the escalations listed there.

- Each ownership port has one open PR at a time, targeting `rust-migration` directly.
- Integrate an owner's checkpoint once it meets all of the following:
  - It removes at least 100 fork C++ lines net, counting C++ under `tools/` and `src/tests`. This threshold is provisional; integrations 93 to 96 took 14 to 70 minutes from review pass to merge. Cleanup-stage PRs instead pass their stream's proof in the roadmap, grow neither fork C++ nor the boundary, and integrate on review and exact-head Full. A #734/#735 PR finishes every file it touches ([work order](roadmap.md#cleanup-work-order)); it is not a checkpoint of a larger owner.
  - No moved field remains behind a service.
  - It has no boundary design fault.
  - It passes review and exact-head Full.

  The owner continues in a fresh PR on the same issue, and the PR claims only what it delivered. A checkpoint that increases fork C++ does not integrate; take the [cost decision](planning.md#cost-decisions) first.
- Batch ready small maintenance items in one PR. These include cleanups, coverage, speed, tooling and fidelity fixes. List them on the tracking issue's checklist (#156, #275, #449-#452). Use one reviewer and one Full run. Move an item that fails review or CI to the next batch. A fix needed by a draft or live crash may go alone.
- Keep at most six unintegrated component branches, including drafts and WIP (#157). Integrate reviewed work before starting a component. Start from integrated `rust-migration` unless the roadmap names a dependency.
- A shared accessor or service is never such a dependency. The first integrating PR adds it to the shared layer in final typed form; other drafts adopt it when they update from base. Root builds a shared service or shared-path fix needed by multiple drafts once in its own small PR from `rust-migration`. Drafts do not build parallel copies.
- Integrate drafts in readiness order, except a sweep across most of `src/`, such as #581 PR-B. It integrates before any other `src/` change; other drafts continue implementation and review and hold integration until it lands, because each integration would force a new sweep. No draft waits for another draft's services. A reviewed PR gains no new scope before integration.
- Batch two or three ready, reviewed component heads only when their diffs touch disjoint files. Integrate a lone ready PR without waiting. Integrate overlapping components one at a time at their reviewed heads.
- Assemble an integration PR from current base. Preserve component commits and link their reviews. Review only substantive conflict resolutions under [review](workflow.md#review); unchanged components need no new review. Full on the combination needs no separate component Full first. A stack organizes dependent PRs but does not combine validation runs. No extra local integration build or full local simulation is required. Keep batches small: the integration merge is the rollback unit.
- Spot-check reviewers' fix commits before integration ([review](workflow.md#review)). Under a Claude Code root, each finding's reviewer verifies them instead ([review loop](team.md#review-loop)).
- A reviewed PR's base update needs no new review if CI passes and one of these conditions holds:
  - The update has no conflicts in `src/`, `rust/`, `tools/` or `.github/`.
  - It resolves every conflict by keeping both sides' lines unchanged. `git show --remerge-diff` removes only conflict markers.
- Root records the applicable condition on the PR. Such an update needs only CI: no recount, local build, proof or new evidence. Record metrics once, at the head that integrates. Otherwise a fresh reviewer checks and fixes only the conflict resolution.

For final validation and merge, root follows these steps:

1. Finish routine base-doc updates. Update the PR or integration PR from current `origin/rust-migration`. Update from base only when preparing final Full or resolving a blocking conflict, not continuously.
2. Freeze batch membership. Coordinate root sessions to avoid merges during the run; implementation and review continue. Docs pushes may continue.
3. Request `python3 tools/ci.py request PR --profile full` for the exact head and wait. Only a successful Full at that head publishes `Full validation`. Any head, membership or base change requires a fresh Full, including removal of a failing component. Strict protection also requires it after a push or base move. The one exception is a push that keeps the tested non-Markdown tree: the invalidation workflow carries the passing status to the new head within seconds ([carried validation](rust-migration.md#continuous-integration)). After updating from base, read the head's `Full validation` status before requesting; request Full only if it is pending. A clean textual merge, local build, older success or post-merge CI is no substitute.
4. Serialize integration. Merge only the passing current head with `gh pr merge --match-head-commit` and the reviewed SHA. Never bypass the gate. A failure leaves the shared branch unchanged.
5. Record completion and close included component PRs. Clean up as described below.

## CI administration

- `CI_ON_DEMAND=true` enables `tools/ci.py` requests after root installs and verifies the status; without it automatic Full remains the bootstrap.
- For a workflow-changing PR, root reviews its workflow, temporarily unsets `CI_ON_DEMAND`, and requests `--profile full --bootstrap` through `ci:full`. After merge, restore the variable to `true` and record both changes in the PR.
- Run macOS and the [platform comparison](rust-migration.md#continuous-integration) on `rust-migration` after every fifth integration (user, 2026-10-22; #357): `gh workflow run ci-build.yml --ref rust-migration`. Fix a red result before further integrations. An uncovered protected push also runs them. Windows x86, Linux Clang and GCC Dedicated run in every Full (user, 2026-11-10; #860), so the dispatch's builds of them repeat a tested tree.
- After two hosted-capacity cancellations of a macOS run, run `gh variable set CI_MACOS_HEAVY --body blacksmith-6vcpu-macos-26`, rerun, then `gh variable delete CI_MACOS_HEAVY` when it finishes.

## Stocktake facts

| Command | Output |
| --- | --- |
| `python3 tools/retrospective.py readiness 705 704 --json` | Observed head, PR base (its last sync point), current base-branch tip, mergeability, report SHAs/links, Full status including carry description, and changed-file intersections between the supplied PRs. |
| `python3 tools/retrospective.py latency 705 704 --json` | Available PR/review/Full-run milestones and completed created-to-merge intervals, with both summed per-PR seconds and their elapsed union. |
| `python3 tools/retrospective.py stocktake START END` | Same-version whole-fork endpoint delta table; `--json` retains both full metric endpoints and scope metadata. |

These commands read through the existing CI/`gh` helpers and metric. A matching report SHA is a text fact, not an inferred independent approval under shared credentials. Readiness is an observation, not merge authorization; `stable_snapshot` flags a head, PR-base or base-tip change during collection. Absent milestones remain absent and causes remain unknown. Full workflow `updated_at` is labeled as its last update, not a precise completion timestamp; a dispatch run's Git head belongs to the workflow branch, so it is never substituted for the PR head. Edited report text uses the latest edit time. Issue placement remains `python3 docs/skills/steer/issues.py`.

## Cleanup

- A worktree and local branch exist only while their PR is open, or before it while the roadmap work order holds the issue; an open PR keeps one. Once commits are pushed, root removes a review or update task's own worktree. For a merged PR use `git worktree remove PATH` and `git branch -d BRANCH`; these refuse dirty, locked or unmerged state. GitHub deletes the merged head branch.
- When work stops for good (closed, superseded or paused with no next step), close its draft PR, keep its branch on origin, record its head on the issue, and remove its worktree and local branch without archiving or copying anything out.
- `.local/` holds caches and rebuildable output. Delete ad hoc output once the PR or issue records it; establish provenance before calling a file user-owned, since untracked files may be prior agents' output. Preserve runtimes, libraries and profiles referenced by current speed anchors or active evidence until replacements are recorded.

### Housekeeping

- At session start and after integrations, root runs `python3 tools/housekeeping.py` and reads the dry run. It lists sizes and reasons. After a final live-output check, root uses `--apply` for eligible `remove` entries.
- The tool removes linked worktrees whose branch merged into `origin/rust-migration` or whose PR merged or closed. It also removes a clean worktree with no PR of its own whose HEAD, beyond base, is on an `origin/` branch, such as a review or update checkout. It uses `git worktree remove` without `--force`. It then deletes local branches if merged or held at the same head on origin. It keeps other no-PR branches.
- A verified merged tree or a pushed head with a merged or closed PR bypasses the 12-hour recent-activity fallback. Other T3 tasks retain that fallback because delegates may be between commands. All worktrees still pass the live-process, output, dirty, lock and evidence-reference checks.
- A lock is stale when every issue or PR its note names (`#123` or a URL) is closed or merged. The tool reports it, `--apply` unlocks it, and the ordinary rules then apply. It keeps any other lock and reports its note and age.
- The tool reports worktree records whose folder is missing; `--apply` prunes them.
- The tool never removes the main checkout, `.local/reference`, or dirty or currently locked trees. It keeps a live worktree's `.local`.
- The tool may remove a clean detached or no-commit T3 task worktree only if no process has its cwd inside. The roadmap and open issue and PR bodies must not name the path, including shortened T3 names. Current deep reports, speed anchors and pinned runtimes must not retain its output. If cwd inspection fails, the tool keeps the tree, except for the user's session managers (`systemd`, `(sd-pam)`), whose cwd is hidden by design.
- In the main checkout's `.local`, the tool removes finished `simulation`, `verification`, `coverage` and `deep` runs when the report's `candidate_commit` is an ancestor of `origin/rust-migration`. For deep reports it uses `commit`. It keeps other finished runs as possible PR evidence and reports without a commit as unclassified.
- The tool removes unfinished runs and loose files in run folders idle for 12 hours, and loose logs. It first clears read-only bits. It keeps unknown entries for a manual decision.
- The tool keeps runs explicitly named in the roadmap or open issue and PR bodies, including paths through output symlinks and files inside runs. It also keeps the current deep report and its runs, and each scenario's newest candidate `callgrind.out`. It keeps shared builds, caches, toolchains, locks, the reference, retained speed profiles and CI request records. It also keeps folders containing `provenance.json` for pinned runtimes.
- The tool checks live cwd and output again at deletion. Linux proc descriptors protect same-user writable outputs and containing candidates by path and file identity during planning and immediately before deletion. If inspection is incomplete, every regular file must admit a kernel read lease. The tool retains candidates with live writers or unavailable leases. An output opened after planning can keep a listed candidate.

## Automatic rollback and wind-down

A confirmed code regression on `rust-migration` stops integrations (user, 2026-10-22). When the breaking merge has only docs commits after it or none, root automatically removes it without asking, using the rollback procedure below: preserve its work remotely, restore its first parent plus corrected docs, and report afterwards. This is authorized even during wind-down, without a fix PR or new CI wait. Runner or API failure alone is not a code regression. If later code prevents this bounded rollback, preserve the state and handle recovery before further integrations; during wind-down hand it off instead of starting repairs.

Wind-down is bounded by actions, not time estimates. Finish an edit underway and save its state even if incomplete; finish an active review by recording findings. Start no new implementation, review, repair round, build, simulation, CI request or retry; do not wait on CI. Merge only already-reviewed, already-validated combinations that still meet the exact-head Full gate; a required base update leaves the PR for resume. Perform an eligible automatic rollback, record each unfinished item's next action and running jobs, report and stop. Pending or later CI results do not restart wind-down. For resume, preserve each open PR's one checkout and every worktree holding uncommitted or unpushed work. Remove review and update checkouts whose commits are pushed, and verified merged work: a checkout is not unfinished because its PR is (11-05: 36 such trees held 85 GB). The user's explicit action scope overrides these defaults.

### Rollback procedure

Read the failing result and identify its code cause; a red check does not justify rolling back an arbitrary latest commit.

1. Fetch the current remote tip; identify the breaking merge and its first parent. Check that later changes are docs-only. Preserve the old tip on an origin recovery branch and record the component PR's head on its issue before changing the shared branch. Do not discard dirty work in any checkout.
2. Prepare the replacement from that first parent in a clean checkout. Reapply later docs with completion claims corrected for removed work; keep new integration or rollback rules among those docs.
3. Push with an explicit lease: `git push --force-with-lease=refs/heads/rust-migration:OLD_REMOTE_SHA origin REPLACEMENT_SHA:refs/heads/rust-migration`. If it fails, fetch and reassess; never use bare `--force`. Root may temporarily enable force pushes if protection requires it. Preserve all other settings, restore the prior force-push setting even on failure, and read it back. Administrative access also permits documented direct docs pushes, never bypassing validation for new code integration.
4. Reopen the tracking issue if closed; record the recovery branch, head and next repair step, and report the reset afterwards. GitHub keeps the old PR marked merged; it cannot be reopened. Resume through a replacement PR linked to it. Wind-down records that next step without opening the repair PR or waiting for post-reset CI. Keep the recovery ref until replacement work is safely retained; do not restore the rejected merge from a stale local branch.

Report a failed recovery as failed, never as a reset.
