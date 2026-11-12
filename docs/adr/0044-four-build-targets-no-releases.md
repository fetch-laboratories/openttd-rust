---
status: accepted
may change: user
---
# The fork builds for four targets only and publishes no releases

Each platform the game is built for, called a target, needs its own Rust build path, checks that Rust and C++ agree on how data is laid out and how functions are called, and CI time. The original ships more targets than the fork could check on its first day.

The game is built for 64-bit Linux on x86, macOS on Apple silicon, 64-bit Windows with Microsoft's compiler, and the browser (Emscripten). The build refuses Intel and universal macOS builds, Windows on ARM, Windows built with MinGW (the GNU tools for Windows) and cross compilation. Linux on ARM gets through the build but is not tested in CI, so it is not supported ([docs/rust-migration.md](../rust-migration.md)). The fork ships no binaries: upstream's release, preview and nightly workflows were deleted on 2026-10-15 (commit `92d04db254`).

Agents made this decision. On 2026-10-02 the founding agent enabled only the targets it could check, and later housekeeping removed the release workflows as unused. No words of the user chose the set. The user's own platform choices are dropping 32-bit Windows ([ADR 0007](0007-no-32-bit-windows.md)) and, on 2026-11-11, "fix the linux arm claim", after the docs had listed Linux on ARM as supported without a CI job. It is the user's to change because it sets which platforms the game runs on.

Adding a target back grows dearer as the C++ goes: it needs a Rust build path, layout checks and CI jobs, and any platform code retired without a Rust version for that target would have to be written again.
