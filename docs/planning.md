# Planning

## Goal and phases

- Measure distance to the destination, a Rust-only game ([ADR 0001](adr/0001-full-rust-rewrite.md)), by the remaining original and fork C++ in game, tools and tests, including generated code ([ADR 0022](adr/0022-progress-is-net-cpp-removed.md)).
- The roadmap sizes these overlapping phases with exit criteria:
  1. Port simulation logic: map generation, landscape and tile loops, towns, industries, vehicles, cargo, economy, link graph, pathfinding and commands that change them.
  2. Move simulation storage: every pool, then the map.
  3. Port readers of simulation data: save/load, script API, network and NewGRF.
  4. Port support code: core, strings, settings, viewport and graphics.
  5. Port the GUI, simulation drawing and platform backends.
  6. Replace or port vendored libraries in `src/3rdparty`.
- Execution order: the cleanup stage, then phase 3, then phase 2, then phases 4, 5 and 6 ([ADR 0003](adr/0003-work-order-cleanup-readers-storage.md)). Phase 1 has exited. A pool's save/load chunks move with that pool in phase 2.
- The cleanup stage pays down the existing code before porting resumes ([ADR 0004](adr/0004-cleanup-before-porting.md)). It exits when every stream reaches the end state the roadmap's "Phases" names.
- Before the cleanup stage's last items are dispatched, plan phase 3 in full from current base: re-estimate the queued readers and give every remaining phase-3 source file an owner or a named later phase. The planner brief links [#890's evidence inputs and limits](https://github.com/fetch-laboratories/openttd-rust/issues/890#evidence-inputs-for-the-phase-3-planner). Planning does not wait for #893's results; the first reader dispatch does.
- Plan only the next phase in the execution order ([ADR 0005](adr/0005-plan-only-the-next-phase.md)).
- A phase exits when its ownership is complete and every remaining body, shell, facade and generated declaration has a named later phase that removes it; an estimate or a forwarding facade is not completed ownership. Recount and place the remainder at each phase exit, and plan the newly adjacent phase before the ready queue runs dry.
- The final exit retires all remaining fork C++, generated declarations and native glue, and removes C++ compilation and link targets from the candidate game, tools and tests, with needed evidence ported to Rust or the harness.
- The first issue of each of phases 3 to 6 names the evidence for what the harness cannot see, such as network packets or script API results.
- Replace a vendored library with a maintained Rust crate when it reproduces observed behavior; otherwise port it. Plan no Squirrel port ([ADR 0006](adr/0006-bundled-libraries-and-squirrel.md)).

## Selection

- Fill a free slot with the highest-ranked ready item of the earliest stage in the execution order with ready work.
- In phases 1 and 2, port support code only when a selected component needs it. A port keeping data in C++ behind one setter per field is support code.
- Rank by stage, then by what each item removes, protects or saves: fork C++ removed and state still owned by C++, the speed profile's largest net costs, fidelity risk the harness cannot see, recurring costs such as CI time, and integrated design faults. Ease of scoping does not affect rank.
- In the cleanup stage, measure, rank and report each stream by its own inventory: #734 by workset files finished, #735 by functions over seven parameters, #736 by C++ under `tools/` and `src/tests`. The roadmap's work order sets each stream's share of slots.
- Keep one roadmap work order for ports, maintenance, testing and housekeeping. Upstream unit-test coverage alone does not select a component; do not fall back to utility kernels or `deferred-improvement` issues.
- Select concurrent owners with disjoint state changes. A genuine data or lifetime dependency waits for its owning port.

## Sizing

- Size work in counts, such as lines, files or functions, not durations.
- Size each owner to repay its fixed issue, review and Full cost: fold an item too small for that, and any batch item under about 150 counted C++ lines, into an item of the same phase that shares its files or data.
- A file pass finishes every file of its workset in one PR. Pass a workset before its callers, ordering worksets by fan-in: the number of files outside the workset that reference it through `crate::`.
- Split a workset only at a file boundary, and only when its diff would exceed about 2,000 changed lines (provisional).
- A file over about 1,500 lines gets its own single-file pass, placed by fan-in: before its workset's other files when they call it, otherwise immediately after that workset's pass. A single-file pass does not split.

## Issue placement and deferral

- Place every open issue in the roadmap: in the work order, selected or queued behind the recorded condition that brings it up for selection; on a tracker's checklist; or excluded with what brings it back, such as an owned issue's result or the `vendored-last` or `deferred-improvement` policy.
- Create an issue before leaving a component out of the work order. Root places every unplaced issue, the user's included, at startup and each time it plans, and re-checks every exclusion condition; `python3 docs/skills/steer/issues.py` lists them. Return an excluded or paused item when its condition is met.
- A historical mention or completed checklist entry does not place remaining work. Close an issue when its remaining scope finishes.
- A deferral names an owned issue whose result can lift it; waiting on another deferred or unscheduled item is circular.

## Cost decisions

- Estimate and decide every owner's cost by net fork C++ removed, counting C++ under `tools/` and `src/tests`, not original lines retired ([ADR 0022](adr/0022-progress-is-net-cpp-removed.md)).
- Decide an ownership port's cost at its first concrete boundary or evidence checkpoint from its fork-C++ change and tooling: make a bounded repair or explain the necessary cost specifically. Run the whole-fork metric there and at the integrating head only.
- The roadmap shows each owner's issue estimate beside its measured change; a revised bound cites its evidence and sits next to the estimate. Do not add separate approval gates or replan solely to force a line ratio; fix source and boundary defects.
- Infrastructure (CI, caches and evidence plumbing) states its expected tooling size in its issue and prefers platform defaults such as LRU eviction and concurrency groups to custom controllers.
- Root decides infrastructure cost at the first draft as for an ownership port; a change that raises paid CI cost needs the user's approval ([ADR 0008](adr/0008-paid-ci-cost-needs-approval.md)). An overrun alone parks nothing. Infrastructure review fixes only defects that can change a game comparison or speed decision; hypothetical defects go in PR limits.
