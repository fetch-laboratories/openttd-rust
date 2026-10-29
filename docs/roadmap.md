# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-29, code at `44cb5f49f5`)

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh.

- The resumed root session integrated #588 as integration 31 (`44cb5f49f5`):
  #578 (#572 goals, story pages and league tables), #580 (#168 clock and world
  boundary batch) and #589 (the `town-growth-differing-road-type` interactive
  seed fix). `python3 tools/port-metrics.py` at `ee3a0481d8`, before a 3-line
  MSVC width fix and #589's 4 tooling lines joined: Rust 1,133, tooling 376,
  original C++ retired 388, glue added 315, glue removed 51, net glue +264. No
  rollback ran.
- Whole fork: not re-measured this session. The Phases chart below holds the
  inventory at `06a644505a`; add integration 31's figures above.
- Four component branches are open: #568, #576, #579 and #583. The checkpoint
  gives heads and next actions.
- Speed: the profile after integration 30 (`06a644505a`) completed. Grok and
  Opus rose 13.7% and 12.0% over their anchors and moved into the play band;
  P2016's record is at the band edge. Attribution and fix-forward and a speed
  pass are due (AGENTS.md "Speed"); #569's follower crossings (about 2 to about
  15 per follow) are the first suspect, and #565 item 1 the first candidate fix.

| Scenario | Latest quiet anchor | Profile-30 projection | Record with growth |
| --- | ---: | ---: | ---: |
| TGP | 1.1117x | 1.1117x | 1.1541x |
| Grok | 1.3871x | 1.5777x | 1.5760x |
| Opus | 1.3939x | 1.5618x | 1.5625x |
| P1996 | 1.3366x | 1.3816x | 1.3828x |
| P2000 | 1.4006x | 1.4288x | 1.4297x |
| P2016 | 1.5229x | 1.5644x | 1.5223x |

- Deep checks: the sanitized rotation after integration 29 was interrupted at a
  session end after 406 cases. Its one failure was the scenario's
  `_interactive_random` timing, which #589 fixed. The restart on `06a644505a`
  stopped during its candidate configure without a report.
- Original coverage: 330 of 449 traces are complete; gaps are on #156.
- Housekeeping still keeps every T3 worktree on this host (#582); root removed
  merged worktrees by hand.

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite (AGENTS.md "Migration method"). Each free
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
   command over every `src/` file they touch (AGENTS.md, step 3).
2. #570 starts after the sweep integrates. #570 also makes
   `tools/port-metrics.py` count the C++ declarations it generates.
3. Phase 2 takes every slot that ready phase 1 work leaves free, ranked by code
   retired, crossings removed and glue deleted.
4. Before each stocktake and planning round, reread AGENTS.md, this chart and
   the latest steering section if they changed (`/start-development`, step 4).

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
3. **The map view costs a load, not a call** (AGENTS.md map bullet,
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
   rose more than 3%, attribution follows AGENTS.md, and correction 3 is the
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
behavior fixes get a scoped review; deep checks run every second integration;
active speed dependencies are preserved. #559 and the parked NewGRF corpus stay
excluded with the return conditions in the work order. AGENTS.md carries the
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

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **Finish the five drafts in readiness order, with the steering fixes.** Each
   draft's next action is in the checkpoint, and its before-integration fixes are
   in its PR comment of 2026-10-29. No reviewed owner gains other scope; a fix
   the audit found is not new scope.
2. **#581 Rust on every CI configuration and retirement of the portable
   fallback.**
   - PR-A, Linux builds Rust (about 40 lines), takes the first free slot.
   - PR-E, Emscripten links Rust (a bounded spike first), takes the next. The
     emsdk 4 move, if needed, is the fix.
   - PR-B, the sweep plus moving the fallback-body oracles onto the pinned
     reference, follows PR-A, PR-E, #568, #576 and #580.
   - Expected size and evidence are in the issue.
3. **#582 tooling batch**: the rebound-callee quick-CI check and the
   housekeeping cwd fix (80-150 tooling lines). After it integrates, run
   housekeeping with `--apply`.
4. **#565 map view by a load, then the remaining conversions.**
   - Item 1 is root's own small PR, once #576 integrates: `map_func::World` is
     the one descriptor, reached by a load, and failed preconditions call the
     native assertion.
   - Drafts adopt it at their base update.
   - Item 2 deletes the C++ fallbacks and converts the remaining rail, crossing,
     depot, stop, water and tunnel leaves and `tile_slope_z`.
   - Planning range: production net glue -300 to -600.
5. **#199 integrated-fault batch**, after #582: #569's service copies, duplicate
   fields and orphan declaration, #577's single-use carriers, and the
   `CargoFlow` bounce once #568 integrates.
6. **Queued phase-1 owners under #3**, sized by issues-phase1 (remaining code
   lines, estimated net glue):
   1. #587 simulation-setting transitions (317, +90 to +170);
   2. #584 world-input generation, heightmap input and town-data import (276,
      +90 to +160);
   3. #585 NewGRF town-name storage and generation plus ActionF (168, +40 to
      +85);
   4. #586 sign and depot naming (90 selected, +35 to +70);
   5. clear-ground clearing (22) and editor tree-group mutation (36): one small
      PR beside the ground and tree owners; root issues it when a slot takes it.

   Do not fall back to utility helpers. Port
   support code only where the component needs it, and keep an operation on
   data that stays in C++ as one call, not a service per field. Vehicle base
   lifecycle runs after #570, on its movement and consist record contract
   (estimate 190-240 retired, 70-160 net glue), never concurrently in the
   shared vehicle files.
7. **#566 drawing workload and #570 first shared vehicle records**, after
   #581's PR-B.
   - #570 follows the accepted world-state design. It deletes the replaced
     readers and the direct Tick bounces, and makes `tools/port-metrics.py`
     count the C++ declarations it generates.
   - Estimated net glue -500 to -1,100. Its 0.86% instruction credit is a
     model.
   - Start #566 beside it and obtain its reference and candidate pixels and
     drawing Ir before storage integrates (150-250 tooling lines).
8. **Queued later phase-2 pools.** The order is:
   1. the remaining vehicle pool;
   2. stations and road stops;
   3. orders and order lists;
   4. towns and industries;
   5. companies, groups, engines and the smaller pools;
   6. map allocation and definitions.

   Move the coupled save adapters with each record owner, generate native
   declarations, verify the layout, and delete the displaced readers and
   services. Each pool gets an issue and a design section when selected.
9. **Phase-3 reader queue.** #573 codec and chunk execution, #574 game network
   protocol and queues, #575 script host, commands and events. Each issue
   carries explicit unsaved-output evidence. Then complete NewGRF text,
   config/Action14 and definition ownership: #497 with its engine and feature
   binding payloads, #498 with its ObjectSpec payloads. Plan nothing for phases
   4-6 and no Squirrel port.
10. **#168/#199/#500 completed-job flow publication; #501 probe typing.**
    Zero retirement; estimated -30 to +80 net glue.
11. **#503 coverage continuation.** The bounded original-only clock-preparation
    fix, then 119 unfinished traces with provenance checks. No recollection just
    to fill a slot.

Recurring:
- Profile the merged tip after every second integration; the profile after
  integration 30 is due first.
- Read the running deep member's report, then launch the next rotation under
  the existing lock at idle priority.
- The fifth-integration platform cadence is due after integration 30.
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- Root checks actual issue placement and exclusion conditions at stocktake.

Excluded, with return conditions checked by this review:
- #559 command invocation lifecycle returns when one of these happens:
  - a selected game owner needs Rust-owned top-level `Do`/`Post` admission;
  - a #275 profile ranks top-level dispatch among the largest net costs;
  - the queued ownership candidates run out.
- #507's NewGRF corpus (#525, #526) stays parked by the user (2026-10-28)
  until the user reselects it. #525's branch stays on origin.
- #69's remaining tile-list/support work returns when a selected simulation
  component needs it.
- #64 and #68 become maintained Rust crates that reproduce the game's
  observable cryptography. They return when phase 3's network ports reach
  authentication, or with phase 6, whichever comes first.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until
  near-full reproduction under #513's original-default setting policy. #513
  needs implementation when the first such improvement is accepted.

## Resume checkpoint (2026-10-29, paused after integration 31)

Paused at the user's request after integration 31 (`44cb5f49f5`); no rollback
ran. The user's wind-down scope ended with #588, so the remaining drafts wait
for resume with every worktree and branch preserved.

| Draft | Pushed head | Next action on resume |
| --- | --- | --- |
| #561 / #579 | `fec98f7e42` | Reviewed by review579; fix579win's Windows static-initialization repair (`FleetGroupServices()` built on first use) is a platform repair needing no new review, and diagnostic platform run 40900481042 passed Windows x64 build and tests. 1. Update from base: it overlaps #588 in `lib.rs`, `tunnelbridge_cmd.rs` and `docs/rust-migration.md`; keeping both sides needs no new review. 2. Exact-head Full. 3. Merge. |
| #562 / #568 | `a5d99f00cb` | Steering fixes made; review568-r2 fixed its findings and all 21 scenarios passed; root spot-checked; F3 is on #199. 1. Update from base: it overlaps #588 in 23 files, so a resolution beyond keeping both sides gets a fresh conflict review. 2. Full. 3. Merge. |
| #565 / #576 | `6cb71c295e` | review576-r3 fixed its findings. Root's spot-check found that `World::asserted` aborts after a disabled native assertion where the original accessor returns the raw bits (PR comment). 1. Fix it (fix576-assert was cancelled before any edit). 2. Fresh scoped review. 3. Base update and Full. Root's #565 item-1 PR follows its integration. |
| #581 PR-A / #583 | `379b993d00` | Reviewed by review583 and spot-checked; Linux Clang and Dedicated built Rust and passed in platform run 40895107702. 1. Update from base. 2. Bootstrap Full: unset `CI_ON_DEMAND`, request `--profile full --bootstrap`, restore the variable after merge. 3. Merge. |

| Running or due work | Saved state / next action |
| --- | --- |
| Speed attribution and fix-forward, speed pass | Due: Grok and Opus rose more than 3% and are in the play band (table above). Profile-30 output is in `.local/worktrees/profile-after30-569`; keep it and profile28 until replaced. |
| Instruction profile | Next due after integration 32. |
| Deep rotation | Due at session start on the current tip: sanitized, threaded, TSan and multiplayer (the soak member ran last at `0b4646f1e7`). `.local/worktrees/deep-after30-569` is locked and holds no report. |
| Fifth-integration platform cadence | Run 40893525725 on `70193e535e`: Linux Clang, Dedicated, Windows x64 and x86 and macOS Release passed; macOS Debug was not acquired by a hosted runner, so the annotation checks failed. Not a code regression; a rerun is due. |
| CI | No request pending. Diagnostic platform run 40900481042 on #579 may still be finishing. |
| Work order after the drafts | Items 2 to 6: #581 PR-E, #582, #565 item 1, #199, then #587, #584, #585, #586. |

Keep the pinned reference unchanged. Preserve unfinished checkouts and existing
profile and deep evidence.
