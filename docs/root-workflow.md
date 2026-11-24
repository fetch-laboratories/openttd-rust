# Root change workflow

Implementers take their PRs through review and the merge queue under the [PR lifecycle](workflow.md#pr-lifecycle).

## Integration

A merge is one merged PR, counted from the first-parent history of `origin/rust-migration` since the switch to the merge queue.

- Queue an owner's checkpoint only once it meets all of the following, then continue in a fresh PR on the same issue:
  - It removes at least 100 fork C++ lines net, counting C++ under `tools/` and `src/tests` (provisional).
  - No moved field remains behind a service.
  - It has no boundary design fault.
  - It passes review.

  A checkpoint that increases fork C++ does not merge; take the [cost decision](planning.md#cost-decisions) first.
- Cleanup-stage PRs instead pass their stream's proof in the roadmap, grow neither fork C++ nor the boundary, and merge on review and the queue's Full. A #734/#735 PR finishes every file it touches ([work order](roadmap.md#cleanup-work-order)).
  - Callback slots may grow where a pass replaces a carrier written for one call site with shared typed accessors, and fork C++ does not grow with them. The porting reviewer accepts the growth record, and the record names the pass or owner that retires the slots.
- A completion claim, in a PR or the roadmap, names only the scope delivered.
- A work order may batch small maintenance items in one PR: cleanups, coverage, speed, tooling and fidelity fixes.
- Keep at most six unmerged component branches, drafts and WIP included. Queue reviewed work before starting a component. Start from current `rust-migration` unless the roadmap names a dependency.
- A shared accessor or service is not such a dependency. The first PR to merge adds it to the shared layer in final typed form; other drafts adopt it when they update from base. Root builds a shared service or shared-path fix needed by several drafts once, in its own small PR from `rust-migration`.
- A sweep across most of `src/` merges before any other `src/` change; other drafts continue but queue only after it merges. A reviewed PR gains no new scope before it merges, and no condition from a rule root changed after its review began, unless the PR cannot merge correctly without it.

### Merge stops

A red [platform cadence](#cadences) or a confirmed code regression on `rust-migration` stops merges. Root decides whether a red result is such a stop, then removes each queued PR from the queue:

```sh
gh api graphql -f query='mutation($id:ID!){dequeuePullRequest(input:{id:$id}){mergeQueueEntry{id}}}' -f id="$(gh pr view N --json id --jq .id)"
```

Implementers queue nothing until root lifts the stop. A confirmed regression also triggers [automatic rollback](#automatic-rollback-and-wind-down).

## Cadences

| When | Who | What |
| --- | --- | --- |
| Session start | [Routine agent](team.md#routine-agent) | [Housekeeping](#housekeeping) |
| After every fifth merge | Routine agent runs and reports; root decides | Platform cadence: `gh workflow run ci-build.yml --ref rust-migration` ([ADR 0018](adr/0018-builds-before-every-merge.md)); a red result [stops merges](#merge-stops) until fixed. Housekeeping runs with it. |
| After every twelfth merge | Root | Stocktake under [`/start-development` step 4](skills/start-development/SKILL.md) ([ADR 0013](adr/0013-stocktake-every-12-merges.md)), with `python3 tools/retrospective.py stocktake START END`. Profile the merged tip in the background ([speed](speed.md#instruction-projections), [ADR 0052](adr/0052-speed-on-the-merged-branch-and-per-pass-counts.md)). Check issue placement and exclusion conditions, report #892's remaining production macro counts from the inventory's `Assertion macros` line (runtime sites, with the const assertions beside them) beside the #734 inventory, and pick the next interval's [tools batch](planning.md#selection). |
| Cleanup exit and phase-3 exit | Root | All-member deep check ([ADR 0017](adr/0017-deep-checks-on-request.md), [deep checks](deep-checks.md#policy)) |

## CI administration

- A PR that changes `.github/workflows/` needs a reviewed **bootstrap Full** on its head before it queues. Root reviews its workflow after the reviewer accepts the source, and checks that each tool call the workflow changes still runs against the tool as open branches have it: a requested run takes its workflow from base and its tools from the PR's head. The [routine agent](team.md#routine-agent) then:
  1. unsets `CI_ON_DEMAND`;
  2. runs `python3 tools/ci.py request PR --profile full --bootstrap`;
  3. sets the variable back to `true` as soon as that run's `plan` job has succeeded;
  4. reruns any "Invalidate previous validation" run skipped for a PR push inside that window.

  Record both variable changes in the PR. Root decides whether the result lets the PR queue. The implementer queues it when the run passes and pushes nothing in between.
- After two hosted-capacity cancellations of macOS jobs (distinct run attempts whose macOS jobs GitHub cancelled with the annotation that no hosted runner acquired them), run `python3 tools/ci.py macos-fallback RUN [RUN ...] [--pr N --agent PATH --model MODEL --effort EFFORT]` as a background job. It sets `CI_MACOS_HEAVY` to the paid runner, reruns the last run's failed jobs, deletes the variable when the rerun's macOS jobs finish or the command stops, and records both changes on the PR (or prints them for the cadence). It creates the variable only if absent, so two invocations cannot share it; it refuses under two cancellations, when the variable exists, or when the run has no failed macOS job, and a SIGTERM still deletes the variable it created. Which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)); this fallback is part of it.

## Cleanup

- A worktree and local branch exist only while their PR is open, or before it while the roadmap work order holds the issue. Once its commits are pushed, root removes a review or update task's own worktree.
- When work stops for good (closed, superseded or paused with no next step), close its draft PR, keep its branch on origin, record its head on the issue, and remove its worktree and local branch.
- `.local/` holds caches and rebuildable output. Delete ad hoc output once the PR or issue records it; untracked files may be prior agents' output, not the user's.
- Preserve speed anchors, raw traces, frozen runtimes and failed outputs, and the runtimes, libraries and profiles that current speed anchors or active evidence reference, until replacements are recorded.

### Housekeeping

`python3 tools/housekeeping.py` lists stale worktrees and `.local` output; `--apply` removes the listed `remove` entries. The routine agent reads the dry run and runs `--apply` without asking root; its spawn message adds no condition of its own. It reports what was removed, each `kept:` line the apply printed and the free space.

- A worktree's `remove` line means a clean tree whose head is merged into the base, or is on origin, with no open PR for its branch; or a T3 task's checkout still at a base commit; or a record whose folder is gone.
- The tool keeps, whatever the head: an open PR's checkout; a lock, unless every issue or PR its note names is closed; a folder whose name or path is in the roadmap or an open issue or PR body; output the current deep report or speed anchors name, and pinned runtimes; a checkout with a live process's working directory inside; a merged or closed PR's checkout whose head origin lacks; and, outside `.t3/worktrees/`, a detached head or a branch with no PR still at a base commit. A branch at a base commit whose PR merged or closed is listed: a base commit holds nothing unpushed.
- Under `.t3/worktrees/` it also keeps a checkout active in the last twelve hours, unless the checkout's own PR merged or closed with its head on origin. A reviewer's checkout, which has no PR of its own, is listed only after twelve idle hours, whether or not the PR it reviewed has merged. A checkout elsewhere has no such grace.
- One `--apply` run plans and removes in the same invocation. Before each removal it checks again the references it read at the start, open output handles, live processes' working directories, the twelve-hour activity where it applies, that the tree is clean and is the folder it planned, and that its head and branch are the ones it planned; it deletes the branch only at the planned head. It does not ask GitHub again for the PR's state.
- A missing folder's record goes through `git worktree prune`, which removes every unlocked record whose folder is gone, named in the roadmap or not. Nothing is on disk to lose.
- Root keeps an entry the tool would remove by naming it in the roadmap, or by locking it with a note that names its issue, before the run. Naming a path or run in the roadmap, or in an open issue or PR body, protects it; removing the name releases it. Name a worktree by its whole folder name: a shortened one, such as `t3-bc6721f0`, protects nothing.
- A lock stays until every issue or PR its note names is closed or merged, so name the work in a lock note.

## Automatic rollback and wind-down

A confirmed code regression on `rust-migration` [stops merges](#merge-stops). When the breaking merge has only docs commits after it, or none, root removes it without asking ([ADR 0016](adr/0016-automatic-rollback.md)), by the procedure below, and reports afterwards.

- This holds even during wind-down, without a fix PR or new CI wait.
- A runner or API failure alone is not a code regression.
- If later code prevents this bounded rollback, preserve the state and handle recovery before further merges; during wind-down, hand it off instead.

Wind-down is when the user pauses the project. It is bounded by actions, not time estimates:

- Finish an edit underway and save its state even if incomplete; finish an active review by recording findings.
- Start no new implementation, review, repair round, build, simulation, CI request or retry; do not wait on CI. Queue no PR. A PR already in the queue stays there and merges on a pass; one the queue removes, or one that needs a base update, waits for resume.
- Perform an eligible automatic rollback, record each unfinished item's next action and running jobs, report and stop. Pending or later CI results do not restart wind-down.
- Wait on no child's report. Send each child its stop, then write the checkpoint from GitHub, with `python3 tools/ci.py state PR` and the status comment of each open PR, commit it and report. Housekeeping follows the commit. A child's last message can fail to arrive (#710), and its checkpoint is on its PR either way.
- Keep each open PR's one checkout and every worktree holding uncommitted or unpushed work. Remove review and update checkouts whose commits are pushed, and verified merged work.
- The user's explicit action scope overrides these defaults.

### Rollback procedure

Read the failing result and identify its code cause; a red check does not justify rolling back an arbitrary latest commit.

1. Fetch the current remote tip; identify the breaking merge and its first parent, and check that later changes are docs-only. Preserve the old tip on an origin recovery branch and record the component PR's head on its issue. Do not discard dirty work in any checkout.
2. Prepare the replacement from that first parent in a clean checkout. Reapply later docs with completion claims corrected for removed work.
3. Push with an explicit lease: `git push --force-with-lease=refs/heads/rust-migration:OLD_REMOTE_SHA origin REPLACEMENT_SHA:refs/heads/rust-migration`. If it fails, fetch and reassess; do not use bare `--force`. If protection requires it, enable force pushes temporarily and preserve every other protection setting: the update replaces the whole configuration. Restore the prior setting even on failure, then read back the force-push setting and the required checks.
4. Reopen the tracking issue if closed; record the recovery branch, head and next repair step. Resume through a replacement PR linked to the old one, which GitHub keeps marked merged. Keep the recovery ref until replacement work is safely retained.

Report a failed recovery as failed, not as a reset.
