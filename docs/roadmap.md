# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-02, twenty-eighth steering at integration 65)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

- Code is `6afdcbf390` (integration 65). Root has been paused since the wind-down at 16:20Z. The 11-02 session integrated #648 (61), #645 (62), #640 (63, phase 1's exit), #642 (64) and #651 (65), with no rollbacks.
- Whole fork at `6afdcbf390`, as in the Phases chart:
  - 56,176 of 284,355 original code lines retired (19.76%);
  - compiled C++ 262,065 (92.16%) and glue 33,886;
  - 1,443 exports and 4,064 callback slots;
  - Rust 127,673 and tooling 53,611.
  Stocktake 64 (`5e414c5a64`) is in Git history (`dca42d6c02`); stocktake 66 has not run.
- Compiled C++ is undercounted: `tools/port-metrics.py` skips `.inc` files, and three `src/rust/*.inc` files hold 445 code lines (487 physical lines) of fork glue. #658 fixes the count; record the +445-code-line step as a re-baseline separately from cleanup savings.
- Integrations 61 to 65 removed 2,437 compiled lines, 2,310 of them from #642's vendored swap. The other four removed 127 lines and added 1,507 Rust and 826 tooling lines.
- Open drafts at their heads (`python3 tools/port-metrics.py --head HEAD`, steering measurement):

| PR (issue) | Head | Retired | Net glue | Compiled C++ | Rust | Tooling |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| #655 (#652) | `54c44c68c5` | 752 | +113 | -639 | 1,756 | 584 |
| #636 (#570) | `6191d5f9b0` | 742 | +365 | -377 | 1,710 | 517 |
| #649 (#604, #199) | `ba07c85bb1` | 453 | +428 | -25 | 437 | 118 |
| #628 (#574) | `2711496991` | 1,940 | +2,002 | +62 | 5,291 | 60 |
| #647 (#644, #646) | `9e0d5af362` | 0 | +173 | +173 | 870 | 214 |

- Speed: the table is profile 64 at `5e414c5a64`, flat against profile 62 (-0.11% to +0.07% Ir). Every scenario stays 0.12% to 0.90% under its anchor count. The drawing workload (#566) runs at 1.0027x reference Ir. The instruction harness rejects the anchors because #642 changed `rust/openttd-kernels/Cargo.toml`, part of the build identity. Projections use the speed formula by hand until a quiet speed pass re-anchors (recurring list).

| Scenario | Candidate Ir, profile 64 | Projection | Carried three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 2,731,301,085 | 1.111722x | 1.154106x |
| Grok | 26,898,981,923 | 1.387147x | 1.385653x |
| Opus | 28,297,683,609 | 1.393949x | 1.394556x |
| P1996 | 4,542,468,375 | 1.336551x | 1.337764x |
| P2000 | 7,400,813,786 | 1.400577x | 1.401446x |
| P2016 | 13,400,911,478 | 1.522911x | 1.481910x |

- **The platform cadence after integration 65 is red** (run 41466312349 at `6afdcbf390`, both attempts).
  - macOS Debug (attempt 1) and macOS Release (attempt 2) fail `tools/check-rust-platform.py:146-156`: "rust-build.log: native libraries differ from the configure query". The Windows and annotation failures cascade from it.
  - The steering read the Release job log. The build prints `native-static-libs: -lc -lm -liconv -lSystem -lc -lm`.
  - The configure probe (`cmake/Rust.cmake:137-156`) compiles a staticlib with no dependencies and writes its line to `rust-native-libs.log`, which the job log does not show.
  - Root's diagnosis stands: `libc` reached `Cargo.lock` through `cpufeatures` in #642's merge `5e414c5a64`.
  - #651's code merge follows it, so the bounded rollback is unavailable. The repair comes first; no integration happens before it.
- Coverage: #156 holds the gaps, including those from this review's #642, #651 and #655 audits. #503 comes last.
- Unintegrated branches: five (#647, #636, #628, #649 and #655).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite ([goal and phases](planning.md#goal-and-phases)). Each free
slot takes ready work from the earliest phase, and later phases fill the rest
(user decision, 2026-10-28). Position at code `6afdcbf390` (integration 65) in
code lines from `python3 tools/port-metrics.py --whole-fork`, with the change
since the previous navigation's position at `2a89fb023f` (integration 47) in
parentheses. Source areas are an accounting classification: they do not move
drawing or executable header logic into a different phase. "Retired" counts a
rewritten consumer line as retired and its replacement as glue, so compiled C++
(original plus glue) is the distance.

| Area | Original | Retired | Left | Glue added | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 37,640 (+69) | 9,784 | 6,460 (-514) | 16,244 (-583) |
| Simulation storage | 13,724 | 2,767 (+50) | 10,957 | 1,313 (-22) | 12,270 (-72) |
| NewGRF | 10,785 | 5,201 (-79) | 5,584 | 349 (-79) | 5,933 (0) |
| Save/load | 13,388 | 1,075 (+539) | 12,313 | 727 (+142) | 13,040 (-397) |
| Script API | 14,908 | 1,202 (+324) | 13,706 | 542 (+89) | 14,248 (-235) |
| Network | 12,713 | 561 (+350) | 12,152 | 234 (+52) | 12,386 (-298) |
| Support | 21,617 | 930 (-52) | 20,687 | 452 (-61) | 21,139 (-9) |
| Graphics and viewport | 10,985 | 4 | 10,981 | 1 | 10,982 |
| GUI | 52,179 | 115 (-2) | 52,064 | 162 (-3) | 52,226 (-1) |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 2,317 (+1,795) | 52,831 | 0 (-163) | 52,831 (-1,958) |
| Upstream tests | 2,248 | 0 | 2,248 | 5,292 (+174) | 7,540 (+174) |
| Boundary layer | 0 | 0 | 0 | 18,352 (+198) | 18,352 (+198) |
| **Whole fork** | **284,355** | **56,176 (19.76%, +2,994)** | **228,179** | **33,886 (-187)** | **262,065 (92.16%, -3,181)** |

The boundary has 1,443 Rust exports (+66) and 4,064 callback slots (-142).
Rust totals 127,673 code lines (+3,448) and tooling 53,611 (+2,301).
Simulation-source retirement is 40,407 of 61,148 (66.08%, +119). The duplicate
baselines at `5e414c5a64` hold 418 rebound-callee rows (581), 126
shared-accessor rows (128) and 54 orphan exports (55). #607 (integration 48)
re-baselined the count: its sweep removed 812 retired and 813 glue lines that
`WITH_RUST` fallbacks had counted on both sides, leaving compiled C++ unchanged.

| Period (steering and navigation commits) | Days | Retired | Per day | Compiled C++ change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start (`6ced7c097e`, 10-02) to 10-27 | 24.7 | 49,536 | 2,010 | -17,908 | -730 |
| 10-27 to the navigation at integration 47 (10-31) | 4.1 | 3,646 | 890 | -1,201 | -290 |
| Navigation to the twenty-sixth steering (11-01) | 1.12 | 13 (825 without #607) | 10 (730) | -488 | -430 |
| Twenty-sixth to twenty-seventh steering (11-02) | 0.59 | 197 | 330 | -256 | -430 |
| Twenty-seventh steering to now | 0.22 | 2,784 | | -2,437 (-479 without #642's vendored swap) | |
| **Since the last navigation** | **1.93** | **2,994** | **1,550** | **-3,181 (-1,223 without the swap)** | **-1,650 (-630)** |
| **Whole history** | **30.75** | **56,176** | **1,830** | **-22,290** | **-730** |

Days are calendar days. Integrations 48 to 65 landed in three root sessions
(10-31 19:47Z to 21:33Z, 11-01 16:42Z to 11-02 00:17Z, 11-02 12:00Z to 14:52Z);
the gaps were Claude session limits, user pauses and steering. What set the rate:
- #642 replaced bundled Monocypher with maintained crates: 1,958 of the
  3,181-line fall. Without it, compiled C++ fell 630 lines per day, twice the
  previous period's 290 but under the whole-history 730. At that rate the
  209,000 non-vendored compiled lines take about 330 days.
- The other ownership ports moved compiled C++ between +28 (#645) and -224
  (#633) each. Phase-3 owners delivered under their estimates: #575 planned
  1,000 to 1,350 retired and delivered 354 with +220 net glue (compiled -134).
- #570, phase 2's template, has not integrated: its checkpoint stands at
  compiled C++ -377.

Why the course changed:
1. Phase 2 could exit with C++ kept indefinitely. Its exit criterion here
   required "no per-field service ... for moved data", and
   [storage ports](porting.md#storage-ports) deleted only "replaced" services.
   #563's design keeps "Money/date fields and unselected vehicle fields in the
   shell" (`docs/design/world-state.md`, #563 records), yet Rust's maintenance
   owner reads reliability, age, value, profit and breakdown fields through
   `OpenTTDVehicleMaintenanceServices` and the shared vehicle table's
   `write_profit`, `write_reliability` and service-date slots. No later phase
   moves simulation fields, so those services and the shells would never
   retire. A pool is now moved only when no Rust code reaches any of its
   fields through a service, and #656 owns the vehicle remainder.
2. Planning ranked and costed owners by retired lines everywhere except phase 2
   ([selection](planning.md#selection)). Retired lines count rewritten lines
   and ignore glue, so #575's +220 net glue never counted against it. Every
   phase now ranks and decides cost by compiled C++ removed.
3. The planning trigger in `/start-development` (step 3) counted unstarted
   owners, including blocked ones. Phase 2's unstarted owners all wait for
   #570, and phase 3's queued #653 and #573 slice 3b wait for drafts, so
   planning would not run while no owner is ready and about 35,000 phase-3
   lines have none. It now counts ready owners.

| Phase | Left (code lines) | Exit criterion | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | Its bucket's 9,784: about 3,780 drawing and description (phase 5), 490 startup in `openttd.cpp` (phase 4), 1,975 facade shells that retire with their callers, 540 file scope and about 3,000 non-drawing bodies | Every remaining non-drawing body belongs to the phase-2 owner of its file | Exited at integration 63 (#640, #559) |
| 2. Simulation storage | 10,957 in the storage bucket, the phase-1 bodies above, and the per-field services within the boundary layer's 18,352 glue lines and 4,064 callback slots | No Rust code reads or writes a pool or map field through a C++ service or per-call-site reader ([storage ports](porting.md#storage-ports)). Records and map arrays are defined once in Rust, keep the original pool, lifetime and save behavior ([world-state design](design/world-state.md#transferring-shared-storage)), and are read in place by both languages; C++ calls Rust only for operations. Fields only unported C++ touches stay in shells, each named with the phase that moves it. Each pool owner also ports the remaining non-drawing bodies in its files | Current. #570 (#636) is the first owner; -377 is its historical checkpoint measurement, not its current head. No unstarted phase-2 owner is ready before it integrates. Then #634; combined #656/#598/#599; #635; link graph/jobs; #637 with #497; orders/lists/backups; companies/groups; #638 with #498; then map storage and the remaining map bodies (ranked work order below). |
| 3. Simulation data readers | 39,962: save/load 12,313, script 12,782, network 9,283, NewGRF 5,584. Script and network GUI (3,793) are phase 5 | These run in Rust. The Squirrel VM and the C++ binding shims it calls may stay until phase 6 | Next, beside phase 2. #575 integrated (65); current owners are #573, #574 (#628), #652 (#655) and #653. READY: #666 infrastructure APIs, #667 content and #668 coordinator protocols. #669 queues the other reader owners and their selection events. The 39,962 left and former roughly 35,000 unowned figures are remaining-original accounting; the compiled-source planning envelope is distinguished below. |
| 4. Support | 45,100: support 20,687, graphics and viewport 10,981, startup 490, and data tables 12,931, which retire with their consumers | Not planned yet | Planned when phase 2 exits and phase 4 becomes the next phase (user direction, 2026-10-28) |
| 5. Interface | 71,570: GUI 52,064, platform 11,941, script and network GUI 3,793, and about 3,780 drawing lines in the logic bucket | Not planned yet | As above |
| 6. Vendored | 52,831. Only Squirrel (8,774) and md5, ICU and the social API (452) need a port or crate. nlohmann JSON (14,452), fmt (11,986), Catch2 (11,469) and the OpenGL headers (5,604) retire with their last C++ user | Not planned yet | As above |

Upstream unit tests (2,248) and fixture glue retire with the code they test.
Boundary-layer glue retires as its C++ callers do; phase 2 deletes its per-field part.

Root's first actions from this chart, after the resume checkpoint's cadence repair:
1. Finish the open branches and take #507 in the first free branch slot. No
   unstarted phase-2 owner is ready before #570; three phase-3 owners are READY.
2. Finish #570's checkpoint 2 and integration. Before briefing #634/#635, root
   decides record placement using the merged #570 profile and #566 drawing
   workload, after the owed quiet speed pass. Then use the accepted ranking.
3. Record each owner's compiled-C++ estimate in its issue and actual change at
   integration, and compare them at each stocktake. #656/#598/#599 are one owner.

## Accepted reader and pool placement (2026-11-03, after integration 67)

Agent: /root/place-after67-reader-plan | Model: gpt-6.1-sol | Reasoning effort: high

Root accepted the planning report. Phase 1 stays exited; this places current
phase-2 and next-phase reader work, with no broad phase-3 retirement promise.
The measured **41,814 compiled-source envelope** comprises save/load 13,040,
script/AI/Game excluding GUI 13,317, network excluding GUI 9,524 and NewGRF
5,933. Reserving current codec/game-network/text owners (#573/#574/#652) and
1,057 dedicated Squirrel wrappers leaves roughly 33,600; reserving API
registration/declaration/binding surfaces and the largely VM-facing
`ScriptInstance` shell gives about 30,200. Thus **roughly 30,000 to 33,600** is
the unowned planning envelope, not removable compiled C++. Newly placed owners
and #669 allocate it. #653 already owns industry definitions; specification
files classified under simulation storage/logic add work outside the NewGRF
bucket. Retain the VM, registration glue, stack conversions and
exception/suspension shims. Executable API header methods belong to their API
owners; whole headers and new forwarding methods are not savings.

Pool briefs include allocation bookkeeping (`data`, `used_bitmap`, `first_free`,
`first_unused`, `items`, assertion credit `checked`, `cleaning` and applicable
allocation-cache state), allocation/indexed allocation, reuse, iteration,
reset/cleanup and construction publication/destruction hooks. The generic
native pool template earns no repeated deletion credit while any C++ pool
instantiates it. Every Rust-used name/container/cache field moves in phase 2;
only exact original-C++-only fields may await a named later owner. Record lookup,
coupled save descriptors and generated declarations follow integrated #570.

Exact remaining shell dispositions constrain briefs:

| Boundary | Phase-2 ownership and exact later-owned remainder |
| --- | --- |
| Vehicle/type shells | Combined #656/#598/#599 takes maintenance, cargo/identity/loading, resolver fields, names and Rust-used sprite/cache fields; existing controller/cargo/order/visual/occupancy owners stay canonical. `coord`, `bounds`, viewport hash links and `fill_percent_te_id` belong to phase-5 drawing/viewport; `old_orders` belongs to #669 legacy conversion. Identity/vtable/owner-pointer scaffolding retires with final callers. |
| Stations/road stops/cargo | #634 takes names and Rust-used caches, spec-reference vectors, custom road-stop data, catchment/loading/nearby-list facades, GoodsEntry cargo/flow attachments and CargoPacket/CargoPayment pool completion. Link-graph owner takes flow/completed-job publication. `sign` belongs to phase 5; spec definitions belong to #669. |
| Town/industry/subsidy | #635 takes town `psa_list`, Rust-used `cached_name`/caches, industry `psa`, town/station links and subsidy flags; existing Rust name/cargo/count/nearby owners stay canonical. Town `cache.sign`/`show_zone` belong to phase 5. Industry `cached_name` may await phase-4 formatting only without a Rust field route; presentation facade storage retires with phase-4 string callers. Persistent-storage payload/pool is #638; industry definitions #653 and HouseSpec #669 after #628. |
| Engine | #637/#497 takes info, vehicle-specific info, original image/type/list position, GRF/override/badge payloads, lifecycle and name access after adopting #655. Only `display_flags`/`display_last_variant` await phase-5 build-list GUI. |
| Orders/lists/backups | All current fields and pool state move in phase 2; `old_order_index` and `OldOrderSaveLoadItem::{index,next,order}` belong to #669 legacy conversion. |
| Company/group | Phase 2 takes settings, finance/statistics aliases, identity, names/livery and free-unit/free-group state used by Rust. AI instance/info/config payload belongs to #669 script orchestration; authorization-list payload to #669 network remainder. Rust-used links move with the record. Only `Group::folded` awaits phase-5 GUI. |
| LinkGraph/LinkGraphJob | Graph/settings snapshots, join/abort state, annotations, flows and publication move in phase 2, including #168/#199/#500 completed-job publication. A native thread handle may remain behind a named join/start operation until platform threading moves; no field-callback exception follows. |
| Small pools | #638 includes objects, depots, signs, goals, story pages/elements, league tables/elements and persistent storage, plus #498 payloads and Rust-used strings. Only `Sign::sign` awaits phase-5 drawing/viewport. |

All issue estimates are net compiled C++ removed, including retained/generated
declarations, facades and fixtures. Rewritten consumers receive no reduction
credit. #658's **+445 code-line .inc re-baseline** is reported separately from
its small cleanup savings and #657's deletion savings. Reader evidence compares
API return values/errors/command results/enumeration and protocol packets,
ordering, timeout/error/connection decisions; world-state equality alone misses
these observations. #666/#667/#668 use integrated interfaces only. If a necessary
new service is unintegrated, its owner blocks; it does not use #628 draft
services or build a parallel service.

## Twenty-eighth steering review (2026-11-02: missed child resumes and compiled C++ on open drafts)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

**What was audited.** Five independent read-only audits compared source with the pinned original, with no builds or games:
- #642 and #651 (integrated) and #655 (draft), by Codex `gpt-6.1-sol` at high effort, because the Claude session window was projected near its limit;
- #636 and #628 (drafts), by Claude Opus at high effort.

The steering checked every claim acted on below against the source and the CI logs.

**Root's track.** The 11-02 Claude Code session ran from 11:18Z to 16:22Z.
- It applied the twenty-seventh corrections in its first turn.
- It integrated five PRs (61 to 65) with eight Full runs. Five passed and three failed: #651's first and #649's two. Two of the failures were generated-ABI include paths.
- It routed implementation to Codex while the Claude session window was projected over its limit. After the user's correction that only the 5-hour window paces (user, 2026-11-02), it returned to the normal Claude Code root setup.
- The navigation after the wind-down (`c05fdd45e6`) moved every phase to compiled-C++ ranking, required every Rust-used field to move, and made the planning trigger count only ready owners.

**No reachable divergence** in #642, #651, #655, #636 or #628. #642's crate composition matches Monocypher 4.0.2 for X25519, BLAKE2b, the incremental AEAD and cofactored Ed25519, and it draws random bytes in the original order.

1. **Codex children are not always resumed when their job exits.**
   - Four times on 11-02, a T3-delegated Codex child ended its turn on a named job that then exited without waking it:
     - #647's reviewer, from 11:55Z to 14:10Z;
     - the same reviewer again, from 14:11Z until root cancelled it at 15:56Z;
     - #636, from 15:08Z to the wind-down;
     - #655, from 15:55Z to the wind-down.
   - That is about 6 agent-hours. It is why #647, ranked first in maintenance, did not integrate while five other PRs did. Root noticed the first stall after 2 h 15 min.
   - Correction: a child checks its job once before ending a turn (`AGENTS.md`). Root checks every child idle on a named job at each of its own wakes ([team](team.md#spawning), `/start-development` step 1).
2. **Ports add Rust and glue faster than they remove compiled C++.**
   - Excluding #642's vendored swap, integrations 61 to 65 removed 127 compiled lines and added 1,507 Rust and 826 tooling lines.
   - #628 adds compiled C++ (+62) after 5,291 Rust lines. About 24% of its glue is native fixtures, and 172 lines are per-message log slots.
   - The navigation's compiled-C++ rule had not yet reached open drafts.
   - Correction: #628 makes the bounded repair in its steering comment before review, for an expected -240 to -360 once #657 lands. #655 and #636 record compiled-C++ cost decisions in their PRs.
3. **Ports add one log service per message.**
   - `rust-migration` has 166 such services (41 `Debug`, 125 `GrfMsg`), each a table field, a declaration and a lambda. #628 adds about 86 more.
   - Correction: #657 ports both macros once. [Boundary](porting.md#boundary) names a per-message service a design fault once #657 integrates.
4. **Compiled C++ omits `.inc` glue** (487 physical lines in three files). Correction: #658, which also carries one boundary leftover from #642 and two from #651.
5. **A dependency change reached the platform matrix only after integration.** #642 added crates, the cadence after integration 65 went red, and it now holds every integration. Correction: a change to `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` or `cmake/` runs the platform profile once before Full ([checks](workflow.md#checks)).
6. **#636 is on course.**
   - Every saving so far comes from deleted services (about -420). The roughly 740 rewritten consumer lines net zero.
   - Finishing checkpoint 2 should reach about -900 to -1,250 compiled C++.
   - Before integration, `train::Handle` must carry the record: today 42 sites in `train.rs` call `record_from_shell` on each use, every train tick.
   - The steering comment on #636 lists the remaining inventory and the dependent-load table for the placement decision.
7. **#655 has two Rust-to-C++-to-Rust bounces and a broken comparison fixture.** The bounces are `MapGRFStringID` → `GetGRFStringID` → Rust, and `HandleChangeInfoResult` → `DisableGrf` → Rust. Because of the fixture, no scenario covers its head yet.
8. **#647's resume round is scoped** under the infrastructure review rule:
   - its four gate gaps are hypothetical forms; fix the first if it is cheap and list the rest as limits;
   - one fresh reviewer finishes checks 5 to 7.

No user direction is reversed. On 2026-11-02 the user reselected #507's NewGRF corpus, which #654 had asked about. The steering's first answer wrongly treated #526's GPL source work as a prerequisite; the resume plan on #507 already kept the sets uncommitted. [Evidence](porting.md#evidence) and `AGENTS.md` now state that rule, and #526 is closed. The restart prompt gains one standing step (finding 1).

| Twenty-seventh correction | Verdict and cost/result |
| --- | --- |
| 1. #644 and #646 take the first free slot | Slipped, through finding 1. Root started #647 in its first turn, but its reviewer stalled twice, so five integrations went first. Non-test `bool` returns grew by one, and #647's base merge converts them. No divergence was observed. |
| 2. Phase-2 cost by compiled C++; placement from the post-#570 profile | Applied to #636, which reports compiled C++ -377. The navigation extended the rule to every phase. The placement decision is not due yet. |
| 3. Read failed Fulls to the end | Applied: #645 integrated at its first Full after the packing fix. Generated-ABI include paths still failed #651's and #649's Fulls. #649's `128c1b9c96` adds `build-rust/generated-abi` to `CPATH` for every driver compile, which ends that class of failure when it integrates. |
| 4. Issue placement | Applied: #604 is in #649, and #497's Action 3 joined #652. |

| Change since the twenty-seventh review | Verdict and next observation |
| --- | --- |
| Navigation `c05fdd45e6`: move every Rust-used field, rank every phase by compiled C++, and count only ready owners in the planning trigger | Untested. Watch whether the resume's planning round ranks by compiled C++, whether #656 is sized from #636's final inventory, and whether open drafts record a compiled-C++ cost decision before review. |
| Only the 5-hour session window paces Claude agents (user, 2026-11-02, in the user's global instructions) | Root applied it at once. |

New since this review, for the next steer to judge:
- Whether any child stays idle on an exited job across more than one root wake.
- Whether #628 reaches review with compiled C++ below zero and a named end for each remaining bounce.
- Whether #657 lands before #628 and #655 integrate, and how much compiled C++ it removes.
- Whether #636 finishes checkpoint 2 near -900 to -1,250, with `train::Handle` carrying the record.
- Whether the cadence repair and the platform-profile rule keep the next cadence (after integration 70) green.

## Earlier steering, still in force

Ownership, original behavior, process scope, named callees, typed shared services
and exact random-draw order come first. Preserve default semantic evidence,
record gaps on #156 and keep retirement distinct from glue. No hot allocation,
opcode dispatch, whole-record copies or owner bounces; no C++ frame is kept only
for attribution. Rust reaches the map by a load through `map_func::World`, with
native assertions and no C++ fallback, and descriptor reads stay within
`docs/design/world-state.md`. The rebound-callee, shared-accessor and
orphan-export baselines only shrink. An owner does not port support code whose
data stays in C++ behind one service per field. Ready owners proceed beside
speed work; only timed collection holds builds and games. Ceiling and band
decisions use the three-pass record; the projection factor and speed thresholds
remain provisional, and 0.1% to 0.3% speed candidates get no attribution
research. Testing is maintenance: scenarios arrive with the ports that need
them. Six branches, batched maintenance, fixing reviews, strict exact-combination
Full, bounded rollback and action-based wind-down remain. A historical mention
is not placement; CI-found behavior fixes get a scoped review; deep checks run
only on the user's request; active speed dependencies are preserved. Third-party
test inputs stay out of Git and CI artifacts ([evidence](porting.md#evidence)). Every agent runs
its own long jobs and ends a turn only on a named job ID; root checks any child
that ends a turn without one, and any child idle on a job that has exited. A
base update that meets a review condition needs only CI. Each new scenario,
fixture or checker names the behavior existing checks cannot distinguish; this
rule is provisional, on a basis of 1.5 tooling lines per retired line. Roadmap
commits happen only at stocktakes, course changes and wind-down. #627's
per-container typed leaves for STL containers are a phase-3 necessity; they do
not justify per-field leaves in phase-2 records. Every owner's cost is its
compiled-C++ change, decided before review. Rust-defined `extern "C"` functions
return 0-or-1 `u8`, not `bool` (#644). Read a failed Full to its last job before
recording its cause. Before briefing #634 and #635, root decides record
placement from the post-#570 profile. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence.

## Recent completed work

Rows cover the period since the twenty-sixth steering review; earlier rows are
in their PRs and Git history. Metrics are `tools/port-metrics.py` code lines:
Rust / tooling / original C++ retired / net glue.

| Integration | Issues / PR | Merge commit | Metrics |
| --- | --- | --- | --- |
| 57 | #629 multiplayer comparison isolated from host loopback probes / #632 | `46a2f240d7` | 0 / 8 / 0 / 0 |
| 58 | #622 later batch: shared NewGRF spec and GRF-file bindings / #639 | `83c62ef441` | net glue -34 / rebound rows 523 to 420 |
| 59 | Shared-record declaration generator (#570, #641) / #643 | `2d4a063e33` | 48 / 236 / 0 / +8 |
| 60 | #69 tile-area geometry and the diagonal iterator / #633 | `b97004e599` | 529 / 5 / 197 / -27 |
| 61 | Platform cadence: landscape fixture `shared` initializer / #648 | `cdfc2de482` | 0 / 0 / 0 / 0 |
| 62 | #641 packet state and byte storage in a generated Rust record / #645 | `bf2c4632a2` | 718 / 175 / 52 / +80 |
| 63 | #559 command invocation lifecycle (phase 1's last owner) / #640 | `7940e700c2` | 403 / 287 / 136 / +115; rebound rows 420 to 418, orphan exports 55 to 54 |
| 64 | #64 maintained crypto crates and Rust network authentication / #642 | `5e414c5a64` | 2,027 / 480 / 2,242 / -68 |
| 65 | #575 script host, command context and event lifecycle / #651 | `6afdcbf390` | 386 / 364 / 354 / +220 |
| 66 | #659 carried Full validation across pushes that keep the tested non-Markdown tree / #660 (CI only; the user allowed it ahead of the cadence repair) | `f47064fc57` | 0 / tooling only / 0 / 0 |
| 67 | #661 Emscripten compiler cache / #662 (CI only; the user allowed it ahead of the cadence repair) | `d93afabee9` | 0 / tooling only / 0 / 0 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

1. **Repair the red platform cadence first, #664/#665** (resume checkpoint). Local build passed and review is clear; root owns platform and exact-head Full, integration and the cadence rerun. No other integration happens until the merged repair's cadence is green.
2. **Finish the open branches in readiness order**, under the cadence gate:
   - #649 (#604 with #199's disjoint items): scoped fixture review is clear at `ba07c85bb1`; update from current base, exact-head Full and merge.
   - #647 (#644 with #646): head `166c852026`; root resolves the scoped review's six genuine orphan findings, then affected checks, Full and merge. The hypothetical gate limits stay documented.
   - #570/#636: finish checkpoint 2's moved-field route deletion, record-carrying train handle, disaster naming and exact-head focused evidence; review and integrate. Expected checkpoint-2 compiled C++ reduction is 900 to 1,250. Current active implementation is in its PR status comment.
   - #574/#628: finish the bounded cost repair and slice 5's default two-client command/quota/replay/reconnect observations in both server roles, then one independent review. The twenty-eighth steering requirements and named bounce endpoints remain; adopt shared #657 when integrated, with no private helper.
   - #652/#655: head `37f3c58534` is ready for independent function-group review, which root launches. Reported compiled C++ reduction is 645; exact commands/limits stay in its status comment. Formatting-time GRF strings (about 170 lines) remain phase 4, engine GRF/override payload #637, ObjectSpec #638. Full remains pending.
3. **#507 NewGRF corpus** (user reselected, 2026-11-02, closing #654; #526 closed as unneeded), in the first free slot. It is maintenance with one reviewer and one Full.
   - Open a new PR from `rust-migration`. Reuse #525's scenario module (`tools/simulation/newgrf_sets.py`) and its scenario AI from `testing/corpus507-newgrf` at `ccc28e2ed0`, but not its committed `.grf` files or source archive.
   - Pin the three OpenGFX+ sets (Trains 0.3.0, Road Vehicles 0.4.1, Industries 0.3.5) by name, version and SHA-256, and fetch them from the BaNaNaS CDN into ignored `.local/` storage ([evidence](porting.md#evidence)).
   - Extending `tools.simulation.community_ai` and its cached manifest avoids a workflow change. Keep the sets out of every uploaded artifact.
   - The default slice is `newgrf-sets-arctic` (about 7 s of game time); `newgrf-sets-arctic-long` runs in soak. The same-NewGRF larger save follows as a later #507 PR.
   - Integrate it before #655 if it is ready first; #655 does not wait for it.
4. **Maintenance batch: #657 + #658**, built by root from `rust-migration` after #647 and #649 integrate, with one reviewer and one Full. #657 removes about 300 to 450 compiled lines on base and prevents roughly 172 lines in #628 by replacing about 166 per-message services. Open drafts adopt it at their next base update. #658's cleanup saving is small; show the **+445 code-line .inc accounting re-baseline separately** from deletion savings (487 physical lines is not the code-line count).
   - #565 item 2 with #168's unfinished world-generation leaves follows #570, because both edit `movement.rs`: remove map fallbacks, remaining rail/crossing/depot/stop/water/tunnel leaves, `tile_slope_z` and per-tile world-generation services. Estimated compiled C++ removed: 300 to 600. Delete duplicated `bridge_above` and `rail_tile_type` too. This completes the phase-2 boundary; it creates no phase-1 owner.
   - #500 typing adoption and #168/#199 residual boundary cleanup attach to the owner touching those records; completed-job publication stays with link graph/jobs.
   - #501 proceeds in bounded evidence-tool maintenance batches. No standalone annotation sweep displaces ownership work.
5. **Ranked phase-2 queue after #570 integrates.** Before briefing #634 or #635, root decides whether later records sit beside or inside shells using the merged #570 profile and #566 drawing workload, after the owed quiet speed pass. Use the dependent-load table/profile checklist in the twenty-eighth #636 steering comment; record a material placement decision in the [world-state design](design/world-state.md).

   | Rank | Owner | Estimated compiled C++ removed | Selection event / coupled scope |
   | ---: | --- | ---: | --- |
   | 1 | #634 station/road-stop/cargo attachments | 800 to 1,400 | #570 and placement decision; include CargoPacket/CargoPayment pool management, station vectors/names/caches, stored references, 110 to 160 field slots, `SharedForEachStation` and spatial callbacks. Coordinate flow publication with link graph. |
   | 2 | Combined #656 + #598 + #599 vehicle owner | 750 to 1,250 | #570 integrated and remainder remeasured, including checkpoint 3/`SetDestTile`; residual records, names/sprite caches, maintenance, routing, lifecycle and pool allocation in one ownership PR. No duplicate #570 credit. |
   | 3 | #635 town/industry/subsidy | 500 to 1,000 | #570 and placement decision; 40 to 70 slots, mirrors/references, town coordinates, `psa_list` and remaining Rust-used caches. Disjoint from #634. |
   | 4 | Link graph/jobs | 450 to 800 | #570 and integrated #628 schedule changes; graph/job storage, station flow publication and #168/#199/#500 completed-job publication. |
   | 5 | #637 engine/renewal with #497 | 400 to 750 | #570; adopt integrated #655 before deleting engine publication/name leaves; include GRF/override and vehicle-specific spec payloads. |
   | 6 | Orders/order lists/backups | 350 to 650 | Combined vehicle owner's BaseConsist changes integrated; current fields, native pool management and field-service round trips into Rust order owners. |
   | 7 | Companies/groups | 300 to 600 | Vehicle unit-number ownership settled; adopt integrated network/company changes; include settings, finance/statistics aliases, names/identity and remaining Rust-used state. |
   | 8 | #638 small pools with #498 | 300 to 550 | #570; explicitly objects, depots, signs, goals, story pages/elements, league tables/elements, persistent storage and ObjectSpec payloads. |
   | 9 | Map storage and remaining map bodies | 2,000 to 3,200 | All simulation pools complete and #565 item 2 integrated; allocation/reset and remaining rail/road/water/tunnel/landscape/tile bodies. Pools-before-map ranks this last despite size. |

   Pools without an issue get one and a design section when selected. Every brief includes the accepted allocation/lifetime and exact shell dispositions above. No new phase-1 owners. Coupled save adapters, generated native declarations/layout checks and displaced field routes belong to each record owner.
6. **Phase 3 fills slots phase 2 leaves free**, after finishing the open owners and preserving #507's first-free-slot placement:
   - **READY #666 infrastructure script APIs**, 650 to 950 compiled C++ removed from 1,649 .cpp source lines. Rail/road/tile/map/bridge/tunnel/marine/infrastructure and associated lists use integrated map/command owners; keep VM/typed-command shims. Compare API returns/errors, command results and enumeration order.
   - **READY #667 content/download protocol**, 500 to 700 from 923 source lines. Own metadata, dependency and download state; native HTTP/file/decompression operations remain. Compare packets, dependency order, cancellation, completion and defined malformed-input behavior locally.
   - **READY #668 coordinator/STUN/TURN**, 400 to 600 from 912 source lines. Own token/negotiation state using integrated packet handling and native socket/connect leaves. Compare packets, fallback order, controlled-time timeouts and connection disposition.
   - **#669 reader-remainder tracker** holds the named queued owners, ranges and selection events: chunk schemas/handlers; afterload/legacy; network core/discovery and remaining protocol payloads; admin; NewGRF configuration/common publication/actions; grouped world/economy, vehicle/order and story/text/list/event APIs; script discovery/configuration and AI/Game orchestration; remaining feature definitions. Activate its highest-ranked ready owner by expected net reduction, without double phase-2 credit.
   - **#573 continuation**, 600 to 1,000 provisional, after #570: preserve save-descriptor macros so slice 3b can run beside #634/#635, otherwise the second waits. Finish slice 4, corrupt-input/LoadCheck probes and native `CopyFromOldName`; remeasure excluding completed slices.
   - **#653 industry/industry-tile definitions**, 270 to 400, after #647/#649; vector fields are decided at the first checkpoint. HouseSpec follows #628 under #669; station/road-stop/airport definitions are grouped there. #622 is completed.

   These owners use only integrated interfaces; a needed unintegrated service blocks its owner. No #628 draft services or parallel replacements. Keep VM/binding surfaces, and size owners to repay review/Full ([selection](planning.md#selection)).
7. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect merely to fill a slot. Deep execution remains user-only.

Recurring:
- After every second integration (next after 66), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip with the pinned `LD_LIBRARY_PATH` from [speed](speed.md#instruction-projections).
- Run the platform cadence after every fifth integration. The cadence after 65 is run 41466312349; the next is after 70. Fix a red cadence before further integrations.
- A quiet speed pass is owed in the next quiet window ([speed passes](speed.md#speed-passes)), because #642 changed `rust/openttd-kernels/Cargo.toml` and `simulate --instructions` now rejects the anchors' identity. It pauses builds and games for timed collection, so run it when no agent is building, and at the latest before the post-#570 record-placement decision.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-03; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-03, accepted placement after integration 67)

Code remains `d93afabee9` (integration 67); this docs-only update changes no
validation result. `CI_ON_DEMAND` is `true`. #664/#665 and the merged cadence
rerun gate all other integrations. Finish reviewed branches first; #507 takes
the first free branch slot. The accepted plan is placed in #666/#667/#668 and
#669 above. Stocktake 66 and the quiet speed pass stay owed; root decides
post-#570 record placement before #634/#635 briefs. Root owns NewGRF
function-group reviews and cadence platform validation in its own checkout.

Heads below were read from the active PRs; linked task status/review comments
hold evidence. A newer pushed head does not inherit an unreported validation.

| Issue / PR | Head | Next action / reported status | Validation still pending |
| --- | --- | --- | --- |
| #664 / #665 cadence repair | `a460b649d3` | Local configure/build passed (root report); [status](https://github.com/fetch-laboratories/openttd-rust/pull/665#issuecomment-6595942684) and [scoped review](https://github.com/fetch-laboratories/openttd-rust/pull/665#issuecomment-6595960751). Review is clear. Root runs platform and Full, integrates, then reruns cadence. | Platform and exact-head Full; merged cadence rerun. |
| #644, #646 / #647 boundary bool/expand | `166c852026` | [Scoped review](https://github.com/fetch-laboratories/openttd-rust/pull/647#issuecomment-6595903875) fixed the transparent-wrapper gate gap and found six genuine pre-existing orphan exports. Root is resolving those findings; hypothetical gate limits remain documented. | Finding resolution, affected checks and exact-head Full after cadence repair. |
| #604, #199 / #649 settings access | `ba07c85bb1` | [Scoped fixture review](https://github.com/fetch-laboratories/openttd-rust/pull/649#issuecomment-6595841556) is clear. Reported affected family evidence and native tests pass; update from base for final Full. | Current-base exact-head Full after cadence repair. |
| #570 / #636 vehicle records | `ff194457a4` | Active `/root/vehicle-570-checkpoint2`; [task status](https://github.com/fetch-laboratories/openttd-rust/pull/636#issuecomment-6575942666) reports record-carrying train handles, 29 train carriers removed, stable owner links and disaster naming repaired. Finish reservation/road/mixed-consumer routes and exact-head focused evidence/cost. | Cargo/unit/generator/gates/preflight reported passing; native build, CTests and train-edge comparison starting. No complete current-head simulation claim; independent review remains. |
| #574 / #628 network protocol | `555fe97b39` | Active `/root/network-574-cost-repair`; [task status](https://github.com/fetch-laboratories/openttd-rust/pull/628#issuecomment-6575922784) reports direct SendNet/accept-ID paths, shared handler template/flag access and three decision fixtures moved to Rust. Cargo/generated-ABI tests/preflight pass. | Native build at `7530b14058` failed include order; `88b4b0b36e` fixes it and native validation is rerunning. Default slice-5 observations, final cost and independent review remain. |
| #652 / #655 NewGRF text/Actions 3, 4, 13 | `37f3c58534` | [Task status](https://github.com/fetch-laboratories/openttd-rust/pull/655#issuecomment-6580176883) reports ready for root-arranged function-group review, focused loading evidence and compiled C++ -645. Native verification was at the final production-source head `66de651c78`; the later fixture-only repair has its own passing loading run. | Independent function-group review and exact-head Full; no Full requested by this placement task. |

PR worktrees (preserve): #647 `t3-c4634554`, `t3-17a0c009` and the cancelled reviewer's `t3-2efde48a`; #649 `t3-13298c8f`, `t3-6e7160e6` and `t3-b4561a0d`; #636 `t3-c67aada9` and `t3-41b02f9e`; #628 `t3-eb8e9c59`, `t3-1e4ffc20`, `t3-3208e14e`, `t3-5aa8a17f`, `t3-fc38354a` and `t3-e0d0c168`; #655 `t3-986b5854`. The merged and finished review checkouts are T3-locked; housekeeping keeps them until T3 releases them.

Keep unchanged: `t3-120ef117` (old #603), `t3-431c84b4` (cancelled #565 source with 58 dirty entries), `t3-d78d5f8b` (native fixture evidence), `.local/reference/openttd` at `14ec60f248` and the instruction-profile worktrees under the main checkout's `.local/worktrees/`.
