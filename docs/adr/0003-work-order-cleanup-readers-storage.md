---
status: accepted
may change: user
---
# Work order: cleanup, then readers, then storage

The plan has six numbered phases ([docs/planning.md](../planning.md)). Until 2026-11-06 agents took work in phase-number order, so phase 2, which moves the storage of game objects into Rust, came before phase 3, which ports the code that reads game state. Since the 2026-10-31 sweep, reader ports had removed about 7,700 net lines of C++ and storage about 3,150. Each open storage draft touched 140 to 210 files to remove between 30 and 900 lines net, because every reader still in C++ needed new glue for each record that moved.

First, clean up the existing Rust code. Second, port the code that reads game state: saving and loading, the script interface, networking and NewGRF mod support. Third, move the storage of game objects into Rust. Then the three remaining phases in number order. Readers go before storage because each reader already in Rust is one less piece of C++ a storage move has to rewrite. The five unfinished storage drafts (#700, #704, #714, #723 and #731) were closed and will be redone afterwards. The save and load code for each kind of game object moves with that object's storage, because it addresses the stored fields directly.

On 2026-11-06 the user asked: "does it make more sense to re-order the phases if doing readers first makes storage easier?" An agent proposed this order, including closing the five drafts, and the user replied "approved".

Reversing it would leave the five drafts and the reader planning built for this order stranded. No hours are recorded.

## History

This replaced the earlier rule, recorded on 2026-10-28, of filling work slots from the earliest-numbered phase first. The agent that proposed the change called that rule the user's. The user's only recorded choice on order that day was "Overlap phases (Recommended)", picked in an agent's question on when work outside the simulation should start. The change cost the unfinished work in the five closed drafts, which will be redone.
