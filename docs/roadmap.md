# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-05, stocktake 83)

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh.

- Code is `c9de8d52ca` (integration 83). Since stocktake 80, three integrations landed:
  - #690 (81) batches #684 (#507's fifty-year same-NewGRF save, closing #507) with #688 and #689 (#501 typing batches 5 and 6).
  - #636 (82) is #570's shared vehicle records with direct Rust Tick; #570 closed.
  - #691 (83) is #666's tile, map, rail and road script APIs.

  No rollback is recorded. Integration 80's platform cadence passed (run 42087493081). macOS ran on `CI_MACOS_HEAVY` after two hosted-capacity cancellations, and the variable is deleted. The next cadence follows integration 85.
- `python3 tools/port-metrics.py --whole-fork` at `c9de8d52ca`, with changes against stocktake 80 at `a2e417114b`:
  - Retired: 59,125 of 284,355 original code lines (20.79%, **+1,706**).
  - Compiled original plus glue: **258,888** (91.04%, **-2,668**). #636 removed 1,952, almost twice its checkpoint-2 measure of -1,014, because the reviews deleted more routes. #691 removed 716, inside #666's 650 to 950 estimate for the whole issue.
  - Glue: 33,658 (-962). Exports 1,494 (+48: #636 -20, #691 +68). Callback slots **3,523** (-337).
  - Rust: 131,863 (+1,615). Tooling: 57,427 (+1,081).
  - Simulation-source retirement: 41,081 of 61,148 (67.18%).
- Retired code by phase-associated source bucket, phases 1 to 6: 38,213 / 2,868 / 10,064 / 5,326 / 337 / 2,317, or 59,125 in all. These accounting buckets do not reassign drawing, headers or data to owners; the **Phases** definitions and exit criteria remain in force.
- Baselines: 376 rebound-callee rows (-34), 120 shared-accessor rows (-16) and 43 orphan exports (-5), all from #636.
- **Speed.** The merged-tip profile at integration 82 (`2e6958801e`, report `.local/simulation/20261105T215010Z-1842682`) found all 7 scenarios equal. #636's direct Rust Tick cut play-save candidate Ir by 3.9% to 4.9%, so every play-save projection sits at its anchor, the formula's floor. TGP rose again.

| Scenario | Anchor ratio | Candidate Ir at `27de6f8c18` | Candidate Ir at `2e6958801e` | Projection | Change vs anchor |
| --- | ---: | ---: | ---: | ---: | ---: |
| TGP | 1.160107x | 2,773,615,856 | 2,819,307,093 | 1.198329x | +3.29% |
| Opus | 1.345115x | 28,611,669,801 | 27,380,883,235 | 1.345115x | 0.00% (Ir -4.3%) |
| Grok | 1.346652x | 27,178,405,243 | 26,113,430,594 | 1.346652x | 0.00% (Ir -3.9%) |
| P1996 | 1.352194x | 4,744,876,185 | 4,517,945,710 | 1.352193x | 0.00% (Ir -4.8%) |
| P2000 | 1.444531x | 7,834,343,043 | 7,451,081,511 | 1.444531x | 0.00% (Ir -4.9%) |
| P2016 | 1.382593x | 14,200,776,421 | 13,630,714,787 | 1.382593x | 0.00% (Ir -4.0%) |
| Drawing | 1.004635x | 2,154,533,064 | 2,154,342,152 | 1.004635x | 0.00% |

- TGP's Ir is +1.65% over its anchor: +1.33% by integration 80, then +0.32% with #636. The projection is 1.198x, below the 1.235x generation band. Since the previous tip (integration 80) it rose 0.63%, so the policy's 3%-since-previous-tip trigger is not met. The cumulative rise passed 3% against the anchor, though, so root is attributing it anyway: `/root/attribute-tgp-ir-rise` reports per function and per PR on #275 (#647, #655, #679 and #636 are the candidates).
- Coverage: unchanged. #156 adds #691's four unreached script behaviors. #503 still has 119 unfinished original traces. No coverage run or deep check was performed.
- Unintegrated components: **four**, under the six-branch limit:
  - #628 (network protocol), fixing a CI-found harness console and merging #636;
  - #692 (#653, industry and tile specs), being finished for review;
  - #696 (#501 typing batches 7 and 8), in Full;
  - #666's second PR, starting.
- `ty` over all of `tools/` without the include list reports 212 diagnostics on base. #696 brings that to 2, leaving `test-generated-abi.py`, the last #501 file (it waits for #628 and #692), and an unresolvable `yaml` import in `test-ci.py` in that configuration.
- Issue placement: 37 open. #686 (the user's merge-latency measurement) and #693 (the multiplayer late-sync flake) are placed below; zero unplaced remain. #513's first-accepted-improvement trigger is unmet.
- **Process lesson.** Two Full runs in a row failed on scenarios no review had run:
  - #636's `track-follower`: the follower probe passed vehicle shells where Rust expects records.
  - #628's commands console: it read the moved command queue.

  Reviews pick scenarios by name prefix, and harness consoles compile only when their scenario runs. From now on, root briefs for PRs that change the Rust/C++ boundary (moved records, deleted services, changed signatures) require a CI `native` profile run (`python3 tools/ci.py request PR --profile native`) before review and Full. That profile is the full migration comparison and simulation shards, about 10 to 20 minutes. Running 481 scenarios locally at `--jobs 3` takes more than 4 hours here, and #628's local attempt hit the 2-hour background limit at 214 scenarios.
- Host: unchanged. #678 (TSan startup aborts) and #693 remain tools maintenance.

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
     1. Type checking goes first (user, 2026-11-05): #673's typing PR, with #501's free cleanup, then #501's mechanical batches until the ignore list is empty ([work order](#work-order) item 2).
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
| 69 | #604 canonical settings access with #199's disjoint items / #649 | `e76c94b306` | 437 / 118 / 453 / +394; compiled C++ -59, reported apart from the +445 `.inc` re-baseline; callback slots 4,064 to 4,026 |
| 70 | #507 pinned NewGRF sets and arctic corpus scenarios / #670 (the same-NewGRF larger save remains) | `a114e1f8a7` | 0 / tooling only / 0 / 0 |
| 71 | #671 housekeeping of pushed and stale-locked worktrees, #672 harness inventory guards, the instruction-cache cross-disk fix / #675 (workflow change; bootstrap Full) | `64bbc057bf` | 0 / 436 / 0 / 0 |
| 72 | #644 `u8` boolean returns across the boundary, #646 `TileArea::expand` int wrapping / #647 | `3c30fb8129` | 870 / 242 / 0 / +173 at its reviewed head (+167 compiled on base): inline bool wrappers; orphan exports 54 to 48 |
| 73 | #673 typed `Scenario`, `Run` and `Result` records and owner-only hook dispatch / #674 | `02393552fd` | 0 / tooling only / 0 / 0 |
| 74 | #501 first typing batch: six simulation scenario modules / #677 | `566b87c35d` | 0 / tooling only / 0 / 0 |
| 75 | #652 GRF text and NewGRF Actions 3, 4 and 13 / #655 | `da7faa8d0b` | compiled C++ **-648** (262,618 to 261,970; 645 reported before review); original retired +761; exports 1,443 to 1,447, callback slots 4,026 to 4,044 |
| 76 | #501 typing batch 2: thirteen comparison tools / #680 | `9260dc5148` | 0 / tooling only / 0 / 0 |
| 77 | This host's quiet speed pass: stale-anchor acceptance, reference identities per role, one collection per bundle and the new anchors / #676 | `80d778897a` | 0 / tooling only / 0 / 0 |
| 78 | #501 typing batch 3: landscape, companies, industries and signals scenario modules / #681 | `5493d11679` | 0 / tooling only / 0 / 0 |
| 79 | #501 typing batch 4: CI and platform tooling / #682 | `54eeaee3a6` | 0 / tooling only / 0 / 0 |
| 80 | #657 Debug and GrfMsg in Rust once, #658 auth and script-host leftovers with `.inc` glue counted / #679; state-loop profile entry / #683; batched as #685 | `a2e417114b` | compiled C++ **-414** (261,970 to 261,556); callback slots 4,044 to 3,860; exports 1,447 to 1,446; rebound rows 418 to 410; shared-accessor rows +10 (`.inc` coverage re-baseline) |
| 81 | #507's fifty-year same-NewGRF save / #684; #501 typing batches 5 and 6 / #688, #689; batched as #690 with root's combined-typing fix | `7c067ad699` | 0 / tooling only / 0 / 0; #507 closed |
| 82 | #570 shared vehicle movement and consist records with direct Rust Tick / #636 | `2e6958801e` | compiled C++ **-1,952** (261,556 to 259,604); callback slots 3,860 to 3,524; exports 1,446 to 1,426; rebound rows 410 to 376, shared-accessor 136 to 120, orphan exports 48 to 43; original retired +760 |
| 83 | #666 tile, map, rail and road script APIs (first PR) / #691 | `c9de8d52ca` | compiled C++ **-716** (259,604 to 258,888); exports +68, callback slots -1; original retired +946 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

1. **Finish the open branches in readiness order.** Children check `.local/deep.lock.holders` before a native build, link or game, and pass `--jobs 3`. Rebuild an open PR's worktree only when its work resumes, after pulling it to the PR head. Briefs for PRs that change the Rust/C++ boundary require a CI `native` profile run before review and Full (stocktake 83's process lesson), not a local full set.
   - **#574 / #628.** Scoped review accepted, then Full failed: the commands console read the moved command queue.
     - `/root/fix-628-commands-console` reads the queue through a test-support-only entry and merges #636.
     - It then runs the full default simulation set.
     - A fresh scoped review covers the console fix and the #636 merge, then Full.
     - #667 and #668 wait for #628.
   - **#653 / #692.** Checkpoint 1 accepted at -211, below the -270 to -400 estimate, because the typed C++ readers keep their fields. The remainder projects to about -250 to -270. `/root/industry-653-specs` finishes the PR body and marks it ready; then one review and Full.
   - **#666, second PR**: bridge, tunnel, marine, infrastructure and the lists, projected -260 to -330. `/root/script-666-infrastructure` starts it from integration 83 and stops at one checkpoint.
2. **Type checking first (user, 2026-11-05).** Typing takes each slot an integration frees, ahead of the harness batch and new owners.
   - Batches 1 to 6 integrated (#677, #680, #681, #682, #688, #689). Batches 7 and 8 are reviewed and batched as #696.
   - Only `tools/test-generated-abi.py` then remains. It converts after #628 and #692 integrate, because both edit it.
   - Done when the ANN ignore list is empty and `ty.toml` covers all of `tools/`.
   - **#673's consolidation PR** follows typing. Inventory, scenario digests and comparison results must stay identical. It is the only harness refactor; no broader rewrite.
3. **Maintenance and measurement.**
   - **#686 (the user's request):** measure where time goes between a ready PR and its merge, including Windows x64's share of Full. `/root/measure-686-merge-latency` reports per-PR timelines, buckets and a ranked recommendation. Policy changes stay with root and the user. A table-rebuilding script under `tools/` goes to a coding agent only if the report recommends one.
   - **TGP Ir attribution:** `/root/attribute-tgp-ir-rise` reports on #275. A repair batch follows if candidates credit at least 1% of TGP loop Ir.
   - **#565 item 2** with #168's unfinished world-generation leaves, now that #570 has integrated:
     - remove map fallbacks;
     - remove the remaining rail, crossing, depot, stop, water and tunnel leaves;
     - remove `tile_slope_z` and the per-tile world-generation services;
     - delete the duplicated `bridge_above` and `rail_tile_type`;
     - retire the map-leaf callbacks in #691's overlap table (recorded on #565) by reusing #691's helpers.

     Estimated compiled C++ removed: 300 to 600. It completes the phase-2 boundary and creates no phase-1 owner.
   - #500 typing adoption and the #168/#199 residual boundary cleanup attach to the owner touching those records. Completed-job publication stays with link graph and jobs.
   - **#678** (TSan startup aborts counted as clean runs) and **#693** (the mixed multiplayer diagnostic's late sync under host load) fold into the next tools maintenance PR, with that PR's reviewer and Full. Neither may change the original's frame and sync behavior or weaken an evidence check.
4. **Ranked phase-2 queue (#570 integrated as 82).** Placement is decided: later records sit **beside** their shells ([world-state design](design/world-state.md#later-pool-record-placement-634-635-decided-2026-11-05-after-integration-82)). Briefs state its two conditions: no out-of-line record↔shell call, and no stored references. Cargo-packet co-allocation is decided at #634's pool-completion checkpoint.

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
5. **Phase 3 fills slots phase 2 leaves free**, after finishing the open owners:
   - **#666 infrastructure script APIs**: the first PR (#691, tile, map, rail and road) integrated as 83 at -716. The second PR (bridge, tunnel, marine, infrastructure and the lists) is under way (work order item 1). The issue's original scope was 650 to 950 from 1,649 .cpp source lines. Rail/road/tile/map/bridge/tunnel/marine/infrastructure and associated lists use integrated map/command owners; keep VM/typed-command shims. Compare API returns/errors, command results and enumeration order.
   - **READY #667 content/download protocol**, 500 to 700 from 923 source lines. Own metadata, dependency and download state; native HTTP/file/decompression operations remain. Compare packets, dependency order, cancellation, completion and defined malformed-input behavior locally.
   - **READY #668 coordinator/STUN/TURN**, 400 to 600 from 912 source lines. Own token/negotiation state using integrated packet handling and native socket/connect leaves. Compare packets, fallback order, controlled-time timeouts and connection disposition.
   - **#669 reader-remainder tracker** holds the named queued owners, ranges and selection events: chunk schemas/handlers; afterload/legacy; network core/discovery and remaining protocol payloads; admin; NewGRF configuration/common publication/actions; grouped world/economy, vehicle/order and story/text/list/event APIs; script discovery/configuration and AI/Game orchestration; remaining feature definitions. Activate its highest-ranked ready owner by expected net reduction, without double phase-2 credit.
   - **#573 continuation**, 600 to 1,000 provisional, after #570: preserve save-descriptor macros so slice 3b can run beside #634/#635, otherwise the second waits. Finish slice 4, corrupt-input/LoadCheck probes and native `CopyFromOldName`; remeasure excluding completed slices.
   - **#653 industry/industry-tile definitions** (PR #692, work order item 1): records with Rust `Vec` containers behind a `RustVector<T>` view. HouseSpec follows #628 under #669; station/road-stop/airport definitions are grouped there. #622 is completed.

   These owners use only integrated interfaces; a needed unintegrated service blocks its owner. No #628 draft services or parallel replacements. Keep VM/binding surfaces, and size owners to repay review/Full ([selection](planning.md#selection)).
6. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect merely to fill a slot. Deep execution remains user-only.

Recurring:
- After every second integration (next after 85), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip in the background ([speed](speed.md#instruction-projections)).
- Run the platform cadence after every fifth integration; the next is after 85. A failure stops integrations. After two hosted-capacity macOS cancellations, use `CI_MACOS_HEAVY` as documented, and delete it afterwards.
- This host's first quiet pass integrated with #676 (77). Further passes follow the [speed policy](speed.md#policy) triggers.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-05; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-05, stocktake 83)

Root runs in Claude Code (`claude-opus-5-5`, xhigh). Code is `c9de8d52ca` (integration 83, #691), and `CI_ON_DEMAND` is `true`. `CI_MACOS_HEAVY` is unset. The next platform cadence and stocktake follow integration 85.

The main checkout is on `rust-migration` and clean, with the integration-82 profile recorded. Root's docs commits use `.local/worktrees/root-docs-72`. Phase 2's next owners (#634, #635) wait for root's beside-or-inside placement decision, made from the integration-82 profile and the #566 drawing workload (work order item 4).

| Issue / PR | Head | Next action on resume | Checkout |
| --- | --- | --- | --- |
| #574 / #628 network protocol | `f8e393b9cb` | `/root/fix-628-commands-console`: full default simulation set on the #636-merged tree. Then a fresh scoped review of the console fix and merge, then Full. | its T3 worktree |
| #653 / #692 industry specs | `0b4a612fda` | `/root/industry-653-specs` finishes the PR body and marks it ready. Then one review and Full. | its T3 worktree |
| #501 / #696 typing batches 7 and 8 | `814778abdd` | Full is running. Merge it when green. | `.local/worktrees/integrate-694-695` |
| #666 second PR | not yet opened | `/root/script-666-infrastructure` converts the remaining families to one checkpoint. | new T3 worktree |
| #686 merge latency | analysis | `/root/measure-686-merge-latency` reports on #686. | shared |
| TGP Ir attribution | analysis | `/root/attribute-tgp-ir-rise` reports on #275. | shared |

Preserve the following:
- `t3-3b491ddb`, with five dirty files;
- `t3-431c84b4`, with #565's 58 dirty files;
- `t3-d78d5f8b`, the native-fixture evidence;
- the locked `.local/worktrees/profile-after68-664`;
- the unchanged pinned original and the retained profiles:
  - profile 64's run, `.local/simulation/20261102T135838Z-2198633`;
  - the anchor-head profile, `.local/simulation/20261105T091320Z-509137`;
  - the integration 80 and 82 profiles.

  Housekeeping lists the first two for removal.
- this host's speed-pass bundle and anchors under `.local/speed-pass/`.
