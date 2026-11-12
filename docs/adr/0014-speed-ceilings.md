---
status: accepted
may change: user
---
# The Rust game may cost at most 1.6 times the original in play and 1.3 times in map generation

While the Rust game still crosses into C++ for much of its work, it runs slower than the original. The fork needs a limit on how much slower it may get, so that speed work happens before the game becomes unplayable but does not crowd out porting.

The Rust game may cost at most 1.6 times what the original costs to run a saved game, and at most 1.3 times to generate a new map.

The user raised the play ceiling. On 2026-10-28 they wrote: "okay, raise the threshold from 1.5x to 1.6x." At the time the slowest saved game stood at about 1.485 times the original, close to the old 1.5 ceiling (commit `432ab228fc`). The 1.3 ceiling for map generation comes from [#282](https://github.com/fetch-laboratories/openttd-rust/issues/282) (2026-10-18), an agent-written issue that says the user decided it. How speed is measured, and when a speed pass runs, is in [ADR 0027](0027-speed-measured-on-merged-branch.md) and [docs/speed.md](../speed.md).

A lower ceiling would cost agent-hours spent on speed work in place of porting.

## History

The play ceiling was 1.5 until 2026-10-28.
