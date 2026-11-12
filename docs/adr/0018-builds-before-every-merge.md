---
status: accepted
may change: agents; a change that raises paid CI cost needs the user's approval (ADR 0008)
---
# Clang and the GCC dedicated server build run before every merge; macOS runs after every fifth merge

CI is the set of automated builds and tests that run on GitHub. A build that runs before every merge costs CI minutes on every merge. A build that runs only after every fifth merge is cheaper, but a failure there is found late and stops all merging until it is fixed. The before-merge set is called Full; the after-every-fifth-merge run is called the cadence.

These builds must pass before every merge: the native comparisons and simulation, 64-bit Windows, the 32-bit Rust tests, Linux GCC with SDL2, the GCC dedicated server, Clang, the browser build, the quick checks and the annotation checks. macOS and the cross-platform comparison run after every fifth merge instead, and a failure there stops merges.

Clang and the GCC dedicated server run before every merge because their compiler warnings kept slipping through. In the week before 2026-11-10, four of 19 cadence runs failed, three of them on those warnings, and two stopped merging for 134 minutes ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860), [#896](https://github.com/fetch-laboratories/openttd-rust/issues/896)).

macOS stays out because GitHub's free macOS runners were cancelled for lack of capacity 19 times in one day, and macOS had caught no defect ([#357](https://github.com/fetch-laboratories/openttd-rust/issues/357)). [docs/rust-migration.md](../rust-migration.md) lists the jobs.

Moving Clang back to the cadence would now save little, since it runs on a free runner ([ADR 0009](0009-paid-runner-assignment.md)). The cost would be merge stops when a warning slips through.

## History

macOS left Full for the cadence on 2026-10-22 (#357). This has been reversed twice for the other builds. On 2026-10-26, Clang, the GCC dedicated server and 32-bit Windows were moved out of Full to save cost: they took 28 of 87 runner-minutes ([#514](https://github.com/fetch-laboratories/openttd-rust/issues/514), an agent-written issue recording it as the user's decision).

On 2026-11-10 all three were moved back (#862). Paid Blacksmith minutes went from 844 on 2026-11-09 to 3,724 on 2026-11-10, about 4.4 times, and the user saw "my blacksmith bill was 5x today". The agent that measured it found three causes together: the three builds roughly doubled the cost of each Full run, the new merge queue ([ADR 0019](0019-merge-queue.md)) ran Full again for each change, and there were about twice as many changes.

The proposal the user approved gave the cost as "about $0.25 more per run" and did not say it undid the earlier decision. The user wrote afterwards: "I didn't think it was talking about putting them back in because we'd already removed them."

32-bit Windows, 23% of that day's weighted paid minutes, was cut to its Rust checks on 2026-11-11 and then dropped ([ADR 0007](0007-no-32-bit-windows.md)). [ADR 0008](0008-paid-ci-cost-needs-approval.md) was made so that this cannot happen again unnoticed.
