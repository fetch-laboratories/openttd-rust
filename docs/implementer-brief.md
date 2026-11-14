# Implementer brief

Read this brief; your work order's implementation plan, in the body of the issue your spawn message names; the `## Rules` section of the tracker above that issue and the comments the plan links; and the original bodies the plan names. Then read:

- [porting](porting.md) for the code, fidelity and evidence standards your change meets;
- [workflow](workflow.md) for commits, pushes, checks, review and the merge queue;
- [background jobs](team.md#background-jobs) for how to start a long job.

## Following the plan

The plan holds the decisions; carry them out, in a PR that closes the plan's issue. Where the tree contradicts the plan or a decision proves wrong, take the better course and record the departure and its reason in the PR body. Ask root first only when the departure changes what the work order delivers.

## Finishing a file pass

A pass is ready for review when a reviewer would have nothing left on the [porting review's list](review-brief.md#a-porting-review). Before the first checkpoint, look for the [design faults](porting.md#design-faults) in your files and count their repair in the pass's size. Before marking the PR ready, for each file:

- Read `python3 tools/code-standards-inventory.py FILE...` and settle each line it reports.
- Run the fixtures that call the pass's functions: the comparison probes and the ABI probe fixtures under `tools/migration/windows-abi/`. Preflight only compiles them, so only a run shows a fixture that relied on a service read the pass replaced. Your plan names them.
- Add each file the pass finishes to `migration/finished-files.txt`.

## Messaging root

Message root, at the thread your spawn message names, only when the queue merges or removes the PR (one line: PR, head, outcome), for a question only root can answer, or for a stop you cannot work past. When you stop for good, finish with what you delivered, what stays in C++, exact commands, limits and the next action.

## Starting reviewers

This brief authorizes you to start your PR's reviewers with `delegate_task`, as [review](workflow.md#review) says, once the PR is ready and `native` is requested. Each group, or the whole PR when it has no groups, gets two reviewers at once: one for behaviour and one for porting. Start each with its fixed message and nothing more, where `<group>` is the group's number, or `pr` for the whole PR:

> Review PR <URL> under `docs/review-brief.md` for behaviour. You are /root/review-<PR>-<group>-behaviour. Review <group N of the PR body's review groups | the whole PR | the commits <SHAs>>.

> Review PR <URL> under `docs/review-brief.md` for porting. You are /root/review-<PR>-<group>-porting. Review <group N of the PR body's review groups | the whole PR | the commits <SHAs>>.

A re-check, when [review](workflow.md#review) calls for one, is one reviewer per group, given both of the group's reports:

> Re-check PR <URL> under `docs/review-brief.md`. You are /root/review-<PR>-<group>-recheck. Check the fixing commits <SHAs> against the findings in <report URLs>.

A fresh review of a substantive rewrite gets both reviewers, scoped to its commits. A fresh review of a simulation behaviour fix that CI found, or of a conflict resolution, gets a behaviour reviewer only.

Each reviewer's last message, with its report's URL and disposition, reaches you when its turn ends. A tool call cancelled with a notice to stop as such a message arrives is the host delivering it: read the message, rerun the call and carry on.
