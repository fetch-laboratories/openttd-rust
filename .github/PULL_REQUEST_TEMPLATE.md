<!-- Target rust-migration. See docs/workflow.md#records. -->

Closes #<issue>. Roadmap item: <phase / issue>.

Agent: <agent path> | Model: <exact model> | Reasoning effort: <effort>

## Moved to Rust

<!-- Describe the state and control flow Rust owns. -->

## Stays in C++

<!-- Name the remaining facade and callbacks. Explain why each is needed. -->

## Checks

<!-- List exact commands and results. Name the harness scenarios that exercise the component. -->

## Metrics

<!-- Paste python3 tools/port-metrics.py: keep original C++ retired, fork glue added and removed, and signed net glue separate. Conversions report net glue. If original retirement is no greater than net glue plus tooling added, explain why. Use --whole-fork for total retirement and simulation-source share. This measures source lines, not behavioral completion. -->

## Known limits

<!-- List divergences, uncovered domains and follow-up issues, or write "None". -->

<!--
Review report (reviewer posts it as a separate comment):
Reviewed commit: <sha>
Agent: <path> | Model: <exact model> | Reasoning effort: <effort>
Findings, checks and fixing commits: <finding, check and commit, or why it went to root; or "none">
Compared against the original body: <functions or files>
New callbacks duplicating a shared accessor: <each one is a finding; or "none">
WITH_RUST bodies and openttd_rust_* exports touched and their production callers; orphan bodies, exports or declarations: <each orphan is a finding; or "none">
Shared credentials require an attributed report, not a GitHub platform approval.
-->
