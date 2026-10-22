<!-- Target rust-migration. About 60 lines; see the evidence budget in AGENTS.md. -->

Closes #<issue>. Roadmap item: <phase / issue>.

Agent: <agent path> | Model: <exact model> | Reasoning effort: <effort>

## Moved to Rust

<!-- The state and control flow Rust now owns. One short paragraph or list. -->

## Stays in C++

<!-- The facade and callbacks that remain, and why each one is needed. -->

## Checks

<!-- Exact commands run, and their results. Harness scenarios that exercise this component. Exact-head CI is the evidence. -->

## Metrics

<!-- Paste python3 tools/port-metrics.py: original C++ retired, fork glue added/removed and signed net glue remain separate. Conversions report net glue. If original retirement is no greater than net glue plus tooling added, explain why. Use --whole-fork for total retirement and simulation-source share; this is a source-line measure, not behavioral completion. -->

## Known limits

<!-- Divergences, uncovered input domains, and follow-up issues. "None" is a valid answer. -->

<!--
Review report (reviewer posts it as a separate comment, about 20 lines):
Agent: <path> | Model: <exact model> | Reasoning effort: <effort>
Reviewed commit: <sha>
Findings and fixing commits: <finding -> commit, or why it went to root; or "none">
Compared against the original body: <functions or files>
New callbacks duplicating a shared accessor: <each one is a finding; or "none">
WITH_RUST bodies, exports or declarations left without a caller: <each one is a finding; or "none">
Shared credentials require an attributed report, not a GitHub platform approval.
-->
