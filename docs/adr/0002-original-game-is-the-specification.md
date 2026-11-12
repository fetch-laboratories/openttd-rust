---
status: accepted
may change: user
---
# The original game defines correct behaviour; improvements wait

A rewrite can drift from the original in thousands of small ways. In multiplayer, a single changed random-number draw makes two computers disagree about the game and drop the connection (a desync). Players also expect their saved games and mods to keep working. The fork needs one answer to "what is correct?" that no agent can argue with.

The original game, pinned at version 15.3 ([ADR 0030](0030-pinned-to-openttd-15-3.md)), defines correct behaviour, quirks included. The Rust game must match it in multiplayer, saved games, mods, the interface and the exact order of random-number draws. Improvements are filed as issues labelled `deferred-improvement` and wait until the rewrite is nearly complete. An agent does not change the code and the expected result together to make a check pass. The original's function boundaries are not part of the specification, so code may be restructured where behaviour stays the same ([porting](../porting.md#boundary)).

The user set this in the founding session on 2026-10-02: "the original project's priority is to remain faithful to the original game, even if it's 'worse' than it could be. … just take note of any potential improvements you encounter along the way, they can be implemented once we've achieved close to full Rust reproduction." Earlier that day the user had suggested reworking odd mechanics; the agent replied that a deliberate change would make every mismatch ambiguous, and the user agreed. On 2026-11-06 the user asked whether flaws could be fixed along the way, and an agent answered with the function-boundary clause above.

Matching the original keeps the game playable at every step and makes side-by-side testing possible. Every port, expected result and comparison scenario rests on this. No cost figure is recorded.
