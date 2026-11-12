---
name: brief-storage-owner
description: Prepare an OpenTTD-Rust storage-owner brief or assess its checkpoint when root commissions a selected pool/map ownership port or hands it off for implementation or review. Excludes selecting the next owner, ordinary algorithm ports, and unrelated storage advice.
---

# Brief a selected storage owner

Start with root's selection in the [roadmap](../../roadmap.md) and what the [reader and pool placement](../../design/reader-pool-placement.md) requires of every pool brief. Apply the [world-state design](../../design/world-state.md), [storage-port contract](../../porting.md#storage-ports), [boundary rules](../../porting.md#boundary) and [component notes](../../components/README.md) to this owner; do not select new work or create another design framework.

Trace the actual constructors, allocation, consumers and destruction/reset paths in the pinned original and candidate. Build a compact inventory in the brief: **record or allocation | canonical owner now and after this checkpoint | Rust/C++ readers and writers | acquisition, invalidation and destruction order**. Include GUI/drawing, save/load and fixups, scripts, NewGRF and networking where they consume this state. Name remaining C++ fields, shells and owners, and the consumer whose migration releases each. An ID does not guarantee a live object after reuse. Name pointer/descriptor validity across growth, mutation, reentry, load/reset and destruction.

Reuse the existing [ABI macros](../../../rust/openttd-kernels/src/abi.rs) and [generator](../../../tools/generate-abi.py).

Return an executable brief or checkpoint assessment: scope and concrete C++ retirements, the inventory, ownership delivered versus remaining, focused production-path evidence for affected lifetimes and consumers, and the next checkpoint with its remaining C++ owner. Add no independent build or review gate.
