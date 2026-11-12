---
status: accepted
may change: agents; a change that raises paid CI cost needs the user's approval (ADR 0008)
---
# Scenario comparisons run on CI, not on this machine

A scenario comparison runs a piece of gameplay in the original game and in the Rust game and checks that they match. There are hundreds of scenarios. On 2026-11-10 an agent ran the default set of 485 on this machine with nothing else running. It had not finished after 25 minutes, had written 55 GB to the hard disk, and spent 34% to 52% of its CPU time waiting on that disk. One CI machine ran 186 scenarios in about 3 minutes. The night before, scenarios had taken 303 of the 751 minutes of implementers' background jobs ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860)).

Comparisons of game scenarios run on CI, not on this machine. An implementer runs a scenario locally only by name, to diagnose a failure.

This was the third of the nine changes an agent proposed on 2026-11-10, which the user approved. [docs/workflow.md](../workflow.md) holds how to request a CI run.

Running scenarios here again would cost about 55 GB of disk per full run and hours of machine time. Running them on CI costs paid minutes: CI runs that implementers requested used 1,487 of the 3,724 paid Blacksmith minutes on 2026-11-10.
