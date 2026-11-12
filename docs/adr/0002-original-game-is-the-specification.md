---
status: accepted
may change: user
---
# The original game defines correct behaviour; improvements wait

A rewrite can drift from the original in thousands of small ways. In multiplayer, a single changed random-number draw makes two computers disagree about the game and drop the connection (a desync). Players also expect their saved games and mods to keep working. The fork needs one answer to "what is correct?" that no agent can argue with.

The original game, pinned at version 15.3 (`migration/baseline.json`), defines correct behaviour, quirks included. The Rust game must match it in multiplayer, saved games, mods, the interface and the exact order of random-number draws. Improvements are filed as issues labelled `deferred-improvement` and wait until the rewrite is nearly complete. An agent never changes the code and the expected result together to make a check pass.

Matching the original keeps the game playable and compatible at every step, and makes it possible to test the Rust game against the original side by side. Changing a check's expected result along with the code would hide a difference instead of fixing it. The record does not show who first set this rule; it is part of the fork's founding instructions. On 2026-10-28 the user wrote, about the scripting engine, "by that time, we'll be so close to done that we can start to deviate from the original", which treats changing behaviour as a choice for later ([ADR 0006](0006-bundled-libraries-and-squirrel.md)).

Every port, every expected result and every comparison scenario rests on this. No cost figure is recorded.
