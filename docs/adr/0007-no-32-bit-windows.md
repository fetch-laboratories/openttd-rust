---
status: accepted
may change: user
---
# 32-bit Windows is not supported

The 32-bit Windows build (Windows x86) caused more CI trouble than any other platform. On 2026-11-10 it took 23% of the paid CI minutes ([#896](https://github.com/fetch-laboratories/openttd-rust/issues/896)), and when it failed after a merge it stopped all merging until fixed. The one x86 failure traced to its cause was a test that held results captured on a 64-bit machine, not a bug in the game. The browser build is also 32-bit, though, so 32-bit Rust tests had to keep running somewhere.

No CI job builds or tests 32-bit Windows, and the build refuses a 32-bit Windows configuration. A free CI job runs the Rust tests in 32-bit mode on Linux instead, 64-bit Windows and the browser build stay.

The user wrote on 2026-11-11: "is it unreasonable to drop windows x86 support? this is my project, so I can do what I want. it's caused us more problems than any other platform… if it's a big deal, then it can be added back later probably." They then told root to open the issue, and agreed to the 32-bit Linux test job. Agents carried it out in [#899](https://github.com/fetch-laboratories/openttd-rust/issues/899) and #900, merged on 2026-11-11.

Adding the platform back means restoring the special cases for 32-bit Windows in the build and in the C++/Rust glue. The agent that assessed it judged this cheaper once the C++ is gone.

## History

[ADR 0018](0018-builds-before-every-merge.md) records the earlier moves of 32-bit Windows in and out of the before-merge build, and their cost.
