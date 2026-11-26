# Implementer brief

Read this brief; your work order's implementation plan, in the body of the issue your spawn message names; the `## Rules` section of the tracker above that issue and the comments the plan links; and the original bodies the plan names. Then read:

- [porting](porting.md) for the code, fidelity and evidence standards your change meets;
- [workflow](workflow.md) for commits, pushes, checks, review and the merge queue;
- [background jobs](team.md#background-jobs) for how to start a long job.

## Following the plan

The plan holds the decisions; carry them out, in a PR that closes the plan's issue. Do the work yourself: start no agent but your PR's reviewers, since a helper that edits or builds is one more implementer than the [slots](team.md#slots) allow. Where the tree contradicts the plan or a decision proves wrong, take the better course and record the departure and its reason in the PR body. Ask root first only when the departure changes what the work order delivers.

## Finishing a file pass

A pass is ready for review when a reviewer would have nothing left on the [porting review's list](review-brief.md#a-porting-review). Before the first checkpoint, look for the [design faults](porting.md#design-faults) in your files and count their repair in the pass's size. Before marking the PR ready, for each file:

- Read `python3 tools/code-standards-inventory.py FILE...` and settle each line it reports.
- Run `python3 tools/code-standards-inventory.py --compare origin/rust-migration` and put its table in the status comment. Settle each row it lists as new or grown at the head, or name in the PR body the rule that allows it, and scope each module `unsafe_code` allow it lists ([code standards](porting.md#code-standards)).
- Run the fixtures that call the pass's functions: the comparison probes and the ABI probe fixtures under `tools/migration/windows-abi/`. Preflight only compiles them, so only a run shows a fixture that relied on a service read the pass replaced. Your plan names them.
- Write each test the plan's Evidence names. One you cannot build from the fixtures the pass's files have is a departure: record it in the PR body and put its gap on #156's list.
- Add each file the pass finishes to `migration/finished-files.txt`.

### Speed check

Where the plan has a speed check, count instructions at the base and the head ([ADR 0054](adr/0054-per-pass-counts-run-beside-review.md)):

- Start the count as a background job when you mark the PR ready, and start the reviewers without waiting for it. Run only the plan's scenarios: `python3 tools/migration.py simulate SCENARIO... --instructions`. The line in [speed](speed.md) is the stocktake's, which runs all seven. A base already counted at the same commit, by you or in root's retained stocktake profile, needs no second count. The run exits 1 on the anchor identity check (#275); read each candidate's count from its report.
- Keep both Callgrind outputs until the PR merges. If a count rises, attribute the rise to functions from the two outputs and put the largest movers in the PR body before merge: a count that only stays under the plan's bound is not explained. A rise worth removing is a finding for the fix round then running. Reads repeated with no write, call or random draw between them may share one lookup ([fidelity](porting.md#fidelity)).
- Count again only at the final head, and only when a fix or base update since the last count touches the measured path. Start that count beside the final head's preflight, and enqueue once both pass. Do not wait on a count anywhere else.

## Messaging root

Message root, at the thread your spawn message names, only when the queue merges or removes the PR (one line: PR, head, outcome), before a PR's second [scoped acceptance](workflow.md#review), for a question only root can answer, or for a stop you cannot work past. When you stop for good, finish with what you delivered, what stays in C++, exact commands, limits and the next action.

## Starting reviewers

This brief authorizes you to start your PR's reviewers with `delegate_task`, as [review](workflow.md#review) says: one for behaviour and one for porting, both for the whole PR, or the behaviour reviewer alone for a PR with no game code. When the pass is [ready for review](#finishing-a-file-pass), request `native` and mark the PR ready in one step, both as background jobs (`ci.py request PR --profile native --wait`, `ci.py ready PR --wait`), and start the reviewers then. Neither run's result holds them or the ready mark: put the `native` result in the PR when it arrives, and fix a failure while they review. Start each with its fixed message and nothing more:

> Review PR <URL> under `docs/review-brief.md` for behaviour. You are /root/review-<PR>-behaviour. Review <the whole PR | the commits <SHAs>>.

> Review PR <URL> under `docs/review-brief.md` for porting. You are /root/review-<PR>-porting. Review <the whole PR | the commits <SHAs>>.

A re-check, when [review](workflow.md#review) calls for one, is one reviewer for the whole PR, given every first-stage report:

> Re-check PR <URL> under `docs/review-brief.md`. You are /root/review-<PR>-recheck. Check the fixing commits <SHAs> against the findings in <report URLs>.

The scoped behaviour reviews that [review](workflow.md#review) allows after a review use the behaviour message with `the commits <SHAs>` and a name that says what they cover, such as /root/review-<PR>-base-behaviour. A [scoped acceptance](review-brief.md#a-scoped-acceptance) has its own message:

> Give PR <URL> a scoped acceptance under `docs/review-brief.md`. You are /root/review-<PR>-acceptance-<n>. Review the commits <SHAs>.

Each reviewer's last message, with its report's URL and disposition, reaches you when its turn ends. A tool call cancelled with a notice to stop as such a message arrives is the host delivering it: read the message, rerun the call and carry on.
