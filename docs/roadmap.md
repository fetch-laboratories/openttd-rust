# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-28, code at `c625d9d543`)

- 79 ownership ports. Whole-fork inventory: 72,102 of 470,612 original lines
  retired (15.32%), including 56,403 of 103,451 simulation lines (54.52%). Rust,
  tooling and glue total 129,354, 51,156 and 46,170 lines. All of these count
  comments and blank lines. In code lines alone, a rough steering count gives
  17.3% of the whole fork and 63.0% of simulation code (37,014 of 58,736). #564
  moves the metric to code lines.
- Integration 26 added #558's town-growth input evidence, with no simulation
  source change. The phase code-line table below is unchanged: 50,136 original
  lines retired and 266,542 compiled C++ lines including glue (93.7% of the
  original), with the navigation's provisional boundary count of 1,271 exports
  and 2,634 callback slots. #564's reviewed counter will replace those manual
  counts, including private fields and restricted-visibility exports. Five
  component branches remain unintegrated; root's shared-helper preparation
  takes the next branch slot.
- What the remaining 47,835 simulation-inventory lines are: 26,113 blank,
  comment or preprocessor lines; 10,464 header code (class layouts and inline
  accessors, which retire when their data and C++ callers move: about 2,600
  code lines of map accessors, 2,200 of pool object classes and 5,700 of other
  inline helpers and types); 11,258
  `.cpp` code, of which about 6,900 is drawing or tile description and about
  4,100 is non-drawing function bodies of five or more lines. Most game-logic
  control flow already runs in Rust. What remains is leftover helpers that Rust
  calls back into (#560, #561, #562 and the queued candidates below) and the
  shared storage whose glue the speed profile now measures (#563).
- Since the twenty-second steering (`4e3b45bfef`): two owners (clocks #539,
  world generation #547) retired 884 lines for 943 net glue; tooling grew 6,325
  lines, mostly the user's testing queue (#504 to #511), which is now complete
  except #503's recurring checks and #507's user-parked NewGRF corpus. The period
  before retired 3,353 lines. Fifteen Full runs: 13 passed, two failed on real
  defects (a history fixture definition, a native-only linkage warning), none
  cancelled.
- Speed: the play-save ceiling is 1.6x (user decision, 2026-10-28; it was
  1.5x), so the band starts at 1.52x. P2016's quiet anchors over five passes
  were 1.4838x, 1.5023x, 1.4534x, 1.4695x and 1.5229x. Its record, the mean of
  the last three carried forward by the last profile's growth, is about
  1.485x: below its band, as are all other scenarios. On P2016 the loop gap over the
  original is 29.9% of instructions, and 99% of it is boundary glue spread over
  about 1,500 wrapper functions (#275 comment of 2026-10-28). #555's loading and
  acceptance repair integrated; its estimated 1.065% is uncredited until the
  next profile.

| Scenario | Quiet anchor `755609b8ea` | Last profile `7f62b1bcba` |
| --- | --- | --- |
| TGP | 1.1117x | 1.1122x |
| Grok | 1.3871x | 1.3945x |
| Opus | 1.3939x | 1.4022x |
| P1996 | 1.3366x | 1.3445x |
| P2000 | 1.4006x | 1.4062x |
| P2016 | 1.5229x | 1.5263x |

- Evidence: the #503 deep set's sanitizer member found a real production
  defect (road/rail availability exports returned non-canonical `bool` bytes)
  and a fixture lifetime bug, both fixed in #552. Its soak member passes all 521
  scenarios. #511's mutation study detected 31 of 40 mutants; integrated #558
  covers the differing town-buildable road input. Its sensitivity experiment
  used a separate OR-to-AND mutation, so no exact M23 rerun is claimed. Original coverage recollection reached 330 of 449 traces.
- CI: the cadence retry passed Windows x64/x86, macOS Debug/Release, Linux
  Clang and Dedicated, including prepared Windows/macOS semantic comparisons.
  #558's exact-head Full passed and integrated. The earlier vcpkg download and
  hosted-capacity failures require no repair or rollback.

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite (AGENTS.md "Migration method"). Phases
overlap: each free slot takes ready work from the earliest phase that has some,
and later phases fill the rest (user decision, 2026-10-28). This chart is the
first navigation (2026-10-28, code at `06ed8c4a6f`). Code lines exclude
comments, blank and preprocessor lines; #564 makes this per-area count the
standard metric. It differs slightly from the steering's rough count at the same
code (234,980 left), because the two counters handle comments differently.

| Area | Original code lines | Retired | Left | Glue added |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (inventory `.cpp`) | 46,111 | 35,144 (76.2%) | 10,967 | 6,728 |
| Commands outside the inventory (`command`, `goal`, `story`, `league`, `signs`, `cheat` and others) | 2,252 | 159 (7.1%) | 2,093 | 45 |
| Simulation storage (inventory headers, `map.cpp`, `*_map.cpp`, pools) | 13,165 | 2,321 (17.6%) | 10,844 | 1,304 |
| NewGRF outside the inventory | 10,785 | 5,218 (48.4%) | 5,567 | 423 |
| Save/load | 13,388 | 536 (4.0%) | 12,852 | 585 |
| Script API and hosts | 15,474 | 878 (5.7%) | 14,596 | 453 |
| Network | 12,713 | 211 (1.7%) | 12,502 | 182 |
| Support (core, strings, settings, misc) | 19,319 | 665 (3.4%) | 18,654 | 424 |
| Graphics and viewport | 11,726 | 1 (0%) | 11,725 | 1 |
| Data tables (`src/table`) | 17,295 | 4,364 (25.2%) | 12,931 | 0 |
| GUI | 52,679 | 117 (0.2%) | 52,562 | 165 |
| Platform | 12,079 | 0 (0%) | 12,079 | 2 |
| Vendored | 55,149 | 522 (0.9%) | 54,627 | 163 |
| Upstream tests | 2,248 | 0 | 2,248 | 4,487 |
| Boundary layer (`src/rust/`, 227 files) | - | - | - | 17,333 |
| **Whole fork** | **284,383** | **50,136 (17.6%)** | **234,247** | **32,295** |

- Distance to the goal (AGENTS.md: the C++ the Rust build compiles): 234,247
  original plus 32,295 glue is 266,542 code lines, 93.7% of the original. Rust
  is 116,194 code lines.
- Boundary: 1,271 `openttd_rust_*` exports and 2,634 callback slots in Rust
  service tables. On P2016, 349M glue calls (79 per vehicle tick) are 99% of
  the gap to the original (#275, 2026-10-28).
- Rate over 25.9 days: 1,935 retired and 1,247 glue lines a day, so the C++
  shrinks by 689 lines a day. Recent steering periods, retired and glue per
  day: 10-22 to 10-23 3,437 and 891; 10-23 to 10-24 3,450 and 2,189; 10-24 to
  10-26 1,987 and 1,206; 10-26 to 10-27 3,251 and 1,282; 10-27 to 10-28 552
  and 640. In the last period the build gained more C++ than it lost.

Why the course changed. Ports moved control flow into Rust and left the data
in C++, and the rules then forbade the cheap ways to reach that data: shared
layouts ("neither a whole-record view nor a call per field") and map reads
outside audited loops ("until audited"). So every field read became a call into
C++, and each call site got its own reader (`TrainLoco26Read`). Speed loss and
glue both grow with this boundary. The phase 2 plan would then have turned the
boundary around: Rust storage behind exported calls, read by about 117,000
lines of C++ GUI, drawing, save/load, script, network and NewGRF code, with
drawing never timed.
AGENTS.md, world-state.md and #563 now state the outcome instead. Data crosses
in place through layouts defined once in Rust, and only operations cross as
calls. The map is read in place from every Rust entry under one proved interval
(#565), and drawing gets measured (#566).

| Phase | Left | Exit criterion | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | 13,060: inventory 10,967 (about 6,900 of it drawing and tile description, which belong to phase 5) and commands outside the inventory 2,093 | No non-drawing function body of the inventory or the command families compiles under `WITH_RUST` | Nearly done: #560, #562, #561 and the queued candidates; #559 is excluded with its return condition |
| 2. Simulation storage | 10,844 header and map code, plus the glue it deletes | Every pool record and the map arrays are defined once in Rust and read in place by both languages. C++ calls Rust only for operations, and no per-field service or per-call-site reader remains for moved data | Starts with #563's design and #565 |
| 3. Simulation data readers | 45,517: save/load 12,852, script API 14,596, network 12,502, NewGRF 5,567 | These run in Rust; the Squirrel VM may stay C++ until phase 6 | First planning round selected (work order) |
| 4. Support | 30,379: support 18,654, graphics and viewport 11,725; data tables (12,931) retire with their consumers | Not planned yet | Not planned until it is the next phase (user direction, 2026-10-28) |
| 5. Interface | 64,641: GUI 52,562 and platform 12,079, plus the phase 1 drawing | Not planned yet | As above |
| 6. Vendored | 54,627 | Not planned yet | As above |

Upstream unit tests (2,248) retire with the code they test.

Root's first actions from this chart:
1. #564's metric item (distance and boundary size per area) lands before the
   next planning round, which reads it. "Where the fork stands" then reports
   the compiled C++ and the boundary size at every stocktake.
2. #563's design follows the shared-layout outcome (its body and the comment of
   2026-10-28).
3. #565 starts in the next free slot. New phase 1 ports (#560, #561, #562) read
   the map in place and use each record's shared typed accessors. They add no
   per-call-site reader.
4. #566 runs beside #563, so the first storage port reports its drawing cost.

## Twenty-third steering review (2026-10-28, leftover game logic, storage and boundary faults)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

Three fidelity audits compared #554 (session lifecycle, pending), #539
(clocks) and #547 (world-generation lifecycle) line by line against the pinned
original: 1,475 retired lines. A planning audit sized the remaining simulation
inventory, and a speed audit attributed P2016's gap from retained profiles. No
build, game or wall measurement ran; the steering verified each claim below.

No reachable divergence was found. The findings are boundary faults and
orphans that fixing reviews missed:
- #554 calls back into Rust through the C++ `StateGameLoop` and `_GenerateWorld`
  frames, kept only for callgrind attribution.
- Rust reads Rust-owned clock and mode state through C++ service tables.
- Twenty-two exports or declarations across #547 and #554 have no production
  caller.

Root's ownership selections had drifted to lifecycle and dispatcher code
where net glue roughly equals retirement. The only queued owner, #559, retired
less than its glue, outside the simulation inventory.

The root cause was the goal itself. AGENTS.md and the restart prompt named the
goal as "a game whose simulation runs in Rust". That lane is three-quarters
done, while the whole rewrite is 17% done and nothing planned the rest. So
every planning round looked for ever smaller simulation leftovers, testing
tools and repairs. The goal is now the complete rewrite in six overlapping
phases (user direction and decisions, 2026-10-28; "Phases" above).

Apply these corrections through the work order below:

1. **Fix #554 before it integrates** (PR comment of 2026-10-28):
   - Call the Rust state loop and generation directly; profile the candidate
     at the Rust entry (AGENTS.md "Speed").
   - Read mode and pause state in Rust.
   - Use one shared service for `post_pause`, `desync_level` and `Map::LogY`.
   - Delete the orphans, moving harness callers onto Rust entries.

   One fresh fixing review covers these fixes. Then integrate the existing
   #554/#557 combination after its exact-head Full. Its coverage gaps went to
   #156 and do not block.
2. **Run the leftover game logic concurrently.** Start #560 (track follower,
   best ratio and the largest speed item in reach) and #562 (station naming,
   tile loop and lifecycle) now, each from integrated `rust-migration`. Start
   #561 (company identity) when a branch slot frees after #554/#557 integrate.
   Each issue carries its scope, its evidence and a planning range. Root
   decides each cost once at the first checkpoint. Keep `GetPlatformLength`
   with #560.
3. **Move the data: storage ports are part of the rewrite** (user direction,
   2026-10-28; AGENTS.md "Migration method", world-state.md "Transferring
   shared storage"). Game-logic ports alone stop at about 70% of simulation
   code: the remaining header code (map accessors, pool object classes) retires
   only when the data it describes moves. A planning agent writes #563's design
   note now and opens the first implementation issue. Per-tick vehicle records
   and the `X::Tick` dispatch shells come first, because their glue dominates
   P2016's gap. Then the other pools, then the map. Storage and leftover game
   logic run side by side within the branch limit.
4. **Speed.**
   - The user raised the play-save ceiling to 1.6x. AGENTS.md now judges
     ceilings and bands on a record that averages the last three passes. No
     scenario is in its band, so no speed pass or repair batch is due.
   - The station tick's single typed read per station (1.44%) and in-Rust
     assertion conditions with cached service tables (0.52%) stay notes on
     #275. They are selected when a scenario's record enters its band.
   - Do no further attribution research on 0.1% to 0.3% candidates. Speed
     beyond that comes from #560 and #563.
   - #564 carries the tooling change: `speed_budget` reports the 1.6x
     ceiling and the three-pass record. Until it integrates, the report's
     ceiling field lags AGENTS.md.
5. **Exclude #559 with its return condition** (issue comment). This reverses
   root's own selection, not a user direction.
6. **Maintenance from the audits.** Take the #168 batch (clock-table bounce,
   world-generation abort round trip, orphans) after #554 integrates, since
   they share files. Take #564 (a quick-CI orphan-export check and cleanup of
   finished T3 task worktrees, 42 using 56 GB) at any time.
7. **Testing is maintenance again.** New scenarios arrive with the ports that
   need them. Finish #558. Rank #503's coverage continuation last and recollect
   coverage only when a planned testing item needs the numbers, not after every
   scenario change.
8. **Work toward the whole rewrite.**
   - Plan and fill slots by phase. AGENTS.md and the restart prompt now state
     the full-rewrite goal, the six overlapping phases and the vendored policy
     (Rust crates where one reproduces observable behavior).
   - Start phase 3 now. A planning agent sizes save/load, the script API,
     network and the NewGRF remainder into ownership candidates, each with
     the evidence that shows its output (work order item 9). The first phase 3
     owner takes the next spare slot.
   - Save/load chunk handlers for a pool move with that pool's storage port
     where they share it.
9. **Cadence.** Re-dispatch the cadence run on the current tip and read it
   before the next integration. If macOS gets no runner again, that is its
   second hosted-capacity cancellation and the `CI_MACOS_HEAVY` rule applies. A
   repeat of the vcpkg 502 selects a vcpkg binary cache, not retries.

AGENTS.md changes:
- The play-save ceiling is 1.6x (user decision); ceiling and band decisions
  use the mean of the last three passes, carried forward by growth.
- Spread glue ranks the crossing-removing port above repair batches.
- Storage ports (every pool, then the map) are ownership ports in the work
  order; pools and map access are C++ services only until their port.
- The goal is the complete rewrite in six overlapping phases; vendored
  libraries become Rust crates where one fits, and Squirrel waits for phase 6;
  screenshots and recordings attached to PRs are evidence for graphical work
  (user decisions, 2026-10-28).
- No C++ frame is kept for attribution.
- The review report cites #564's orphan check once it integrates.

The restart prompt now states the full-rewrite goal. It plans by phase,
keeping three ownership items in the current phase and the next phase queued,
and gives the planner no preferred candidate.

| Twenty-second correction | Verdict and cost/result |
| --- | --- |
| 1. Finish #518 after its infrastructure failure | Applied: one Full, merged. |
| 2. Observation, helper and terrain batch | Applied in #534/#538 (96 tooling lines). The repaired terrain fixture let the soak member pass 521 of 521. |
| 3. Resume #525 under #526 source research | Superseded: the user parked #525 on 2026-10-28 after a distribution question. No further cost. |
| 4. Tile-loop batch and anchor renewal | Applied: #536, a 144-game quiet pass and anchors via #543. That single pass put P2016 at 1.5229x; this review attributes the breach to inter-pass spread. About six attribution research tasks followed for #555's estimated 1.065%: costly for the credit, hence correction 4. |
| 5. #511 mutation study | Applied and closed: 41 minutes, 31 of 40 detected; it produced #558. |
| 6. #527 clock ownership | Applied in #539: 517 retired, 495 net glue, 504 tooling. No divergence; a bounded table-bounce repair is on #168. |
| 7. Placement and current artifacts | Applied: zero unplaced issues, and the planner checked current actions. |
| CI-found behavior fixes need a scoped review | Applied (#554 callback resolution, #552 export follow-up). |
| Deep checks every second integration | Worth its cost: about six hours of idle CPU per two-member set and one invalid mixed-source run, but it caught a real production ABI defect. Keep it with #557's rotation. |
| Preserve active speed dependencies | Applied: no anchor dependency was lost; host library drift was recovered. |

| User-directed change since the twenty-second steering | Verdict and next observation |
| --- | --- |
| T3 worktree isolation for code agents (#530) | Applied: children ran in T3 worktrees with no `cd` prefixes. Research and review worktrees never close, so 42 remain; #564 cleans them. |
| Long jobs in the background, no sleep or polling (#531) | Applied: root's log has no sleep calls, and agents ended turns while jobs ran. |
| Housekeeping at start and after integration (#532) | Ran 13 times but keeps detached and no-commit T3 worktrees and about 60 ad hoc `.local` files; #564 covers the worktrees. |
| Apt mirror retry (#529); host RPATH and `migration.py env` (#528) | Applied; #529 closed after ten qualifying installs. |
| Opus-only code under a Claude Code root | No Claude Code root session yet. The next steer checks authorship and cost. |
| Parked #525/#526 NewGRF corpus | User decision; stays excluded until the user reselects it. |

New since this review, for the next steer to judge:
- The three-pass record rule and the factor-2 projection. The anchors give wall/Ir excess ratios of 0.98 to 1.23 on the play saves, and `projection()` floors at the anchor, so it never shows a repair.
- Whether three concurrent leftover owners keep retirement per net glue near the planning audit's 2 to 5.
- Whether #563's design names a storage port with a measured Ir removal.
- Whether #564's check ends orphan findings at review.

## Earlier steering, still in force

Ownership, original behavior, process scope, named callees, typed shared services
and exact random-draw order come first. Preserve default semantic evidence,
record gaps on #156 and keep retirement distinct from glue. No hot allocation,
opcode dispatch, whole-record copies or owner bounces. Descriptor reads stay
within `docs/design/world-state.md`. Ready owners proceed beside speed work;
only timed collection holds builds and games. Compare one compatible anchor; the
projection factor and the speed thresholds remain provisional. Six branches,
batched maintenance, fixing reviews, strict exact-combination Full, bounded
rollback and action-based wind-down remain. A historical mention is not
placement; CI-found behavior fixes get a scoped review; deep checks run every
second integration; active speed dependencies are preserved. AGENTS.md carries
the rules; issue and PR bodies carry current actions and evidence.

## Recent completed work

Rows cover the period since the twenty-second steering; earlier rows are in
their PRs and Git history. Metrics are `tools/port-metrics.py`: Rust / tooling /
gross C++ glue / original C++ retired; net glue is separate.

| Issue | Component | PR | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #527 | Calendar/economy clocks and subscriptions | #539/#543 | `74cea95cf8` | 820 / 505 / 571 / 519; net glue 482 |
| #546 | Complete world-generation lifecycle | #547 | `591282249e` | 746 / 293 / 422 / 365; net glue 344 |

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #528 host RPATH and `migration.py env`; #529 apt retry; #531 `ci.py wait`; #532 housekeeping | #533 | direct | `f0a576839d` | 0 / 727 / 0 / 0; net glue 0 |
| #504 original simulation coverage | #518 | direct | `3b75d5c5fb` | 0 / 743 / 0 / 0; net glue 0 |
| #156/#503 observations and terrain; #199/#500 helpers and types | #534 | #538 | `db624a6f00` | 17 / 96 / 4 / 0; net glue 0 |
| #505 sanitizer game comparison and report identity | #535 | #538 | `db624a6f00` | 0 / 551 / 0 / 0; net glue 0 |
| #275/#312 tile ambient observations and orphan removal | #536 | #538 | `db624a6f00` | 151 / 17 / 4 / 0; net glue -1 |
| #509 prepared platform semantic comparison | #540 | direct | `c5b522f1eb` | 0 / 402 / 0 / 0; net glue 0 |
| #537 testing-off linkage; #168 typed effect mutations | #541 | #543 | `74cea95cf8` | 78 / 0 / 49 / 0; net glue 28 |
| #508 seeded random-command semantic scenarios | #544 | direct | `d26b9447bc` | 0 / 501 / 0 / 0; net glue 0 |
| #506 threaded semantic and C++ TSan evidence | #542 | direct | `18fb574219` | 0 / 933 / 0 / 0; net glue 0 |
| #507 community AI and larger vanilla save | #545 | direct | `a297bed748` | 0 / 440 / 0 / 0; net glue 0 |
| #168/#199/#500 payment boundaries; #503 evidence; #509 Windows diagnostics | #548 | direct | `467b89bcbd` | 113 / 378 / 140 / 0; net glue 68 |
| #510 mixed multiplayer semantic comparison | #549 | direct | `9e6d9d70e8` | 0 / 669 / 0 / 0; net glue 0 |
| #550 four boolean wire returns and shared house fixture | #552 | #556 | `3443388264` | 16 / 3 / 58 / 0; net glue 50 |
| #168/#199/#500 payment delivery; #501 execution typing | #553 | #556 | `3443388264` | 291 / 49 / 81 / 0; net glue -25 |
| #275 loading dispatch and acceptance traversal | #555 | direct | `06ed8c4a6f` | 58 / 30 / 0 / 0; net glue -3 |
| #156 differing town-buildable road input | #558 | direct | `c625d9d543` | 0 / 100 / 0 / 0; net glue 0 |


## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **#564 tooling batch, PR #567.** Independent review is complete, including
   root's dangling-runtime-link cleanup repair. Prepare the current-base Full
   with the documented workflow-change procedure, then integrate. The metric
   lands before the next ranking round, which reads it.
2. **#554/#557 integration.** The four #554 steering corrections are pushed;
   fixing review and remaining native/semantic checks are in progress. Preserve
   reviewed #557. Update the saved combination from current base, inspect
   conflicts, open its integration PR and run exact-head Full. Add no scope.
3. **#199 canonical map-header helper preparation.** Root's small PR supplies
   the helper path needed by #560 and #562, with an existing production map
   consumer. Both drafts adopt that integrated implementation and remove their
   copies. Reuse existing road-map helpers. This takes the next branch slot;
   it is selected support required by those owners, not a separate utility port.
4. **#560 track follower ownership, PR #569.** Continue the accepted first
   boundary (458 retired, net glue -18; legacy metric), finish fixture migration,
   source-helper adoption and default path/reservation/RNG evidence. Keep
   `GetPlatformLength` here. Review after completion.
5. **#562 station naming, tile loop and lifecycle, PR #568.** Continue the
   accepted boundary (424 retired, 179 net glue; legacy metric). Destructor
   ordering stays in Rust; the engine-runtime swap is unnecessary. Finish
   shared-helper adoption, direct callers and component scenarios before review.
6. **#565 map fields in place.** After the common helper preparation, the next
   free component slot converts the hottest typed map services (road/water,
   pixel height and slope) and deletes them. Leave `CFollowTrackT` to #560.
   Planning range: net glue -300 to -800.
7. **#570 first shared vehicle records.** #563's selected design is complete
   in world-state.md. Queue the complete movement/consist records, displaced
   reader deletion and direct Rust Tick dispatch as one ownership port. Root
   assigns it when a phase 2 branch and agent slot are available; no unmerged
   owner is its base. The design's net glue reduction is 500 to 1,100 lines;
   its 0.86% instruction credit is an estimate. Obtain #566 before integration.
8. **Phase 3 planning.** After #564 integrates, a fresh high-effort planner
   sizes save/load, script API/hosts, network and remaining NewGRF ownership,
   naming evidence for their outputs. Root creates issues for the top three.
   This remains the already-selected planning task; no branch.
9. **#566 drawing workload.** Run alongside #570 when a component slot frees;
   root owns assigning it. It supplies reference/candidate images and drawing
   instruction costs before the first storage port integrates. 150 to 250
   tooling lines.
10. **#561 company identity.** Start when a slot frees after item 2. Planning
    range: 450 to 550 retired, 120 to 200 net glue.
11. **#168 audit batch**, after #554 integrates: the checklist in the #168
    comment of 2026-10-28.
12. **#168/#199/#500 completed-job flow publication; #501 `tools/probe.py`
    typing.** Select only the completed link-graph job service table. Zero
    retirement, 120 to 250 Rust, -30 to +80 net glue.
13. **#503 coverage continuation.** The bounded original-only clock-preparation
    fix (at most 30 production and 50 test lines), then 119 unfinished traces
    with provenance checks. Preserve the fixed checkout and failed attempts.

Queued ownership candidates. Root creates each issue when it selects the
candidate; ranges come from the steering planning audit:
- Engine runtime: `Engine::Engine`, `GetCost`/`GetRunningCost`, `GetPower`/`GetRange`, `EngineOverrideManager`. 250 to 330 retired, 80 to 150 net glue. The constructor's `src/table/engines.h` copy stays behind a typed accessor or out of scope.
- Town helpers: `ClosestTownFromTile`, `UpdateTownMaxPass`, road-type introduction, `GetTownRadiusGroup`, `CleanUpRoadBits`, `GetClosestWaterDistance`. 250 to 330 retired, 60 to 120 net glue.
- Vehicle base lifecycle: `Crash`, `HandlePathfindingResult`, `PreDestructor`/`~Vehicle`, unit numbers, `SendToDepot`, shared-order links, the yearly timer. 450 to 550 retired, 200 to 300 net glue; sequence it with #563.

Queued storage ports, after #570; each gets a design section in
world-state.md and an issue when selected:
- The rest of the vehicle pool, after #563's per-tick records.
- Stations and road stops (`station_base.h`, `base_station_base.h`, `roadstop_base.h`), with cargo lists and flows.
- Orders and order lists (`order_base.h`).
- Towns and industries (`town.h`, `industry.h`), then companies, groups, engines and the smaller pools.
- The map's definitions and allocation (`map_func.h`, `tile_map.h`, `map.cpp`), last among storage. Rust already reads the map in place through #565, and C++ tile references stay valid over the one allocation. The `*_map.h` accessors retire with their last C++ caller.

Recurring:
- The instruction profile after integration 26 uses the fixed checkout below;
  read it and record all six projections. The next profile follows integration 28.
- After #557 integrates, launch the due rotated deep run under the existing lock at idle priority.
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- Root checks actual issue placement and exclusion conditions at stocktake.

Excluded, with return conditions checked by this review:
- #559 command invocation lifecycle returns when one of these happens:
  - a selected game owner records that it needs Rust-owned top-level `Do`/`Post` admission;
  - a #275 profile ranks top-level dispatch among the largest net costs;
  - the queued ownership candidates run out.
- #507's NewGRF corpus (#525, #526) is parked by the user (2026-10-28) until the
  user reselects it. The resume plan on #507 keeps the sets uncommitted,
  fetched by pinned hash into `.local/` and cached in CI; #525's branch stays
  on origin.
- #497 Action3 and #498 object definitions return with the storage port of the
  data they publish (Action3 bindings with the vehicle pool's storage, object
  definitions with the object pool's), or earlier if a selected owner needs
  them; none of #560 to #563 does.
- #69's remaining tile-list/support work is deferred until a selected simulation
  component needs it; catchment's rectangle/bitmap part is complete in #522.
- #64 and #68 hand-port monocypher algorithms. Under the vendored policy they
  become maintained Rust crates that reproduce the game's observable
  cryptography. They return when phase 3's network ports reach authentication,
  or with phase 6, whichever comes first. Phase 5 GUI and platform ownership
  under #3 uses PR screenshots and recordings as evidence; #509's completed validation and
  recurring cadence are unaffected.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until
  near-full reproduction under #513's original-default setting policy.
  #513 needs implementation when the first such improvement is accepted.

## Resume checkpoint

Resumed under Codex root (`gpt-6.1-sol`, xhigh). Integration 26 is
`c625d9d543` (#558). No confirmed code regression or rollback target exists.
Cadence and exact-head Full passed. Five component branches remain (#554,
#557, #567, #568 and #569); root's shared-helper PR takes the next slot. The
instruction profile is due and its fixed checkout is prepared. Deep remains
queued after #557. The current issue audit has no unplaced work after #570's
placement below.

| Item | Saved state | Next action on resume |
| --- | --- | --- |
| #564 / #567 | Reviewed head `cc7e8f8ca2`, including root's cleanup repair; no findings. Local regression/lint/type/preflight checks pass. | Current-base update and workflow-change Full, then integration. Root restores `CI_ON_DEMAND=true` after bootstrap validation/merge; PR records its live state. |
| #554 | Boundary corrections and narrow lint/ABI-fixture repairs pushed at `bd48a13bb9`. Fixing review and remaining validation run separately. | Read both results, spot-check fixes and use the final reviewed head in the combination. |
| #557 | Reviewed head `9db71fb247`; local checks and fixing review pass. No Full; behind base. | Keep with disjoint #554; preserve the same-revision ordinary-build prerequisite for sanitizer-only rounds. |
| #554/#557 combination | Saved unpushed branch `integrate/session-and-deep-rotation` in its retained worktree. | Reassemble the final heads on current base; inspect conflicts, open integration PR and request exact-head Full. |
| #199 helper preparation | Selected root item on #199; implementation not started. | Use the next branch slot; both owner drafts adopt its integrated canonical helpers. |
| #560 / #569 | Fixture migration at `a4c77d5df8`; Cargo/native checks running. First cost accepted. | Complete component evidence and common-helper adoption, then independent review. |
| #562 / #568 | First cost accepted at `1770ed92fb`; cleanup and scenario work continuing. | Finish direct callers, common helpers and evidence, then independent review. |
| #563 / #570 | Design accepted in world-state.md and first storage issue queued in item 7. | Close #563 once the docs commit is pushed. Assign #570 under the phase/slot order; #566 must precede storage integration. |
| #565, #566, #561 and phase 3 planning | Queued with their selection events in the work order. | Follow that order; the ranking task waits for the integrated #564 metric. |
| #503 | Coverage has 330 of 449 complete traces; clock-preparation fix not started. | Work order item 13. |
| Profiles and deep | Profile24 is the last completed instruction record. Profile26 checkout prepared at code `c625d9d543`. Deep rotation waits for #557. | Launch/read profile26, retain compatible anchors and record all six projections; then follow cadence. |

Worktrees kept for the unfinished tasks and their evidence:

| Purpose | Retained worktree(s) |
| --- | --- |
| Unpushed combination | `.local/worktrees/integration554557` |
| #554 older implementation and reviews | `/home/fetch/.t3/worktrees/openttd-rust/t3-50116b81`, `t3-d8fb8d59` and `t3-32d82fdf`; root's older validation is in `.local/worktrees/repair554-callback`. Fetch the reviewed head rather than trusting an older local branch. |
| #557 implementation and review | `/home/fetch/.t3/worktrees/openttd-rust/t3-a73f1edd` and `t3-dfe15b32` |
| #558 merged cleanup | `.local/worktrees/town-road-input` is clean but retained by the current tool because of a dangling harness link; #567 fixes that observed refusal. |
| Current implementations | #554: `/home/fetch/.t3/worktrees/openttd-rust/t3-e40e2681`; #567: `t3-e82b639d`; #568: `t3-070c9aac`; #569: `t3-2eb3d6fd`. |
| Instruction profile26 | `.local/worktrees/profile-after26-558`, detached at `c625d9d543` and locked while profiling. |
| Fixed original coverage | `/home/fetch/.t3/worktrees/openttd-rust/t3-117e7bcf` at `e52d475713` |
| Completed deep and instruction evidence | `.local/worktrees/deep-74cea95cf8`, `.local/worktrees/profile-after18-544`, `.local/worktrees/profile-after20-545`, and `/home/fetch/.t3/worktrees/openttd-rust/t3-1ef78f07` and `t3-e3e3aaeb` (profile24's six candidate profiles) |
| Merged-only cleanup | #555's final integration, implementation and review worktrees were removed by startup housekeeping. |

Relative project paths are under `/home/fetch/projects/openttd-rust`; shortened
T3 names share `/home/fetch/.t3/worktrees/openttd-rust/`. Other research
worktrees hold no evidence named here; #564 automates their removal.
