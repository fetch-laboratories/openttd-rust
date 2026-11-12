---
status: accepted
may change: user
---
# This machine's build and test output lives on the hard disk behind symlinks

The project runs on a machine with a 220 GB SSD and a 1 TB hard disk. Simulation and build output filled the SSD: on 2026-11-05 the user reported "it overflowed the storage on the new machine". The hard disk has room but is slower.

Output that is written once and kept, such as simulation runs, coverage and each worktree's `.local` folder, lives on the hard disk behind symlinks (entries that point to a folder elsewhere). Caches, toolchains and the game builds stay on the SSD. Git hooks, which are not tracked in the repo, make the links for each new worktree, and tool code must handle `.local` paths so that the links are kept ([output disk](../rust-migration.md#output-disk)).

The plan is the user's. On 2026-11-05 they wrote: "i moved this project to a new machine. the new machine has a much smaller hard drive. 220gb ssd + 1tb hdd. i discussed with another agent and it suggested putting the simulation output on the hdd with a symlink, and possibly all of .local". An agent wrote the hooks.

The cost is speed. On 2026-11-10 an agent measured a local scenario run spending 34% to 52% of its CPU time waiting on the hard disk, part of why scenario comparisons run on CI ([ADR 0020](0020-scenario-comparisons-on-ci.md)). Undoing it here is a move and a hook removal, but a later reader who "fixes" the links or the tools' handling of them breaks builds.
