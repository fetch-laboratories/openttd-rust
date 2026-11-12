---
status: accepted
may change: user
---
# The Rust game may cost at most 1.6 times the original in play and 1.3 times in map generation

While the Rust game still crosses into C++ for much of its work, it runs slower than the original. The fork needs a limit on how much slower it may get, so that speed work happens before the game becomes unplayable but does not crowd out porting.

The Rust game may cost at most 1.6 times what the original costs to run a saved game, and at most 1.3 times to generate a new map.

The user raised the play ceiling. On 2026-10-28 they wrote: "okay, raise the threshold from 1.5x to 1.6x." The slowest saved game then stood at about 1.485 times the original (commit `432ab228fc`). The 1.3 ceiling for map generation was an agent's proposal: on 2026-10-18 the user asked "what would be a reasonable bound for generation" and then replied "proceed" ([#282](https://github.com/fetch-laboratories/openttd-rust/issues/282)). How speed is measured is in [ADR 0040](0040-speed-by-instruction-counts.md) and [ADR 0027](0027-speed-measured-on-merged-branch.md).

A lower ceiling would cost agent-hours spent on speed work in place of porting.

## History

The play ceiling was 1.5 until 2026-10-28. That day the user asked whether 1.3 should rise too, and it was kept.
