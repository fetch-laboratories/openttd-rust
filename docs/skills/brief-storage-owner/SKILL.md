---
name: brief-storage-owner
description: Prepare an OpenTTD-Rust storage-owner brief or assess its checkpoint when root commissions a selected pool/map ownership port or hands it off for implementation or review. Excludes selecting the next owner, ordinary algorithm ports, and unrelated storage advice.
---

# Brief a selected storage owner

Start with root's selection in the [roadmap](../../roadmap.md) and the contents the [reader and pool placement](../../design/reader-pool-placement.md) requires of every pool brief: allocation bookkeeping and lifetime, exact shell dispositions and the [placement conditions](../../design/world-state.md#later-pool-record-placement-634-635-decided-2026-11-05-after-integration-82). Read the applicable [world-state design](../../design/world-state.md), [storage-port contract](../../porting.md#storage-ports), [boundary rules](../../porting.md#boundary) and [component notes](../../components/README.md). Instantiate their decisions for this owner; do not select new work or create another design framework.

Trace the actual constructors, allocation, consumers and destruction/reset paths in the pinned original and candidate. Build a compact owner-consumer-lifetime inventory in the brief: **record or allocation | canonical owner now and after this checkpoint | Rust/C++ readers and writers | acquisition, invalidation and destruction order**. Include GUI/drawing, save/load and fixups, scripts, NewGRF and networking where they consume this state. Name remaining C++ fields, shells and owners, and the consumer whose migration releases each one. An ID does not guarantee a live object after reuse.

Distinguish record/layout adaptation, actual pool or map ownership, and selected control-flow or dispatch completion under the [storage-port contract](../../porting.md#storage-ports). State what this checkpoint delivers and leaves; a direct Tick route alone does not transfer pool allocation. Reuse the existing [ABI macros](../../../rust/openttd-kernels/src/abi.rs) and [generator](../../../tools/generate-abi.py).

Apply the linked fidelity, boundary and placement contracts to this owner. Name pointer/descriptor validity across growth, mutation, reentry, load/reset and destruction.

Return an executable brief or checkpoint assessment: scope and concrete C++ retirements, the inventory, delivered versus remaining ownership, focused production-path evidence for affected lifetimes/consumers, and the next selected checkpoint with its remaining C++ owner. Use [existing evidence](../../porting.md#evidence) and [check scope](../../workflow.md#checks); add no independent build or review gate.
