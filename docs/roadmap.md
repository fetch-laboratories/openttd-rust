# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues. Keep about 200 lines; evidence stays in PRs.

## Where the fork stands (2026-10-26, code at `97c32c48b1`)

- 67 ownership ports. Original C++ retired: 63,130 of 470,612 source lines
  (13.41%); simulation sources: 54,254 of 102,615 (52.87%). Whole-fork Rust,
  tooling and glue: 115,422, 40,918 and 41,179 lines. These measures include
  comments/data, not completed behavior. #480 integrated effects, cargo and
  road/reader maintenance: 531 retired lines and 744 net glue.
- Under integrated `04fa3753bf` quiet anchors, the six merged-tip projections
  are TGP 1.158x, Opus 1.474x, Grok 1.468x, P1996 1.418x, P2000 1.460x and
  P2016 1.544x. All semantic/compatibility checks pass. Opus, P1996 and P2000
  rose over 3%; #275 owns attribution and the next maintenance repair. P2016
  exceeds its ceiling; a quiet pass must follow the repairs. Ready owners proceed.
- Four owner branches remain: engine loading #474, interpreter #473,
  industry definitions #479 and rail/road definitions #476. #477 follows.
  #199 selects the next checker/shared-reader maintenance; #156 retains corpus gaps.

## Twentieth steering review (2026-10-24, recovery and production evidence)

Five fresh Codex gpt-6.1-sol/high audits covered both drafts, shared code, evidence
and process. Root verified source, the retained stack and mocked collector behavior.

- #462 is an inherited reachable source defect, not a #446 fixture failure.
  Restore the nullable original callback contract through one shared repair.
- #450 accepts compiler-overlapped timings after stopping the compiler.
  Reopened for a bounded repair using existing observations; keep clean-checkout checks.
- #454 deleted two meaningful serialized-thread tests. #156 selects their
  restoration and production tree ambient/reread coverage for #275. This corrects
  nineteenth-steering test cleanup: an explicit state contract is still behavior.
- #446/#447 have per-field engine boundaries and unfinished evidence. Their PR
  comments hold the fixes and measured costs; complete a bounded repair and one
  cost disposition. No new arithmetic divergence or shared-heap regression was established.

| Standing correction from reviews eighteen/nineteen | Verdict and cost/result |
| --- | --- |
| Ownership; necessary owner cost once | Applied before the user's scoped pauses: 11 owners and 4,707 retired lines since review eighteen. |
| Strict combinations, batching, fixing review | Keep: 24 Full requests since review eighteen, 16 passed, 6 failed, 2 cancelled; 239.7 elapsed run-minutes. Housekeeping used five passing Fulls, 59.0 minutes, and 17 tasks. |
| Ready owners alongside speed; no per-PR timing | Applied in development; sequential resume wording and old issue gates risked recurrence. Work order and #168/#199/#312 corrected. |
| Same anchors, merged-tip cadence, bounded quiet holds | Applied through `00ab107424`; final profile interrupted. Five completed passes released after 720 timed exits. New collector needs #450. |
| macOS cadence, focused local checks, review exemption | Applied; one no-finding conflict review unnecessarily reran tests. No new gate needed. |
| Shared callees, scope, orphan checks, #156 capacity | Improved; nine coverage PRs before the scope restriction. #462 and pending boundary findings show remaining gaps. |
| Housekeeping, current lists, cleanup | Applied with limits: 3,845 lines removed, local output 25 to 8.5 GiB; #275's body was stale, now refreshed. Two useful tests must return. |
| Checkpoints, wind-down, rollback | User pauses respected; drafts preserved. Newly confirmed #462 requires forward recovery because later code rules out bounded rollback. |

No user policy is reversed. AGENTS.md clarifies test retention; the restart prompt is unchanged.

## Earlier steering, still in force

Ownership and original behavior come first: named callees, shared typed services,
process scope and RNG; no extra hot allocation, opcode dispatch, whole-record
copies or owner bounces. #274 stays bounded. Keep default semantic evidence, #156
gaps, strict exact-combination CI and original retirement separate from glue.
Ready owners proceed alongside speed work; holds cover timed collection only.
Compare one anchor; 3% and factor 2 remain provisional. Owner evidence has no
infrastructure budget. Keep six branches, batched maintenance, fixing reviews,
bounded rollback and action-based wind-down. AGENTS.md is authoritative.

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

Maintenance rows use original retirement and net glue (#223); evidence stays in PRs.

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #355, #356, #357 batch and free-core `--jobs` | #358 | direct | `f141b6e91a` | 0 / 84 / 9 / 0; net glue +5 |
| #281 part 1 vehicle maintenance direct calls | #340 | direct | `9a991a1c23` | in PR |
| #274 shared assertion-faithful map reads | #351 | direct | `71d4f1d020` | in PR |
| #261 cleanup and selected CI maintenance | #361, #365 | #368 | `4ddd8cfab4` | in PRs; ABI tooling 172/180, CI changes 376/400 |
| #261 canonical station reservation setter | #376 | #379 | `dd31616e7a` | in PR; other drafts adopt this definition |
| #261 shared linear nearby-industry cursor | #381 | #382 | `032d0d150c` | 47 / 25 / 28 / 0; net glue 11; 100/100 added lines |
| #275, #356 typed routing and quiet anchors | #390, #391 anchors | direct | `824ebd56c5`, `ac7d2b60f2` | #390: 226 / 13 / 34 / 0; net glue 20; fixture 24/80 |
| #394 CI idle-time trims | #395 | direct | `cb4a01e86f` | 0 / 10 / 0 / 0; net glue 0 |
| #261/#281/#156 source, checker, coverage; #275 repairs/shared leaf | #396, #401, #402 | #403 | `6a2416df4e` | 301 / 399 / 91 / 4; net glue 40 |
| #275 complete quiet-ac7 six anchors | #404 | #406 | `77b8e663f2` | 0 / 8 / 0 / 0; net glue 0 |
| #156 structure coverage; #261 shared resolver metadata | #405, #408 | #412 | `6bb78266d2` | 0 / 136 / 40 / 0; net glue 28 |
| #261 shared metadata; #156 bribe coverage and frozen industry loader | #413, #414; #415 repair | #415 | `268c7c140d` | in PRs; combined 3120 / 1099 / 979 / 1592, net glue 723 |
| #261 goods/base scope; #275 anchors and repairs; #156 affordability/PBS observation | #419, #420, #421; #423 repair | #423 | `09eda66766` | in PRs; combined with #411: 1297 / 439 / 350 / 561, net glue 329 |
| #261 station/spec metadata; #156 vehicle real groups; MSVC animation return | #424, #426; #427 repair | #427 | `445ce4f30c` | in PRs; combined with #422: 880 / 305 / 362 / 369, net glue 286 |
| #156 wagon overrides; #275 track/orders and quiet anchors | #431, #432, #433 | #434 | `1569158485` | combined with #416: 966 / 308 / 303 / 430, net glue 209 |
| #156 property sign/failure boundaries; MSVC assertion-only warning | #435; #437 repair | #437 | `8b3dd5ebef` | combined with #425: 1120 / 290 / 520 / 458, net glue 479 |
| #156 statue search; #275 whole anchors; concrete animation ABI test | #441, #443; #444 repair | #444 | `103ee7cff8` | combined owners: 1768 / 800 / 840 / 563, net glue 627 |
| #275 aircraft witnesses, direct orders and quiet anchors; #156 order flags | #442, #445, #448 | #453 | `951eec41e5` | 189 / 183 / 12 / 0; net glue -7 |
| #450 one comparison driver and one platform check (bootstrap Full) | #455 | direct | `79f6598254` | 0 / 1331 / 0 / 0; net tooling -1,863 |
| #449 tests that cannot fail and workflow text pins | #454 | direct | `1168de46e6` | 14 / 518 / 89 / 0; net glue -200; net -750 lines |
| #451 shared header constants; #450 harness helpers and tracked speed collector | #456, #457 | #458 | `00ab107424` | 921 / 1207 / 8 / 0; net glue -12 |
| #452 comment pass outside the #451 structural files (comments only) | #459 | direct | `e5c8c62a96` | 221 / 49 / 248 / 0; net glue -191 |
| #451 YAPF heaps, `GameState`, shared macros, raw-access contracts, reservation callbacks | #460 | direct | `d4a0700869` | 2479 / 9 / 968 / 0; net glue -83; net -1,030 lines |
| #462 nullable clone refit callback recovery | #463 | direct | `4677a4fe4f` | 6 / 71 / 0 / 0; net glue 0 |
| #275/#450/#461/#156 source, collector and coverage maintenance | #464, #465 | #469 | `04fa3753bf` | 150 / 121 / 0 / 0; net glue 0 |
| #275/#199 road boundaries, quiet anchors and checked reads; Windows startup binding | #478; #480 repair | #480 | `97c32c48b1` | #478: 123 / 9 / 200 / 4; net glue 187 |

## Phase 1: maintenance and coverage

#275 owns the measured rise, forward repair and quiet remeasurement. #199 selects
diagnostic-literal checker repair (100 net tooling lines) and one shared Rust
checked-reader API for #475/#476. #168 retains boundary debt without timing gates;
#312 stays unselected. Closed housekeeping issues are not standing tasks.

## Phase 2: work order

1. **Ready owners:** #473/#474 adopt integrated cargo services, then fresh scoped
   fixing review and exact-current-base Full. Batch reviewed heads when useful.
2. **Maintenance:** attribute the #275 rise and fix forward; #199 supplies shared
   reads and the checker repair. Only timed collection holds builds/profiles/games.
3. **Whole owners:** complete industry definitions #475 and active rail/road
   definitions #476; house definitions #477 follow. Reuse canonical state/services;
   decide necessary cost once. Plan successors when a slot frees.
4. Catchment, #64/#68/#69, vendored, GUI and platform stay outside this order.

## Resume checkpoint

Root owns final combinations and speed; completed reports remain in the PRs.

| Item | Saved head | Next action |
| --- | --- | --- |
| #470 engine definitions / #474 | `dc0a43263f` | Review passed through `683ce27b2d`; validate/review cargo adoption after provider interruption. |
| #471 interpreter / #473 | `80abc98b99` | Split reviews pass; integrated cargo adoption and scoped review in progress. |
| #475 industry definitions / #479 | `3744aad3e5` | Checkpoint passes; bind integrated reader/cargo, then shared finalization helper. |
| #476 rail/road definitions | `97c32c48b1` base | Implement registry/property/translation/saved-identity owner. |
| #199 maintenance | selected | Repair literal identity and publish one shared Rust reader API. |
| #275 speed | `97c32c48b1` | Attribute rise from `04fa3753bf`, repair forward, then quiet pass. |
| #477 house definitions | selected | Start after ready work within branch and agent limits. |

macOS cadence passed on `f6cec101d3`; one integration has followed it.

Selection: /root, gpt-6.1-sol, xhigh; planning agents run gpt-6-astra, high.
Scopes and evidence plans are in the selected issues; PRs carry current task status.
