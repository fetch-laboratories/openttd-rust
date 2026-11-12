<!-- Target rust-migration. See docs/workflow.md#records. -->

<!-- GitHub closes an issue on merge when "close", "fix" or "resolve" (any form) precedes its number, even in "does not close #N". Write "Closes #N" only when this PR finishes the issue's remaining scope; otherwise "Part of #N". -->

Part of #<issue>. Roadmap item: <phase / issue>.

Agent: <agent path> | Model: <exact model> | Reasoning effort: <effort>

## Moved to Rust

<!-- Name the state and control flow Rust owns: types, functions and files as a list. The diff and commit titles show the rest, so do not narrate them or list mechanical callers one by one. -->

## Stays in C++

<!-- Name the remaining facade and callbacks. Explain why each is needed. -->

## Checks

<!-- List exact commands and results, one line each. Name the harness scenarios that exercise the component. -->

## Metrics

<!-- Paste python3 tools/port-metrics.py: keep original C++ retired, fork glue added and removed, and signed net glue separate. Conversions report net glue. If original retirement is no greater than net glue plus tooling added, explain why. Use --whole-fork for total retirement and simulation-source share. This measures source lines, not behavioral completion. -->

## Known limits

<!-- List divergences, uncovered domains and follow-up issues, or write "None". -->

<!--
Review report (reviewer posts it as a separate comment):
Reviewed commit: <sha>
Agent: <path> | Model: <exact model> | Reasoning effort: <effort>
Findings: <one to three sentences each: file:line, what it departs from, how it was found, the remedy. A duplicate callback or an orphan body, export or declaration is a finding. Leave this line out when there are none.>
Disposition: <accepted | findings to remediate>, <whole PR | group N>
An acceptance is these three lines. A re-check gives one line per finding: resolved in <sha>, or what remains.
Shared credentials require an attributed report, not a GitHub platform approval.
-->
