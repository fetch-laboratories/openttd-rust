---
name: brief-storage-owner
description: Prepare an OpenTTD-Rust storage-owner brief or assess its checkpoint when root commissions a selected pool/map ownership port or hands it off for implementation or review. Excludes selecting the next owner, ordinary algorithm ports, and unrelated storage advice.
---

# Brief a selected storage owner

Start with root's selection in the [roadmap](../../roadmap.md). Read the applicable [world-state design](../../design/world-state.md), [storage-port contract](../../porting.md#storage-ports), [boundary rules](../../porting.md#boundary) and [component notes](../../components/README.md). Instantiate their decisions for this owner; do not select new work or create another design framework.

Trace the actual constructors, allocation, consumers and destruction/reset paths in the pinned original and candidate. Build a compact owner-consumer-lifetime inventory in the brief: **record or allocation | canonical owner now and after this checkpoint | Rust/C++ readers and writers | acquisition, invalidation and destruction order**. Include GUI/drawing, save/load and fixups, scripts, NewGRF and networking where they consume this state. Name remaining C++ fields, shells and owners, and the consumer whose migration releases each one. An ID does not guarantee a live object after reuse.

State separately what this checkpoint delivers and leaves:

- Shared record/layout and consumer adaptation: one Rust definition, both languages accessing canonical fields in place, with replaced services and carriers retired. Reuse the existing [ABI macros](../../../rust/openttd-kernels/src/abi.rs) and [generator](../../../tools/generate-abi.py); generated headers are build outputs, not another field schema.
- Actual pool ownership: slot array, occupancy and allocation bookkeeping, allocation/free/iteration/reset policy and lifetime ordering. For map ownership, include array allocation/reset and remaining map bodies. Record scaffolding alone does not complete either; pool completion also requires that Rust reaches no record field through a C++ service.
- Control-flow or dispatch completion, where selected: direct production calls into Rust owners and retirement of replaced C++ dispatch, lifecycle bodies and facades. A direct Tick dispatch checkpoint does not itself transfer pool allocation.

Preserve source allocation/ID reuse and hook order, save representation, process-wide state, callback observation points and the game-state mutex contract. Name pointer/descriptor validity across growth, mutation, reentry, load/reset and destruction; end affected borrows before invalidating callbacks. Apply the selected design's shell and owner placement rather than inventing an arena or mirror.

Return an executable brief or checkpoint assessment: scope and concrete C++ retirements, the inventory, delivered versus remaining ownership, focused production-path evidence for affected lifetimes/consumers, and the next selected checkpoint with its remaining C++ owner. Use [existing evidence](../../porting.md#evidence) and [check scope](../../workflow.md#checks); add no independent build or review gate.
