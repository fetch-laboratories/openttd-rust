---
status: accepted
may change: user
---
# A change that raises paid CI cost needs the user's approval

The user pays for part of the fork's CI (the automated build and test runs) through Blacksmith, a paid service that supplies faster machines (runners) for CI jobs. GitHub's own runners are free for this public repo.

On 2026-11-10 an agent proposed a list of nine process changes. One read "add macOS, Windows x86, Clang and GCC Dedicated to full CI… about $0.25 more per run". It did not say that this undid the 2026-10-26 decision to take those builds out, which the user later said they remembered. The user agreed without recognising the reversal. That day paid runner minutes were about 4.4 times the day before; this change was one of three causes. [ADR 0018](0018-builds-before-every-merge.md) records the details.

A change expected to raise what the user pays for CI needs the user's approval first. The agent states the estimated cost and whether the change undoes an earlier decision. A change that lowers the cost needs no approval.

The user made this decision on 2026-11-12, in the docs cleanup that created these records. It applies to every change, including changes to the agents' own ADRs on CI ([0018](0018-builds-before-every-merge.md), [0019](0019-merge-queue.md) and [0020](0020-scenario-comparisons-on-ci.md)) and to the choice of runners ([0009](0009-paid-runner-assignment.md)).
