# Implementer brief

Readers: implementation agents. Read this brief, your work order (issue and spawn brief) and the original bodies it names. CI checks and reviewers carry the remaining standards; open another doc when your work order or a failing check points there.

## Behaviour

- The pinned original (`.local/reference/openttd`) is the specification, quirks included. Reproduce its reachable behaviour: arithmetic, wrapping and truncation (`wrapping_*`, `as`), assertion and fatal paths, and nothing it lacks.
- Keep the exact sequence of random draws, and every state read and write at its original observation point.
- Keep saves, networking, mods and the interface working.
- Expected results come from the unchanged original. Never change expected results to make candidate behaviour pass; keep the reference worktree unchanged.

## Code

- Write with the original's vocabulary. Each enum, flag set, ID and named constant has one Rust definition in the module named after its original header (`track_type.rs` for `track_type.h`). Use that name wherever the original does; when it is missing, add it there.
- Carry the original's explanatory comments with every ported body, at the matching branch or operation.
- Pass a command's arguments as its argument struct, its flags as `DoCommandFlags` rather than a projected `execute: bool`, and a helper's shared state as a context struct.
- Port each original helper once, in the module named after its original file, and call it everywhere. Rust code calls the typed Rust function, never a `_raw` adapter or an `openttd_rust_*` export that has one ([code standards](porting.md#code-standards)).
- Cross the C++ boundary through typed `noexcept` services with synchronous calls, allocating only where the original allocates. End Rust borrows before a callback that can reenter Rust.
- A C++ global or static becomes a process-wide Rust static.
- Delete the C++ you replace in the same change, with its exports, declarations and service fields.

## Evidence

- Use the existing harness (`python3 tools/migration.py simulate`) and Rust tests. New evidence lives in Rust or in harness scenarios; C++ under `tools/` and `src/tests` shrinks or holds.
- Add a scenario or test only for behaviour existing checks cannot distinguish, and name that behaviour in the PR.

## Finishing a file pass

A pass is ready for review when a reviewer would have nothing left on the [every-line list](review-brief.md#what-to-examine-in-your-group). #883 reached review with ten standards findings; the same classes recurred across most passes on 11-10/11. Before marking the PR ready, for each file:

- Read `python3 tools/code-standards-inventory.py FILE...` and settle each line it reports.
- Search the file for `_raw(`, `openttd_rust_` calls and scratch pools, and check each original callee name against existing Rust definitions.
- Compare each body with its original, comments and assertions included.

List anything you leave in the PR body with its reason. Residue not listed there is a finding.

## Checks and commits

- Before pushing, apply the [preflight exception for inert Markdown](workflow.md#commits-and-pushes). For other changes, run your work order's focused checks and `python3 tools/preflight.py --base origin/rust-migration`, which runs every gate Quick runs. [Quick cleanup checks](rust-migration.md#quick-cleanup-checks) explains each gate and its reviewed exception contracts. Fix failures while preserving the original's operations.
- Take scenario evidence from CI, not from this host: push, then run `python3 tools/ci.py request PR --profile native` and wait on it in the background ([checks](workflow.md#checks)). Run a scenario locally, by name, only to diagnose a failure.
- Run long jobs in the background as `exec python3 tools/signal-witness.py -- <command>`, name the job ID and end your turn ([AGENTS.md](../AGENTS.md#every-agent)).
- Commit coherent steps under titles the preflight commit checker accepts ([commits](workflow.md#commits-and-pushes)). Push at each passing milestone, keep the PR in draft until done and edit one status comment at milestones.
- Finish with what you delivered, what stays in C++, exact commands, limits and the next action.

## Review

Each reviewer tells you in one line when its report is posted, and gives its thread ID. Root relays nothing ([review loop](team.md#review-loop)). A tool call cancelled with a notice to stop as such a message arrives is the host delivering it, not a refusal: read the message, rerun the call and carry on ([spawning](team.md#spawning)).

- Read the reports on the PR. Fix each finding in its own commit, or one per coherent group, ending `Found-by: review`.
- If a fix changed behaviour, evidence or a body's operations, ask root for one re-check, in one line naming the fixing commits ([review loop](team.md#review-loop)). For fixes limited to names, comments, types or argument structs with unchanged operations, say so in the status comment instead.
- Record the passing `native` run at the reviewed head in the status comment, then queue the PR and wait on the queue yourself under [integration](root-workflow.md#integration), which also covers a conflict with base and a failure in the queue.
- Otherwise ask root only for a fresh reviewer after a substantive rewrite or a base update that needs one, or when you and a reviewer disagree.
