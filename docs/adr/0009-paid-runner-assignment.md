---
status: accepted
may change: user
---
# The user decides which CI jobs run on paid runners

CI jobs run on machines called runners. Blacksmith runners cost the user money and are faster. GitHub's own runners are free for this public repo but slower, so Blacksmith buys only speed. After paid minutes jumped on 2026-11-10 ([ADR 0018](0018-builds-before-every-merge.md)), an agent measured that moving the simulation, comparison and Clang jobs to free runners would save about 29% of that day's paid minutes, at the cost of jobs taking about 1.7 times as long ([#896](https://github.com/fetch-laboratories/openttd-rust/issues/896)).

The user decides which CI jobs run on paid Blacksmith runners and which run on GitHub's free runners. Today the simulation jobs run on Blacksmith, and the comparison jobs and the Clang build run on free runners. The simulation jobs went back to Blacksmith because free runners had network hangs in 3 of 11 runs, against 3 of 75 on Blacksmith.

The user agreed to the free-runner trial on 2026-11-11 ("this sounds good") and later the same day wrote "you can restore the CI linux thing", which moved the simulation jobs back. [docs/rust-migration.md](../rust-migration.md) holds the runner settings.

Either way the choice costs something: money on Blacksmith, or slower jobs and more hangs on free runners.

## History

On 2026-11-11 the simulation, comparison and Clang jobs all moved to free runners as a trial. The simulation jobs returned to Blacksmith the same day, after the hang rate above. The comparisons and Clang have not failed on free runners and stayed there.
