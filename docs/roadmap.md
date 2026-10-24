# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-24, code at `00ab107424`)

- 63 ownership ports. Original C++ retired: 62,169 of 470,612 source lines
  (13.21%); simulation sources: 53,345 of 102,615 (51.99%). Whole-fork Rust,
  tooling and glue (`tools/port-metrics.py`, which does not measure behavior):
  113,870, 39,261 and 40,347 lines. The nineteenth-steering housekeeping so far
  removed 224 Rust, 2,001 tooling and 212 glue lines; no regression or rollback.
- Projections at `00ab107424` from the 103ee anchors: TGP 1.209x, Opus 1.420x,
  Grok 1.415x, P1996 1.344x, P2000 1.413x, P2016 1.484x, equal to the anchors
  (`79f6598254` was within 0.5%). P2016 stays in its band; the 103ee quiet pass
  already measured it and #275 holds its attribution and two repair candidates,
  so no new pass ran. macOS passed at `79f6598254` and `00ab107424`.
- Unintegrated branches: paused #446 and #447, plus the #451 structural and
  #452 code-comment PRs in progress (four of six).

## Nineteenth steering review (2026-10-24, housekeeping)

The user asked for a tech-debt audit. Five fresh Claude Code (claude-opus-5-5)
audits covered tests, Rust and glue, Python tooling, comments and docs, and
retained evidence; steering verified each claim acted on. No fidelity audit ran.

- **Tests** are mostly behavioral; none asserts on docs. About 40 cannot fail,
  restate their setup or pin YAML and source text. One pin cost #444 a Full.
- **Rust and glue** repeat what AGENTS.md says to port once: `DAY_TICKS` is a
  literal `74` in 8 places, 41 constants have 104 definitions, three
  pathfinders keep private heaps, 739 of 1,379 SAFETY lines are copies. One
  export has no caller; the orphan check misses `extern "C"` declarations.
- **Housekeeping slipped:** #261's 2026-10-15 glue and Rust plan never ran
  while Rust grew from 60K to 114K lines; #261 became a selection log.
- **Tooling:** the harness is proportionate; comparison drivers, platform
  evidence tools and copied scenario helpers carry about 2,000 extra lines.
- **Comments and docs:** about 55 module docs are review checklists; "shell",
  "leaf", "canonical" and "owner" are undefined. `rust-migration.md` grew 304
  lines in a day, mostly evidence; AGENTS.md repeats 13 rule groups.
- **Evidence:** the speed pass has no tracked collector. Each pass copies the
  last pass's `.local/` script, now from a stale worktree, with the removed CPU
  cutoff stripped by string replace. Stale worktrees and quiet-pass output hold
  about 23 GB. #156 has 202 comments and a stale body.

Actions:
1. **Local cleanup (#259):** done; #275 received the retained conclusions.
2. **#449 tests, #450 harness and tooling, #451 Rust and glue, #452 comments
   and docs.** Each gets the PRs its issue names, with no behavior change;
   batch integration when several are ready. The user may assign them to a
   separate Claude Code session: before starting one, root checks the issue
   for an owner comment or linked PR. #450's collector lands before the next
   speed pass; #452's code-comment PR follows #451. Line targets are audit
   estimates (provisional): a PR that misses one explains why, it does not hold.
3. **AGENTS.md** adds comment, prose and test rules, tracked speed tooling,
   evidence-free component entries, and tracker bodies as current lists with
   task status kept on the PR. #261 is closed; batching names #449-#452.

| Eighteenth-steering correction | Verdict and cost |
| --- | --- |
| Ready owners do not wait for speed maintenance | Applied: nine integrations on 2026-10-24. |
| Quiet holds end at the last timed exit; no CPU cutoff | Applied, through untracked code; #450 tracks it. |
| Owner evidence without infrastructure budgets | Applied: 25 owners since the seventeenth steering. |
| Same-anchor projection comparisons | Applied. |
| Standing #156 coverage worker | Applied; status went to #156 and the PR. Now PR only. |
| Strict tested combinations, batching | Applied: 44 Full requests, 24 passed, 12 failed, 8 cancelled. Keep. |

No user direction is reversed. The start-development prompt is unchanged.

## Earlier steering, still in force

Ownership and original observable behavior come first; C++ retains canonical map
arrays except the audited #274 read descriptor. Preserve original callees,
shared services, process scope and RNG; no per-call allocation the original
lacks, opcode dispatch, whole-record copies or owner bounces. Evidence is
default semantic checks plus exact-combination CI, gaps on #156, and retirement
excluding glue (#223). Ready owners never wait for speed maintenance, quiet
holds cover only timed collection, projections compare under one anchor, and
owner evidence has no infrastructure budget. The 3% trigger and factor 2 stay
provisional. Keep six component branches, strict protection, no admin merge,
automatic bounded rollback and action-based wind-down. AGENTS.md is authoritative.

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


## Phase 1: maintenance and coverage

#275 owns measured speed, #156 missing branches and #451-#452 the remaining
housekeeping (#259, #449 and #450 are closed). #168 and #199 own the remaining
boundary conversions (disaster's async protocol, opcode switches). Housekeeping
PRs count toward the six branches.

## Phase 2: work order

1. **#451 structural PR** (YAPF heaps, registries, macros, SAFETY accessors,
   `train_reservation.rs` forwarders; it applies #452's comment rules to the
   files it rewrites) and **#452 code-comment PR** (every other file), then
   review, Full and integration.
2. **#275 growth repair** (#436's per-call ambient gate, the airport animation
   entry in the shared dispatcher), then the next speed pass with the tracked
   collector (`python3 tools/simulate.py speed prepare|collect DIR`).
3. **#446** investigates the refit-cost clone crash; **#447** validates its
   TownID fixture repair. Both need cost checkpoints, review and Full.
4. **Next owners** (planned, unselected): vehicle effect emission and cache
   policy, then the cargo registry and callback lifecycle, once the branch
   limit allows. Catchment, #64/#68/#69 and vendored, GUI and platform work
   stay unselected.

## Resume checkpoint

Three integrations this session (#455, #454, #458). Merged worktrees and
branches are removed; `.local/` holds only caches, the pinned zlib runtime and
`.local/speed-collector/` (superseded by `tools/simulation/speed.py`; delete
after the first tracked pass).

| Item | Saved head | Next action |
| --- | --- | --- |
| #446 engine/articulated (#439) | `e96e45c240` | Refit-cost clone crash; simulate passed 1 of 3. |
| #447 type admission (#440) | `c3fdd2227d` | Validate TownID fixture repair; last pair failed. |
| #451 structural | `housekeeping-451-structure` | Implementation running; then review and Full. |
| #452 code comments | `housekeeping-452-comments` | Implementation running; then review and Full. |
| Merged-tip profile | `00ab107424` | Done; next after two more integrations. |

The next macOS run is due after five more integrations. The user's tar files stay.
