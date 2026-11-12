---
status: accepted
may change: agents; a change that raises paid CI cost needs the user's approval (ADR 0008)
---
# Clang and the GCC dedicated server build run before every merge; macOS runs after every fifth merge

CI is the set of automated builds and tests that run on GitHub. A build that runs before every merge costs CI minutes on every merge. A build that runs only after every fifth merge is cheaper, but a failure there is found late and stops all merging until it is fixed. The before-merge set is called Full; the after-every-fifth-merge run is called the cadence.

Clang and the GCC dedicated server are among the builds that must pass before every merge. macOS and the cross-platform comparison run after every fifth merge instead, and a failure there stops merges. [docs/ci.md](../ci.md) lists every job.

Clang and the GCC dedicated server run before every merge because their compiler warnings kept slipping through. In the week before 2026-11-10, four of 19 cadence runs failed, three of them on those warnings, and two stopped merging for 134 minutes ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860), [#896](https://github.com/fetch-laboratories/openttd-rust/issues/896)).

macOS stays out because GitHub's free macOS runners were cancelled for lack of capacity 19 times in one day, and macOS had caught no defect ([#357](https://github.com/fetch-laboratories/openttd-rust/issues/357)).

Moving Clang back to the cadence would now save little, since it runs on a free runner ([ADR 0009](0009-paid-runner-assignment.md)). The cost would be merge stops when a warning slips through.

## History

Checking before merge instead of on every push began with the user on 2026-10-07: "full CI does not to run every single push. CI should be run before merge." On 2026-10-22 the user moved macOS to the cadence: "test macos like every fifth merge or something. not every time" (#357). On 2026-10-26 the user picked an agent's recommended option to move Clang, the GCC dedicated server and 32-bit Windows out of Full to save cost; they took 28 of 87 runner-minutes ([#514](https://github.com/fetch-laboratories/openttd-rust/issues/514)).

On 2026-11-10 all three were moved back (#862), as one of nine changes the user approved together. The proposal gave the cost as "about $0.25 more per run" and did not say it undid the earlier decision. Paid Blacksmith minutes went from 844 on 2026-11-09 to 3,724 on 2026-11-10, about 4.4 times. An agent found three causes: the three builds roughly doubled each Full run's cost, the new merge queue ([ADR 0019](0019-merge-queue.md)) ran Full again for each change, and changes about doubled. The user wrote afterwards: "I didn't think it was talking about putting them back in because we'd already removed them."

32-bit Windows, 23% of that day's weighted paid minutes, was then dropped ([ADR 0007](0007-no-32-bit-windows.md)). [ADR 0008](0008-paid-ci-cost-needs-approval.md) guards against a repeat.
