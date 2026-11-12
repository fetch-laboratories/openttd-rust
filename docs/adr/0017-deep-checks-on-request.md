---
status: accepted
may change: user
---
# Deep checks run only on the user's request, plus at the end of cleanup and of the reader phase

Some checks are too long to run on every change. Sanitizer runs, for example, rebuild the game with extra checking and run it for a long time.

Deep checks run only when the user asks, with two standing exceptions: once when the cleanup finishes and once when the reader phase finishes.

Both parts are the user's. On 2026-10-31, finding a six-hour check planned for every second merge, they asked "did I agree to that!?" and chose "on request" from an agent's options. On 2026-11-11 they added "a soak at cleanup exit … then again at phase 2 exit", then corrected "soak" to "deep run". Their "phase 2" is the phase after cleanup, the reader phase ([#893](https://github.com/fetch-laboratories/openttd-rust/issues/893)). [docs/deep-checks.md](../deep-checks.md) holds how to run them.

Each extra run costs hours of machine time and pauses the implementers.
