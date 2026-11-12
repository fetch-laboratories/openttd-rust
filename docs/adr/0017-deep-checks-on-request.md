---
status: accepted
may change: user
---
# Deep checks run only on the user's request, plus at the end of cleanup and of the reader phase

Some checks are too long to run on every change. Sanitizer runs, for example, rebuild the game with extra checking and run it for a long time. They tie up this machine for hours and pause the implementers' builds and measurements.

Deep checks run only when the user asks, with two standing exceptions: once when the cleanup finishes and once when the reader phase finishes.

The two standing requests are recorded in [#893](https://github.com/fetch-laboratories/openttd-rust/issues/893), an agent-written issue that says the user chose them on 2026-11-11. The "only when the user asks" part appears in the docs without a source. No message from the user on either was found. The user assigned the decision to themselves on 2026-11-12, because it spends this machine's time. [docs/deep-checks.md](../deep-checks.md) holds how to run them.

Each extra run costs hours of machine time and pauses the implementers.
