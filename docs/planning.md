# Planning

Readers: Root and agents selecting or planning work.

## Goal and phases

- The destination is OpenTTD as a Rust program: every original C++ line retired, the game behaving as the original does, and root's agents getting there on their own. Distance includes remaining original and fork C++ in game, tools and tests, including generated code. Distinguish candidate dependencies from reference-only adapters, but keep all fork-authored C++ visible until retirement; only the separate pinned original is outside that inventory. [Metric scope](rust-migration.md#shared-code-checks-and-port-metrics) records the current limits.
- The roadmap sizes these overlapping phases with exit criteria (user, 2026-10-28):
  1. Port simulation logic: map generation, landscape and tile loops, towns, industries, vehicles, cargo, economy, link graph, pathfinding and commands that change them.
  2. Move simulation storage: every pool, then the map.
  3. Port readers of simulation data: save/load, script API, network and NewGRF.
  4. Port support code: core, strings, settings, viewport and graphics.
  5. Port the GUI, simulation drawing and platform backends.
  6. Replace or port vendored libraries in `src/3rdparty`.
- Plan no phase beyond the next; decide nothing about later phases unless it affects the current or next phase (user, 2026-10-28).
- A phase exits when its ownership is complete and every remaining body, shell, facade and generated declaration has a named later owner or consumer phase that removes it. Phase 2 includes pool allocation and lifetime machinery and map allocation, as well as records ([storage ports](porting.md#storage-ports)). Recount and place the remainder at each phase exit; an estimate or a forwarding facade is not completed ownership. Plan the newly adjacent phase then, before the ready queue runs dry.
- The final exit has no later owner: retire remaining fork C++ sources, generated declarations and native glue, and remove C++ compilation/link targets from the candidate game, tools and tests. Port needed evidence to Rust or the harness while preserving its observations; calling a fork-authored adapter reference-only does not exempt it. Keep the pinned original unchanged as the separate oracle.
- The first issue of each of phases 3 to 6 names the evidence for what the harness cannot see, such as network packets or script API results.
- Replace a vendored library with a maintained Rust crate when it reproduces observed behavior; otherwise port it (user, 2026-10-28).
- Decide Squirrel when phase 6 reaches it; deviation from the original may then be acceptable, so do not plan a Squirrel port (user, 2026-10-28).

## Selection

- Fill a free slot with the highest-ranked ready item of the earliest phase with ready work. Later phases fill slots earlier phases leave free (user, 2026-10-28).
- In phases 1 and 2, port support code only when a selected game component needs it. A port keeping data in C++ behind one setter per field is support code.
- Rank by phase, then by what each item removes, protects or saves: compiled C++ removed (original plus glue) and state still owned by C++, the speed profile's largest net costs, fidelity risk the harness cannot see, recurring costs such as CI time, and integrated design faults. Ease of scoping does not affect rank.
- Estimate and decide every owner's cost by compiled C++ removed, not original lines retired. A consumer line rewritten to reach moved data counts as one original line retired and one glue line added, so it moves neither. Phase 2 owners also count crossings removed and services deleted.
- Size each owner to repay its fixed issue, review and exact-head Full cost. Fold an item too small for that into an owner of the same phase that shares its files or data, rather than queueing it alone.
- Keep one roadmap work order for ports, maintenance, testing and housekeeping. Upstream unit-test coverage alone does not select a component; never fall back to utility kernels or `deferred-improvement` issues.
- Select concurrent owners with disjoint state changes. A genuine data or lifetime dependency waits for its owning port. Shared accessors follow [integration](workflow.md#integration): the first integrating PR supplies the definition, or root extracts a shared service needed by multiple drafts into its own PR. Other consumers adopt that integrated definition without parallel copies; an accessor alone does not park a whole owner behind an unrelated draft.

## Issue placement and deferral

- Place every open issue in the roadmap: in the work order, selected or queued behind its selection event; on a tracker's checklist; or excluded with what brings it back, such as an owned issue's result or the `vendored-last` or `deferred-improvement` policy.
- Create an issue before leaving a component out of the work order. Root places every unplaced issue, including the user's, at startup and whenever it plans, and re-checks every exclusion condition. `python3 docs/skills/steer/issues.py` lists them.
- Return an excluded or paused item when its recorded condition is met. A simulation owner set aside after a failed gate remains yours to re-select once a profile shows the fix.
- A historical mention or completed checklist entry does not place remaining work. Check the current action or selection event. Close an issue when its remaining scope finishes.
- A deferral names an owned issue whose result can lift it. Waiting on another deferred or unscheduled item is circular. Fix the boundary of a core loop (tile loop or vehicle tick) that misses the retirement ratio instead of abandoning it.

## Cost decisions

- Decide an ownership port's cost at its first concrete boundary or evidence checkpoint using its compiled-C++ change and tooling: make a bounded repair or explain the necessary cost specifically. Run the whole-fork metric twice per PR, at that boundary and at the integrating head; briefs request no other recounts.
- The roadmap shows each owner's issue estimate beside its measured change. A revised bound cites its source evidence and is listed next to the estimate, never in place of it. Do not create separate implementation, native-test and tooling approval gates or replan solely to force a line ratio; fix source and boundary defects.
- A new evidence requirement names the observable behavior existing checks cannot distinguish. Tracing pure helpers solely to reproduce the source call sequence is unnecessary.
- Infrastructure (CI, caches and evidence plumbing, excluding scenario or test support within an ownership port) states expected tooling size in its issue and prefers platform defaults and settings such as LRU eviction and concurrency groups to custom controllers.
- Root decides infrastructure cost at the first draft as for an ownership port, explaining necessary cost in the PR. An overrun alone parks nothing and needs no user decision (user, 2026-10-26). Review fixes only defects that can change a game comparison or speed decision on this host or in CI; hypothetical defects go in PR limits.
