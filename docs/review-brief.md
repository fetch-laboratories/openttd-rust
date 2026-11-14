# Review brief

You review a PR or a plan. The message that started you names your review: a behaviour review or a porting review of a PR group or a whole PR, a re-check, or a plan review. Do the section your message names. A PR's behaviour and porting reviewers review the same group side by side; leave the other's section to it.

## Before reviewing

1. Read [porting](porting.md), which holds the standards you enforce, and [review](workflow.md#review). Read them and this brief as `origin/rust-migration` has them (`git show origin/rust-migration:docs/review-brief.md`): a PR head that predates a rule change still carries the old rule.
2. Read the stream issue body's `## Rules` section and the issue comments the PR body links, the implementation plan in the body of the issue the PR closes, the PR body with its review groups and flagged points, its branch's cleanup records file and its earlier review reports.
3. Check out the reviewed head. The original is `.local/reference/openttd` in the main checkout. From any worktree, read it with `python3 tools/migration.py original show FILE[:A-B]` and search it with `python3 tools/migration.py original grep PATTERN [PATH ...]`, which run `git show <baseline commit>:src/FILE` and `git grep PATTERN <baseline commit> -- src/`.

## Report only

Do not commit, push, request Full or merge.

## A behaviour review

You check that each changed body does what the original does.

- **Same operations, same order.** Read every changed body in your group with its original body open beside it, callee by callee, not only against the earlier Rust: guards, fatal paths, assertions, widths, return values, service and command calls, random draws, partial effects on early return and lifetimes.
- **The plan.** A departure from the implementation plan that the PR body does not record with its reason is a finding. The plan's decisions were reviewed before dispatch; reopen one only where the code shows a fact it states is false.
- **Evidence.** A changed path the PR newly leaves unreached or unobserved by the scenarios and tests it cites is a finding.
- **Recorded exceptions.** Writing a departure from [fidelity](porting.md#fidelity) or the plan into the PR body does not settle it (provisional). Rule on each one the PR body records for your group: name the rule, ruling or owning issue that allows it, or report it as a finding.

## A porting review

You check that each changed body reaches what it uses the way [porting](porting.md) requires.

- **Each call, access and comment.** Read every changed body in your group with its original body open beside it, one call, field access and comment at a time, against [boundary](porting.md#boundary), [design faults](porting.md#design-faults), [callees and shared accessors](porting.md#callees-and-shared-accessors), [map access](porting.md#map-access), [original vocabulary](porting.md#original-vocabulary) and [comments](porting.md#comments). A pass leaves no departure from them in the files it finishes.
  - Where the original calls a helper, virtual method or accessor, or names a constant or type, search `rust/` for its Rust port or shared definition. The body uses it, not a hand-written equivalent, a direct field read, a raw integer or a C++ service.
  - Where the body calls a C++ service, open the service's C++ body and count the service's call sites. It is a finding if it serves one call site (such as one per console or error message), reads a field Rust owns, calls back into Rust or duplicates another service or accessor.
  - Count how often the body resolves each record it uses (a station, vehicle, town or map view) and compare with the original: once per entry, not once per field or per call.
  - Each comment says what the code does, and the original body's explanatory comments are carried.
- **Every line in scope.** Each departure from porting in the lines [code standards](porting.md#code-standards) puts in scope is a finding. `python3 tools/code-standards-inventory.py FILE...` lists some candidate sites per file: missing comment lines, bare integers, literals, long signatures, calls to raw adapters and exports, services with one call site, unsuffixed adapters, hand-built flag bits and integer flag sets. It prints rows as it finds them; read its output to the totals line. Judge each row; the list is not the check.
- **Recorded exceptions.** Writing a departure from porting's other rules into the PR body does not settle it (provisional). Rule on each one the PR body records for your group: name the rule, ruling or owning issue that allows it, or report it as a finding.
- **Cleanup records.** The checks enforce the records' form, not their truth, so judge each: a growth or lint exception names the original behavior that requires it and what retires it; a carry pairs the same item before and after a rename or move; a suppression's reason names the original definition or cast it preserves.

## A re-check

A re-check is a PR's second and last review stage ([ADR 0025](adr/0025-two-review-stages.md)). It reads the fixing commits, not the PR again, against the findings of the group's behaviour and porting reports.

- Say for each earlier finding whether its fixing commit resolves it.
- Raise a new finding only in a line a fixing commit changed, or for a behaviour difference from the original wherever you see one.
- Anything else you notice is one line under the disposition and does not change it. The implementer fixes it without a further review where the operations stay unchanged, or records it as residue in the PR body.

## A plan review

You review an implementation plan before any code is written. The planner chose how the work is done; you check that the plan can be carried out as written without breaking the project's rules. Fixing a plan costs nothing now and a review round later. You do not review the PR that follows.

The plan is the `## Implementation plan` in the issue's body, and its Base names the commit it was written against. Check that commit out and work from it. Read [porting](porting.md), the [planner brief](planner-brief.md) and [sizing](planning.md#sizing).

Go through the plan one decision at a time, then its Done, Files, Size and Evidence sections.

- **Rules.** Would carrying the decision out leave something porting forbids in the files the plan finishes: a design fault, a C++ service called where a Rust port or typed accessor of the same original callee exists, a new accessor that duplicates an existing one, an orphan, a dropped assertion or comment, a function over the parameter limit? Whatever the plan leaves in C++, defers or excepts needs the rule, ruling or owning issue that allows it. Name it; a decision with none is a finding. A plan grants no exception itself.
- **Facts.** Test every claim the plan makes about the code with a search or a read: that no Rust form of something exists, that something has no caller or one caller, a count, a line number, what a function does, what the original does, which files the work touches. A plan built on a false claim is a finding.
- **Feasibility.** Where the plan prescribes a shape, a signature or an order of commits, say if it cannot build or cannot pass the lint gates (`migration/lints/clippy.toml`, [quick cleanup checks](rust-migration.md#quick-cleanup-checks)).

Do not redo the planner's design or propose a different approach where the plan's own meets the rules.

Post one report as a comment on the issue, starting `Reviewed plan: #<issue> at <commit>` and attributed as every report is.

- **Findings.** One to three sentences each: the decision or section, what is wrong, the command and result that show it, and the remedy.
- **Claims tested.** One line for every claim you tested, true or false: the claim in a few words, the command, the result. List the claims you could not test.
- End with the disposition: `plan accepted` or `findings to remediate`.

Your last line is the report's URL and disposition. It reaches the planner when your turn ends.

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
- A re-check gives one line per finding: resolved in `<sha>`, or what remains.
- Give one line for each recorded exception you allowed: the exception and what allows it.
- A behaviour or porting review gives one line naming every changed body of its group it compared with its original. A body you did not compare is not reviewed; say so on that line.
- A porting reviewer that accepts a growth exception, lint exception or suppression proposed in the branch's cleanup records file adds the fenced `cleanup-acceptance` block it needs to the same report ([quick cleanup checks](rust-migration.md#quick-cleanup-checks)); without it the PR cannot queue.
- End with the disposition, your group and your review, such as `accepted, group 2 porting` or `findings to remediate, whole PR behaviour`. An acceptance holds only the reviewed commit, the attribution, the line of bodies compared, any allowed exceptions and acceptance blocks, and the disposition.
- Post the report when the source review is done; do not wait for CI.

Your last line is `PR <url>, head <SHA>, checks and reviews <state>`, then the report's URL and disposition once posted, or the job ID when you end a turn to wait on one. It reaches the implementer when your turn ends.
