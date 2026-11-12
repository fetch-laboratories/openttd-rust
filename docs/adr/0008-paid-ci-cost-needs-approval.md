---
status: accepted
may change: user
---
# A change that raises paid CI cost needs the user's approval

The user pays for part of the fork's CI (the automated build and test runs) through Blacksmith, a paid service that supplies faster machines (runners) for CI jobs. GitHub's own runners are free for this public repo.

On 2026-11-10 the user approved a proposal that put builds back into the before-merge CI for "about $0.25 more per run" without being told it undid the 2026-10-26 decision to take them out. That day paid runner minutes were about 4.4 times the day before; this change was one of three causes. [ADR 0018](0018-builds-before-every-merge.md) records the details.

A change expected to raise what the user pays for CI needs the user's approval first. The agent states the estimated cost and whether the change undoes an earlier decision. A change that lowers the cost needs no approval.

The user made this decision on 2026-11-12, in the docs cleanup that created these records. It applies to every change, including changes to the agents' own ADRs.
