# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-31, code at `16e0235d0d`)

Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh.

- Since the integration-42 stocktake, two ownership PRs integrated without
  rollback. Metrics are `tools/port-metrics.py` code lines
  (Rust / tooling / original retired / net glue):
  - 43, #608 town spatial index and policy (`5345e29939`):
    699 / 112 / 120 / -16 (-26 production, +10 fixture);
  - 44, #609 industry servicing and policy (`16e0235d0d`):
    309 / 158 / 105 / +37 (+20 production, +17 fixture).
  Both passed fresh fixing review, root spot-checks and current-base exact-head
  Full. #600 and #601 are closed.
- Whole fork: retired 53,049 of 284,355 (18.66%, +225 since integration 42);
  glue 33,814 (+21); compiled C++ 265,120 (93.24%, -204). Rust totals
  123,488 code lines (+840) and tooling 50,706 (+245). The boundary has
  1,364 exports (+15) and 4,212 callback slots (-17).
- Simulation-source retirement is 40,155 of 61,148 (65.67%). The source-area
  inventory below retains executable headers and drawing in their source
  classification; these totals do not establish phase completion. Remaining
  storage-bucket code is 11,007 lines, including executable header logic.

| Area | Original | Retired | Left | Glue | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 37,438 | 9,986 | 6,988 | 16,974 |
| Simulation storage | 13,724 | 2,717 | 11,007 | 1,334 | 12,341 |
| NewGRF | 10,785 | 5,280 | 5,505 | 428 | 5,933 |
| Save/load | 13,388 | 536 | 12,852 | 585 | 13,437 |
| Script API | 14,908 | 878 | 14,030 | 453 | 14,483 |
| Network | 12,713 | 211 | 12,502 | 182 | 12,684 |
| Support | 21,617 | 982 | 20,635 | 513 | 21,148 |
| Graphics and viewport | 10,985 | 4 | 10,981 | 1 | 10,982 |
| GUI | 52,179 | 117 | 52,062 | 165 | 52,227 |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 522 | 54,626 | 163 | 54,789 |
| Upstream tests | 2,248 | 0 | 2,248 | 4,848 | 7,096 |
| Boundary layer | 0 | 0 | 0 | 18,152 | 18,152 |
| **Whole fork** | **284,355** | **53,049** | **231,306** | **33,814** | **265,120** |

- Six component slots are occupied: #603, #606, #607, #611, #612 and the
  selected engine-runtime owner #613. Engine implementation started from the
  integrated tip, with no unpublished dependency. Clear/tree is next after a
  slot frees. There is no new planning trigger or unplaced issue.
- #611's first fixing review is complete with two duplicate-binding repairs:
  canonical road town-error encoding and allow-list notifications. Root
  spot-checked both at `c047d643cc`; affected Cargo/build/native/company/road
  checks are running. Earlier implementation evidence passed both builds,
  Cargo4, 21 native tests, 40,320 financial cases and seven scenarios.
  Review-fixed metrics: 70 retired, 393 Rust, 181 tooling, +12 production
  glue and +159 fixture glue. Current-base update and Full follow those results.
- #612's station index and selected production consumers are implemented at
  `2d2f8b01da`. Static gates pass; affected builds/tests/scenarios are running.
  Root accepted 88 retired, 548 Rust, 241 tooling, -52 production glue and
  +29 fixture glue. The bounded native STL selection primitive retains 10
  production-glue lines for configured-platform traversal compatibility;
  Rust owns the tree. Actual crash/trigger-order evidence remains unfinished.
  Preserve the running pin; resolve its real base conflict and finish that
  evidence before first independent review. Native GUI k-d tree and retained
  GetTileArea selectors receive no new retirement credit.
- Every CI configuration builds Rust. #581 PR-B is draft #607. Its fresh
  fixing review and root final workflow/README inspection are complete.
  Prior evidence includes all 224 CTests, Cargo, generators and affected
  comparisons, exact 1,809-path sweep replay with eight explicit migrations,
  and four bounded preprocessing commands. The real conflict after #609 is
  being resolved with the integrated changes retained and a fresh full-source
  sweep/proof. These are prior-head results until that continuation finishes.
  Current-base bootstrap Full follows #606; CI_ON_DEMAND remains unchanged.
  #570 starts after PR-B integrates, with #566's drawing workload alongside.
- Instruction profile 44 is running at `16e0235d0d`; its results are pending.
  Profile 42 remains the last completed six-scenario comparison and projection
  record below. Its identities matched profile 40 with no 3% rise; all records
  exceed ceilings. #606 remains the next maintenance integration before #603
  and #607, targeting profile 36's Ir or lower on every scenario. Its latest
  measured head still misses TGP and all three Padhattan targets; six new
  counts are pending at `1ed7c0045b`. Ready ownership proceeds meanwhile.
  The quiet pass waits for the repair and an idle host. No wall anchors changed.
- The integration-40 platform cadence passed Windows x86/x64, Linux
  Clang/Dedicated, macOS arm64 Release/Debug, annotation gates and the bounded
  platform semantic subset. The next cadence is due after integration 45.
- Deep checks run only on the user's request. #503 retains the interrupted
  rotation's missing sanitized native-test-support archive build and bounded
  original clock preparation fix; it does not restart by itself.
- Automated takeover housekeeping removed 53.1 GB. Clean merged #596, #605,
  #608 and #609 worktrees and the completed #607 review checkout are removed.
  Active source checkouts, the immutable reference and retained profile/deep
  evidence remain protected.
- Original coverage remains 330 of 449 completed traces. #156 records concrete
  remaining observable-domain gaps; no new coverage recollection was selected.

| Scenario | Latest quiet anchor | Profile-36 projection | Profile-42 projection | Record with growth |
| --- | ---: | ---: | ---: | ---: |
| TGP | 1.1117x | 1.1117x | 2.0715x | 2.1505x |
| Grok | 1.3871x | 1.5766x | 2.4384x | 2.4358x |
| Opus | 1.3939x | 1.5612x | 2.3552x | 2.3563x |
| P1996 | 1.3366x | 1.3817x | 1.7674x | 1.7690x |
| P2000 | 1.4006x | 1.4284x | 1.7823x | 1.7834x |
| P2016 | 1.5229x | 1.5653x | 1.9813x | 1.9279x |

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite ([goal and phases](planning.md#goal-and-phases)). Each free
slot takes ready work from the earliest phase, and later phases fill the rest
(user decision, 2026-10-28). Position at code `06a644505a` in code lines from
`python3 tools/port-metrics.py --whole-fork`, with the change since the
previous navigation's inventory at `7105e24d49` in parentheses. Source areas
are an accounting classification: they do not move drawing or executable
header logic into a different phase.

| Area | Original | Retired | Left | Glue added | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 35,744 (+444) | 11,680 | 6,698 (-69) | 18,378 (-513) |
| Simulation storage | 13,724 | 2,643 (+344) | 11,081 | 1,312 (+2) | 12,393 (-342) |
| NewGRF | 10,785 | 5,218 | 5,567 | 423 | 5,990 |
| Save/load | 13,388 | 536 | 12,852 | 585 | 13,437 |
| Script API | 14,908 | 878 | 14,030 | 453 | 14,483 |
| Network | 12,713 | 211 | 12,502 | 182 | 12,684 |
| Support | 21,617 | 665 | 20,952 | 424 | 21,376 |
| Graphics and viewport | 10,985 | 4 (+3) | 10,981 | 1 | 10,982 (-3) |
| GUI | 52,179 | 117 | 52,062 | 165 | 52,227 |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 522 | 54,626 | 163 | 54,789 |
| Upstream tests | 2,248 | 0 | 2,248 | 4,683 (+192) | 6,931 (+192) |
| Boundary layer | 0 | 0 | 0 | 17,497 (+164) | 17,497 (+164) |
| **Whole fork** | **284,355** | **50,902 (17.90%, +791)** | **233,453** | **32,588 (+289)** | **266,041 (93.56%, -502)** |

The boundary has 1,297 Rust exports (-8) and 4,074 callback slots (+117).
Rust totals 117,779 code lines (+2,109) and tooling 47,674 (+1,024).
"Compiled C++" counts the Rust build. The Linux and Emscripten CI builds have
no Rust and still compile all 284,355 original lines; #581 ends that.

| Period (steering commits) | Days | Retired | Per day | Compiled C++ change |
| --- | ---: | ---: | ---: | ---: |
| Fork start to 10-09 | 7.0 | 24,228 | 3,470 | -7,428 |
| 10-09 to 10-21 | 11.3 | 7,987 | 705 | -1,485 |
| 10-21 to 10-27 | 6.4 | 17,321 | 2,720 | -8,995 |
| 10-27 to 10-28 (22nd to 23rd steering) | 1.0 | 575 | 550 | +92 |
| Previous navigation to now | 0.4 | 791 | 1,830 | -498 |
| **Whole history** | **26.3** | **50,902** | **1,930** | **-18,314** |

Days are calendar days, including pauses; root ran about half of the last two.
Glue took 64% of all retirement: compiled C++ fell 695 lines per day while
original C++ retired at 1,930. The 22nd steering's testing round explains the
10-28 dip: 5,459 tooling lines against 575 retired. Phase 2 storage ports
(#565, #570) remain the course's fix for the glue.

Why the course changed. Every port kept the original body under `#else` for
builds without Rust, and four CI configurations are such builds: Linux GCC
SDL2 and Emscripten in every required Full, and Linux Clang and Dedicated at
cadence. So no rule ever deleted an original line, the required Full spent two
builds on code the Rust game does not run, and phase 2 would have needed a
second, C++-owned definition of each record it moves, which #570's design
lacks. #581 now builds Rust on every configuration and deletes the fallback in
one mechanical sweep, before #570 starts. AGENTS.md also told the work order to
rank storage "beside leftover game logic", against the user's earliest-phase
rule; root followed the user's rule, and the text now says so. Generated C++
declarations count as compiled C++. Root read the course only at restart, so
each steering and navigation stopped it; it now rereads changed course
documents at each stocktake and planning round.

| Phase | Left | Exit criterion | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | 11,680 in the logic bucket, of which about 4,100 are drawing, description and viewport bodies (phase 5) | No non-drawing function body of the inventory or the command families compiles in the Rust build | Nearly done: #562, #561 and #572 in draft (#568, #579, #578), the queued owners in work order item 7; #559 excluded with its return condition |
| 2. Simulation storage | 11,081 in the storage bucket; credit only moved records and operations, and deleted glue | Every pool record and the map arrays are defined once in Rust and read in place by both languages. C++ calls Rust only for operations, and no per-field service or per-call-site reader remains for moved data | #565 in draft (#576). #581 lands before #570; #566 runs beside #570 |
| 3. Simulation data readers | 41,158 after excluding script and network GUI: save/load 12,852, script 13,099, network 9,640, NewGRF 5,567 | These run in Rust; the Squirrel VM may stay C++ until phase 6 | #573 to #575, #497 and #498 queued |
| 4. Support | 31,933 support and graphics lines; data tables (12,931) retire with their consumers | Not planned yet | Not planned until it is the next phase (user direction, 2026-10-28) |
| 5. Interface | 67,796 GUI and platform lines including script and network GUI, plus drawing in other buckets | Not planned yet | As above |
| 6. Vendored | 54,626 | Not planned yet | As above |

Upstream unit tests (2,248) retire with the code they test.

Root's first actions from this chart:
1. #581 at the top of the work order, in the three PRs its body lists (the
   24th steering review rescoped it): Linux builds Rust, Emscripten links Rust,
   then the sweep, which also moves every oracle that reads a fallback body onto
   the pinned reference. Drafts adopt the sweep by rerunning its recorded
   command over every `src/` file they touch ([review](workflow.md#review)).
2. #570 starts after the sweep integrates. #570 also makes
   `tools/port-metrics.py` count the C++ declarations it generates.
3. Phase 2 takes every slot that ready phase 1 work leaves free, ranked by code
   retired, crossings removed and glue deleted.
4. Before each stocktake and planning round, refresh changed guidance and apply
   its actions (`/start-development`, step 4).

## Twenty-fourth steering review (2026-10-29: per-entry map tolls, rebound callees and #581's hidden consumers)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: xhigh.

**What was audited.** Five independent read-only audits ran, with no builds or
games; the steering checked every claim acted on below against the source.
- Fidelity: #569 (integrated), #568 and #579, each compared line by line with
  the pinned original.
- #576's in-place map reads, together with the boundary growth.
- #581's plan, checked against the code.

**No reachable divergence.** Random draw counts and order, widths, wrapping
and side-effect order match in all four ports. The findings are boundary and
process faults.

**1. The in-place map rule pays a toll on every entry.**
- Every entry fetches the map descriptor through two or three C++ calls
  (`World::new`, `with_context`, and a fetch slot per table).
  - #576 left pixel Z at two crossings per call and raised rail, station and
    tunnel status from one crossing to two.
  - A rail pathfinder follow went from about 2 crossings to about 15 in #569.
- Converted helpers fall back to the C++ accessor, so its service can never be
  deleted. That is why #576 ended at +13 net glue (production -24, fixtures
  +37) against #565's -300 to -800.
- Three descriptor idioms exist, and each draft adds its own fetch slot.
- Root cause: world-state.md said "acquire at an entry" and "remaining unaudited
  callers share the canonical native fallback". Root followed it exactly.

**2. Rebound callees and helpers added beside the copies they replace, in
every port.** All four PRs bind the same C++ callee into a second table:
#569 three times, #568 about ten, #579 four or more, and #578 too. #569 and
#568 also add helpers beside the services they replace. Reviews passed all
of it; #568's review recorded "wrapper-callee audit: reused". The AGENTS.md
reviewer grep is not holding, so the check becomes a tool (#582).

**3. #581 as written breaks default CI.**
- `src/tests/roadvehicle.cpp` and `road_yapf.cpp`, `tools/probe.py`'s
  `candidate-cpp` label and two comparisons read the fallback bodies.
- About 30 `tools/` harness files need `WITH_RUST` defined.
- Emscripten may need emsdk 4 with WebAssembly exceptions.
- The retired count moves by about -770 through diff matching.

**4. Housekeeping never removes a finished T3 worktree on this host.**
`/proc/1/cwd` is unreadable, and the tool treats that as "keep". There are 47
worktrees using 82 GB, against 42 and 56 GB when #564 was asked to clean them.
Root's dry run showed the refusal 33 times, and root never raised it.

**5. Phase 1 drafts ported support code whose data stays in C++.**
- #568's road-stop tile data costs 9 per-field services to retire about 22
  lines.
- #579's face slice costs about 100 glue lines to retire about 40.

AGENTS.md already classes both as support code. The PR comments say what
changes.

Apply these corrections through the work order below:

1. **Finish the five drafts with the audit fixes** (PR comments of 2026-10-29).
   - #568: fix duplicate wrappers, keep one `TileAddWrap` and
     `DistanceFromEdge` definition, return road-stop tile data to one C++
     operation, use the `IsTileFlat` callee, and call the station tile loop
     directly. One fresh scoped review covers these with its conflict
     resolution.
   - #576 adopts `map_func::World` at its base update.
   - #579's fresh review fixes its bounce, rebound accessors and livery
     offsets.
   - #578's review checks the same pattern.
   - Coverage gaps went to #156.
2. **#581 in three PRs** (issue body rescoped).
   - PR-A, Linux builds Rust, takes the first free slot.
   - PR-E, the Emscripten spike, takes the next. If it needs the emsdk 4 move,
     that move is the fix and keeps #581's rank.
   - PR-B, the sweep, follows PR-A, PR-E and the reviewed #568, #576 and #580.
     It moves every oracle that reads a fallback body onto the pinned
     reference, sweeps `src/` only, and adds a quick-CI check against
     `WITH_RUST` in `src/`.
   - #570 still starts after PR-B.
3. **The map view costs a load, not a call** ([map access](porting.md#map-access),
   world-state.md "Shared map view", #565 body).
   - After #576 integrates, root's own small PR makes `map_func::World` the one
     descriptor, reached by a load from the canonical C++ statics. A failed
     precondition calls the native assertion, as `map_value` does.
   - Drafts adopt it at their base update.
   - The next #565 PR then deletes the fallbacks and converts the remaining
     rail, crossing, depot, stop, water and tunnel leaves. Planning range:
     production net glue -300 to -600.
4. **#582 tooling batch.**
   - Quick CI rejects a callee rebound into a second service table, with a
     baseline that only shrinks.
   - Housekeeping skips other users' processes.
   - Then run `housekeeping.py --apply` and remove the finished worktrees.
5. **#199 integrated-fault batch** (checklist on #199), after #582. It covers:
   - #569's service copies, duplicate fields and orphan declaration;
   - #577's single-use carriers;
   - the `CargoFlow` bounce.
6. **Speed.** Run the due profile after integration 30 first. If a projection
   rose more than 3%, attribution follows [speed policy](speed.md#policy), and correction 3 is the
   first candidate.

AGENTS.md changes:
- The map bullet now requires a load, not a call, `World` as the one type, and
  a native assertion with no C++ fallback.
- The shared-accessor bullet names #582.
- Step 3 reruns the sweep command over every `src/` file a draft touches. Drafts
  #578 and #579 add guards in files without them on base, so a conflict-only
  rerun would keep their fallbacks.

No user direction is reversed. The restart prompt is unchanged.

| Twenty-third correction | Verdict and cost/result |
| --- | --- |
| 1. Fix #554 before integration | Applied: a boundary repair, two scoped reviews, root's orphan removal and two Fulls (one probe-link failure). It integrated as #577 with 429 retired and +319 net glue. It still added the 10 single-use carriers now on #199. |
| 2. Leftover owners concurrently | Applied. #569 integrated at 362 retired and -30 net glue, the best ratio so far. #568 is reviewed and #579 implemented. #579's 1.2 retired per net glue line is below the planning audit's 2 to 5. |
| 3. Storage design | Applied: #570's design defines each record once in Rust with generated declarations. Its 0.86% instruction credit is a retained-profile model, not a measurement. Implementation waits for #581's sweep (navigation). |
| 4. Speed: no pass, no small-candidate research | Applied: profiles 26 and 28 ran in the background, and no pass or attribution research ran. |
| 5. Exclude #559 | Applied. Its return conditions still hold. |
| 6. Maintenance (#564, #168) | #564 integrated as #567 (1,013 tooling lines). Its orphan check works: it caught #554's three orphan exports in quick CI before review. Its T3 worktree cleanup does not work on this host (finding 4). The #168 batch is #580, in draft. |
| 7. Testing is maintenance | Applied: #558 integrated (100 lines), and #503's continuation stays last. |
| 8. Phase-3 planning | Applied: #573 to #575 are queued with output evidence. |
| 9. Cadence re-dispatch | Applied: it passed on every platform before integration 26. |

| Navigation change since the twenty-third review | Verdict and next observation |
| --- | --- |
| In-place data boundary; per-call-site readers are a design fault; map in place everywhere (`8fb4499105`) | Root's session ran under it. #569 and #576 read the map in place correctly, but with per-entry tolls and C++ fallbacks (correction 3). #577 still added 10 single-use carriers (on #199). #570's design follows the shared-layout rule. |
| #581 first, Rust as the only build, root rereads the course (`6c4d9ac528`) | No root session yet. #581's plan needed the rescope above. Watch whether root rereads changed course documents at each stocktake, and PR-E's outcome. |

New since this review, for the next steer to judge:
- Whether #565 item 1 removes the per-entry crossings in the next profile, and
  whether #565's next PR reaches negative production glue.
- Whether #582's check ends rebinding findings at review.
- Whether PR-E links on emsdk 3.1.57 or needs the emsdk 4 move.
- Whether the remaining phase-1 owners stop porting support code whose data stays
  in C++.

## Earlier steering, still in force

Ownership, original behavior, process scope, named callees, typed shared services
and exact random-draw order come first. Preserve default semantic evidence,
record gaps on #156 and keep retirement distinct from glue. No hot allocation,
opcode dispatch, whole-record copies or owner bounces; no C++ frame is kept only
for attribution. Descriptor reads stay within `docs/design/world-state.md`.
Ready owners proceed beside speed work; only timed collection holds builds and
games. Ceiling and band decisions use the three-pass record; the projection
factor and speed thresholds remain provisional, and 0.1% to 0.3% speed
candidates get no attribution research. Testing is maintenance: scenarios
arrive with the ports that need them. Six branches, batched maintenance,
fixing reviews, strict exact-combination Full, bounded rollback and
action-based wind-down remain. A historical mention is not placement; CI-found
behavior fixes get a scoped review; deep checks run only on the user's request;
active speed dependencies are preserved. #559 and the parked NewGRF corpus stay
excluded with the return conditions in the work order. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence.

## Recent completed work

Rows cover the period since the twenty-third steering review; earlier rows are
in their PRs and Git history. Metrics are `tools/port-metrics.py` code lines:
Rust / tooling / original C++ retired / net glue.

| Integration | Issues / PR | Merge commit | Metrics |
| --- | --- | --- | --- |
| 26 | #156 differing town-buildable road input / #558 | `c625d9d543` | 0 / 100 / 0 / 0 |
| 27 | #564 metrics, orphan check, cleanup and speed records / #567 | `d8704269c7` | 0 / 1,013 / 0 / 0 |
| 28 | #199 canonical live map helpers / #571 | `7105e24d49` | 72 / 0 / 0 / +4 |
| 29 | #551 session lifecycle (#554) and #503 deep rotation (#557) / #577 | `7473e63624` | 1,118 / 919 / 429 / +319 |
| 30 | #560 track follower and platform scans / #569 | `06a644505a` | 1,234 / 205 / 362 / -30 |
| 31 | #572 goals, story and league (#578), #168 clock batch (#580), #589 / #588 | `44cb5f49f5` | 1,133 / 376 / 388 / +264 |
| 32 | #561 company identity, faces, colours and liveries / #579 | `0de459cbe1` | 995 / 205 / 373 / +315 |
| 33 | #581 PR-A, Rust by default and on Linux CI / #583 | `d531622d32` | 0 / 0 / 0 / 0 (CI and CMake) |
| 34 | #582 rebound-callee gate and housekeeping cwd fix / #590 | `29b65dd186` | 0 / 964 / 0 / 0 |
| 35 | #581 PR-E, Emscripten links Rust on emsdk 4.0.23 / #591 | `69f02851be` | 0 / 21 / 0 / 0 |
| 36 | Clang missing-initializer fixes in `src/tests` / #595 | `063e7b28f1` | 0 / 0 / 0 / +18 |
| 37 | #584 heightmap input and town-data import / #593 | `b74888c4bc` | 554 / 389 / 278 / +193 |
| 38 | #565 movement terrain and water entries in place / #576 | `e48846d8fd` | 546 / 61 / 0 / +15 |
| 39 | #562 station naming, tile loop and lifecycle / #568 | `4733594d75` | 983 / 248 / 295 / +70 |
| 40 | #587 simulation-setting transitions and commands / #594 | `b01e7fd5c8` | 726 / 180 / 322 / +167 |
| 41 | #585 NewGRF town-name state, generation and ActionF / #596 | `a2429cda1f` | 393 / 209 / 190 / +37 |
| 42 | #586 sign commands and depot naming / #605 | `73c5a4797c` | 230 / 260 / 76 / +128 |
| 43 | #600 town spatial index and policy / #608 | `5345e29939` | 699 / 112 / 120 / -16 |
| 44 | #601 industry servicing and policy / #609 | `16e0235d0d` | 309 / 158 / 105 / +37 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **Finish the remaining drafts in readiness order, with the steering fixes.**
   #568, #594, #596, #605, #608 and #609 integrated as integrations 39 to 44.
   #611's first full fixing review is running; #612 follows its completed
   implementation and review.
   #606 must be the next maintenance integration before #603. Each next action
   is in the checkpoint. No reviewed owner gains other scope; a fix the audit
   found is not new scope.
2. **#581 Rust on every CI configuration and retirement of the portable
   fallback.**
   - PR-A integrated as #583 (integration 33).
   - PR-E integrated as #591 (integration 35), using emsdk 4.0.23 and
     `-fwasm-exceptions`.
   - PR-B, the sweep plus moving the fallback-body oracles onto the pinned
     reference, follows PR-A, PR-E, #568, #576 and #580.
   - PR-B now takes the slot freed by #568, whose integration completes its prerequisites.
     Keep dual-build harness guards and move fallback-reading oracles to the
     pinned reference. Surviving drafts rerun the recorded sweep over every
     touched `src/` file when updating from base. Expected size and evidence
     are in the issue.
3. **#592 recent T3 worktree protection and service-table parser repair**,
   both in #606. The original 20-40 tooling-line estimate covered the
   worktree item only. #582 integrated as #590 (integration 34). Until #592
   integrates, root locks running Codex worktrees before `--apply`.
4. **#565 map view by a load with #275's fix-forward batch, then the remaining
   conversions.**
   - Item 1, once #576 integrates: `map_func::World` is the one descriptor,
     reached by a load, and failed preconditions call the native assertion. It
     goes in #606 with #275's two fix-forward items (the movement table by
     a load; the road kind once per road search), #592 and the profile-38
     helper regression repair. #606 must be the next maintenance PR to
     integrate, before #603; ready ownership PRs may proceed meanwhile.
   - Drafts adopt it at their base update.
   - Item 2 deletes the C++ fallbacks and converts the remaining rail, crossing,
     depot, stop, water and tunnel leaves and `tile_slope_z`.
   - Planning range: production net glue -300 to -600.
5. **#199 integrated-fault batch**, now that #582's gate is integrated: #569's service copies, duplicate
   fields and orphan declaration, #577's single-use carriers, and the
   `CargoFlow` bounce once #568 integrates. Finish #603's reviewed scope after
   #606 without expanding it; take remaining repairs in the next bounded batch.
6. **Queued phase-1 owners under #3**, ranked by plan-phase1-rest on
   2026-10-31 at `11c285f939` (remaining code lines, estimated net glue):
   1. #587 simulation-setting transitions: integrated as #594 (integration 40);
   2. #584 world-input generation, heightmap input and town-data import:
      integrated as #593 (integration 37);
   3. #585 NewGRF town-name storage, generation and ActionF: integrated as
      #596 (integration 41), with 190 retired and +37 net glue;
   4. #586 sign and depot naming: integrated as #605 (integration 42), with
      76 retired, +128 net glue and 260 tooling; root accepted the necessary
      native-operation and evidence cost;
   5. #600 town spatial index and town policy: integrated as #608 (integration
      43, `5345e29939`), with 120 retired, -26 production glue, +10 fixture
      glue, 699 Rust and 112 tooling. Its shared index is available to #602;
      the retained native GUI template receives no retirement credit;
   6. #601 industry remainder in #609. Root accepted the necessary servicing/
      news, selection, history/reload and price-width evidence cost. Final
      fixing-review checkpoint: 105 retired, 309 Rust, +20 production glue,
      +17 fixture glue and 158 tooling. Integrated as #609 (integration 44,
      `16e0235d0d`) after current-base Full. Its services remain separate from
      the town index;
   7. #610 company control is selected in #611. Its seven functions contain
      89 surviving original code lines plus two fork-glue lines. Planning
      ranges are 55-75 retired, +30 to +80 production glue, +50 to +100 fixture
      glue, 60-120 tooling and 250-450 Rust. Reuse #594's in-place vehicle
      defaults; neither #600 nor #604 is a prerequisite. Root accepted the
      first checkpoint: 70 retired and +33 production glue. Completed source/
      fixture authoring and caller repairs report 70 retired, +27 production
      glue, +159 fixture glue, 181 tooling and 379 Rust. Necessary native
      error/context evidence cost is accepted. All affected checks pass;
      the first fresh full fixing review is running;
   8. #602 station spatial queries, road-stop selection and rail trigger areas
      is selected in #612, ahead of engine runtime after fresh planning.
      Both prerequisites are integrated. Its 80-line inventory supports an
      estimated 55-75 retirement, +10 to +70 production glue, +80 to +140
      fixture glue, 100-180 tooling and 350-650 Rust. Transfer all radius
      consumers and `Station::MoveSign` mutations. Shared index median
      selection may change unsorted visitation; repair necessary support and
      compare consumer observations, not sorted membership. Root accepted the
      first partial checkpoint: 36 retired, -28 production glue, -2 fixture
      glue, zero tooling and 140 Rust. A bounded stateless native STL selection
      primitive is approved for platform permutation fidelity; Rust keeps
      tree state/build orchestration/mutations/traversal. Count its retained
      native support and final glue explicitly. Keep the two native
      `GetTileArea` selectors as whole operations with zero retirement credit;
   9. #613 engine runtime is selected in the slot freed by #609. Six methods
      total 78 original code lines, about 60 retireable with native facades.
      The +40 to +80 production glue range remains provisional. Reuse the
      canonical engine/property/price interfaces and delete displaced direct
      and compound callbacks. Presentation and pool-coupled constructors
      receive no simulation-retirement credit;
   10. clear-ground clearing (22) and editor tree-group mutation (36), one
      issued owner; then snow line and map-edge water (46 executable original
      lines, about 51 including state/definitions), road-bit cleanup (45,
      including `IsPossibleCrossing`), link-graph job lifecycle (37 original
      lines in the six principal spans, including 12 pool-coupled constructor
      and destructor lines; size retained operations and executable headers
      with flow-publication cleanup), and the elrail setting change plus
      service clamp (29 + 6, #594 prerequisite satisfied). Root issues each
      when selected and coordinates shared map helpers with #606.

   After #570 these outrank smaller remaining owners, one at a time in the
   shared vehicle files: #598 vehicle routing
   decisions (263, -20 to +60), then #599 vehicle base lifecycle (245-line
   inventory, +60 to +140 provisional glue). The latter includes 13 constructor
   and 16 destructor lines that cannot receive wholesale phase-1 retirement
   credit; remeasure its boundary after #570. The airport automata (83,
   negative net glue) follow #570's declaration generator. Pool-coupled bodies,
   such as the engine constructor and station destructors, move with their
   pool in item 8.

   Do not fall back to utility helpers. Port support code only where the
   component needs it, and keep an operation on data that stays in C++ as one
   call, not a service per field.

   The planner found two inventory items outside phase 1: process startup in
   `openttd.cpp` (468 lines, phase 4) and drawing support (about 515 lines:
   the rail, road and canal NewGRF resolvers, `UpdateDeltaXY`, elrail wire
   geometry, `IsWateredTile` and the bridge pillar helpers, phase 5). The next
   navigation reclassifies them in the Phases table.
7. **#566 drawing workload and #570 first shared vehicle records**, after
   #581's PR-B.
   - #570 follows the accepted world-state design. It deletes the replaced
     readers and the direct Tick bounces, and makes `tools/port-metrics.py`
     count the C++ declarations it generates.
   - Estimated net glue -500 to -1,100. Its 0.86% instruction credit is a
     model.
   - Start #566 beside it and obtain its reference and candidate pixels and
     drawing Ir before storage integrates (150-250 tooling lines).
   - #570 unblocks #598, #599 and the airport automata.
8. **Queued later phase-2 pools.** The order is:
   1. the remaining vehicle pool;
   2. stations and road stops (1,604 storage lines, about 600 callback slots);
   3. towns and industries (1,295, about 344);
   4. companies, groups, engines and the smaller pools (952, about 286);
   5. orders and order lists (390, about 111), earlier if the vehicle pool
      design needs order lists first;
   6. map allocation and definitions (3,988).

   Move the coupled save adapters with each record owner, generate native
   declarations, verify the layout, and delete the displaced readers and
   services. Each pool gets an issue and a design section when selected.
9. **Phase-3 reader queue.** #573 codec and chunk execution, #574 game network
   protocol and queues, #575 script host, commands and events. Each issue
   carries explicit unsaved-output evidence. Then complete NewGRF text,
   config/Action14 and definition ownership: #497 with its engine and feature
   binding payloads, #498 with its ObjectSpec payloads. Plan nothing for phases
   4-6 and no Squirrel port.
10. **#604 shared settings/state access**, under #199 after the map/CargoFlow
    repairs and after #594, #581 PR-B and #570's declaration generator. It
    does not block #570. Preserve configured generation dimensions, client
    and timer inputs, and exception-safe company continuation state; a
    `GameSettings` overlay cannot replace all seven snapshots. The issue
    carries the revised boundary and cost limits. Then **#168/#199/#500
    completed-job flow publication; #501 probe typing**.
    Zero retirement; estimated -30 to +80 net glue.
11. **#503 coverage continuation.** The bounded original-only clock-preparation
    fix and sanitized native-test-support build fix, then 119 unfinished
    traces with provenance checks. No recollection just to fill a slot.

Recurring:
- Profile the merged tip after integration 44, or when #606 integrates,
  using the pinned `LD_LIBRARY_PATH` from
  docs/rust-migration.md.
- Deep checks run only on the user's request (user decision, 2026-10-31).
- The integration-40 platform cadence passed; the next is due after
  integration 45. Fix a red cadence before further integrations.
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- Root checks actual issue placement and exclusion conditions at stocktake.

Excluded, with return conditions checked by this review:
- #559 command invocation lifecycle returns when one of these happens:
  - a selected game owner needs Rust-owned top-level `Do`/`Post` admission;
  - a #275 profile ranks top-level dispatch among the largest net costs;
  - the queued ownership candidates run out, or the item-6 ready queue
    empties. `command.cpp` (141 lines) is in phase 1's exit inventory, so #559
    returns before phase 1 exits.
- #507's NewGRF corpus (#525, #526) stays parked by the user (2026-10-28)
  until the user reselects it. #525's branch stays on origin.
- #69's remaining tile-list/support work returns when a selected simulation
  component needs it or the item-6 ready queue empties. `tilearea.cpp` (172
  lines) is in phase 1's exit inventory, so it returns before phase 1 exits.
- #64 and #68 become maintained Rust crates that reproduce the game's
  observable cryptography. They return when phase 3's network ports reach
  authentication, or with phase 6, whichever comes first.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until
  near-full reproduction under #513's original-default setting policy. #513
  needs implementation when the first such improvement is accepted.

## Resume checkpoint (2026-10-31, integration 44)

Codex root has taken over from Claude Code root at `9cf6b7898e`.
Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh.
#568, #594, #596, #605, #608 and #609 integrated as integrations 39 to 44,
each after current-base exact-head Full. Latest code is `16e0235d0d`; #601
is now closed. Drafts integrate one at a time because service files overlap.
#611 passed affected implementation validation and its first fixing review;
root spot-checked both review repairs and their affected validation is running.
#612's station/index source is implemented; native/scenario checks are running
at the fixed pin. Its real base conflict and unfinished crash/trigger evidence
are next before first review. #606 remains the next maintenance integration before #603 and #607.
#613 engine runtime takes the slot freed by #609; six component slots remain
occupied. Integration-44 whole-fork metrics are recorded above. Instruction
profile 44 is running; profile 42 remains the last completed speed record.

| Draft | Pushed head | State and next action | Worktree |
| --- | --- | --- | --- |
| #581 / #607 | `6600d951ab` | First fixing review and root final workflow/README spot-checks complete; no remaining findings. Source/evidence pin remains `9f2986d588`. Real base-conflict update and full-source sweep/proof running after #609. Bootstrap Full remains after #606; CI_ON_DEMAND unchanged. Retain -825 retirement/-826 net-glue reclassification, 136 tooling. Completed review checkout removed. | `t3-4076556a` |
| #199 / #603 | `70215c21cb` | Reviewed four native-assertion fixes; root spot-checked. Current-base update and Full after #606, without expanding the accepted scope. Item 4 is #604. | `t3-120ef117` |
| #565 / #275 / #592 / #606 | `1ed7c0045b` | Latest measured six scenarios are semantically equal, but TGP and all three Padhattan targets miss. Shared clear-map preconditions and fixture caller repairs are pushed; Cargo4 passed, native validation/six counts pending. Fresh fixing review and current-base Full follow all-six acceptance; next maintenance integration. | `t3-d6cdd816` (implementation); `t3-431c84b4` (locked source) |
| #610 / #611 | `c047d643cc` | First fixing review complete; two canonical-binding repairs, root spot-checked. Affected validation runs in `.local/review611-affected-c047d643cc.log`; earlier `aa0bbf7777` implementation evidence passed. Metrics: 70 retired, +12 production/+159 fixture glue, 181 tooling, 393 Rust. Current-base update/Full after the fix results. | `t3-4b253100` (implementation); `t3-f50302f3` (root fix validation) |
| #602 / #612 | `2d2f8b01da` | Index and all selected direct production consumers implemented; YAPF dependency fix preserves cases/expectations. Static gates pass; pinned affected validation running in `.local/station602/validate-checkpoint2.log`. Keep pin fixed, then resolve real base conflict and finish crash/trigger-order evidence before first review. Cost accepted: 88 retired, -52 production/+29 fixture glue, 241 tooling, 548 Rust. | `t3-56a3afc8` |
| #613 engine runtime | Implementation active | Started from integrated `16e0235d0d` and docs `499724da46`, with no unpublished service dependency. Six runtime decisions and compound callers; first concrete cost checkpoint before long checks. | T3 prepared checkout; first checkpoint pending |

| Running or due work | State / next action |
| --- | --- |
| Branch limit | Six component slots (#603, #606, #607, #611, #612 and #613); start no seventh. |
| Planning | Fresh Astra ranked now-ready #602 ahead of engine runtime; both are now selected. Clear/tree follows, then the existing remainder. No new planning trigger. |
| Platform cadence | Integration 40 passed every required configuration and the bounded platform semantic subset. Next cadence after integration 45. |
| Deep checks | On the user's request only (user decision, 2026-10-31). If one is requested, use `.local/worktrees/deep-after30-569` (locked). Move it with `git checkout --detach origin/rust-migration` and relock it. Then run `python3 tools/migration.py build --jobs 2`, then `cmake --build build-rust-sanitized --target openttd_rust_native_test_support_build` until #503's fix lands, then `python3 tools/migration.py deep` with the requested `--member` options. Launch outside the session's process group, for example as a `systemd-run --user` unit. |
| Instruction profile | Profile 42 passed all six comparisons; no 3% rise from 40, and all records remain above ceilings. Profiles 36, 38, 40 and 42 remain in their locked worktrees. Profile 44 is running in `.local/worktrees/profile-after44-609`, pinned to `16e0235d0d`; log `/tmp/openttd-profile-after44.log`. Profile again when #606 integrates. |
| Quiet speed pass | Waits for the profile-38 regression repair and an idle host. |
| Maintenance | #606 is revalidating with four Ir targets still unmet. #607's full source/evidence review and root workflow spot-check are complete; bootstrap Full waits for #606. #606 remains the next maintenance integration. #604 remains under #199 after map/CargoFlow repairs, PR-B and #570's generator, before typing/coverage. |
| Issues | #601 closed and #613 issued/selected: 37 open, zero unplaced, three excluded with recorded return conditions. |
| Housekeeping | Automated takeover cleanup removed 53.1 GB. Merged #596/#605/#608/#609 worktrees and local branches, and the completed #607 review checkout, are removed. Active Codex component worktrees are locked until #592 integrates. |

Keep the pinned reference unchanged. Preserve unfinished checkouts and existing
profile and deep evidence.
