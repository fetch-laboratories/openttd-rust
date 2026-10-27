# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of the umbrella
issue #3. Update it in place; `/steer` prunes it.
Evidence stays in PRs.

## Where the fork stands (2026-10-27, code at `be18afafda`)

- 77 ownership ports: catchment #522 (#516) integrated at `4c73c991e9`.
  Maintenance since the wind-down: #502 (cadence split, typing gates), #521
  (macOS annotation warnings), #517 (#275 accessor audit), #519 (#515
  orphans, #500's first ID types, #506's check), #523 (#275 speed recovery)
  and #520 (#168 disaster boundary), each at its exact-head Full. Eleven
  integrations since the twenty-first steering review; no rollback target.
- Whole-fork inventory at `be18afafda`: 71,218 of 470,612 original lines
  retired (15.13%), including 55,546 of 102,615 simulation lines (54.13%);
  Rust, tooling and glue are 127,585, 44,489 and 45,227 lines. These measures
  include comments/data, not completed behavior.
- Speed: #517's audited reads raised play loop counts 2.2% to 4.0% (P2016
  +3.0%, projecting about 1.53x). #523 recovered it by restoring callback-path
  codegen and returning audited pixel heights to their typed services:
  pinned-runtime counts are within +0.11% of `c04219b0d8` on every scenario
  (TGP -0.38%). Projections stay at their `c04219b0d8` values (TGP 1.1788x,
  Grok 1.3948x, Opus 1.4018x, P1996 1.3354x, P2000 1.3934x, P2016 1.4695x),
  but #517 changed the build identity, so the f492 anchors no longer match;
  a quiet pass re-anchors once the host is quiet.
- The tenth-integration cadence dispatch on `4481a442f6` passed every platform
  and annotation check. #512's first tally (19 finders: review 11, comparison
  4, scenario 2, CI 2) is posted and the issue closed.
- Unintegrated branches: #518 (coverage corpus rerun with a longer per-game
  timeout, about 2.5 hours). Five component slots are free.

## Twenty-first steering review (2026-10-26, orphans, concurrency, cadence and accessor cost)

Two Claude Code passes. The first (Fable) ran four Opus fidelity audits (#473,
#479, #488, #481) and short process and speed audits before its session budget
ran out. The second (Opus) audited #495, which integrates next and retires more
than three of those four, checked the process and speed claims against CI logs
and retained profiles, and profiled `4853789508` against the pinned original on
P2016. The user's housekeeping between wind-down and this review (#500 to #514,
#502, nine docs commits) was reviewed as root's work.

Findings:
- No reachable divergence in #473, #479, #488, #481 or #495. Every one left
  `WITH_RUST` bodies or `openttd_rust_*` exports whose only callers are tests
  or the harness, and every review report said "no orphans". #473 also carries
  two dead `#ifdef WITH_RUST` branches nested inside the retained
  `AfterLoadGRFs`; #495 keeps `CommitVehicleListOrderChanges` and its export
  alive for two harness probes and runs the C++ `Slide` over a Rust vector.
- Load-time boundary cost repeats one pattern: a callback per field or slot
  (industry finalise about 2,700 calls per load, label lookup 64 calls, station
  cache per slot). The house, airport and object manager callbacks that bounce
  into Rust-owned storage are removed by #486, #499 and #498's scope.
- Process: 86 delegated tasks and 14 Fulls (8 passed, 6 failed, 221 minutes)
  for 9 owners and 8 maintenance items. Owners built concurrently over each
  other's unintegrated services (cargo registry, Reader, type registry, house
  manager) cost about 20 tasks, 29 adoption commits and 4 scoped reviews. The
  six failed Fulls came from Windows or portable builds (#468 solo, #480,
  #494), scenario fixtures still binding a removed facade or slot (#483, #489,
  #494's second run) and apt timeouts; none came from combining components.
- Speed: three quiet passes in one session (P2016 1.502x, 1.453x, 1.469x)
  spread 3.3% while the six credited repair bounds total 154M Ir, 1.13% of the
  loop, and the wall ratio moved opposite to the instruction count between
  passes. Five repair PRs, nine profiling or attribution tasks and 432 timed
  games recovered nothing a pass can show.
- Speed attribution at `4853789508` (P2016, inclusive costs under
  `StateGameLoop()`): 13.434G Ir against the original's 10.395G. The 3.04G gap
  is road vehicles +1.00G (1.31x), aircraft +0.77G (1.31x), trains +0.52G
  (1.57x), tile loop +0.25G, station tick +0.22G, vehicle dispatch about +0.14G
  and loading +0.08G; ships run 0.12G below the original. Map and tile accessor
  callbacks from Rust take 630M Ir self (4.7% of the loop) over 114 wrappers.
  The top seven (`RustNormalRoadStatusTail`, `RustPixelLocation`,
  `SharedTileType`, `RustTilePixelSlope`, `SharedNormalRoadBits`,
  `RustHasTileRoadType`, `RustWaterTileType`) take 437M over 31.9M calls at 8 to
  27 instructions each, where the original inlines a few: about 220M Ir (1.6%)
  of net callee overhead after an estimated inline cost per call, before
  caller-side savings, more than all six earlier repairs together. #274's
  design keeps them typed until audited, and #275 forbade enlarging the
  descriptor. Candidate profiles survive under `.local/simulation/`; the
  reference keeps only its Ir count, so this review retained a reference P2016
  profile under `.local/speed-reference-profiles/`. No save loads a NewGRF, so
  definition and Action2/3 parsing cannot move a ratio; #486 and #499 change
  runtime accessors, #495 none on the game loop.
- Direction: the C++ still compiled in simulation sources is mostly drawing,
  GUI, mode control and the generation driver. The exception is station
  catchment (about 400 compiled lines), which root's planning dropped on
  2026-10-23 with no issue or reason while the work order moved to NewGRF
  loading, support code that keeps spec storage native behind per-field setters
  (net glue 491 to 715 against 780 to 881 retired). #516 now tracks it.
- Passed-over issues: besides the testing set, #198's remaining scope landed on
  2026-10-09 but the issue stayed open (now closed), and #312 waited on an
  attribution nobody applied (trees +65M Ir, 0.48% of the P2016 loop; houses
  +76M and water +71M beside it).
- Hygiene: 3 of 7 draft worktrees are redundant, 3 drafts sit on local
  branches named differently from their PR branches, and root made 17 roadmap
  commits in one session.

Corrections (AGENTS.md carries the rules; apply these in order):
1. Work from the ranked work order at the end of this roadmap. It holds
   ports, maintenance, testing and housekeeping in one list, ranked by what each
   item retires, protects or saves; a category is not a rank. Each free slot
   takes the highest unblocked item. This replaces the earlier order that put
   every new port ahead of testing and left housekeeping to "ride a batch".
2. #497 Action3 and #498 object definitions are parked. Like the recent
   definition ports, they keep the data in C++ behind one setter per field
   (glue near the lines retired) and port load-time code no speed scenario
   runs; no selected game component needs them. They return when a selected
   component needs their data in Rust, re-planned so Rust owns that storage.
   Planning ranks by value, never by how easy an item is to scope.
3. Concurrent owners need disjoint unintegrated services; planning names each
   owner's service needs. #516 catchment is planned now.
4. Batch only disjoint-file components into one integration PR; overlapping
   ones integrate one at a time at their reviewed heads. Its stated basis was
   wrong (see Process), so the next review judges it by Fulls per integrated
   owner and by base-update reviews.
5. Review reports start with `Reviewed commit:` and list each `WITH_RUST` body
   and export with its production caller; a harness or test caller is not one.
   Before requesting Full, a PR that removes a name greps `tools/simulation/`
   and `src/tests/` for it.
6. Speed: while a scenario sits in band, a pass runs only after a 3%
   projection rise or a repair batch crediting 1.5% of the loop; a candidate
   under 0.5% of the loop stays a #275 note and candidates batch to 1%
   (provisional, from the 3.3% spread and 1.13% credited). Keep each profile's
   candidate output; attribute against the retained reference profile.
7. Next #275 item, its own maintenance PR within six branches (it does not wait
   for #515): audit the seven ground-vehicle accessors named above for #274's
   descriptor under its existing rules (field-sized loads, fresh at the
   original observation points, ended before unproved callbacks), replace each
   audited callback with the shared header-module helper, and credit the result
   as net P2016 Ir against the retained reference profile. An accessor the
   audit cannot prove keeps its typed service. This lifts #275's "do not
   enlarge shared raw descriptors" for these accessors only. If the batch
   credits 1.5%, the in-band pass follows; trains (1.57x) are the next
   attribution target. #312's tree excess then batches with the house and water
   tile-loop excess (together about 1.6%) as the following #275 item.
8. Remove the redundant worktrees `t3-d6418e16`, `t3-9bff1490` and
   `t3-758125c7`; keep one worktree per PR, on its PR branch. Commit the
   roadmap when taking stock or changing the work order, not per integration.
9. The user's testing issues, ranked in the work order by the defects the
   fork has had: #461 (thread scope), #462 and #75 (reachable branches no
   scenario ran) and #355 (MSVC ABI).
   1. #504 coverage, selected now. Audits find coverage gaps by reading code,
      and #156 holds 234 unranked entries. A measured report ranks them, names
      default scenarios that add no unique coverage (shortening Full) and gives
      #507, #508, #505 and #511 their targets. It is reference-only, with no
      CI cost.
   2. #503 deep tier with `--soak`, which runs nowhere today; it hosts the long
      runs below.
   3. #507, NewGRF sets first: six NewGRF owners are in flight or queued and no
      play save loads a NewGRF file. Community AIs and the long save follow.
   4. #505 ASan and UBSan.
   5. #508 random-command AI, aimed at command code #504 shows unexecuted.
   6. #511 mutation study, classified with #504's report.
   7. #506: first a quick-CI check rejecting `thread_local!` outside
      `#[cfg(test)]` (production has none; #461's class), riding a maintenance
      batch. The threaded and TSan run follows once its deterministic stop is
      designed.
   8. #509 Windows and macOS subset, after #514 changes the cadence run.
   9. #510 multiplayer spike.
   #514 carries #502 and #501's gate configuration (ty and ruff `ANN` checking
   new and converted files) so one bootstrap covers all three; #501's
   annotation batches ride maintenance PRs. #512's first tally is due at the
   tenth integration after `4853789508`. #513 needs no action until an
   improvement is accepted. None of these adds per-PR CI.

| Twentieth-steering correction | Verdict and cost/result |
| --- | --- |
| #462 recovery before further integrations | Applied: #463 merged first; one PR, one review, one Full. |
| #450 collector repairs before the next pass | Applied: #465 in #469; three passes ran with zero overlap records. |
| Restore the two serialized-thread tests and tree ambient coverage | Applied: #464 in #469. |
| #446/#447 bounded boundary repair, one cost disposition | Applied: both integrated (#446 direct, #447 via #469). |
| Owners alongside speed; #168/#199/#312 refreshed | Applied: 9 owners beside 5 #275 repairs; #199 current, #312 unselected. |
| Clean-checkout checks, explicit state contracts | Applied; the audits found no new thread-local state. |
| Provenance before calling a file user-owned | Applied; the checkout holds no stray artifacts. |
| Nineteenth: comment, prose and test rules; evidence-free component entries; current tracker bodies | Applied: nine component entries without evidence narration; #156 body current to 2026-10-24, later gaps in comments. |

User rule changes no root session has run under yet (carried forward; the next
review judges them by their effects):

| Change | Commit | Watch for |
| --- | --- | --- |
| Wind-down rereads root before sending, cancels clean workers, re-sends to silent ones, makes cleanup optional | `42972a002c` | Time from message to stop; stale or overrun instructions |
| Roadmap has no line cap; edits update in place; only `/steer` prunes | `414ff1daab` | Growth between steers, sediment |
| Issue, PR, component-entry and skill-output caps become content rules | `987af0b0be`, #502 | Narration or evidence logs; needed facts dropped |
| Tooling overruns decided at the first draft, no raise count | `40b20044f1` | Overruns without a recorded reason |
| New Python code is type-annotated; #501 converts existing code | `122754f627` | Unannotated new functions passing review; #501's tooling size |
| Testing issues queued; #514 the next workflow PR | `1f2bc8e228` | Testing work starting ahead of ready owners or #515 |
| Steer judges user rule changes and carries untested ones forward | `bcbfa60afb`, `805c0111c8` | Applied by this review at no cost |
| Every open issue has a place in the roadmap; root runs `docs/skills/steer/issues.py` at start, when planning and when taking stock (user-directed after this review found catchment, the testing set, #312 and #198 passed over) | twenty-first review | Unplaced and excluded counts at each steer; exclusion lines without a return condition; time root spends placing issues |

## Earlier steering, still in force

Ownership and original behavior come first: named callees, shared typed services,
process scope and RNG; no extra hot allocation, opcode dispatch, whole-record
copies or owner bounces. #274 stays bounded. Keep default semantic evidence, #156
gaps, strict exact-combination CI and original retirement separate from glue.
Ready owners proceed alongside speed work; holds cover timed collection only.
Compare one anchor; 3% and factor 2 remain provisional. Owner evidence has no
infrastructure budget. Keep six branches, batched maintenance, fixing reviews,
bounded rollback and action-based wind-down. Recover a confirmed regression
before integrating when later merges block bounded rollback; explicit state
contracts are behavior; establish provenance before calling a file user-owned.
AGENTS.md is authoritative.

## Completed ownership ports

Metrics are `tools/port-metrics.py`: Rust / tooling / C++ glue / C++ retired.

| Issue | Component | PR | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #73 | TGP terrain generation | #94 | `014968bd38` | 658 / 85 / 84 / 991 |
| #74 | Link graph job computation | #95 | `7a2858c469` | 1040 / 159 / 135 / 1120 |
| #96 | Built-in town names (retired count is mostly data tables) | #98 | `ff3648ba09` | 3296 / 241 / 122 / 3848 |
| #99 | Tree generation, simulation, planting | #102 | `602b5a60ff` | 1288 / 561 / 247 / 816 |
| #101 | Effect vehicles | #105 | `f6a0d66b63` | 494 / 553 / 412 / 548 |
| #104 | Water-region cache and graph service | #114 | `9732d8bfa2` | 459 / 610 / 273 / 407 |
| #103 | Disaster scheduling, vehicles and event control | #118 | `d5edfcb4c5` | 1404 / 795 / 306 / 968 |
| #117 | Cargo payment and delivery | #123 via #134 | `adbe063372` | 396 / 618 / 370 / 315 |
| #119 | Ship YAPF and canonical path cache | #126 via #134 | `adbe063372` | 881 / 54 / 506 / 646 |
| #120 | Town growth and private state | #127 via #134 | `adbe063372` | 1430 / 668 / 625 / 1548 |
| #121 | Road controller/private state (includes 1475 movement-data lines) | #128 via #134 | `adbe063372` | 3159 / 471 / 1188 / 2842 |
| #122 | Rail YAPF search, cache and reservation | #133 via #141 | `ea80a746a3` | 1704 / 144 / 595 / 1517 |
| #124 | Road YAPF search and path construction | #135 via #141 | `ea80a746a3` | 663 / 55 / 329 / 532 |
| #125 | Station cargo-service control and state | #142 | `ba920555ca` | 1516 / 241 / 1022 / 1332 |
| #130 | Rail controller, reservation and private state | #143 via #166 | `dace87c9b1` | 4087 / 613 / 1660 / 2585 |
| #129 | Industry production, histories and builder state | #144 via #166 | `dace87c9b1` | 1508 / 474 / 678 / 932 |
| #136 | Aircraft controller and airport blocks | #145 | `f0e0b3d712` | 1773 / 764 / 380 / 1880 |
| #137 | Company finance, economy and lifecycle | #149 | `dbce82a309` | 1607 / 702 / 1052 / 1394 |
| #139 | Cargo storage and movement | #151 via #184 | `2a76d5f287` | 2838 / 460 / 973 / 2445 |
| #138 | Orders lifecycle and commands | #176 via #184 | `2a76d5f287` | 4554 / 519 / 1752 / 2518 |
| #146 | Ship controller and private state | #152 via #184 | `2a76d5f287` | 1101 / 270 / 893 / 721 |
| #147 | Fleet groups and autoreplace | #197 | `a7a6e6650a` | 2686 / 577 / 1199 / 1454 |
| #150 | Industry construction and lifecycle | #218 | `648d44e940` | 3066 / 701 / 1433 / 1736 |
| #148 | Town canonical state and periodic policy (narrow scope) | #216 | `59385bce59` | 1848 / 694 / 1418 / 340 |
| #220 | Engine lifecycle and canonical design state | #248 | `d2115d1c3d` | 1218 / 369 / 833 / 579 |
| #219 | Rail signal blocks and fixed work sets | #245 | `9487f3079f` | 1229 / 812 / 445 / 646 |
| #246 | Town founding, generation and names | #258 | `09bb0b4df8` | 1123 / 321 / 470 / 492 |
| #265 | Rail track, depot and signal commands | #270 | `0cb20cec28` | 2965 / 301 / 740 / 1578 |
| #213 | Vehicle scheduling and base maintenance | #257 | `8bfcbd3abd` | 1833 / 375 / 1160 / 388 |
| #212 | Landscape scheduling, ground evolution and flooding | #255 | `377b374579` | 2065 / 553 / 1095 / 846 |
| #267 | Station construction and removal commands | #303 | `ca2b181d88` | 5233 / 333 / 1388 / 2427 |
| #294 | Water construction, clearing and restoration | #317 | `067e3c3b93` | 871 / 249 / 228 / 512 |
| #256 | House lifecycle and tile simulation | #269 | `5ac2437aa5` | 1659 / 212 / 685 / 890 |
| #266 | Whole road command policy | #290 | `4e7753122e` | 2547 / 375 / 545 / 1265 |
| #295 | Object lifecycle, commands and NewGRF policy | #313 | `883ce69671` | 2474 / 429 / 585 / 928 |
| #314 | Vehicle transactions and non-rail construction | #329 | `43257e3744` | 3231 / 206 / 1200 / 1312 |
| #316 | Subsidy state and route lifecycle | #346 | `7e83fe925e` | 915 / 177 / 277 / 430 |
| #293 | Tunnel/bridge commands and ownership | #352 | `802ec7d6a4` | 1862 / 178 / 325 / 854 |
| #348 | Landscape clear, terraform and leveling | #353 | `33fc37e973` | 1289 / 189 / 301 / 447 |
| #354 | Road-stop bays and drive-through occupancy | #362 | `4075c50384` | 875 / 164 / 181 / 501 |
| #247 | Town commands, authority actions and deletion | #347 | `8c370210b4` | 977 / 270 / 298 / 589 |
| #315 | Rail construction and depot consist transactions | #360 | `eccfe3f240` | 1082 / 129 / 347 / 864; net glue 246 |
| #349 | Vehicle visits and route-link prediction | #359 via #368 | `4ddd8cfab4` | 1588 / 160 / 411 / 769; net glue 222 |
| #350 | Remaining generation and complete river construction | #364 via #379 | `dd31616e7a` | 1864 / 316 / 416 / 1214; net glue 316 |
| #372 | Ground-vehicle dynamics and canonical cache | #375 via #379 | `dd31616e7a` | 943 / 116 / 381 / 475; net glue 190 |
| #366 | Complete waypoint command policy | #369 via #382 | `032d0d150c` | 1093 / 142 / 194 / 484; net glue 139 |
| #367 | Complete PBS reservation service | #370 via #382 | `032d0d150c` | 1279 / 267 / 349 / 459; net glue 102 |
| #371 | Complete tile movement policy | #374 via #382 | `032d0d150c` | 2455 / 304 / 753 / 985; net glue 542 |
| #373 | Live link-graph scheduling and queues | #380 via #391 | `ac7d2b60f2` | 303 / 263 / 278 / 185; net glue 245 |
| #378 | Canonical live link-graph contents | #383 via #391 | `ac7d2b60f2` | 922 / 82 / 364 / 378; net glue 214 |
| #377 | Vehicle tile occupancy and queries | #384 via #391 | `ac7d2b60f2` | 1061 / 161 / 222 / 233; net glue 17 |
| #386 | Cargo-monitor maps and lifecycle | #388 via #391 | `ac7d2b60f2` | 371 / 240 / 157 / 196; net glue 117; acceptance basis 193 |
| #385 | Complete animated tiles and NewGRF animation | #389 via #406 | `77b8e663f2` | 1154 / 633 / 594 / 472; net glue 429; acceptance basis 415 |
| #393 | NewGRF transactional storage and feature policies | #400 via #412 | `6bb78266d2` | 399 / 221 / 249 / 258; net glue 247 |
| #392 | Complete vehicle NewGRF resolver/cache runtime | #397 via #415 | `268c7c140d` | 2047 / 442 / 599 / 1076; net glue 378 |
| #398 | Industry/industry-tile NewGRF resolver runtime | #407 via #415 | `268c7c140d` | 1076 / 436 / 364 / 516; net glue 329 |
| #399 | Complete station/waypoint NewGRF resolver runtime | #411 via #423 | `09eda66766` | 1207 / 261 / 267 / 561; net glue 254 |
| #410 | Road-stop resolver and spec associations | #422 via #427 | `445ce4f30c` | 876 / 187 / 319 / 369; net glue 285 |
| #409 | House/shared-town NewGRF resolver runtime | #416 via #434 | `1569158485` | 927 / 215 / 303 / 430; net glue 209 |
| #417 | Complete common Action2 execution and invocation | #425 via #437 | `8b3dd5ebef` | 1120 / 213 / 520 / 458; net glue 479 |
| #418 | Airport/airport-tile runtime and live spec selection | #428 via #444 | `103ee7cff8` | 1087 / 245 / 496 / 264; net glue 337 |
| #429 | Generic callback registry, AI and ambient runtime | #436 via #444 | `103ee7cff8` | 349 / 188 / 164 / 248; net glue 138 |
| #430 | Object invocation and complete eligibility | #438 via #444 | `103ee7cff8` | 334 / 178 / 182 / 51; net glue 152 |
| #440 | Rail/road type introduction and construction availability | #447 via #469 | `04fa3753bf` | 457 / 327 / 229 / 158; net glue 165 |
| #439 | Engine cargo and articulated queries | #446 | `f6cec101d3` | 854 / 452 / 310 / 272; net glue 197 |
| #466 | Vehicle effect emission, cache and creation | #468 via #480 | `97c32c48b1` | 845 / 461 / 504 / 288; net glue 254 |
| #467 | Cargo registry, translation and callbacks | #472 via #480 | `97c32c48b1` | 718 / 366 / 370 / 239; net glue 301 |
| #470 | Engine definitions, identity and finalization | #474 via #483 | `b959f89059` | 1183 / 506 / 456 / 520; net glue 442 |
| #471 | Staged NewGRF loading interpreter and state | #473 via #483 | `b959f89059` | 1640 / 340 / 442 / 1778; net glue 413 |
| #476 | Rail/road definitions, translation and saved identity | #481 via #489 | `da1e7b7ade` | 1252 / 406 / 557 / 780; net glue 491 |
| #475 | Industry definitions, overrides and publication | #479 via #494 | `4853789508` | 1415 / 408 / 607 / 881; net glue 596 |
| #484 | Station/road-stop definitions and associations | #488 via #494 | `4853789508` | 1410 / 398 / 749 / 806; net glue 715 |
| #485 | Action2 graph construction and publication | #493 | `5e39efe901` | 1206 / 300 / 195 / 708; net glue 159 |
| #491 | Vehicle properties and deferred purchase ordering | #495 | `3b3382ce1c` | 1148 / 338 / 367 / 1031; net glue 290 |
| #477 | House definition loading and publication | #486 | `fc3bf4b20b` | 714 / 372 / 221 / 532; net glue 193 |
| #492 | Airport and airport-tile definitions | #499 | `c04219b0d8` | 590 / 297 / 311 / 336; net glue 251 |
| #516 | Station catchment, rectangles and nearby lists | #522 | `4c73c991e9` | 2216 / 315 / 461 / 702; net glue 204 |

Maintenance rows use original retirement and net glue (#223); evidence stays in PRs.
Batches before #453 (#358 to #448) are recorded in their PRs.

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #275 aircraft witnesses, direct orders and quiet anchors; #156 order flags | #442, #445, #448 | #453 | `951eec41e5` | 189 / 183 / 12 / 0; net glue -7 |
| #450 one comparison driver and one platform check (bootstrap Full) | #455 | direct | `79f6598254` | 0 / 1331 / 0 / 0; net tooling -1,863 |
| #449 tests that cannot fail and workflow text pins | #454 | direct | `1168de46e6` | 14 / 518 / 89 / 0; net glue -200; net -750 lines |
| #451 shared header constants; #450 harness helpers and tracked speed collector | #456, #457 | #458 | `00ab107424` | 921 / 1207 / 8 / 0; net glue -12 |
| #452 comment pass outside the #451 structural files (comments only) | #459 | direct | `e5c8c62a96` | 221 / 49 / 248 / 0; net glue -191 |
| #451 YAPF heaps, `GameState`, shared macros, raw-access contracts, reservation callbacks | #460 | direct | `d4a0700869` | 2479 / 9 / 968 / 0; net glue -83; net -1,030 lines |
| #462 nullable clone refit callback recovery | #463 | direct | `4677a4fe4f` | 6 / 71 / 0 / 0; net glue 0 |
| #275/#450/#461/#156 source, collector and coverage maintenance | #464, #465 | #469 | `04fa3753bf` | 150 / 121 / 0 / 0; net glue 0 |
| #275/#199 road boundaries, quiet anchors and checked reads; Windows startup binding | #478; #480 repair | #480 | `97c32c48b1` | #478: 123 / 9 / 200 / 4; net glue 187 |
| #275/#199 visual boundaries, shared reader and checker; fixture compile repair | #482; #483 repair | #483 | `b959f89059` | #482: 175 / 50 / 42 / 0; net glue 16 |
| #199 train/order boundaries, loading disable helpers; #275 quiet anchors; town fixture repair | #487; #489 repair | #489 | `da1e7b7ade` | #487: 61 / 8 / 61 / 0; net glue -3 |
| #275 primary dispatch, compact visual observations and deferred train resolution | #490 | direct | `f4923c79eb` | 28 / 3 / 18 / 0; net glue 1 |
| #275 road compiler boundaries, shared movement cache and six quiet anchors; direct class resets | #496; #494 combination | #494 | `4853789508` | #496: 55 / 9 / 2 / 0; net glue 0; combination retires 22 class-default lines |
| #514 cadence split; #501 gate configuration | #502 | direct | `1fae5d498a` | 0 / 118 / 0 / 0; net glue 0 |
| Cadence annotation warnings; orphan cargo-label export | #521 | direct | `cf8e561d61` | 0 / 0 / 5 / 28; net glue -4 |
| #275 ground-vehicle accessor audit | #517 | direct | `7274b59099` | 536 / 19 / 42 / 0; net glue 34 |
| #515 orphans and accessors; #500 first ID types; #506 quick check | #519 | direct | `fbf9f1cc5a` | 328 / 82 / 144 / 13; net glue 66 |
| #275 recovery of the post-#517 loop rise | #523 | direct | `4481a442f6` | 65 / 0 / 0 / 0; net glue -2 |
| #168 disaster typed services and direct calls | #520 | direct | `be18afafda` | 1168 / 118 / 323 / 3; net glue 111 |

## Work order

One ranked list (twenty-first steering, correction 1). Each free slot takes the
highest unblocked item; finishing review and integration comes first. Rank
reasons are one line each; scope is in the issues and corrections above.

1. **Quiet speed pass to re-anchor** the six scenarios on the post-#517
   build identity, once #518's corpus finishes and no builds run; record the
   anchors in `tools/simulation/instructions.py` through a maintenance PR.
2. **#504 coverage**, draft #518: report, #156 update, independent fixing
   review and Full; its report ranks #156 and names default scenarios that
   add no unique coverage.
3. **#275 tile-loop batch**: #312 trees, houses and water, about 1.6% of the
   P2016 loop, attributed against the retained reference profile.
4. **#503** deep tier with `--soak`, which runs nowhere today.
5. **#507** corpus, NewGRF sets first: no play save loads a NewGRF file.
6. **#505** ASan and UBSan.
7. **#508** random-command AI, aimed at command code #504 shows unexecuted.
8. **#511** mutation study, classified with #504's report.
9. **#506** threaded and TSan run, once its deterministic stop is designed.
10. **#509** Windows and macOS subset; **#510** multiplayer spike.
11. **#501** annotation batches ride maintenance PRs that touch those tools.

Recurring: profile the current merged tip under the f492 anchors after this
two-integration stocktake (a quiet pass only under AGENTS.md's in-band rule);
#512's
tally at the tenth integration after `4853789508`; `issues.py` at start, when
planning and when taking stock. #168's selector audit and #199's mirror audit
ride the items above that touch those modules.

Parked, each with what brings it back: #497 and #498 (correction 2); #64 and
#68, vendored code, after the simulation; the rest of #69 when a selected port
needs it beyond #516; GUI and platform after the simulation; the six
`deferred-improvement` issues and #513 after near-full reproduction.

## Resume checkpoint

Resumed 2026-10-27 by a Claude Code root (claude-opus-5-5, xhigh); reviews
run on gpt-6.1-sol (high) through T3. Code at `be18afafda`; no confirmed
regression or rollback target.

| Item | Head | Next action | Running jobs or CI |
| --- | --- | --- | --- |
| #518 coverage | `0bf8d71c0e` | Corpus finishes, then report, #156 update, independent review and Full. | Corpus run in its worktree since 07:07 UTC. |
| Quiet speed pass | f492 anchors stale | Run when #518's corpus ends and no builds run; anchors land through a maintenance PR. | None. |
| Next work | work order 3 to 6 | Start when the Claude session window allows (it was above 60%). | None. |
| Cadence | green at `4481a442f6` | Next dispatch after the fifteenth integration since steering. | None. |

Selection: /root, gpt-6.1-sol, xhigh; planning agents run gpt-6-astra, high.
Scopes and evidence plans are in the selected issues; PRs carry current task status.
