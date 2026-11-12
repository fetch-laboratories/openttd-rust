# Review brief

The PR's implementer starts each reviewer with this fixed message and nothing more:

> Review PR <URL> under `docs/review-brief.md`. You are <name>. Review <group N of the PR body's review groups | the whole PR>.

A re-check's message adds one sentence: `Re-check the fixing commits <SHAs> for the findings in <report URLs>.` Read the PR body, its review groups, its flagged points and its earlier reports yourself.

## Before reviewing

1. Read [porting](porting.md), which holds the standards you enforce, and [review](workflow.md#review).
2. Read the stream issue body's `## Rules` section and the issue comments the work order or PR body links, the PR body, its branch's cleanup records file and its earlier review reports.
3. Check out the reviewed head. The original is `.local/reference/openttd` in the main checkout.

## Report only

Do not commit, push, request Full or merge.

## What to examine in your group

- **Same operations, same order.** Compare each changed body with its original body, callee by callee, not only with the earlier Rust: guards, fatal paths, assertions, widths, return values, service and command calls, random draws, partial effects on early return and lifetimes.
- **Every line in scope.** Each departure from [porting](porting.md) in the lines [code standards](porting.md#code-standards) puts in scope is a finding. `python3 tools/code-standards-inventory.py FILE...` lists candidate sites per file; judge each.
- **Evidence.** A changed path the PR newly leaves unreached or unobserved by the scenarios and tests it cites is a finding.
- **Cleanup records.** The checks enforce the records' form, not their truth, so judge each: a growth or lint exception names the original behavior that requires it and what retires it; a carry pairs the same item before and after a rename or move; a suppression's reason names the original definition or cast it preserves.

## Policy questions

Rule on a policy question yourself, from the decisions recorded on the stream issues, in `docs/adr/` and in the docs, and state your ruling and its source in the report. A question no recorded decision answers goes in the report; [tell the implementer](team.md#spawning), who asks root.

## Checks

To confirm a suspected difference, run the narrowest check: a focused test through `python3 tools/migration.py exec -- cargo test ...`, `python3 tools/migration.py rust-checks`, or the affected scenarios by name.

- Run long jobs in the background as `exec python3 tools/signal-witness.py -- <command>`.
- Keep `TMPDIR` and build output under your worktree's `.local`. You may borrow another worktree's build read-only if its stamp matches; name the build's commit in the report.

## Report

Post one report on the PR as a separate comment, with the PR template's review skeleton, starting `Reviewed commit: <sha>` and attributed exactly as `Agent: <your name> | Model: <model> | Reasoning effort: <effort>`. The lint gates parse this exact form.

- Write only findings; every later reviewer and the implementer reads each report in full. Leave out what you compared, what you found unchanged and the checks that passed.
- For each finding give, in one to three sentences, its location (file:line at the reviewed head), what it departs from, the check or reading that found it and the suggested remedy.
- End with the disposition: accepted, or findings to remediate. An acceptance is three lines: the reviewed commit, the attribution and the disposition with your group.
- A re-check gives one line per finding: resolved in `<sha>`, or what remains.
- When you accept a growth exception, lint exception or suppression proposed in the branch's cleanup records file, add the fenced `cleanup-acceptance` block it needs to the same report ([quick cleanup checks](rust-migration.md#quick-cleanup-checks)); without it the PR cannot queue.
- Post the report when the source review is done; do not wait for CI.

Your last line is `PR <url>, head <SHA>, checks and reviews <state>`, then the report's URL and disposition once posted, or the job ID when you end a turn to wait on one. It reaches the implementer when your turn ends.
