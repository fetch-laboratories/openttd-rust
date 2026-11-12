---
status: accepted
may change: user
---
# A change that raises paid CI cost needs the user's approval

The user pays for part of the fork's CI (the automated build and test runs) through Blacksmith, a service that rents faster machines for CI jobs ([ADR 0009](0009-paid-runner-assignment.md)).

On 2026-11-10 the user approved a proposal that put builds back into the before-merge CI for "about $0.25 more per run" without being told it undid the 2026-10-26 decision to take them out. It was one of three causes of a 4.4-fold jump in paid minutes that day ([ADR 0018](0018-builds-before-every-merge.md)).

A change expected to raise what the user pays for CI needs the user's approval first. The agent states the estimated cost and whether the change undoes an earlier decision. A change that lowers the cost needs no approval.

The user asked for it on 2026-11-12, after an agent's reversal "cost me $50 in blacksmith usage. It asked me first, but I didn't understand that it was asking to reverse the decision", and added: "If a change to one of those costs me less money they do not need approval." It applies to every change, including changes to the agents' own ADRs.
