# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-05, steering 29; paused, new host)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: xhigh. Figures are stocktake 68's (Agent: /root | Model: gpt-6.1-sol | Reasoning effort: ultra); no code has landed since.

- Code is `e7d12e1508` (integration 68). Since stocktake 64, #651 (65, script host), #660 (66, carried validation), #662 (67, Emscripten cache) and #665 (68, native-library query repair) integrated. No rollback is recorded.
- **On 2026-11-05 the project moved to a new Ubuntu 24.04 host. It builds nothing until the user installs the apt packages** in [migration tools](rust-migration.md#build-and-verification). Old-host output needs glibc 2.43 and is moved off the build paths; builds, profiles and pinned runtimes are rebuilt here. Every speed anchor comes from the old host ([speed](speed.md#instruction-projections)).
- `python3 tools/port-metrics.py --whole-fork` at `e7d12e1508`, compared with stocktake 64 at `5e414c5a64` (doc history `dca42d6c02`):
  - Retired: 56,176 of 284,355 original code lines (19.76%, +354). Remaining original: 228,179 (-354).
  - Compiled original plus glue: 262,065 (92.16%, -134). The code checkpoints are 0.672 calendar days apart, including pauses: about **199 fewer compiled lines per day**. All 134 lines fell at integration 65; integrations 66 to 68 leave compiled C++ unchanged.
  - Glue: 33,886 (+220); 1,443 exports (+18) and 4,064 callback slots (+2).
  - Rust: 127,673 (+385); tooling: 53,827 (+580), about 1.64 tooling lines per original line retired. Since steering 65, tooling grew 216 and the other totals are unchanged. This tooling metric covers `tools/`, not CMake or workflow edits; #665 is CMake-only.
  - Simulation-source retirement: 40,407 of 61,148 (66.08%, unchanged).
- Retired code by phase-associated source bucket follows below. These accounting buckets do not reassign drawing, headers or data to owners; the **Phases** definitions and exit criteria remain in force.

| Phase / source buckets | Retired at 68 | Change since 64 |
| --- | ---: | ---: |
| 1. Simulation logic | 37,640 | 0 |
| 2. Simulation storage | 2,767 | 0 |
| 3. NewGRF, save/load, script and network | 8,039 | +354 (script) |
| 4. Support, graphics/viewport and data tables | 5,298 | 0 |
| 5. GUI and platform | 115 | 0 |
| 6. Vendored | 2,317 | 0 |
| **Whole fork** | **56,176** | **+354** |

- Baselines: 418 rebound-callee rows, 126 shared-accessor rows and 54 orphan exports, all unchanged from stocktake 64 and steering 65. Pending draft reductions are not integrated credit.
- The integrated compiled-C++ metric still excludes 445 code lines (487 physical lines) in three `.inc` files. **#649 now carries that inventory fix**, already reviewed on its branch; its merge must record the +445 accounting re-baseline separately from deletion savings. At pending head `c519c9da92`, the completed metric reports 262,451 compiled lines: 262,065 old main +445 baseline correction -59 actual deletion. None of that pending branch is integrated credit above. #658 retains its other boundary cleanup; do not implement or credit the metric fix again.
- Compiled C++ has stopped falling across the three infrastructure integrations. The accepted response remains finishing reviewed owners and deleting field/log services through #570/#657; no new phase-1 owner is selected. The last steering's open-draft cost table is historical; current heads, costs and actions live in their PR status comments and the checkpoint below.
- **No speed figure applies to this host.** Profile 68 exited 1 at `e7d12e1508` on anchor identity (six of seven scenarios; drawing alone equal), and the anchors, pinned runtime and three-pass record all come from the old host. The first quiet pass here re-anchors every scenario and restarts the record; it is owed before the post-#570 record-placement decision. The table carries **profile 64 at `5e414c5a64`**, flat against profile 62 (-0.11% to +0.07% Ir). Its scenarios were 0.12% to 0.90% below their anchors; drawing (#566) was 1.0027x reference Ir. These are historical measurements, not profile-68 results. The old anchor identity changed with #642's `rust/openttd-kernels/Cargo.toml` dependency change.

| Scenario | Candidate Ir, carried profile 64 | Carried projection (quiet pass owed) | Carried three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 2,731,301,085 | 1.111722x | 1.154106x |
| Grok | 26,898,981,923 | 1.387147x | 1.385653x |
| Opus | 28,297,683,609 | 1.393949x | 1.394556x |
| P1996 | 4,542,468,375 | 1.336551x | 1.337764x |
| P2000 | 7,400,813,786 | 1.400577x | 1.401446x |
| P2016 | 13,400,911,478 | 1.522911x | 1.481910x |

- The platform cadence passed at `e7d12e1508` on GitHub runners (evidence on #665), so the integration gate is clear. The next cadence is after integration 70.
- Coverage: #156 retains the command/network, authentication, script-host, NewGRF and other unreached domains; #647/#649/#655 review limits do not establish their closure. #503 still has 119 unfinished original traces and the clock-preparation/sanitized-support fixes, last in the work order. No coverage run or deep check was performed for this stocktake. Steering 29 added #649's two gaps to #156.
- Unintegrated components: **six**, #649, #647, #670 (#507), #655, #636 and #628. Review and measurement worktrees are not additional components.
- Issue placement: 44 open after steering 29 opened #671, #672 and #673; zero unplaced, one excluded policy issue in the exclusions below. The first-accepted-improvement trigger remains unmet; the six behavior improvements and #650 retain their return conditions below. No residual issue has fully completed scope, so none was closed. Corrected stale wording in #3, #168, #199, #275 and #501; checked completed milestones without closing their remaining scopes.

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
1. Finish the open branches; #507 now occupies the slot freed by #665. No
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

## Twenty-ninth steering review (2026-11-05: new host and the test harness)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: xhigh.

**What was audited.** Four independent read-only audits by Claude Opus at high effort, with no builds or games, because this host had no compiler:
- the Python harness's duplication;
- its type annotations;
- its structure and growth;
- #649's fidelity.

The steering checked every claim acted on below against the source.

**Root's track.**
- **Root.** One Codex session ran on 11-03 from 04:52Z to 06:25Z, then wound down at the user's request. It integrated #665 (68) and the cadence passed.
- **User-directed sessions.** #660 (66) and #662 (67) integrated from user-directed Claude sessions. All three integrations were infrastructure, so retired original and compiled C++ are unchanged since integration 65.
- **The move.** On 11-05 the user moved the project to a new host and removed 36 finished worktrees by hand.

**Findings and corrections.**

1. **The new host cannot build.**
   - It runs Ubuntu 24.04 with no C or C++ compiler, linker, `make`, `ninja` or `valgrind`, and agents cannot run `sudo`.
   - Output built on the old Ubuntu 26.04 host needs glibc 2.43, and the host has 2.39.
   - The driver puts `.local/deps`, which held 26.04 packages, ahead of system paths.

   Steering moved that output (17 GB) to the HDD, and pruned 36 records of worktrees whose folders were gone. Correction: the user installs the apt packages ([migration tools](rust-migration.md#build-and-verification)). The first quiet pass on this host re-anchors every scenario and restarts the three-pass record ([speed](speed.md#instruction-projections)).
2. **The harness is mostly per-component scenario code, not duplication** (the user's question, 2026-11-05).
   - **Size.** `tools/` holds 53,827 code lines:
     - 38,418 physical lines of Python in 103 files: 31 scenario modules hold 19,142 lines, tests of tools 7,438, comparison drivers 2,323, and infrastructure the rest;
     - about 19,700 lines of C++: 13,452 in console adapters relinked into both games, and 5,943 in comparison and ABI fixtures;
     - 1,299 lines of Squirrel.

     Over the whole fork that is 0.96 tooling lines per original line retired. The rate was 3.8 from 10-27 to 10-31, driven by infrastructure, and has been 0.31 since.
   - **Duplication.** 0.71% exact, by jscpd once its 1,000-line file cap is lifted; about 2 to 3% counting renamed copies and the same job written several ways. About 400 to 450 lines are removable mechanically, and 400 to 600 more with judgment. No dead code was confirmed.
   - **Annotations.**
     - 41.7% of functions and 33% of function code are fully annotated; 59 files are ANN-ignored and 38 are in `ty`.
     - New files follow the rule, but about 30% of new functions in ignored files are unannotated.
     - Scenarios, runs and results flow as untyped dicts through an unannotated `core.py`, so `ty` proves less than its pass suggests.
     - At the current pace #501 finishes around March 2027. Its first planned step, `core.py` and `migration.py`, never happened.
   - **Structure.** The code is split by component, with one private reach-in. But it has a 37-module import cycle through `core` and the driver, five second copies of `migration.py`, hooks dispatched to every module through `getattr`, and functions of 268 and 437 lines.
   - **Verdict: no broad refactor.** The size tracks ports, and the six drafts touch 72 `tools/` files. Corrections:
     1. Type checking goes first (user, 2026-11-05): #673's typing PR, with #501's free cleanup, then #501's mechanical batches until the ignore list is empty ([work order](#work-order) item 4).
     2. #672 guards the oracle.
     3. #673's second PR consolidates helpers.
     4. New scenario code uses the shared helpers ([evidence](porting.md#evidence)).
3. **The oracle can weaken silently.** Four gaps:
   - a removed scenario passes Full, because `validate_plan` checks the candidate's own list;
   - a deleted or empty comparison passes;
   - masks can grow;
   - a misspelled scenario key skips a check.

   Correction: #672, batched with #671.
4. **Housekeeping kept 36 finished worktrees.** With the rest, they held 149 GB.
   - Locked trees whose PRs merged are always kept, and so are pushed review checkouts.
   - Wind-down preserved every review checkout.
   - On the 220 GB SSD, with each worktree's `build-rust` on it, this fills the disk.

   Correction: #671. Wind-down now preserves only each open PR's checkout and trees with uncommitted or unpushed work ([wind-down](workflow.md#automatic-rollback-and-wind-down)).
5. **#649 has no reachable divergence.** It integrates as reviewed. Its surviving settings-catalog copies, carriers and small leftovers join #658, and its two coverage gaps are on #156 ([steering comment](https://github.com/fetch-laboratories/openttd-rust/pull/649#issuecomment-6651456306)).
6. **#655 was held behind unstarted #657.** #655 is reviewed, while #657 waits for #647 and #649. Correction: #655 integrates in readiness order. #657 converts the per-message services on base when it is built, #655's included. If #657 lands first, #655 adopts it at its base update.

No user direction is reversed. On 11-05 the user asked to change the project as little as possible while moving it. That bound only that session, and the move is done.

| Twenty-eighth correction | Verdict and cost/result |
| --- | --- |
| 1. Children check their job; root checks idle children at each wake | Applied. No stall was recorded in the 1.5-hour 11-03 session. |
| 2. #628 makes its bounded repair before review | Applied. Compiled C++ is -92 before #657 and about -264 after, against -240 to -360 expected. Its caller-inventory review is unfinished. |
| 3. #657 ports `Debug` and `GrfMsg` once | Slipped. The design is done but nothing is built, behind #647, #649 and the pause. Finding 6 removes #655's wait on it. |
| 4. #658 counts `.inc` glue | Applied inside #649. The +445 re-baseline was verified exactly, with no double count. |
| 5. The platform profile runs before Full for Cargo, toolchain or cmake changes | Applied to #665. The cadence was green at 68. |
| 6. #636 finishes checkpoint 2 near -900 to -1,250 | Unfinished. The worker was cancelled at wind-down, job `23352`'s result is unread, and the head conflicts with base. |
| 7. #655 removes its two bounces | Applied. The ACTIONS and TEXT reviews are clear, and `invalid_engine` is gone at `68bcc7d8d2`. |
| 8. #647's resume round is scoped | Applied. Reviews are clear at `5d809ff395`; Full has been pending since 11-03. |

| Change since the twenty-eighth review | Verdict and next observation |
| --- | --- |
| #660 carried Full validation and #662 Emscripten cache (user-directed, 11-02, answering "CI and merging is a huge bottleneck") | Untested: no integration candidate has run Full since. Watch Fulls per integration and Full wall time. |
| Codex subagent limit raised to eight (user-directed) | Untested. |
| `.local` output on the HDD behind symlinks (user-directed, 11-05) | Applied. Worktree links resolve and the job logs survived. Watch tool code that resolves `.local` paths. |

New since this review, for the next steer to judge:
- Whether the first builds on this host and the first Full from it agree, and what the new anchors measure.
- Whether typing takes the first freed slots: #673's typing PR, then #501's batches, and how fast the ignore list empties.
- Whether #671 and #672 follow, and whether a guard fires on a draft.
- Whether #673 keeps the scenario inventory and digests identical, and how many `ty` diagnostics remain over `tools/`.
- Whether housekeeping after #671 keeps T3 worktrees to open PRs plus unpushed work.

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
placement from the post-#570 profile. Per-message `Debug` and `GrfMsg` services are design faults
once #657 integrates. A change to Cargo, the toolchain or cmake runs the
platform profile once before Full. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence.

## Recent completed work

Rows cover the period since the twenty-seventh steering review; earlier rows are
in their PRs and Git history. Metrics are `tools/port-metrics.py` code lines:
Rust / tooling / original C++ retired / net glue.

| Integration | Issues / PR | Merge commit | Metrics |
| --- | --- | --- | --- |
| 61 | Platform cadence: landscape fixture `shared` initializer / #648 | `cdfc2de482` | 0 / 0 / 0 / 0 |
| 62 | #641 packet state and byte storage in a generated Rust record / #645 | `bf2c4632a2` | 718 / 175 / 52 / +80 |
| 63 | #559 command invocation lifecycle (phase 1's last owner) / #640 | `7940e700c2` | 403 / 287 / 136 / +115; rebound rows 420 to 418, orphan exports 55 to 54 |
| 64 | #64 maintained crypto crates and Rust network authentication / #642 | `5e414c5a64` | 2,027 / 480 / 2,242 / -68 |
| 65 | #575 script host, command context and event lifecycle / #651 | `6afdcbf390` | 386 / 364 / 354 / +220 |
| 66 | #659 carried Full validation across pushes that keep the tested non-Markdown tree / #660 (CI only; the user allowed it ahead of the cadence repair) | `f47064fc57` | 0 / tooling only / 0 / 0 |
| 67 | #661 Emscripten compiler cache / #662 (CI only; the user allowed it ahead of the cadence repair) | `d93afabee9` | 0 / tooling only / 0 / 0 |
| 68 | #664 native-library query from the actual Rust crate / #665 | `e7d12e1508` | 0 / 0 / 0 / 0 (CMake-only; outside the tooling metric) |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

1. **Rebuild on the new host** once the user's apt packages are installed ([migration tools](rust-migration.md#build-and-verification)).
   - Build and verify the main checkout in the background with `python3 tools/migration.py verify`. Then run two short default scenarios to confirm both games play.
   - Rebuild an open PR's worktree only when its work resumes, after pulling it to the PR head. `t3-986b5854` (#655) and `t3-1e4ffc20` (#628) are behind their heads.
   - Run housekeeping. `t3-41b02f9e` (#636), `t3-13298c8f` (#649) and `t3-c4634554` (#647) are older second checkouts whose commits are all on origin.
   - Full runs on GitHub, so the reviewed heads below need no local build to integrate.
2. **Finish the open branches in readiness order:**
   - #649 (#604 with #199's disjoint items): the scoped fixture review is clear, and steering 29 found no reachable divergence at `c519c9da92`. Run exact-head Full and merge at the reviewed head. Its follow-ups are on #658.
   - #647 (#644 with #646): head `5d809ff395`; the six orphan findings are resolved, full local verify and eight scenarios/56 snapshots pass, and fresh cleanup review is clear. Root verified all six old exports absent from the archive and their six replacements support-only. Final base update and Full follow the cadence; hypothetical gate limits stay documented.
   - #570/#636: pushed head `ae72c8c200`; continue checkpoint 2's remaining moved-field routes and exact-head focused evidence, then review and integrate. The latest status reports record-carrying train handles, 29 train carriers removed and disaster naming repaired at `ff194457a4`; it does not validate the newer head. Expected checkpoint-2 compiled C++ reduction is 900 to 1,250; final cost remains pending.
   - #574/#628: implementation reports complete at `b32d5b00a1`, including the bounded cost repair and default slice-5 observations in both server roles. Reported compiled C++ is -92 before #657 (about -264 after adoption); builds, 239 CTests, packet/auth comparisons and default loopback pass. Root arranges a fresh independent review. Named bounce endpoints remain; adopt shared #657 when integrated, with no private helper.
   - #652/#655: head `68bcc7d8d2`; ACTIONS and TEXT reviews are complete with no unresolved finding. Root spot-checked the native assertion fix. Update from base, run Full and integrate; it does not wait for #657. If #657 has integrated by then, adopt it at that base update. The pre-review reported compiled C++ reduction is 645; final metrics belong to the integrating head. Formatting-time GRF strings (about 170 lines) remain phase 4, engine GRF/override payload #637, ObjectSpec #638. Full remains pending.
3. **#507 NewGRF corpus** (user reselected, 2026-11-02, closing #654; #526 closed as unneeded), now starting in the slot freed by #665. Active `/root/corpus-507-newgrf-downloads` opens a new draft from `e7d12e1508`. Root verified all three pinned download archives and old GRF payload hashes; the worker implements the ignored CDN fetch. It is maintenance with one reviewer and one Full.
   - Open a new PR from `rust-migration`. Reuse #525's scenario module (`tools/simulation/newgrf_sets.py`) and its scenario AI from `testing/corpus507-newgrf` at `ccc28e2ed0`, but not its committed `.grf` files or source archive.
   - Pin the three OpenGFX+ sets (Trains 0.3.0, Road Vehicles 0.4.1, Industries 0.3.5) by name, version and SHA-256, and fetch them from the BaNaNaS CDN into ignored local storage ([evidence](porting.md#evidence)).
   - Extending `tools.simulation.community_ai` and its cached manifest avoids a workflow change. Keep the sets out of every uploaded artifact.
   - The default slice is `newgrf-sets-arctic` (about 7 s of game time); `newgrf-sets-arctic-long` runs in soak. The same-NewGRF larger save follows as a later #507 PR.
   - Integrate it before #655 if it is ready first; #655 does not wait for it.
4. **Type checking first (user, 2026-11-05).** Typing takes each slot an integration frees, ahead of #657 + #658, the harness batch and new owners:
   - **#673's typing PR**, now. It types shared `Scenario`, `Run` and `Result` records in `tools/simulation/core.py`, types hook dispatch, adds `core.py` and `migration.py` to `ty.toml`, and carries #501's free cleanup. It does not wait for #670; whichever of the two integrates second resolves the `core.py` registry conflict. Helper consolidation follows as #673's second PR.
   - **#501's mechanical batches** start as soon as #673's typing PR integrates, two at a time on disjoint files, each adding its files to `ty.toml`. They annotate `scenario`, `result` and `run` with #673's records. A file an open draft touches converts as soon as that draft integrates.
   - Done when the ANN ignore list is empty and `ty.toml` covers all of `tools/`. Each typing PR has one reviewer and one Full.
5. **Maintenance batch: #657 + #658**, built by root from `rust-migration` after #647 and #649 integrate, with one reviewer and one Full. #658 now also deletes the settings-catalog copies and carriers #649 left ([steering comment](https://github.com/fetch-laboratories/openttd-rust/pull/649#issuecomment-6651456306)). #657's read-only design is complete; implementation has not started. #657 removes about 300 to 450 compiled lines on base and prevents roughly 172 lines in #628 by replacing about 166 per-message services. Open drafts adopt it at their next base update. #658's cleanup saving is small; show the **+445 code-line .inc accounting re-baseline separately** from deletion savings (487 physical lines is not the code-line count).
   - #565 item 2 with #168's unfinished world-generation leaves follows #570, because both edit `movement.rs`: remove map fallbacks, remaining rail/crossing/depot/stop/water/tunnel leaves, `tile_slope_z` and per-tile world-generation services. Estimated compiled C++ removed: 300 to 600. Delete duplicated `bridge_above` and `rail_tile_type` too. This completes the phase-2 boundary; it creates no phase-1 owner.
   - #500 typing adoption and #168/#199 residual boundary cleanup attach to the owner touching those records; completed-job publication stays with link graph/jobs.
   - **Harness batch: #672 + #671**, delegated from `rust-migration` beside #657 + #658 (disjoint files), with one reviewer and one Full. It guards the scenario and comparison inventories and masks and fixes housekeeping.
   - **#673's consolidation PR** follows its typing PR. Inventory, scenario digests and comparison results must stay identical. It is the only harness refactor; no broader rewrite.
6. **Ranked phase-2 queue after #570 integrates.** Before briefing #634 or #635, root decides whether later records sit beside or inside shells using the merged #570 profile and #566 drawing workload, after the owed quiet speed pass. Use the dependent-load table/profile checklist in the twenty-eighth #636 steering comment; record a material placement decision in the [world-state design](design/world-state.md).

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
7. **Phase 3 fills slots phase 2 leaves free**, after finishing the open owners and preserving #507's first-free-slot placement:
   - **READY #666 infrastructure script APIs**, 650 to 950 compiled C++ removed from 1,649 .cpp source lines. Rail/road/tile/map/bridge/tunnel/marine/infrastructure and associated lists use integrated map/command owners; keep VM/typed-command shims. Compare API returns/errors, command results and enumeration order.
   - **READY #667 content/download protocol**, 500 to 700 from 923 source lines. Own metadata, dependency and download state; native HTTP/file/decompression operations remain. Compare packets, dependency order, cancellation, completion and defined malformed-input behavior locally.
   - **READY #668 coordinator/STUN/TURN**, 400 to 600 from 912 source lines. Own token/negotiation state using integrated packet handling and native socket/connect leaves. Compare packets, fallback order, controlled-time timeouts and connection disposition.
   - **#669 reader-remainder tracker** holds the named queued owners, ranges and selection events: chunk schemas/handlers; afterload/legacy; network core/discovery and remaining protocol payloads; admin; NewGRF configuration/common publication/actions; grouped world/economy, vehicle/order and story/text/list/event APIs; script discovery/configuration and AI/Game orchestration; remaining feature definitions. Activate its highest-ranked ready owner by expected net reduction, without double phase-2 credit.
   - **#573 continuation**, 600 to 1,000 provisional, after #570: preserve save-descriptor macros so slice 3b can run beside #634/#635, otherwise the second waits. Finish slice 4, corrupt-input/LoadCheck probes and native `CopyFromOldName`; remeasure excluding completed slices.
   - **#653 industry/industry-tile definitions**, 270 to 400, after #647/#649; vector fields are decided at the first checkpoint. HouseSpec follows #628 under #669; station/road-stop/airport definitions are grouped there. #622 is completed.

   These owners use only integrated interfaces; a needed unintegrated service blocks its owner. No #628 draft services or parallel replacements. Keep VM/binding surfaces, and size owners to repay review/Full ([selection](planning.md#selection)).
8. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect merely to fill a slot. Deep execution remains user-only.

Recurring:
- After every second integration (next after 70), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip once this host has anchors ([speed](speed.md#instruction-projections)).
- Run the platform cadence after every fifth integration; the next is after 70.
- A quiet speed pass is owed on this host ([speed passes](speed.md#speed-passes)). It re-anchors every scenario, pins this host's runtime and restarts the three-pass record. It pauses builds and games only for timed collection, so run it when no agent is building, and at the latest before the post-#570 record-placement decision.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-03; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-05, paused; new host)

Paused at the user's request since 11-03 06:25Z. Code is `e7d12e1508` (integration 68), and `CI_ON_DEMAND` is `true`. The cadence passed at that head, so the integration gate is clear; the next cadence is after integration 70. Nothing has merged since #665.

On 11-05 the project moved to a new host; work order item 1 rebuilds it. The open-PR worktrees hold source only, at the heads below or older. Builds, profiles and pinned runtimes must be rebuilt. The old-host anchors do not apply here. After #570 integrates, run this host's quiet pass and the merged-tip profile before deciding record placement for #634 and #635.

| Issue / PR | Head | Next action on resume | Checkout |
| --- | --- | --- | --- |
| #604, #199 / #649 settings access | `c519c9da92` | Request Full at this head; merge only if it is green and still current. Steering 29 found no reachable divergence. Follow-ups are on #658. The metrics are on #649: +445 `.inc` re-baseline and 59 compiled lines actually removed. | `.local/worktrees/update649-after68` |
| #644, #646 / #647 boundary bool/expand | `5d809ff395` | Reviews clear. Full local verify, eight scenarios and 56 snapshots pass. Update from base, then Full and merge. | `.local/worktrees/fix647-orphans` |
| #507 / #670 NewGRF corpus | `230cbc075e` | [Independent review](https://github.com/fetch-laboratories/openttd-rust/pull/670#issuecomment-6596875528) clear; short and long cases pass with 26 and 95 equal snapshots. Update from base, then Full. The same-NewGRF larger save remains later #507 work. | `t3-937fa25f` |
| #652 / #655 NewGRF text, Actions 3, 4 and 13 | `68bcc7d8d2` | ACTIONS and TEXT reviews clear; the native assertion fix and affected checks pass. Update from base, Full and integrate. It does not wait for #657. | `t3-986b5854` (at older `54c44c68c5`; pull first) |
| #570 / #636 vehicle records | `ae72c8c200` | The PR conflicts with base. Read job `23352`'s logs in the worktree's `.local`, merge base, then finish the cargo, fleet and rail link routes, final scenarios, cost metrics and PR text before review. Do not claim checkpoint 2 complete. | `t3-d038aacc` (job `23352`'s result is unknown) |
| #574 / #628 network protocol | `0e650fc593` | The PR conflicts with base. The COMMAND, FRAME, SESSION and evidence [review](https://github.com/fetch-laboratories/openttd-rust/pull/628#issuecomment-6596854706) published one test fix and no production divergence. The CONNECTION/TCP [report](https://github.com/fetch-laboratories/openttd-rust/pull/628#issuecomment-6597036887) is published at `b32d5b00a1`, but its caller inventory is unfinished. Merge base, finish that inventory and #647's canonical-byte return adoption, adopt integrated #657, then Full. | `t3-1e4ffc20` (at older `77c0b8295e`); review tree `t3-bbdd0b54` |

Preserve the following:
- `t3-3b491ddb`, with five dirty files;
- `t3-431c84b4`, with #565's 58 dirty files;
- `t3-d78d5f8b`, the native-fixture evidence;
- the locked `.local/worktrees/profile-after68-664`;
- the unchanged pinned original and retained profiles;
- the old-host output on the HDD, until this host's anchors are recorded.
