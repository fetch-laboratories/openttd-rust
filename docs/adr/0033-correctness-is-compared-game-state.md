---
status: accepted
may change: agents
---
# Correctness is judged by running both games on the same scenarios and comparing their saved game state

OpenTTD's own tests cover utilities such as strings, maths and cryptography, not the simulation. Its built-in multiplayer check compares mainly random-number state, and, as the founding agent put it, "Equal random state does not prove equal trains or cargo". The fork needed its own test of whether the Rust game plays the same as the original.

The original and the Rust game run without a screen on identical scenarios and write saves at fixed points. The comparison harness, the fork's tool for this, decodes the saves and compares the game state field by field, with the games' output. Screenshots and byte-equal compressed saves do not count as evidence. A difference may be masked only for a listed reason; a known port bug is recorded with an open issue instead. A game-logic port is integrated only once harness scenarios exercise it, and new scenarios go into the harness, not into a separate tool per component ([evidence](../porting.md#evidence), [simulation comparison](../rust-migration.md#simulation-comparison)).

Agents made this decision. Comparing game state rather than screenshots or bytes was in the founding rules of 2026-10-02. A steering session on 2026-10-03 made the harness the required evidence (commit `923f697159`, #72); its log no longer exists, and no words of the user on the choice were found. On 2026-10-04 the user approved one part of the design, scenarios built by a committed setup AI (#111).

Reversing it would be very costly: all game-logic evidence since 2026-10-03 is harness scenarios.

## History

The founding rules said there was "no mandatory up-front simulation harness". The 2026-10-03 session replaced that, as part of the change of course in [ADR 0036](0036-whole-components.md).
