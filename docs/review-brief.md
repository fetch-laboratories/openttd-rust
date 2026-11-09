# Review brief

A reviewer reads this brief, then the spawn message. The spawn gives:
- the reviewer's name;
- the PR;
- the function group to review, and who covers the other groups;
- the points the PR body flags for review.

Everything else is here ([delegation](team.md#review-loop)).

## Before reviewing

1. Read [AGENTS.md](../AGENTS.md), the [review checklist](review-checklist.md), [review](workflow.md#review) and [Claude Code root](team.md#claude-code-root).
2. Read the docs the PR's stream needs:
   - [porting](porting.md) for code standards, the boundary and evidence;
   - the stream issue, with its recorded decisions (for example #734, #735, #736 or #737);
   - the PR body and its earlier review reports.
3. For evidence changes, apply the [review-evidence-contract skill](skills/review-evidence-contract/SKILL.md).
4. Check out the reviewed head. The original is `.local/reference/openttd` in the main checkout. Keep it unchanged.

## Report only

Do not commit, push, request Full or merge. The PR's Opus implementer fixes each finding in its own commits ending `Found-by: review`. When root asks, verify the fixing commits for your own findings and post the verification, also report only.

## What to examine in your group

- **Same operations, same order.** Compare each changed body with its original body, not only with the earlier Rust. Check:
  - service calls, and owner reads and writes;
  - command and test-mode calls;
  - random draws, with their width;
  - partial effects on early return.

  Cover every point the spawn lists.
- **Every line of the files.** A file pass applies the code standards to every line left in its files, not only to changed lines. Each of these is a finding:
  - a literal for a named value;
  - a raw internal signature where an original type exists (FFI exports, `repr(C)` records and service tables keep base integers, per #355);
  - a missing original comment at a matching branch;
  - a hand-written copy of a shared helper;
  - a non-command helper above seven parameters;
  - a Rust-internal `Cmd*` body without one argument struct.

  Literals the original itself writes as literals stay.
- **Adapters and boundary.** Each raw adapter delegates to one typed definition and has only callers outside the pass. Each touched orphan export is a finding; tests do not keep an export alive.
- **Evidence.** The changed paths stay reached by the scenarios and tests the PR cites. A branch the PR newly leaves unreached or unobserved is a finding. Oracles come from the unchanged original only.
- **Lint records.** Each carry keeps the same lint and scope, and its head magnitude is not above the `from` magnitude ([quick cleanup checks](rust-migration.md#quick-cleanup-checks)).

## Policy questions

Rule on a policy question yourself, using the decisions recorded on the stream issues and in the docs. State your ruling and its source in the report. Only a question that no recorded decision answers waits for root: put it in the report and message your own `parentThreadId`.

## Checks

To confirm a suspected difference, run the narrowest check, after `eval "$(python3 tools/migration.py env)"`: a focused test, `python3 tools/migration.py rust-checks`, or the affected scenarios. You do not need to rerun the implementer's full gates.

- Run long jobs in the background as `exec python3 tools/signal-witness.py -- <command>`. Name the session and end your turn. If root resumes you instead of the session, read its output once and continue.
- The SSD is small. Keep `TMPDIR` and build output under your worktree's HDD-backed `.local`. You may borrow another worktree's build read-only if its stamp matches; name the build's commit in the report.

## Report

Post one report on the PR, with the PR template's review skeleton, starting `Reviewed commit: <sha>` and attributed exactly as `Agent: <your name> | Model: <model> | Reasoning effort: <effort>`. The lint gates parse this exact form.

- Name the original functions you compared.
- For each finding give:
  - its location (file:line at the reviewed head);
  - what it departs from;
  - the check or reading that found it;
  - the suggested remedy.
- End with the disposition: accepted, or findings to remediate.

Your last line is `PR <url>, head <SHA>, checks and reviews <state>`.
