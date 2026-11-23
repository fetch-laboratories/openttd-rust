---
status: accepted
may change: user
---
# This machine keeps all its output on the SSD

This replaces [ADR 0047](0047-output-on-the-hard-disk.md), "This machine's build and test output lives on the hard disk behind symlinks". That record put output on a 1 TB hard disk because the machine's small SSD kept filling.

On 2026-11-23 the user replaced that SSD with a 4 TB one and wrote: "this machine previously had an older 256gb ssd + 1tb hdd. i just installed a new 4tb ssd and moved the 256gb ssd's contents onto it. we were offloading to the hdd because the ssd kept filling up. the files we put on the hdd will need moved onto the 4tb."

All output lives on the SSD in ordinary folders. An agent moved the hard disk's 65 GB of project output there and removed the untracked Git hooks that made the links ([output disk](../rust-migration.md#output-disk)). The hard disk holds no project output.

The cost is disk space, and there is plenty of it: the SSD had 3.3 TB free after the move, and a full local run of the default scenarios wrote 55 GB ([ADR 0020](0020-scenario-comparisons-on-ci.md)). Housekeeping still removes old output. Tool code that still handles the links is harmless and retires with [#1023](https://github.com/fetch-laboratories/openttd-rust/issues/1023).
