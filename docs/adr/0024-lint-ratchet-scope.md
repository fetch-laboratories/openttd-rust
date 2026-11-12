---
status: accepted
may change: agents
---
# The lint check enforces only parameter count, safety documentation and production assertions

Clippy is Rust's lint tool: it flags code patterns that are legal but risky or untidy. The fork runs it as a ratchet, which accepts the violations already in the code but fails CI on any new one.

The cleanup plan the user approved on 2026-11-06 put function length, nesting and complexity under the ratchet too. Those three measure the original's own structure, which a faithful port keeps ([ADR 0002](0002-original-game-is-the-specification.md)), so the Rust code could not meet them without departing from the original. Meanwhile, in root's session of 2026-11-08 to 11-09, the ratchet failed 46 quick CI runs, 1.8 for each change merged, and caught no defect ([#855](https://github.com/fetch-laboratories/openttd-rust/issues/855)).

The Clippy lint check fails CI only for new violations of three rules: too many function parameters, missing safety documentation, and assertion or panic macros in production code (tests may assert freely). Function length, nesting and complexity are not enforced.

An agent made this decision: the 33rd steering review (an audit that corrects root's rules), carried out in [#858](https://github.com/fetch-laboratories/openttd-rust/pull/858) on 2026-11-09, reversing part of the 2026-11-06 plan on measured cost. It followed the user's remark on 2026-11-06: "the length limit is more what I was concerned about. I don't want to implement the refactoring yet". The recorded violations fell from 649 to 59, and ratchet failures from 1.8 to 0.455 per change.

Bringing the three rules back would bring back about 590 recorded violations that a faithful port cannot remove.

## History

From 2026-11-06 to 2026-11-09 the ratchet also covered function length, nesting and complexity. The rule on production assertion and panic macros was added afterwards by #891; #892 tracks the ones that already exist.
