# Review brief

Readers: reviewers, from spawn to the final report. Everything a reviewer needs is here, in the docs it links and in the PR ([review loop](team.md#review-loop)).

The PR's implementer starts each reviewer with this fixed message and nothing more; it never says what to look for:

> Review PR <URL> under `docs/review-brief.md`. You are <name>. Review <group N of the PR body's review groups | the whole PR>. The implementer's thread is <thread ID>.

A re-check's message adds one sentence: `Re-check the fixing commits <SHAs> for the findings in <report URLs>.` Read the PR body, its review groups, its flagged points and its earlier reports yourself.

## Before reviewing

1. Read [AGENTS.md](../AGENTS.md) and [review](workflow.md#review).
2. Read what the PR's stream needs:
   - [porting](porting.md), which holds every code, fidelity and evidence standard you enforce;
   - the stream issue, with its recorded decisions (for example #734, #735 or #736);
   - the PR body, its branch's cleanup records file and its earlier review reports.
3. For a fidelity or completion claim that rests on production-path coverage, or a changed fixture or oracle, apply the [review-evidence-contract skill](skills/review-evidence-contract/SKILL.md). Report only its findings, in the report form below.
4. Check out the reviewed head. The original is `.local/reference/openttd` in the main checkout. Keep it unchanged.

## Report only

Do not commit, push, request Full or merge. The PR's implementer fixes each finding in its own commits ending `Found-by: review`. A re-check reviewer verifies the fixing commits its message names and posts the PR's final disposition, also report only ([review loop](team.md#review-loop)).

## What to examine in your group

- **Same operations, same order.** Compare each changed body with its original body, callee by callee, not only with the earlier Rust ([fidelity](porting.md#fidelity), [boundary](porting.md#boundary)). Check:
  - guards, fatal paths, assertions, widths, wrapping and truncation, and return values;
  - service calls, and owner reads and writes;
  - command and test-mode calls;
  - random draws, with their width;
  - partial effects on early return;
  - lifetimes: allocation and ID reuse, publication and destruction order, and borrows that outlive a callback.
- **Every line of the files.** A file pass answers for every line left in its files; any other PR for every line it changes or adds ([code standards](porting.md#code-standards)). `python3 tools/code-standards-inventory.py FILE...` lists candidate missing comments, bare integers and literals per file; it estimates and gates nothing, so judge each site. This is the every-line list; each item is a finding:
  - a literal for a named value, a raw internal signature where an original type exists, or a projected flag ([original vocabulary](porting.md#original-vocabulary), [boundary](porting.md#boundary));
  - a missing original comment at a matching branch, or a comment that records history or defends the change ([comments](porting.md#comments));
  - a hand-written copy of a shared helper, or of an original helper with no Rust port yet, or a new binding whose callee already has one ([callees](porting.md#callees-and-shared-accessors));
  - a long parameter list, a command or helper body inside an FFI export, or a Rust-internal `Cmd*` body without its argument struct ([argument structs](porting.md#argument-structs));
  - a Rust call to a raw adapter or an `openttd_rust_*` export where a typed Rust form exists, or an adapter serving a finished file ([callees](porting.md#callees-and-shared-accessors));
  - any [design fault](porting.md#design-faults);
  - an assertion the original has and the Rust omits, or a check the original lacks ([fidelity](porting.md#fidelity));
  - a replaced body, export, declaration or service field left behind, a duplicate callback, or an orphan the change touches; Quick's [orphan-export check](rust-migration.md#shared-code-checks-and-port-metrics) lists the exports.
- **Evidence** ([evidence](porting.md#evidence)).
  - Each claim traces to production code and an expectation taken from the unchanged original.
  - The changed paths stay reached by the scenarios and tests the PR cites. A branch the PR newly leaves unreached or unobserved is a finding.
  - A new scenario, fixture, test or checker that names no behavior existing checks cannot distinguish is a finding, and the fix deletes it.
  - A cleanup PR's proof follows [cleanup proof](porting.md#cleanup-proof).
  - A branch no scenario reaches goes on #156's current list, as [evidence](porting.md#evidence) says.
- **Cleanup records.** The [cleanup checks](rust-migration.md#quick-cleanup-checks) enforce the records' form, not their truth, so judge each:
  - a growth or lint exception names the original behavior that requires it and what retires it;
  - a declared carry pairs the same item before and after a rename or move;
  - a fidelity suppression's reason names the original definition or cast it preserves.

## Policy questions

Rule on a policy question yourself, using the decisions recorded on the stream issues, in `docs/adr/` and in the docs. State your ruling and its source in the report. Only a question that no recorded decision answers goes to root: put it in the report and tell the implementer, who asks root.

## Checks

To confirm a suspected difference, run the narrowest check, after `eval "$(python3 tools/migration.py env)"`: a focused test, `python3 tools/migration.py rust-checks`, or the affected scenarios by name. You do not need to rerun the implementer's full gates.

- Run long jobs in the background as `exec python3 tools/signal-witness.py -- <command>`, name the job ID and end your turn ([AGENTS.md](../AGENTS.md#every-agent)). If a message resumes you before the job's exit does, read its output once and continue.
- The SSD is small. Keep `TMPDIR` and build output under your worktree's HDD-backed `.local`. You may borrow another worktree's build read-only if its stamp matches; name the build's commit in the report.

## Report

Post one report on the PR as a separate comment, with the PR template's review skeleton, starting `Reviewed commit: <sha>` and attributed exactly as `Agent: <your name> | Model: <model> | Reasoning effort: <effort>`. The lint gates parse this exact form. Shared credentials allow an attributed report only, never a GitHub approval.

- Write only findings: every later reviewer and the implementer reads each report in full. Leave out what you compared, the functions and files you read, what you found unchanged and the checks that passed; the review group already says what was compared.
- For each finding give, in one to three sentences:
  - its location (file:line at the reviewed head);
  - what it departs from;
  - the check or reading that found it;
  - the suggested remedy.
- End with the disposition: accepted, or findings to remediate. An acceptance is three lines: the reviewed commit, the attribution and the disposition with your group.
- A re-check gives one line per finding: resolved in `<sha>`, or what remains. Do not restate the finding or describe the fix.
- When the branch's cleanup records file proposes a growth exception, a lint exception or a suppression contract and you accept it, add the fenced `cleanup-acceptance` block it needs to the same report, beside those lines ([quick cleanup checks](rust-migration.md#quick-cleanup-checks)). The gates read that block, and without it the PR cannot queue.
- Post the report when the source review is done; do not wait for CI, since a held report delays the fixes. An acceptance covers the source at the reviewed head; the implementer records the passing `native` run ([review loop](team.md#review-loop)).

After posting a report or a verification, tell the implementer in one line with `t3_thread_send` to its thread ID, with `mode: "queue"` ([spawning](team.md#spawning)): your own thread ID, the report's URL and the disposition. Root relays nothing.

Your last line is `PR <url>, head <SHA>, checks and reviews <state>`, with the job ID when you end a turn to wait on one ([AGENTS.md](../AGENTS.md#every-agent)).
