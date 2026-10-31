# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-31, paused after integration 47)

Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh.

- Paused at the user's request. Current code is `2a89fb023f`; the shared branch
  had only documentation after that integration at wind-down (`72f92b2569`).
  Integrations 39 to 47 are #568, #594, #596, #605, #608, #609, #611, #606 and
  #614. There were no rollbacks. Their recorded component totals are 1,241
  original C++ lines retired, 4,573 Rust lines added, 1,858 tooling lines added
  and +682 net C++ glue. Added-line totals are not whole-source inventories.
- #614 passed full fixing review, root spot-checks and exact-head Full
  `41218950045` at `4a3db40ea2` before merging as 47 (`2a89fb023f`). The validated
  and merged trees are identical. Its final cost is 63 retired, 316 Rust,
  352 tooling and -19 production/+95 fixture glue. #613 is closed. No eligible
  main-branch regression or recovery was identified; no recovery was attempted.
- The latest measured whole-fork stocktake is **integration 46**, code
  `a60ef2e616`: 53,119 of 284,355 original lines retired (18.68%); 33,997 glue;
  compiled C++ 265,233 (93.28%); Rust 124,039 and tooling 50,975 code lines.
  Simulation-source retirement is 40,225 of 61,148 (65.78%). The boundary has
  1,370 exports and 4,206 callback slots. The area table below retains that
  measured pin. Integration 47's component delta above is available, but no
  new whole-fork, coverage or speed measurement was launched for wind-down.
  Executable headers and drawing retain their source classification; these
  counts do not establish phase completion.

| Area | Original | Retired | Left | Glue | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 37,508 | 9,916 | 6,973 | 16,889 |
| Simulation storage | 13,724 | 2,717 | 11,007 | 1,335 | 12,342 |
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
| Upstream tests | 2,248 | 0 | 2,248 | 5,023 | 7,271 |
| Boundary layer | 0 | 0 | 0 | 18,174 | 18,174 |
| **Whole fork** | **284,355** | **53,119** | **231,236** | **33,997** | **265,233** |

- Six drafts remain. #603's full/scoped reviews and f273 combination checks
  pass (four Cargo gates, 16 native tests, 17 cases/153 snapshots); final-base
  adoption/Full is unstarted. #612's reviews and A5 combination checks pass;
  its actual-base update/Full is unstarted and it now conflicts with base.
  #607 saved `8bb6678cea` and stopped; new-base sweep/replay/deviation proof
  and validation remain incomplete. #619 saved incomplete implementation in
  draft #620 at `1afd6fe028`; compilation, runtime, cost and review are incomplete.
- #616's exact-head Full **failed** at `b4021e34a1`, run `41221404067`.
  Native/build and Linux GCC SDL2 fail at `src/tests/town_growth.cpp:20`
  (`CHECK(result)` is false); Windows x64 also fails. This is a completed
  failed result, not a stale conflict alert. The candidate is unmerged; no
  repair, retry or new CI request was made during wind-down. The already-started
  standard recount completed: 52 retired, 131 Rust, 196 tooling and
  -1 production/+18 fixture glue. Local/review passes do not satisfy its Full gate.
- #618's existing validation at `45e1350c79` is now recorded: Cargo/native/ABI
  verification passes with 97 reference and 231 candidate tests; ten scenarios
  pass across 14 snapshots, with complete exit/moment comparisons. Its previously started
  standard recount also completed: 34 retired, 159 Rust, 200 tooling,
  +10 production/+78 fixture glue. Nothing was rerun for wind-down. The next
  action is a fresh full fixing review, followed by actual-base Full.
- `plan-after47` completed read-only. Its ranking and corrected inventories
  are recorded below as **unapplied resume input**. Nothing was selected,
  placed or started from that report. Its beginning/end placement checks were
  37 open, zero unplaced, three excluded and zero exclusions without an issue.
- Stocktake/profile remain due after **48**, platform cadence at **50** and
  deep checks only on request. Profile 46 has six equal semantic comparisons,
  compatible identities and all unchanged repair targets met. Carried records
  are below their bands, so no quiet pass is currently triggered; P2016's
  single-anchor projection does not replace its carried record. No wall
  anchors or medians changed. These are the retained profile-46 values:

| Scenario | Candidate Ir, profile 46 | Projection | Carried three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 2,715,619,270 | 1.111722x | 1.154106x |
| Grok | 26,869,118,791 | 1.387147x | 1.385653x |
| Opus | 28,273,410,089 | 1.393949x | 1.394556x |
| P1996 | 4,537,889,943 | 1.336551x | 1.337764x |
| P2000 | 7,385,372,083 | 1.400577x | 1.401446x |
| P2016 | 13,361,928,672 | 1.522911x | 1.481910x |

- Integration-45 platform cadence `41204109835` passed Windows x86/x64,
  Linux Clang/Dedicated and macOS arm64 Release/Debug plus annotations. Both
  semantic artifacts pass five cases/126 snapshots with matching exits/moments.
  #503 retains its bounded clock-preparation and sanitized native-test-support
  build defects and coverage continuation; no deep or coverage restart is authorized.
- Original coverage retains 330 of 449 traces, not a completed coverage result.
  #156 holds concrete observation gaps. Wind-down housekeeping dry run/apply
  removed **0 B**. Unfinished source, open-PR review checkouts, the dirty cancelled
  map branch, old #603 checkout, native fixture evidence, reference, profiles
  and deep evidence remain preserved. This root's PR watches were switched off for pause.

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite ([goal and phases](planning.md#goal-and-phases)). Each free
slot takes ready work from the earliest phase, and later phases fill the rest
(user decision, 2026-10-28). Position at code `2a89fb023f` (integration 47) in
code lines from `python3 tools/port-metrics.py --whole-fork`, with the change
since the previous navigation's inventory at `06a644505a` in parentheses.
Source areas are an accounting classification: they do not move drawing or
executable header logic into a different phase.

| Area | Original | Retired | Left | Glue added | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 37,571 (+1,827) | 9,853 | 6,974 (+276) | 16,827 (-1,551) |
| Simulation storage | 13,724 | 2,717 (+74) | 11,007 | 1,335 (+23) | 12,342 (-51) |
| NewGRF | 10,785 | 5,280 (+62) | 5,505 | 428 (+5) | 5,933 (-57) |
| Save/load | 13,388 | 536 | 12,852 | 585 | 13,437 |
| Script API | 14,908 | 878 | 14,030 | 453 | 14,483 |
| Network | 12,713 | 211 | 12,502 | 182 | 12,684 |
| Support | 21,617 | 982 (+317) | 20,635 | 513 (+89) | 21,148 (-228) |
| Graphics and viewport | 10,985 | 4 | 10,981 | 1 | 10,982 |
| GUI | 52,179 | 117 | 52,062 | 165 | 52,227 |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 522 | 54,626 | 163 | 54,789 |
| Upstream tests | 2,248 | 0 | 2,248 | 5,118 (+435) | 7,366 (+435) |
| Boundary layer | 0 | 0 | 0 | 18,154 (+657) | 18,154 (+657) |
| **Whole fork** | **284,355** | **53,182 (18.70%, +2,280)** | **231,173** | **34,073 (+1,485)** | **265,246 (93.28%, -795)** |

The boundary has 1,377 Rust exports (+80) and 4,206 callback slots (+132).
Rust totals 124,225 code lines (+6,446) and tooling 51,310 (+3,636).
Simulation-source retirement is 40,288 of 61,148 (65.89%).

| Period (steering and navigation commits) | Days | Retired | Per day | Compiled C++ change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start to 10-09 | 7.0 | 23,244 | 3,330 | -10,321 | -1,480 |
| 10-09 to 10-21 | 12.1 | 8,971 | 740 | +1,408 | +120 |
| 10-21 to 10-27 | 6.4 | 17,321 | 2,720 | -8,995 | -1,410 |
| 10-27 to the first navigation (10-28) | 1.2 | 575 | 470 | +92 | +80 |
| First to second navigation | 0.45 | 791 | 1,760 | -498 | -1,110 |
| Second navigation to now | 2.4 | 2,280 | 950 | -795 | -330 |
| **Whole history** | **29.6** | **53,182** | **1,800** | **-19,109** | **-650** |

Days are calendar days, including pauses; root ran for about one day of the
last 2.4. Three things set the last period's rate:
- Retirement per integration fell from about 390 lines (integrations 29 to 32)
  to about 140 (39 to 47), and the four drafts now open hold 34 to 88 each.
- Glue rose 65% as fast as retirement: the boundary layer grew 657 lines and
  native test fixtures 435. So compiled C++ fell 795 lines while 2,280 original
  lines retired. Phase 2's storage ports remain the fix, and none has started.
- Tooling grew 1.6 lines per retired line.

Why the course changed:
1. Phase 1's exit criterion and the planning trigger in `/start-development`
   (fewer than three unstarted owners in the earliest unfinished phase) kept
   every slot on phase 1 after its large owners ran out. Of the 9,853 lines left
   in its bucket, about 4,700 are drawing and description (phase 5) and 510 are
   startup (phase 4). About 1,700 are three- and four-line facade shells
   (signature and braces) that retire with their C++ callers. About 2,250 are
   other non-drawing bodies in about 200 functions across 51 files; the rest
   are file-scope declarations (about 350), `command.cpp` (151, #559) and
   `tilearea.cpp` (172, #69). #621 makes this inventory a tool. Planning cut
   owners of 11 to 88 lines from that tail. Meanwhile phase-3 owners sized at
   1,000 to 2,900 retired lines each (#573, #574, #575) have been queued since
   10-29. Every owner pays
   the same issue, review and exact-head Full cost. Phase 1 now closes with its
   open drafts and the two excluded items whose return condition is its exit.
   Its remaining bodies move with the phase-2 owner of their files.
2. #581 PR-B (#607) was the previous navigation's first action and is phase 2's
   only prerequisite, yet it is still a draft. Integration in readiness order
   let eight integrations pass it on 10-31. Each moved base and forced a new
   sweep and deviation proof, and the checkpoint then queued it behind #603 as
   well. A sweep across `src/` now integrates before other `src/` changes
   ([integration](workflow.md#integration)).
3. #566's drawing evidence gates #570's integration, but the work order started
   it only after PR-B, which it does not need. It starts now.

| Phase | Left | Exit criterion | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | About 2,250 lines of non-drawing bodies in the logic bucket, plus #559's command invocation (`command.cpp`, 151) and #69's tile areas (`tilearea.cpp`, 172) | Its open drafts, #559 and #69 integrate. Every other remaining non-drawing body belongs to the phase-2 owner of its file | Closing. #612, #616, #618 and #619 are in draft. #559 and #69 return now: their recorded condition is phase 1's exit. No further phase-1 owner is planned |
| 2. Simulation storage | 11,007 in the storage bucket, plus the phase-1 bodies left in each pool's files | Every pool record and the map arrays are defined once in Rust and read in place by both languages. C++ calls Rust only for operations, and no per-field service or per-call-site reader remains for moved data. Each pool owner also ports the remaining non-drawing bodies in its files | PR-B (#607) first, then #570 with #566. Pools in order: vehicles (#570, then #598 and #599); stations and road stops; towns and industries; companies, groups, engines, link graph and the smaller pools; orders; then the map with the rail, road, water, tunnel, landscape and tile bodies |
| 3. Simulation data readers | 41,096: save/load 12,852, script 13,099, network 9,640, NewGRF 5,505 | These run in Rust; the Squirrel VM may stay C++ until phase 6 | #573, #574, #575, #497 and #498 are queued and ready. They take every slot phases 1 and 2 leave free |
| 4. Support | 31,616 support and graphics lines, plus startup in `openttd.cpp` (510); data tables (12,931) retire with their consumers | Not planned yet | Not planned until it is the next phase (user direction, 2026-10-28) |
| 5. Interface | 67,796 GUI and platform lines including script and network GUI, plus about 4,700 drawing and description lines in the logic bucket | Not planned yet | As above |
| 6. Vendored | 54,626 | Not planned yet | As above |

Upstream unit tests (2,248) and fixture glue retire with the code they test.
Boundary-layer glue retires as its C++ callers do.

Root's first actions from this chart. They override the plan-after47 input's
#603-before-#607 order and its phase-1 rankings:
1. #581 PR-B (#607) integrates before any other `src/` change, #603 included:
   finish its current-base sweep, replay and deviation proof, then bootstrap
   Full. Other drafts keep implementing and reviewing, then adopt it at their
   base update. #621 changes only `tools/` and may integrate meanwhile.
2. Start #566 in the next free slot. Start #570 when PR-B integrates.
3. Finish #612, #616, #618 and #619, and select #559 and #69 as phase 1's last
   owners. The plan-after47 phase-1 candidates (link-graph job lifecycle, elrail
   transition, land count) join the phase-2 owners of their files.
4. Fill every other free slot with phase 3 (#573, #574, #575, #497, #498),
   ranked under [selection](planning.md#selection).
5. Size new owners to repay their review and Full ([selection](planning.md#selection)),
   and record the compiled-C++ rate at each stocktake (`/start-development`, step 4).

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
| 45 | #610 company control and ownership checks / #611 | `1dd6d32544` | 393 / 181 / 70 / +171 |
| 46 | #565 canonical map/movement loads and speed repair / #606 | `a60ef2e616` | 524 / 158 / 0 / +12 |
| 47 | #613 remaining engine runtime decisions / #614 | `2a89fb023f` | 316 / 352 / 63 / +76 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

First: **#621 per-function remaining-code inventory in `tools/port-metrics.py`**
(navigation, 80-150 tooling lines). It changes only `tools/`, so it may
integrate while #581 PR-B holds `src/` integrations. Planners use it for owner
and phase sizes.

1. **Finish the remaining drafts in readiness order, with the steering fixes.**
   #568, #594, #596, #605, #608, #609, #611, #606 and #614 integrated as 39 to 47.
   #612 and #614 follow completed implementation/runtime evidence and review.
   The integration-45 platform cadence passed; next due at 50.
   #603 is the next maintenance integration after #581 PR-B, which integrates
   before any other `src/` change (Phases, action 1). Each next action
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
     It integrates before any other `src/` change, #603 included; other drafts
     hold integration until it lands ([integration](workflow.md#integration)).
     Keep dual-build harness guards and move fallback-reading oracles to the
     pinned reference. Surviving drafts rerun the recorded sweep over every
     touched `src/` file when updating from base. Expected size and evidence
     are in the issue.
3. **#592 recent T3 worktree protection and service-table parser repair**
   integrated in #606 (integration 46). The original 20-40 tooling-line estimate
   covered the worktree item only. #582 integrated as #590 (integration 34).
   Keep source pins and current profile evidence protected during cleanup.
4. **#565 map view by a load with #275's fix-forward batch, then the remaining
   conversions.**
   - Item 1 integrated in #606: `map_func::World` is the one descriptor,
     reached by a load, and converted failed preconditions call the native
     assertion. The batch includes #275's movement-table load and road-search
     fact cache, #592 and the profile-38 helper repair. Merged-tip profile 46
     is next; no branch measurement substitutes for it.
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
   7. #610 company control integrated as #611 (integration 45, `1dd6d32544`).
      Its final metrics are 70 retired, +12 production glue, +159 fixture glue,
      181 tooling and 393 Rust. Root accepted the necessary native error/context
      observations, spot-checked both fixing-review repairs, and verified
      current-base exact-head Full. Pool storage and #604 remain separate;
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
   9. #613 engine runtime integrated as #614 (integration 47, `2a89fb023f`).
      Six methods had 78 original inventory lines; 63 retired. Final cost is
      316 Rust, 352 tooling, -19 production/+95 fixture glue. The supplied-company
      funding repair, full fixing review, root spot-check and current-base
      exact-head Full are complete. Native engine shells/storage remain;
   10. #615 clear-ground clearing (22) and editor tree-group mutation (36) is
      selected in the slot freed by #611. Port the two decisions and direct
      landscape dispatch, reusing canonical cost/map/tree/geometry/RNG owners.
      Necessary support stays with this owner, not all #69 utilities. First
      concrete split cost and necessary evidence are accepted; reviews are
      complete. #617 now selects the next snow-line/map-edge owner (46 original
      function lines, including signatures/braces), with process-wide state and
      exact date/edge semantics. Its implementation is #618, in first validation.
      Road-bit cleanup (45, including `IsPossibleCrossing`) is now #619.
      Phase 1 plans no further owners (Phases): #559 and #69 are its last,
      and the link-graph job lifecycle and elrail transition candidates join
      the phase-2 owners of their files.

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

   Process startup in `openttd.cpp` is phase 4, and drawing support (the rail,
   road and canal NewGRF resolvers, `UpdateDeltaXY`, elrail wire geometry,
   `IsWateredTile` and the bridge pillar helpers) is phase 5 (Phases).
7. **#566 drawing workload now, and #570 first shared vehicle records after
   #581's PR-B.**
   - #570 follows the accepted world-state design. It deletes the replaced
     readers and the direct Tick bounces, and makes `tools/port-metrics.py`
     count the C++ declarations it generates.
   - Estimated net glue -500 to -1,100. Its 0.86% instruction credit is a
     model.
   - Start #566 in the next free slot; it does not need PR-B. Obtain its
     reference and candidate pixels and drawing Ir before storage integrates
     (150-250 tooling lines).
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

Returned by the navigation of 2026-10-31: #559's command invocation lifecycle
and #69's remaining tile-list and support work. Their recorded condition was
phase 1's exit, which now arrives; they are phase 1's last owners (Phases).

Excluded, with return conditions checked by this review:
- #507's NewGRF corpus (#525, #526) stays parked by the user (2026-10-28)
  until the user reselects it. #525's branch stays on origin.
- #64 and #68 become maintained Rust crates that reproduce the game's
  observable cryptography. They return when phase 3's network ports reach
  authentication, or with phase 6, whichever comes first.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until
  near-full reproduction under #513's original-default setting policy. #513
  needs implementation when the first such improvement is accepted.

## Resume checkpoint (2026-10-31, paused after integration 47)

Paused at the user's request. Root development has stopped; late CI or task
notifications do not restart it. Resume only on the user's instruction.
Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh.

Current code is `2a89fb023ff388ea4c0a7f3464f1d6ffa28d8193` (integration 47),
with only documentation afterward at pre-pause base `72f92b2569`. Integrations
39 to 47 are recorded in the ledger above; no rollback or recovery occurred.
#614's validated `4a3db40ea2` tree equals merged `2a89fb023f`. The user's 17:00 UTC
state check reports base Quick passed. No combination was eligible to merge
at wind-down, and no base update, repair/review round, build, simulation,
recount, CI request or retry was started after the stop. The lenient allowance
saved only #607/#619's edits underway and completed the read-only plan report.

### Remaining drafts and next actions

| Issue / draft | Saved head | Known evidence and next action | Retained checkout |
| --- | --- | --- | --- |
| #199 / #603 | `f2731de370a3025d2345a1836945632c0bf1db99` | Full/scoped reviews and combination validation complete: four Cargo gates, 16 native tests, 17 cases/153 snapshots. Cost: 0 retired, 202 Rust, 3 tooling, -32 production/+63 fixture glue, total +31. Behind base; **actual-base adoption, then exact-head Full**. Do not expand into session/CargoFlow work. | `t3-d31fc312`; old `t3-120ef117` left alone |
| #581 / #607 | `8bb6678cea6c62469ceec6195f8624fd9f5885d2` | Already-underway engine-base resolution saved/pushed; local/remote heads match, clean/locked, no merge/job. A pre-stop sweep is not new-base proof: source replay and explicit-deviation identity are incomplete, with no current-head compilation/metric/runtime result. **Finish actual-base update with touched-source sweep/replay and deviation proof; bootstrap Full before any other `src/` integration, #603 included (navigation)**. CI_ON_DEMAND unchanged. Earlier e090 proof/metrics remain historical. | `t3-4076556a` |
| #602 / #612 | `a5bd61f93dfaac6fe93540a1405d6ae84df80b20` | Full/scoped reviews and A5 builds/nine-native/five-case/eight-snapshot checks complete. Cost: 88 retired, 599 Rust, 587 tooling, -78 production/+29 fixture. Conflicts with base; **actual-base update, then Full**. Preserve approved STL support, one Rust tree and native area/GUI/lifecycle exclusions. | `t3-56a3afc8` |
| #615 / #616 | `b4021e34a128fdcb2a368fb92429601055dc8987` | Full `41221404067` **FAILED**: native/build and Linux GCC SDL2 fail `town_growth.cpp:20` (`CHECK(result)` false); Windows x64 also fails. Earlier local/review passes and marker-only price-constant resolution do not authorize merge. Existing recount: 52 retired, 131 Rust, 196 tooling, -1 production/+18 fixture. **Repair this combination failure, then actual-current-base exact-head Full**. No fix/retry was started. | `t3-68700e40` |
| #617 / #618 | `45e1350c79ca577d96c0fb05b3ff4c7edfa2d8bc` | Existing verification `20261031T162734.730348Z`, simulation driver `20261031T163657.570360Z` and simulation `20261031T163658Z-3509040` pass: 97 reference/231 candidate tests and ten cases/14 snapshots. Previously started standard recount completed: 34 retired, 159 Rust, 200 tooling, +10 production/+78 fixture (109 added/21 removed). **Fresh full fixing review, then actual-base Full**; recount again only if source changes require it. | `t3-0d996c45` |
| #619 / draft #620 | `1afd6fe0287407e8032b7e1b3dad8be2c5d43bc5` | Incomplete initial cleanup/crossing owner, direct town/road-loop consumers and unvalidated native/scenario edits saved/pushed; clean/locked. Compilation/behavior, concrete cost, final evidence audit, #156 update, component docs and review remain incomplete. **Continue implementation from this head**. No further development/validation ran after wind-down. | `t3-90e92030` |

### CI and jobs at pause

- #614 Full `41218950045` passed on exact `4a3db40ea2`/base `3aa9a1058d`;
  merged as 47. No recovery is eligible or performed.
- #616 Full `41221404067` is completed **failure** at b402/base72. Native,
  Linux GCC SDL2 and Windows x64 fail; required completion/annotation gates
  also fail. No retry is authorized by wind-down. The CLI can exit zero on
  failure: its last line here says `CI request failed`, not a Full pass.
- Quick validation and Commit checker were successful at all six saved draft
  heads when inspected. #603/#607/#612/#618/#620 have **no current-head Full**;
  their `Full validation: PENDING` contexts are unvalidated placeholders, not
  active Full requests. #616 has `Full validation: FAILURE`.
- All root-started build/simulation/recount pipelines have completed. #603's
  f273 validation and #618's first validation pass; both existing #616 and #618
  recount results were recorded without launching another measurement.
  No known root build/simulation/CI wait or delegated development job remains
  running. #607/#619 and the read-only planner have stopped. Later automatic
  notifications or checks are not permission to restart work. Any automatic
  checks on this final docs-only pause commit are not awaited.
- This root's PR watches are off. Source checkouts stay held. Stocktake/profile after 48,
  platform cadence at 50, #503 coverage continuation last and deep checks only
  on request remain unchanged. CI_ON_DEMAND is unchanged.

### Completed plan-after47: input only, not applied

Agent: /root/plan-after47 | Model: gpt-6-astra | Reasoning effort: high.
Source pin: Main `72f92b2569`, integration-47 code `2a89fb023f`; original
`14ec60f248`. Both placement runs report 37 open, 0 unplaced, 3 excluded and
0 exclusions without an issue. This report selected, placed, started and
edited nothing. Resume may consider its findings; the work order and other
sections have not been rewritten from them.

| Resume ranking | Source inventory versus credible retirement | Boundary and constraints |
| --- | --- | --- |
| 1. Link-graph job lifecycle coupled to completed-result ownership/publication | Recorded 37 principal-span lines include about 12 pool-coupled constructor/destructor lines and six orphan EraseFlows lines. Estimate 20-45 new original retirement; report orphan/staging deletion separately. | Own completed results and worker control in Rust, call flow owners directly and delete native result staging. Preserve failed-thread-start synchronous fallback, join/publication, acquire-release abort/date control, pool-cleaning suppression, graph/settings identity and LGRJ restart. Native pool/thread/resource operations remain. Estimate 250-500 Rust, 100-200 tooling, +60 to +160 fixture and -60 to -160 production glue; not measured. |
| 2. Elrail setting transition and service-interval clamp | 40 original named-body lines: callback 5, transition 29, clamp 6; estimated 28-35 retirement after native facades. | Preserve intended/current rail types, disabling-only fleet changes, afterload update_vehicles=false, consist/company availability/GUI order and percent/day/minute/menu selection. No established #570 dependency. Estimate 180-350 Rust, 70-150 tooling, +30 to +90 fixture and -10 to +60 production glue. |
| 3. New, unselected generation land-count state and population scaling | CountLandTiles 9 plus ScaleByLandProportion 4 executable original lines, plus scalar declaration/definition; estimated 11-15 retirement. No issue created. | Own initial_land_count, the full count/scaling rules and three direct Rust consumers. Count IsWaterTile exactly, preserve one-twelfth adjustment/cap, 64-bit multiplication/current-map denominator, both generation positions and town RNG placement. Map::Allocate does not reset the scalar. Native map arrays remain. Estimate 40-100 Rust, 30-80 tooling, +10 to +35 fixture and -10 to -25 production glue. Coordinate source overlaps with #618 after integration; no draft-service dependency. |

Further resume findings, not applied:

- #570 remains first phase 2 after #607, with #566 drawing evidence required
  before integration. Shared generated records, reader deletion and direct
  Tick dispatch belong together. Keep the later pool order and earlier-order
  option only for a concrete vehicle-storage dependency.
- After #570, preserve sequential #598 then #599. #598's active inventory is
  about 260 including direction-table lines, with about 220-250 credible
  retirement; ship running cost already belongs to integrated #614 and the
  road-turn algorithm is already Rust. #599 has 216 active function-span lines,
  including 29 constructor/destructor and eight already-ported visit shells:
  about 179 remains before facades, with 150-180 retirement estimated. Remeasure
  after #570 rather than count the old 245 as promised retirement.
- Airport automata have 49 live construction lines; the extra 26-line rotation
  body has only a disabled fallback caller and rotation is already Rust.
  Report that deletion as cleanup. Remaining airport header lookup/offset/exit
  decisions require an explicit payload/storage boundary, not per-field services.
- Phase-1 exit accounting is incomplete. Command admission/execution (#559),
  remaining tile/bitmap/diagonal geometry (#69), map/foundation/rail/autoslope/
  tunnel/distance support and live executable headers need consumer accounting;
  guarded already-ported algorithms and drawing/storage code earn no second credit.
  #559/#69 return conditions are not newly met by this report; no generic sweep.
- Keep #603 next maintenance and #607 after it. A future bounded maintenance
  scope combines remaining #565 conversions with the distinct station CargoFlow
  bounce repair (about -20 to -50 production glue, zero original retirement).
  Couple necessary completed-job publication to the link-graph owner; do not
  expand #603. #604 stays after map/CargoFlow, #594, #607 and #570's generator,
  before typing/coverage; configured generation dimensions, client/timer/company/
  GRF inputs and exception-safe Tick continuation remain. It is not a #570 prerequisite.
- #501 typing remains bounded and below ownership/fault repairs. Attach #156
  evidence to selected owners. #503 retains both clock-preparation and missing
  sanitized native-test-support build defects and provenance-checked continuation
  of 119 traces last; the retained 330 traces are not completed coverage.
  Deep execution remains user-request-only.
- Existing placement remains for #3, #168/#199/#500, #573/#574/#575 and #497/#498.
  #64/#68 return at actual network authentication or phase 6, under the maintained
  behavior-compatible crate policy. #507/#526 require explicit user reselection.
  #12/#26/#41/#67/#71/#83 stay deferred to near-full reproduction; #513 activates
  with the first accepted improvement. No return condition was newly selected.

### Retained worktrees

Housekeeping dry run and apply removed 0 B, with no forced removal or cache/log
audit. The following checkouts remain. Paths beginning `t3-` are under
`/home/fetch/.t3/worktrees/openttd-rust/`; profile/deep names are under the
main checkout's `.local/worktrees/`.

| Group | Retained checkouts / reason |
| --- | --- |
| Main | `/home/fetch/projects/openttd-rust`, branch rust-migration; docs-only paused checkpoint, code at 2a89fb023f |
| Remaining drafts | `t3-d31fc312` (#603), `t3-4076556a` (#607), `t3-56a3afc8` (#612), `t3-68700e40` (#616), `t3-0d996c45` (#618), `t3-90e92030` (#619/#620); clean, pushed, locked and held |
| Explicit older source | `t3-120ef117` (old #603 at 451ebd778e, left alone); `t3-431c84b4` (cancelled #565 source at e48846d8fd with 58 dirty entries, left unchanged) |
| Open-PR reviews | `t3-f636a8e7` (#603), `t3-85d3750f` and `t3-e0ea59bc` (#612), `t3-59d57145` and `t3-056e6bf6` (#616); left alone |
| Merged #614 | `t3-8ff8d8ea` (implementation) and `t3-8a1ccb4f` (review); retained locked where live-process inspection was incomplete |
| Other evidence / planner | `t3-d78d5f8b` (retained native fixture evidence at 51ade2d021); `t3-5da58ec3` (completed read-only planner at 72f92b2569) |
| Immutable reference | `.local/reference/openttd`, unchanged at 14ec60f248547d4d062a1160f0fc26d742319888 |
| Deep evidence | `deep-74cea95cf8`, `deep-after30-569`; preserve outputs and pinned runtimes, no restart |
| Instruction profiles | `profile-after18-544`, `profile-after20-545`, `profile-after26-558`, `profile-after36-595`, `profile-after38-576`, `profile-after40-594`, `profile-after42-605`, `profile-after44-609`, `profile-after46-606`; preserve anchors and reference/candidate evidence |

The three allowed delegated tasks are complete and stopped. No recommendation
from the read-only plan was applied. The final user-authorized action after
this documentation push is one new top-level Claude Code thread, model
claude-opus-5-5, effort xhigh, at this project's root with prompt `/navigate`.
Do not treat that navigation request as resuming this development assignment.
