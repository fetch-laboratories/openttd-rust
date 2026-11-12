---
status: accepted
may change: agents
---
# Speed is measured by counting executed instructions, with occasional timed passes

Timing the game with a clock is unreliable on a machine where other agents are compiling. Waiting for quiet windows took 27% of root's time in one session and caught nothing (commit `7629c65d54`, 2026-10-21). Counting the machine instructions the game loop executes, with the Valgrind tool Callgrind, gives nearly the same answer every run: under 0.04% apart between quiet and loaded runs ([#321](https://github.com/fetch-laboratories/openttd-rust/issues/321)).

Between timed passes, speed is projected from instruction counts against an anchor, a recent timed measurement. Clock timings are taken only in occasional passes on a quiet machine. [Speed](../speed.md) holds the formula and procedure; the factor in the formula is provisional.

On 2026-10-21 the user picked this from an agent's question, choosing "Counts per PR (Recommended)" over keeping quiet-machine timing. The per-PR part was later replaced by [ADR 0027](0027-speed-measured-on-merged-branch.md); the counting method stayed.

Reversing it would mean rebuilding the speed tools, their anchors and every stocktake's series, and waiting for quiet windows again.
