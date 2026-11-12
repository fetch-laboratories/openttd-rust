# Planning

Readers: root and agents selecting, sizing or planning work, or placing issues.

## Goal and phases

- Measure distance to the destination, a Rust-only game ([ADR 0001](adr/0001-full-rust-rewrite.md)), with remaining original and fork C++ in game, tools and tests, including generated code ([ADR 0022](adr/0022-progress-is-net-cpp-removed.md)). Distinguish candidate dependencies from reference-only adapters, but keep all fork-authored C++ visible until retirement; only the separate pinned original is outside that inventory. [Metric scope](rust-migration.md#shared-code-checks-and-port-metrics) records the current limits.
- The roadmap sizes these overlapping phases with exit criteria:
  1. Port simulation logic: map generation, landscape and tile loops, towns, industries, vehicles, cargo, economy, link graph, pathfinding and commands that change them.
  2. Move simulation storage: every pool, then the map.
  3. Port readers of simulation data: save/load, script API, network and NewGRF.
  4. Port support code: core, strings, settings, viewport and graphics.
  5. Port the GUI, simulation drawing and platform backends.
  6. Replace or port vendored libraries in `src/3rdparty`.
- Execution order: the cleanup stage, then phase 3, then phase 2, then phases 4, 5 and 6 ([ADR 0003](adr/0003-work-order-cleanup-readers-storage.md)). Phase 1 has exited. A pool's save/load chunks move with that pool in phase 2, because their descriptors address its record fields directly.
- The cleanup stage pays down the existing code before porting resumes ([ADR 0004](adr/0004-cleanup-before-porting.md)). The roadmap holds its work order; the stage exits when every stream reaches the end state the roadmap's "Phases" names.
- Before the cleanup stage's last items are dispatched, plan phase 3 in full from current base: re-estimate the queued readers and give every remaining phase-3 source file an owner or a named later phase. The planner brief links [#890's evidence inputs and limits](https://github.com/fetch-laboratories/openttd-rust/issues/890#evidence-inputs-for-the-phase-3-planner); planning does not wait for #893 results, but the first reader dispatch does.
- Plan only the next phase in the execution order; decide nothing about later phases unless it affects the current or next phase ([ADR 0005](adr/0005-plan-only-the-next-phase.md)).
- A phase exits when its ownership is complete and every remaining body, shell, facade and generated declaration has a named later owner or consumer phase that removes it. Phase 2 includes pool allocation and lifetime machinery and map allocation, as well as records ([storage ports](porting.md#storage-ports)). Recount and place the remainder at each phase exit; an estimate or a forwarding facade is not completed ownership. Plan the newly adjacent phase then, before the ready queue runs dry.
- The final exit has no later owner: retire remaining fork C++ sources, generated declarations and native glue, and remove C++ compilation/link targets from the candidate game, tools and tests. Port needed evidence to Rust or the harness while preserving its observations.
- The first issue of each of phases 3 to 6 names the evidence for what the harness cannot see, such as network packets or script API results.
- Replace a vendored library with a maintained Rust crate when it reproduces observed behavior; otherwise port it. Plan no Squirrel port; it is decided when phase 6 reaches it ([ADR 0006](adr/0006-bundled-libraries-and-squirrel.md)).

## Selection

- Fill a free slot with the highest-ranked ready item of the earliest stage in the execution order with ready work. Later stages fill slots earlier stages leave free.
- In phases 1 and 2, port support code only when a selected game component needs it. A port keeping data in C++ behind one setter per field is support code.
- Rank by stage, then by what each item removes, protects or saves: fork C++ removed (original, glue, and C++ under `tools/` and `src/tests`) and state still owned by C++, the speed profile's largest net costs, fidelity risk the harness cannot see, recurring costs such as CI time, and integrated design faults. Ease of scoping does not affect rank.
- A stream is one cleanup issue with its own inventory. In the cleanup stage, measure, rank and report each stream by that inventory, not by fork C++ removed: #734 by workset files finished, #735 by functions over seven parameters, #736 by C++ under `tools/` and `src/tests`. #734 and #735 remove no C++ by design. Cleanup PRs still grow neither fork C++ nor the boundary. The roadmap's work order sets each stream's share of slots.
- Keep one roadmap work order for ports, maintenance, testing and housekeeping. Upstream unit-test coverage alone does not select a component; never fall back to utility kernels or `deferred-improvement` issues.
- Select concurrent owners with disjoint state changes. A genuine data or lifetime dependency waits for its owning port. Shared accessors and services follow [integration](root-workflow.md#integration); an accessor alone does not park a whole owner behind an unrelated draft.

## Sizing

- Size work in counts, such as lines, files or functions, never in durations: agents estimate time badly.
- Size each owner to repay its fixed issue, review and Full cost. Fold an item too small for that into an owner of the same phase that shares its files or data, rather than queueing it alone. Within a batch, fold any item under about 150 counted C++ lines into another item of the batch.
- A file pass finishes every file of its workset in one PR. Pass a workset before its callers: a file passed after its callees adopts their typed forms in its one pass, while one passed before them keeps raw calls and is touched again. Order worksets by fan-in, the number of files outside the workset that reference it through `crate::`.
- Split a workset only at a file boundary, and only when its diff would exceed about 2,000 changed lines. That size is provisional; its basis is that the vocabulary PRs averaged about 150 lines.
- A file over about 1,500 lines gets its own single-file pass, so large files are not left for later. Place it by fan-in: before its workset's other files when they call it, and otherwise immediately after the workset pass that skipped it. A single-file pass cannot split, so above about 2,000 changed lines it stays one PR with one reviewer per function group ([starting reviewers](implementer-brief.md#starting-reviewers)).

## Issue placement and deferral

- Place every open issue in the roadmap: in the work order, selected or queued behind its selection event (the recorded condition that brings it up for selection, such as a named merge or phase exit); on a tracker's checklist; or excluded with what brings it back, such as an owned issue's result or the `vendored-last` or `deferred-improvement` policy.
- Create an issue before leaving a component out of the work order. Root places every unplaced issue, including the user's, at startup and whenever it plans, and re-checks every exclusion condition. `python3 docs/skills/steer/issues.py` lists them.
- Return an excluded or paused item when its recorded condition is met. A simulation owner set aside after a failed gate remains yours to re-select once a profile shows the fix.
- A historical mention or completed checklist entry does not place remaining work. Check the current action or selection event. Close an issue when its remaining scope finishes.
- A deferral names an owned issue whose result can lift it. Waiting on another deferred or unscheduled item is circular. Fix the boundary of a core loop (tile loop or vehicle tick) that misses the retirement ratio instead of abandoning it.

## Cost decisions

- Estimate and decide every owner's cost by net fork C++ removed, counting C++ under `tools/` and `src/tests`, not original lines retired ([ADR 0022](adr/0022-progress-is-net-cpp-removed.md)). A consumer line rewritten to reach moved data counts as one original line retired and one glue line added, so it moves neither. The boundary (Rust exports plus callback slots) falls or holds; growth needs a reason the reviewer accepts. Phase 2 owners also count crossings removed and services deleted.
- Decide an ownership port's cost at its first concrete boundary or evidence checkpoint using its fork-C++ change (including `tools/` and `src/tests`) and tooling: make a bounded repair or explain the necessary cost specifically. Run the whole-fork metric twice per PR, at that boundary and at the integrating head; briefs request no other recounts.
- The roadmap shows each owner's issue estimate beside its measured change. A revised bound cites its source evidence and is listed next to the estimate, never in place of it. Do not create separate implementation, native-test and tooling approval gates or replan solely to force a line ratio; fix source and boundary defects.
- A new evidence requirement names the behavior existing checks cannot distinguish ([evidence](porting.md#evidence)).
- Infrastructure (CI, caches and evidence plumbing, excluding scenario or test support within an ownership port) states expected tooling size in its issue and prefers platform defaults and settings such as LRU eviction and concurrency groups to custom controllers.
- Root decides infrastructure cost at the first draft as for an ownership port, explaining necessary cost in the PR; a change that raises paid CI cost needs the user's approval ([ADR 0008](adr/0008-paid-ci-cost-needs-approval.md)). An overrun alone parks nothing and needs no user decision. Review fixes only defects that can change a game comparison or speed decision on this host or in CI; hypothetical defects go in PR limits.
