---
status: accepted
may change: user
---
# A fathom agent and the pull request it opens take no implementer slot

At most two implementers exist at once ([ADR 0049](0049-two-implementers-waiting-keeps-slot.md)), and that record stands. Root fills both slots with the work that removes the most C++, so a fix to the tools or to CI waited behind it. In root's session of 2026-11-12 to 11-13 the interval's tools batch (#945) was planned and never dispatched, and three defects it listed cost about 300 minutes of pull-request time and 9 failed CI runs. The 37th steering review recorded that root "measured almost none of its eight provisional items".

A [fathom](../skills/fathom/SKILL.md) agent takes one problem in how the migration is run to its cause and lands the fix itself. It runs beside the two implementers, and a pull request it opens for a tool or CI fix takes no slot and is not part of the interval's tools batch. No number of fathom agents is set.

The user decided it. Asked on 2026-11-25 whether the skill "may carry its own tools PR outside the two implementer slots (ADR 0049)", they wrote: "yes, the new skill can do work outside of the limits."

The cost is what ADR 0049 limits: a fathom's build runs on the machine beside the implementers' builds, and its pull request takes CI runs and a place in the merge queue. ADR 0049's compile times were measured on the earlier host; nobody has measured a third build on the host of 2026-11-23. [docs/team.md](../team.md#slots) holds the working rule.
