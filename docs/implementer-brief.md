# Implementer brief

Read this brief, your work order (issue and spawn brief) and the original bodies it names, then:

- [porting](porting.md) for the code, fidelity and evidence standards your change meets;
- [workflow](workflow.md) for commits, pushes, checks, review and the merge queue;
- [background jobs](team.md#background-jobs) for how to start a long job.

## Finishing a file pass

A pass is ready for review when a reviewer would have nothing left on the [every-line list](review-brief.md#what-to-examine-in-your-group). Before the first checkpoint, look for the [design faults](porting.md#design-faults) in your files and count their repair in the pass's size. Before marking the PR ready, for each file:

- Read `python3 tools/code-standards-inventory.py FILE...` and settle each line it reports.
- Run the fixtures that call the pass's functions: the comparison probes and the ABI probe fixtures under `tools/migration/windows-abi/`. Preflight only compiles them, so only a run shows a fixture that relied on a service read the pass replaced. Your work order's entry on #734 names them.
- Add each file the pass finishes to `migration/finished-files.txt`.

## Messaging root

Message root only when the queue merges or removes the PR (one line: PR, head, outcome), for a question only root can answer, or for a stop you cannot work past. When you stop for good, finish with what you delivered, what stays in C++, exact commands, limits and the next action.

## Starting reviewers

This brief authorizes you to start your PR's reviewers with `delegate_task`, as [review](workflow.md#review) says, once the PR is ready and `native` is requested. Name each `/root/review-<PR>-<group>`. Each reviewer's last message, with its report's URL and disposition, reaches you when its turn ends. A tool call cancelled with a notice to stop as such a message arrives is the host delivering it: read the message, rerun the call and carry on.
