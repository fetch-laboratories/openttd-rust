---
status: accepted
may change: agents
---
# Speed is judged on the merged branch, and a pass on a hot path compares instruction counts at its base and head

This replaces [ADR 0027](0027-speed-measured-on-merged-branch.md), "Speed is measured on the merged branch, not on each pull request". That record dropped a per-PR speed limit because a timed measurement needs a quiet machine and no ordinary pull request came near the limit. Speed has since been counted in executed instructions ([ADR 0040](0040-speed-by-instruction-counts.md)), which need no quiet machine, and the cleanup rewrites one file of the game loop after another.

A rewrite of a file on a per-tick path can add a fraction of a percent that no stocktake attributes: the stocktake looks only at a rise over 3% across twelve merges. On 2026-11-24 three passes each added about 0.3% to a play scenario (#1031, #1044 and #1045), and the merge-83 profile put the highest play record at 1.40 times the original against a band that starts at 1.52. About 170 files were still to pass. When root had #1045's rise attributed by function, 119 million of its instructions were one pool slot looked up three times for each vehicle visit; reading it once left the pass level with its base. #1044's count was taken before its review fixes and had the wrong sign once they merged.

Speed against the ceilings ([ADR 0014](0014-speed-ceilings.md)) is still judged on the merged branch at each stocktake, and no pull request has a speed limit to stay under. A pass whose plan names a scenario for a body on a per-tick or per-tile path also counts instructions there at its base and its head. Where the count rises, the implementer attributes the rise to functions from the two profiles before review and puts the largest in the PR body, and counts again after a fix or base update that touches the measured path.

An agent made this decision: root wrote the working rule on 2026-11-24, and the 38th steering review recorded it here the same day. [docs/implementer-brief.md](../implementer-brief.md#finishing-a-file-pass) and [docs/planner-brief.md](../planner-brief.md) hold the working rule, and [docs/speed.md](../speed.md) the stocktake's.

The cost is a base build and two profiled runs for each such pass. Root's profile of all seven scenarios from a clean checkout took 27 minutes at merge 83, and implementers made 11 checkouts by hand for their base counts that day.

## History

From 2026-10-22 to 2026-11-24 speed was measured only on the merged branch (ADR 0027). A provisional line in the planner brief had asked for base and head counts on hot paths since 2026-11-18, with no step for a rise that stayed under the plan's bound.
