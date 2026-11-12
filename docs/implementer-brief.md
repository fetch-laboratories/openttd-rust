# Implementer brief

Readers: implementation agents, when starting a work order and again before marking its PR ready. Read this brief, your work order (issue and spawn brief) and the original bodies it names. Each reminder names a rule and links to its full text; open the link when the rule applies to your change.

## Reminders

### Behaviour

- The pinned original is the specification: reproduce its reachable behaviour, quirks included, and nothing it lacks ([fidelity](porting.md#fidelity)).
- Keep the exact sequence of random draws and every state read and write at its original observation point ([fidelity](porting.md#fidelity)).
- Keep saves, networking, mods and the interface working ([AGENTS.md](../AGENTS.md#every-agent)).
- Expected results come from the unchanged original; never change them to make candidate behaviour pass ([evidence](porting.md#evidence)).
- A C++ global or static becomes a process-wide Rust static ([fidelity](porting.md#fidelity)).

### Code

- Write in the original's vocabulary: its types, flag sets, IDs and named constants from their header module ([original vocabulary](porting.md#original-vocabulary)).
- Carry the original's explanatory comments to the matching branch ([comments](porting.md#comments)).
- Pass a command's arguments as its argument struct, its flags as `DoCommandFlags`, and a helper's shared state as a context struct ([argument structs](porting.md#argument-structs)).
- Port each original helper once, in the module named after its original file, and call the typed Rust form, never a raw adapter or an export ([callees](porting.md#callees-and-shared-accessors)).
- Cross the C++ boundary through typed `noexcept` synchronous services, and end Rust borrows before a callback ([boundary](porting.md#boundary)).
- Fix every design fault in the files you touch ([design faults](porting.md#design-faults)).
- Delete the C++ you replace in the same change, with its exports, declarations and service fields ([callees](porting.md#callees-and-shared-accessors)).

### Evidence

- Use the harness and Rust tests; write no new C++ test, and leave a C++ test of code still in C++ where it is ([tests in Rust and C++](porting.md#tests-in-rust-and-c)).
- Add a scenario or test only for behaviour existing checks cannot distinguish, and name that behaviour in the PR ([evidence](porting.md#evidence)).
- `cfg(test)` never changes a production body ([tests](porting.md#tests)).

### Checks and commits

- Run your work order's focused checks and preflight before pushing, except for inert Markdown ([commits and pushes](workflow.md#commits-and-pushes)). [Quick cleanup checks](rust-migration.md#quick-cleanup-checks) explains each gate. Fix failures while preserving the original's operations.
- Take scenario evidence from a CI `native` request, not from this host ([checks](workflow.md#checks)).
- Run long jobs in the background under `tools/signal-witness.py`, name the job ID and end your turn ([AGENTS.md](../AGENTS.md#every-agent), [spawning](team.md#spawning)).
- Push draft checkpoints and mark the PR ready with the commands in [commits and pushes](workflow.md#commits-and-pushes).
- Commit coherent steps, push at each passing milestone, keep the PR in draft until done and edit one status comment at milestones ([records](workflow.md#records)).
- A turn that ends only to wait writes the one last line [AGENTS.md](../AGENTS.md#every-agent) gives, and nothing else.

## Finishing a file pass

A pass is ready for review when a reviewer would have nothing left on the [every-line list](review-brief.md#what-to-examine-in-your-group). Before the first checkpoint, look for the [design faults](porting.md#design-faults) in your files and count their repair in the pass's size. Before marking the PR ready, for each file:

- Read `python3 tools/code-standards-inventory.py FILE...` and settle each line it reports, including its raw adapter calls, export calls, `execute` flags, scratch pools and long commands.
- Check each original callee name against existing Rust definitions.
- Run the fixtures that call the pass's functions: the comparison probes and the ABI probe fixtures under `tools/migration/windows-abi/`. Preflight only compiles them when headers, probes or adapters change; only a run shows a fixture that relied on a service read the pass replaced. Repair a fixture's input, never its expected result. Your work order's entry names its fixtures on #734.
- Add each file the pass finishes to `migration/finished-files.txt`. Quick's residue gate then fails any later PR that adds one of the inventory's classes to it.
- Compare each body with its original, comments and assertions included.

List anything you leave in the PR body with its reason. Residue not listed there is a finding, and a design fault cannot be residue. Write the rest of the body as [records](workflow.md#records) says: names, not a narration of the diff.

## Messaging root

Root needs a message in three cases: the PR merged or the queue removed it (one line: PR, head, outcome), a question only root can answer, or a stop you cannot work past. Only when you stop for good, finish with what you delivered, what stays in C++, exact commands, limits and the next action.

## Starting reviewers

When the PR is ready (finishing step done, local checks passing, `native` requested), start its reviewers yourself with `delegate_task`; this brief authorizes those subagents. Use the role table's review model ([roles](team.md#roles-and-models)) and the [review brief](review-brief.md)'s fixed spawn message and nothing more: never tell a reviewer what to look for. Name each `/root/review-<PR>-<group>`.

- Start one reviewer for the whole PR. Above about 1,000 changed lines, list review groups in the PR body and start one reviewer per group.
- Each reviewer tells you in one line when its report is posted, and gives its thread ID. A tool call cancelled with a notice to stop as such a message arrives is the host delivering it: read the message, rerun the call and carry on ([spawning](team.md#spawning)).
- Fix findings with `Found-by: review` commits, start a re-check only when one is due, record the passing `native` run and queue the PR, all as the [review loop](team.md#review-loop) says. Wait on the queue yourself under [integration](root-workflow.md#integration).
- After a substantive rewrite, or a base update that needs review, start a fresh reviewer the same way. Ask root only when you and a reviewer disagree, or a reviewer raises a policy question no recorded decision answers.
