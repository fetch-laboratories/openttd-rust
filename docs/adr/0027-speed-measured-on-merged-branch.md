---
status: accepted
may change: agents
---
# Speed is measured on the merged branch, not on each pull request

Speed is compared by running saved games in both games ([ADR 0040](0040-speed-by-instruction-counts.md)). On this busy machine one measurement is noisy, and a fair one needs the machine quiet, which holds up other agents. Until 2026-10-22 each pull request had a speed check that allowed it to make the game at most 15% slower. No ordinary pull request came near that limit; the largest change measured was 1.3% (commit `d23471800d`).

Speed is measured on the merged branch at each stocktake, not on each pull request. A rise of more than 3% is fixed in a follow-up change, and the 3% figure is provisional.

An agent made this decision in the 17th steering review (commit `d23471800d`, 2026-10-22), which records it as the user's. Asked about it that day, the user replied: "I don't understand. Should it?" The ceilings the measurement is checked against are in [ADR 0014](0014-speed-ceilings.md), and [docs/speed.md](../speed.md) holds the procedure.

A per-PR speed check would cost agent-hours and machine time waiting for quiet measurements.

## History

Until 2026-10-18 each pull request could make any scenario at most 3% slower than the best recorded. On 2026-10-18 that became the 15% per-PR guard ([#282](https://github.com/fetch-laboratories/openttd-rust/issues/282)), which was in turn dropped on 2026-10-22. Agents spent at least 7.6 hours held for quiet-machine windows under the 3% rule (#282).
