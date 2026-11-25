---
status: accepted
may change: agents
---
# A pass's instruction count runs beside its review, and a rise is explained before merge

This replaces [ADR 0052](0052-speed-on-the-merged-branch-and-per-pass-counts.md), "Speed is judged on the merged branch, and a pass on a hot path compares instruction counts at its base and head". That record kept the per-pass count and had the implementer attribute a rise before review and count again after each fix or base update that touched the measured path.

Speed against the ceilings ([ADR 0014](0014-speed-ceilings.md)) is still judged on the merged branch at each stocktake, and no pull request has a speed limit to stay under. A pass whose plan names a scenario for a body on a per-tick or per-tile path still counts instructions there at its base and head, and a rise is still attributed to functions in the PR body. What changes is when the pass waits for the count:

- The first count starts in the background when the PR is marked ready, and the reviewers start without waiting for it. A rise is attributed before merge. A rise worth removing is a finding for the fix round then running.
- Only the final head is counted again, and only when a fix or base update since the last count touches the measured path. That count runs beside the final head's preflight, and the PR enters the queue once both pass.
- The count runs the plan's scenarios, not the stocktake's seven.

## Why

Over the seven passes of root's session of 2026-11-24 20:57Z to 2026-11-25 06:48Z, implementers were blocked on counts for 150 of 1,618 slot-minutes, 9% of the session (case #1071, from the 39th steering review's slot-time audits). About 105 of those minutes came before the first review: the count was a gate on marking the PR ready. About 42 came from counting again after intermediate fixes, and 13.5 from one pass running all seven scenarios where its plan named two. The record shows nothing that needed the count before review:

- The one rise worth removing, #1051's +3.1%, came from its re-check fixes. A count of the final head finds it.
- The other rises, on #1054, #1061 and #1056, were attributed and kept. Writing the attribution during review loses nothing.
- #1045 found and attributed its rise, one pool slot read three times a visit, while its first review ran, and its fix went into that review's fix round. It was reviewed before the "before review" rule was written.
- From the transcripts: a count held the first review or the queue entry four times in the seven passes after that rule. It never did in the three passes before it (#1031, #1044, #1045), which also counted.

Recomputed from the same record, the rules here would have blocked the seven passes for about 14 to 20 minutes in all, and every rise that was found or explained before merge would still have been. #1055, which skipped a recount after a base update that conflicted in the measured file, would have recounted. The figures and their method are on #1071.

## Cost

A rise worth removing that the first count finds may now reach the reviewers after their first stage. It enters the fix round then running and gets the re-check's review, as #1045's did. A rise that only the final count finds gets the review a late fix gets now ([workflow](../workflow.md#review)). Counts beside preflight compete with it for the host's cores. That was not measured.

An agent made this decision: the fathom agent for #1071 wrote it, from the 39th steering review's measurements. [docs/implementer-brief.md](../implementer-brief.md#speed-check) and [docs/planner-brief.md](../planner-brief.md) hold the working rule, and [docs/speed.md](../speed.md) the stocktake's.
