# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-29, code at `06a644505a`)

Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh.

- Integration 29 landed the reviewed session-lifecycle owner and deep-rotation
  tooling through #577 at `7473e63624`. Integration 30 landed the complete
  track follower and platform scans through #569 at `06a644505a`. Both passed
  current-base exact-head Full; #569's merged tree equals its validated tree.
  No confirmed merged-branch regression or rollback occurred. Details and
  head-specific evidence remain in the linked PRs.

| Integration | Issues / PR | Merge commit | Original C++ retired | Rust added | Net C++ glue | Tooling added |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| 29 | #551 / #554 and #503 / #557, combined in [#577](https://github.com/fetch-laboratories/openttd-rust/pull/577) | `7473e63624` | 429 | 1,118 | +319 | 919 |
| 30 | #560 / [#569](https://github.com/fetch-laboratories/openttd-rust/pull/569) | `06a644505a` | 362 | 1,234 | -30 | 205 |

- These are existing per-PR code metrics against their respective bases. The
  #569 figures were measured at `90d995522c`; its later repairs only added or
  reordered include directives. No measurement was launched for wind-down.
  The last whole-fork inventory remains integration 28 at `7105e24d49`, using
  Pygments 2.19.2 and excluding comments, blanks and preprocessor-only lines:
  79 ownership ports then; 50,111 of 284,355 original lines retired (17.62%);
  simulation source/header buckets 37,599 of 61,148 (61.49%); Rust 115,670 and
  tooling 46,650 code lines. Historical completed rows retain their counting
  method and are not additive whole-fork inventories.
- That last inventory's distance was 234,244 original plus 32,299 fork glue:
  **266,543 compiled C++ code lines, 93.74% of the original**. Its boundary was
  1,305 Rust C exports and 3,957 callback slots, including private/restricted
  definitions. Navigation recounted the whole fork at `06a644505a`, after
  integrations 29 and 30; the "Phases" chart below holds that inventory.
- Source buckets are not pure phase inventories. At that inventory, logic's
  12,124 remaining lines included at least 4,115 drawing/description/viewport
  body lines and other presentation/facade signatures. Storage's 11,425
  included 321 executable follower-header lines, now addressed by #569.
  Script/network buckets included 931/2,862 GUI lines; excluding those gave
  41,158 remaining original lines in the four phase-3 reader families. The
  earlier source-sized ranking corrected engine/group double credit and put
  remaining vehicle-base lifecycle at 190–240 original code lines.
- Five unintegrated component branches remain: #568, #576, #578, #579 and #580.
  All are paused at the user's request; their heads and next actions are in
  the checkpoint. No new component, review, planner or repair round was
  started under wind-down. #579 has not received independent review.
- Speed still uses the mean of three quiet passes carried by the compatible
  instruction projection, with ceilings 1.6x play and 1.3x generation. The
  latest completed profile is profile28 on `7105e24d49`: all six comparisons
  matched, counts were below compatible anchors, and the projection floor
  applied. Those decision records were below the 1.52x/1.235x bands:

| Scenario | Latest quiet anchor | Profile28 projection | Three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 1.1117x | 1.1117x | 1.1541x |
| Grok | 1.3871x | 1.3871x | 1.3857x |
| Opus | 1.3939x | 1.3939x | 1.3946x |
| P1996 | 1.3366x | 1.3366x | 1.3378x |
| P2000 | 1.4006x | 1.4006x | 1.4014x |
| P2016 | 1.5229x | 1.5229x | 1.4819x |

- These are retained records/projections, not new wall measurements or a
  projection for integration 30. Retain compatible anchors and candidate
  profiles. Integration 30 makes the instruction profile, next deep check and
  fifth-integration platform cadence due; none was launched during wind-down.
  Read the existing deep result before starting another deep run.
- The existing idle-priority `simulate-sanitized` deep member remains running
  on fixed code `7473e63624` in `.local/worktrees/deep-after29-577`. No finished
  report was available at this checkpoint; no pass is claimed. It is a rotated
  member, not completion of all five deep members. Leave it running and record
  its result when it finishes.
- Original coverage remains 330 of 449 completed traces; bounded original-only
  clock preparation and 119 unfinished traces remain selected. The earlier
  noncanonical-bool/fixture sanitizer defects were fixed in #552; the previous
  soak passed 521 scenarios. Current source/evidence gaps remain on #156.
- Required Full passed for #577 and #569. #569's earlier candidate failures
  were repaired before its final Full and were not regressions on the dispatch
  base `7473e63624`. No manual CI request remains pending. Automatic
  post-merge/docs-push CI is not awaited here; inspect its state on resume.
  The wider platform cadence is due after integration 30 and was not requested.

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
1. #581 at the top of the work order. Its first PR turns on Rust in the Linux
   jobs (a flag) and adds the Emscripten target; its second removes
   `OPTION_RUST` and runs the `unifdef` sweep. Drafts adopt the sweep by
   rerunning its recorded command (AGENTS.md, step 3).
2. #570 starts after the sweep integrates. #570 also makes
   `tools/port-metrics.py` count the C++ declarations it generates.
3. Phase 2 takes every slot that ready phase 1 work leaves free, ranked by code
   retired, crossings removed and glue deleted.
4. Before each stocktake and planning round, reread AGENTS.md, this chart and
   the latest steering section if they changed (`/start-development`, step 4).

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
     the evidence that shows its output (work order item 10). The first phase 3
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
| #564 metrics, orphan enforcement, cleanup and speed records | #567 | direct | `d8704269c7` | Code: 0 / 1,013 / 0 / 0; net glue 0 |
| #199 canonical live map helpers | #571 | direct | `7105e24d49` | Code: 72 / 0 / 20 / 0; net glue 4 |


## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **#581 Rust on every CI configuration and retirement of the portable
   fallback.** Navigation item, ranked first. First PR: `-DOPTION_RUST=ON` in
   the Linux GCC SDL2, Clang and Dedicated jobs and the `wasm32-unknown-emscripten`
   target for Emscripten. Second PR: remove `OPTION_RUST` and run the recorded
   `unifdef -DWITH_RUST -m` sweep. Drafts adopt it by rerunning that command
   (AGENTS.md step 3). Expected tooling about 150 CI/CMake lines; zero retirement
   change, since guarded bodies already count as retired.
2. **Finish existing components in readiness order.** #554's internal clock/
   setup cleanup passed scoped review; its reviewed #557 combination is #577.
   Repair the standalone probe link failure, then run current-base exact-head
   Full again. #568 adopted #571 and completed source review with one accessor
   fix; inspect its checks before final base adoption and Full. #569 adopted
   #571 and repaired its fixtures; finish verification, evidence and review.
   No reviewed owner gains new scope.
3. **#565 map fields in place.** The common helper is integrated; assign the
   next available component slot now. Convert the hottest road/water, pixel
   height and slope crossings and delete their wrappers/slots. Keep follower
   ownership in #560, reuse the integrated typed helpers, and coordinate shared
   files so no parallel service copy is built. Planning net glue -300 to -800.
4. **#561 company identity.** Start after the #554/#557 combination, preserving
   the session/startup boundary sequence. Updated code estimate: 350–430 retired,
   80–180 net glue, 80–160 evidence/tooling. Group livery and engine mapping
   already run in Rust and get no second retirement credit.
5. **#572 goals/story/league ownership.** Selected for the other free phase-1
   component slot while item 4 waits. Complete command/event/lifecycle policy;
   518 original source code lines, estimated 490–530 retired and 120–230 net
   glue. Required default command-result/GS-event evidence is in the issue.
6. **#168 clock/world audit batch**, after #554 integrates. Bound it to the
   current checklist; fix bounces, the duplicate company writer and obsolete
   exports with the integrated ratchet. Add #156's ordinary single-player
   completion observation if ready; it does not hold the batch.
7. **Queued phase-1 owners under #3.** Root issues each before implementation:
   world-input generation (heightmap input and town-data import, after item 6),
   complete NewGRF town-name storage/generation plus ActionF, clear-ground
   clearing, editor tree-group mutation, sign/depot naming and simulation-setting
   transitions. Measure real remaining bodies; no utility helper fallback.
   Vehicle base lifecycle is sequenced **after #570**, using its canonical
   movement/consist record contract; estimate 190–240 retired and 70–160 net
   glue. It never runs concurrently in the overlapping vehicle files.
8. **#566 drawing workload and #570 first shared vehicle records.** Assign
   phase 2 to every slot that ready phase-1 items leave free; #570 starts after
   #581's sweep integrates and makes `tools/port-metrics.py` count the C++
   declarations it generates. #570 follows the
   accepted world-state design and deletes replaced readers plus direct Tick
   bounces; estimated net glue -500 to -1,100, instruction credit 0.86% is a
   retained-profile model, not a measurement. Start #566 beside it and obtain
   its reference/candidate pixels and drawing Ir before storage integration.
   Drawing tooling estimate remains 150–250 lines.
9. **Queued later phase-2 pools.** Remaining vehicle pool, then stations/road
   stops, orders/lists, towns/industries, companies/groups/engines and smaller
   pools, then map allocation/definitions. Move coupled save adapters with each
   record owner, generate native declarations and verify layout, and delete
   displaced readers/services. Native inline accessors retire with consumers;
   source-header totals are not a retirement promise. Each gets an issue and
   design section when selected. Object instances are distinct from ObjectSpec.
10. **Phase-3 reader queue.** The fresh #564-based survey is complete. Queue
   #573 codec/chunk execution, #574 game network protocol/queues and #575 script
   host/commands/events, with explicit unsaved-output evidence in each issue.
   Respect earliest-ready phase order and stable affected save/session contracts;
   none is implementation authorization yet. Follow with complete NewGRF text,
   config/Action14 and definition ownership: #497 includes its canonical engine/
   feature binding payloads, #498 its ObjectSpec payloads. These are queued joint
   owners, not parsers deferred behind unrelated instance pools. Remaining
   afterload/legacy, chunk, script API/config and other network/NewGRF families
   remain reader selections under #3. Plan no phases 4–6 or Squirrel port.
11. **#168/#199/#500 completed-job flow publication; #501 probe typing.**
    Select the existing completed link-graph job service table and bounded
    tools/probe.py typing only. Zero retirement; estimated -30 to +80 net glue.
12. **#503 coverage continuation.** The bounded original-only clock-preparation
    fix (at most 30 production and 50 test lines), then 119 unfinished traces
    with provenance checks against the retained 330. Preserve the fixed checkout
    and failed attempts; no recollection solely to fill a slot.

Recurring:
- Profile26 completed and its six projections/records are above. Launch the due
  integration-28 profile on fixed `7105e24d49`; the next follows integration 30.
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

## Resume checkpoint (2026-10-29, integration 30)

Paused at the user's request. Integrated code is `06a644505a`: #577 at
`7473e63624` is integration 29, and #569 is integration 30 after successful
current-base exact-head Full. No rollback was needed. Five component branches
remain; all unfinished worktrees and branches are preserved. The objectives
owner saved its work and was cancelled; no company-identity reviewer started.

| Remaining draft | Pushed head | Next action on resume |
| --- | --- | --- |
| #562 / #568 | `d6738e9d4eafa1f1e7cc3c5693f94c10bb8f0c14` | Source review and targeted compilation/boundary checks are complete. Resolve the base conflict, obtain a fresh conflict review, then request current-base exact-head Full. |
| #565 / #576 | `6dfe4a45797d97cd909a305c77bb9dd28c5d736b` | Final reviewer validation was read and recorded on the PR: affected Rust/native/landscape checks pass. Update from base, preserving both #569's follower fixture and the canonical map inputs in `src/tests/road_yapf.cpp`; fresh conflict review if it conflicts, then Full. Remaining #565 scope stays open. |
| #572 / #578 | `14b99180cf88da722a1d0ea3424d9ea6e2e94673` | Story counters now live in Rust; native/Cargo evidence is tied to `b602dc3a79`, with later fixture/storage checks recorded in the PR. Objectives comparison after fixture repairs is unrun; final metrics/preflight remain pending. Run it, handle actual failures and the base conflict, then fresh independent review and Full. Owner stopped; no running job. |
| #561 / #579 | `6046787dd506f378b5456a7c2f211d185e6f5dd3` | Implementation/local validation complete. Start a fresh independent complete-owner review, then final base adoption and Full. No review was started during wind-down. |
| #168 / #580 | `d34f1c8799ece3566f8f80ed23074de554de1fc1` | Rust/native/focused semantic checks pass. Adopt #569's now-integrated canonical relinking helper, finish generation-abort fixture migration/export retirement and related audits, then independent review and Full. No running job. |

| Running or due work | Saved state / next action |
| --- | --- |
| Existing #503 deep check | Idle-priority `simulate-sanitized` member on `7473e63624`, running since about 04:50 UTC in `.local/worktrees/deep-after29-577`. Leave it running; read and record its finished report. No finished report was available when this checkpoint was written. |
| Instruction profile after integration 30 | Due, not launched. Retain profile28/compatible anchors and candidate traces; take the new merged-tip profile on resume. No new speed result is claimed. |
| Next deep rotation | Due, not launched. Read the running member's result and avoid starting a duplicate while it runs. |
| Fifth-integration platform cadence | Due after integration 30, not requested during wind-down. |
| CI | #569's permitted final Full passed and its tested combination merged; no manual request remains pending. Automatic post-merge/docs-push checks are not awaited and their later state is unknown here. |

Keep the pinned reference unchanged. Preserve unfinished checkouts and existing
profile/deep evidence. The work order and all other roadmap sections were left
unchanged during this wind-down; navigation/steering resumes from this checkpoint.
