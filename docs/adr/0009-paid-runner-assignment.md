---
status: accepted
may change: user
---
# The user decides which CI jobs run on paid runners

CI jobs run on machines called runners. Blacksmith runners cost the user money and are faster. GitHub's own runners are free for this public repo but slower, so "Blacksmith buys speed only" ([#896](https://github.com/fetch-laboratories/openttd-rust/issues/896)). An agent measured on 2026-11-11 that moving the simulation, comparison and Clang jobs to free runners would save about 29% of the previous day's paid minutes, with jobs taking about 1.7 times as long.

The user decides which CI jobs run on paid Blacksmith runners and which run on free ones. Today the simulation jobs run on Blacksmith, and the comparison jobs and the Clang build run free. Each job's runner is set by a repository variable with a free fallback ([docs/ci.md](../ci.md#runners-and-variables)).

Blacksmith is the user's choice, and the reason the repo lives in a GitHub organisation: Blacksmith works only with organisations. With merges waiting on slow CI, the user offered to pay, and on 2026-10-14 wrote: "I transferred the repo to an organization so that I can use blacksmith like the 'speed up CI and unblock merges' agent suggested." GitHub Pro and Dagger were raised as cheaper options; neither was taken up, and the record holds no ruling on either.

The user agreed to the trial below ("this sounds good"), then wrote "you can restore the CI linux thing". Either way the choice costs something: money on Blacksmith, or slower jobs and more hangs on free runners. Leaving the organisation would also lose GitHub's merge queue ([ADR 0019](0019-merge-queue.md)).

## History

On 2026-11-11 the simulation, comparison and Clang jobs moved to free runners as a trial. The simulation jobs returned to Blacksmith the same day, after network hangs in 3 of 11 free runs against 3 of 75 on Blacksmith.
