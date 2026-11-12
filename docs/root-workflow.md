# Root change workflow

Readers: root, when deciding what may merge, stopping merges, administering CI, taking stock, reclaiming output or winding down; the routine agent, for the procedures root hands it.
Implementers take their PRs through review and the merge queue under the [PR lifecycle](workflow.md#pr-lifecycle).

## Integration

A merge is one merged PR. Merges are numbered from the switch to the merge queue; earlier merges were called integrations and are numbered separately, up to 147. Root counts merges from the first-parent history of `origin/rust-migration`. Every merge goes through the merge queue ([ADR 0019](adr/0019-merge-queue.md)), by the [procedure](workflow.md#integration) in the PR lifecycle.

- Each ownership port has one open PR at a time, targeting `rust-migration` directly.
- Queue an owner's checkpoint only once it meets all of the following:
  - It removes at least 100 fork C++ lines net, counting C++ under `tools/` and `src/tests`. This threshold is provisional: a checkpoint should bank enough removed C++ to repay a merge's review and queue time.
  - No moved field remains behind a service.
  - It has no boundary design fault.
  - It passes review.

  The owner continues in a fresh PR on the same issue. A checkpoint that increases fork C++ does not merge; take the [cost decision](planning.md#cost-decisions) first.
- Cleanup-stage PRs instead pass their stream's proof in the roadmap, grow neither fork C++ nor the boundary, and merge on review and the queue's Full. A #734/#735 PR finishes every file it touches ([work order](roadmap.md#cleanup-work-order)); it is not a checkpoint of a larger owner.
- A completion claim, in a PR or the roadmap, names only the scope delivered.
- A work order may batch small maintenance items in one PR with one reviewer: cleanups, coverage, speed, tooling and fidelity fixes. An item that fails review or CI moves to the next batch. A fix needed by a draft or a live crash may go alone.
- Keep at most six unmerged component branches, including drafts and WIP, so branches do not drift into conflict with base. Queue reviewed work before starting a component. Start from current `rust-migration` unless the roadmap names a dependency. Agent slots ([team](team.md#slots)) are a different limit: reviews, planning and measurements need no component branch.
- A shared accessor or service is never such a dependency. The first PR to merge adds it to the shared layer in final typed form; other drafts adopt it when they update from base. Root builds a shared service or shared-path fix needed by several drafts once, in its own small PR from `rust-migration`. Drafts do not build parallel copies.
- A sweep across most of `src/` merges before any other `src/` change. Other drafts continue implementation and review but queue only after it merges, because each merge ahead of it would force a new sweep. No draft waits for another draft's services. A reviewed PR gains no new scope before it merges.

### Merge stops

A red [platform cadence](#cadences) or a confirmed code regression on `rust-migration` stops merges. Root decides whether a red result is such a stop, then removes each queued PR from the queue:

```sh
gh api graphql -f query='mutation($id:ID!){dequeuePullRequest(input:{id:$id}){mergeQueueEntry{id}}}' -f id="$(gh pr view N --json id --jq .id)"
```

Implementers queue nothing until root lifts the stop. A removed PR stays open and can be queued again. A confirmed regression also triggers [automatic rollback](#automatic-rollback-and-wind-down).

## Cadences

| When | Who | What |
| --- | --- | --- |
| Session start | [Routine agent](team.md#routine-agent) | [Housekeeping](#housekeeping) |
| After every fifth merge | Routine agent runs and reports; root decides | Platform cadence: `gh workflow run ci-build.yml --ref rust-migration`, which runs macOS and the platform comparison ([ADR 0018](adr/0018-builds-before-every-merge.md)); a red result [stops merges](#merge-stops) until fixed. Housekeeping runs with it. |
| After every twelfth merge | Root | Stocktake under [`/start-development` step 4](skills/start-development/SKILL.md) ([ADR 0013](adr/0013-stocktake-every-12-merges.md)). Profile the merged tip in the background ([speed](speed.md#instruction-projections), [ADR 0027](adr/0027-speed-measured-on-merged-branch.md)); a running profile counts as one of the two implementers. Check issue placement and exclusion conditions, and report #892's remaining production macro counts beside the #734 inventory. |
| A push to `rust-migration` that no Full covered | CI | The cadence's macOS and platform comparison run on their own. A push is covered when its tree, ignoring docs-only files, already passed Full validation. |
| Cleanup exit and phase-3 exit | Root | All-member deep check ([ADR 0017](adr/0017-deep-checks-on-request.md), [deep checks](deep-checks.md#policy)) |
| Stocktake, change of course, wind-down | Root | [Roadmap commit](workflow.md#records) |

Full also builds Linux Clang and GCC Dedicated, so the cadence's builds of them repeat a tested tree. `Rust tests (32-bit)` runs in `native`, Full and the cadence.

## CI administration

- `CI_ON_DEMAND=true` enables `tools/ci.py` requests after root installs and verifies the status; without it automatic Full remains the bootstrap.
- A **bootstrap Full** is a Full run requested on a PR head with `--profile full --bootstrap`. Only a PR that changes `.github/workflows/` gets one: the queue runs a PR's own workflow definitions with write permissions, so root reviews them and they run once on the PR head first.
- For such a PR, root reviews its workflow after the reviewer accepts the source. The [routine agent](team.md#routine-agent) then:
  1. unsets `CI_ON_DEMAND`;
  2. requests `--profile full --bootstrap` through `ci:full`;
  3. sets the variable back to `true` as soon as that run's `plan` job has succeeded;
  4. reruns any "Invalidate previous validation" run skipped for a PR push inside that window, since such a push otherwise has no `Full validation` status.

  Only the `plan` job's condition reads the variable; while it is unset, every PR push runs the automatic heavy jobs and skips the invalidation that admits a head to the queue. Record both variable changes in the PR. Root decides whether the result lets the PR queue. The implementer queues it when the run passes and pushes nothing in between: a new head needs a new bootstrap Full.
- After two hosted-capacity cancellations of a macOS run, run `gh variable set CI_MACOS_HEAVY --body blacksmith-6vcpu-macos-26`, rerun, then `gh variable delete CI_MACOS_HEAVY` when it finishes. Which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)); this fallback is part of it.

## Stocktake facts

| Command | Output |
| --- | --- |
| `python3 tools/retrospective.py readiness 705 704 --json` | Observed head, PR base (its last sync point), current base-branch tip, mergeability, report SHAs/links, Full status including carry description, and changed-file intersections between the supplied PRs. |
| `python3 tools/retrospective.py latency 705 704 --json` | Available PR/review/Full-run milestones and completed created-to-merge intervals, with both summed per-PR seconds and their elapsed union. |
| `python3 tools/retrospective.py stocktake START END` | Same-version whole-fork endpoint delta table; `--json` retains both full metric endpoints and scope metadata. |

- These commands read through the existing CI/`gh` helpers and metric. A matching report SHA is a text fact, not an inferred independent approval under shared credentials.
- Readiness is an observation, not merge authorization; `stable_snapshot` flags a head, PR-base or base-tip change during collection.
- Absent milestones remain absent and causes remain unknown. Full workflow `updated_at` is labelled as its last update, not a precise completion time; a dispatch run's Git head belongs to the workflow branch, so it is never substituted for the PR head. Edited report text uses the latest edit time.
- Issue placement is under [planning](planning.md#issue-placement-and-deferral).

## Cleanup

- A worktree and local branch exist only while their PR is open, or before it while the roadmap work order holds the issue; an open PR keeps one. Once commits are pushed, root removes a review or update task's own worktree. For a merged PR use `git worktree remove PATH` and `git branch -d BRANCH`; these refuse dirty, locked or unmerged state. GitHub deletes the merged head branch.
- When work stops for good (closed, superseded or paused with no next step), close its draft PR, keep its branch on origin, record its head on the issue, and remove its worktree and local branch without archiving or copying anything out.
- `.local/` holds caches and rebuildable output. Delete ad hoc output once the PR or issue records it; establish provenance before calling a file user-owned, since untracked files may be prior agents' output.
- Preserve speed anchors, raw traces, frozen runtimes and failed outputs, and the runtimes, libraries and profiles that current speed anchors or active evidence reference, until replacements are recorded.

### Housekeeping

`python3 tools/housekeeping.py` lists stale worktrees and `.local` output with sizes and reasons; `--apply` removes the listed `remove` entries. The tool's docstring and `--help` describe what it removes. The routine agent runs it on the [cadence](#cadences). Before `--apply`, know:

- Read the dry run, then check once more for live output before applying.
- Naming a path or run in the roadmap, or in an open issue or PR body, protects it, shortened T3 names and paths through output symlinks included. Removing the name releases it.
- A lock stays until every issue or PR its note names (`#N` or a URL) is closed or merged, so name the work in a lock note.
- The tool keeps what it cannot classify, such as unknown `.local` entries and reports without a commit, and reports them. Those need a manual decision.
- It never removes the main checkout, `.local/reference`, or a dirty or currently locked tree, and keeps a tree whose live processes it cannot inspect.

## Automatic rollback and wind-down

A confirmed code regression on `rust-migration` [stops merges](#merge-stops). When the breaking merge has only docs commits after it, or none, root removes it without asking ([ADR 0016](adr/0016-automatic-rollback.md)), by the procedure below: preserve its work remotely, restore its first parent plus corrected docs, and report afterwards. This holds even during wind-down, without a fix PR or new CI wait. A runner or API failure alone is not a code regression. If later code prevents this bounded rollback, preserve the state and handle recovery before further merges; during wind-down, hand it off instead of starting repairs.

Wind-down is when the user pauses the project. It is bounded by actions, not time estimates:

- Finish an edit underway and save its state even if incomplete; finish an active review by recording findings.
- Start no new implementation, review, repair round, build, simulation, CI request or retry; do not wait on CI. Queue no PR. A PR already in the queue stays there and merges on a pass; one the queue removes, or one that needs a base update, waits for resume.
- Perform an eligible automatic rollback, record each unfinished item's next action and running jobs, report and stop. Pending or later CI results do not restart wind-down.
- For resume, preserve each open PR's one checkout and every worktree holding uncommitted or unpushed work. Remove review and update checkouts whose commits are pushed, and verified merged work: a checkout is not unfinished because its PR is, and kept checkouts cost disk.
- The user's explicit action scope overrides these defaults.

### Rollback procedure

Read the failing result and identify its code cause; a red check does not justify rolling back an arbitrary latest commit.

1. Fetch the current remote tip; identify the breaking merge and its first parent. Check that later changes are docs-only. Preserve the old tip on an origin recovery branch and record the component PR's head on its issue before changing the shared branch. Do not discard dirty work in any checkout.
2. Prepare the replacement from that first parent in a clean checkout. Reapply later docs with completion claims corrected for removed work; keep new merge or rollback rules among those docs.
3. Push with an explicit lease: `git push --force-with-lease=refs/heads/rust-migration:OLD_REMOTE_SHA origin REPLACEMENT_SHA:refs/heads/rust-migration`. If it fails, fetch and reassess; never use bare `--force`. Root may temporarily enable force pushes if protection requires it. Preserve all other settings, restore the prior force-push setting even on failure, and read it back.
4. Reopen the tracking issue if closed; record the recovery branch, head and next repair step, and report the reset afterwards. GitHub keeps the old PR marked merged; it cannot be reopened. Resume through a replacement PR linked to it. Wind-down records that next step without opening the repair PR or waiting for post-reset CI. Keep the recovery ref until replacement work is safely retained; do not restore the rejected merge from a stale local branch.

Report a failed recovery as failed, never as a reset.
