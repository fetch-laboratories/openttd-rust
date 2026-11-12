---
status: accepted
may change: agents
---
# Game object pools and IDs keep the original's behaviour

OpenTTD keeps each kind of game object, such as vehicles, stations and towns, in a pool: a table of slots, where an object's ID is its slot number. Rust programmers have common improvements on this: IDs that carry a generation count so that stale ones are caught, tables that compact themselves, and work spread across threads. Each would change what the game observes or the order of its random draws.

Pools keep the original's behaviour: a new object takes the first free slot, IDs are reused, and loading a save puts objects back at their saved IDs. There is no compacting, no generation count, no reference count and no new rejection of stale IDs. The tile loop keeps the original's fixed pseudo-random visiting order and is not sorted or run in parallel ([world state](../design/world-state.md#constraints-in-the-current-game)).

Agents made this decision in the world-state design, accepted after another agent's review on 2026-10-03 (commit `0baa4c43f5`, #90). It follows from [ADR 0002](0002-original-game-is-the-specification.md), but it is the first thing a reader used to Rust would want to "fix".

Changing it would be a deliberate change of behaviour under ADR 0002 and would mean redesigning every storage port.
