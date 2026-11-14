---
status: accepted
may change: user
---
# At most two implementers exist at once, and a waiting one keeps its slot

This replaces [ADR 0010](0010-two-implementers.md), "At most two implementers work at once". That record did not count an implementer whose changes were all waiting on CI, review or the merge queue, so root started a fresh one for each that waited. A waiting implementer does not stay waiting: review findings, base updates and reruns of the local checks bring it back to the machine.

In root's session of 2026-11-12 to 11-13 an agent measured the result. Before 04:07 on 11-13, 5.2 implementer sessions existed on average and 3.0 were building or editing; three or more were active in 16 of 24 quarter-hours. The machine has four cores. One compile step ran a median 234 seconds while three or more implementers were building, against 136 to 140 seconds on an idle machine. Concurrent changes to shared types also failed two merge-queue runs and needed three extra conflict reviews.

At most two implementers exist at once. An implementer holds its slot from dispatch until its pull request merges or root stops it, including while it waits on CI, review or the merge queue. Root gives a waiting implementer no new work.

The user wrote on 2026-11-13: "strict 2 implementer limit for the remainder of the session. when an implementer is idle, its slot is still full." In the 2.4 hours after that, changed lines merged per hour were about the same as before (3,660 against 3,396), a small sample. Asked by the 36th steering review whether to make it the standing rule, the user chose it.

The cost is a slot that sits idle while its pull request waits, about half of a pull request's time in that session. Raising the limit crowds the machine again. [docs/team.md](../team.md) holds the working rule.

## History

ADR 0010's cap of two, with waiting implementers not counted, stood from 2026-11-10 to 2026-11-13.
