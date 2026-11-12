---
status: accepted
may change: agents
---
# Every piece of game data has one copy, read in place from either language

While the port is under way, some game data is kept by C++ and some by Rust. Each side could keep its own copy and keep the two in step, or ask the other side for every field through a function call. Copies drift apart, and a call per field is slow: on 2026-10-28 an agent measured such calls at 99% of the gap to the original on the slowest saved game (commit `8fb4499105`).

Every piece of game data has one copy. Until a storage port moves it, data stays in C++ and Rust reads and writes it in place; the map is shared through one map descriptor. When a kind of game object moves to Rust, its record is defined once in Rust in a layout C++ can read, and the C++ declaration is generated from it and checked when the game is built. Only operations, such as creating, deleting or iterating over objects, cross between the languages as calls ([storage ports](../porting.md#storage-ports), [world state](../design/world-state.md)).

Agents made these decisions: the one-copy principle in the first session on 2026-10-02 ("avoids maintaining two worlds inside one executable"), and the shared layout on 2026-10-28. The user pushed the same way. On 2026-10-16, about slow reads from Rust into C++, they asked: "why isnt the thing they need to read converted to rust first?" On 2026-10-31 they put storage ahead of removing glue ([ADR 0022](0022-progress-is-net-cpp-removed.md)).

Reversing it would undo every storage port and bring back the per-field calls they deleted.

## History

The first answer on map access (#108, 2026-10-04) kept the map in C++ behind a call per field. It was revised on 2026-10-16 (#274) and replaced on 2026-10-28 by the shared layout.
