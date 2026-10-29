# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-28, code at `7105e24d49`)

Agent: /root | Model: gpt-6.1-sol | Reasoning effort: xhigh.

- 79 ownership ports. The integrated #564 counter is now the standard:
  Pygments 2.19.2 code lines, excluding comments, blanks and preprocessor-only
  lines. Whole fork: 50,111 of 284,355 original lines retired (17.62%);
  simulation source/header buckets: 37,599 of 61,148 (61.49%). Rust and
  tooling total 115,670 and 46,650 code lines. Historical completed-row metrics
  below retain their original counting method; do not add them to these totals.
- Distance to the complete rewrite: 234,244 original plus 32,299 fork glue is
  **266,543 compiled C++ code lines, 93.74% of the original**. The boundary is
  1,305 Rust C exports and 3,957 callback slots, including private and
  restricted-visibility definitions. These count interfaces, not dynamic calls.
- Integration 27 (#567) supplied code/area/distance/boundary metrics, shrinking
  orphan enforcement, conservative T3 cleanup and the three-pass speed record.
  Integration 28 (#571) supplied the canonical station/road/tunnel-bridge helper
  path for the two active owners: 72 Rust code lines and 4 net glue, no original
  retirement. Both passed independent review and current-base exact-head Full.
  On-demand CI is restored. Four component branches remain (#554, #557, #568,
  #569); #565 and #572 take the two available slots.
- The source buckets are not pure phase inventories. Logic's 12,124 remaining
  lines include at least 4,115 identified drawing/description/viewport body
  lines, other presentation and already-ported facade signatures. Storage's
  11,425 includes 321 executable follower-header lines. Script/network buckets
  include 931/2,862 GUI lines; excluding those gives 41,158 remaining original
  lines in the four phase-3 reader families. The fresh ranking corrected old
  engine/group double credit and reduced the vehicle-base lifecycle estimate
  from 450–550 to 190–240 original code lines.
- Speed uses the mean of the last three quiet passes, carried by the compatible
  instruction projection, with ceilings 1.6x play and 1.3x generation. Profile26
  matched all six comparisons; counts were below compatible anchor counts, so
  the projection floor applies. Every record is below its 1.52x/1.235x band:

| Scenario | Latest quiet anchor | Profile26 projection | Three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 1.1117x | 1.1117x | 1.1541x |
| Grok | 1.3871x | 1.3871x | 1.3857x |
| Opus | 1.3939x | 1.3939x | 1.3946x |
| P1996 | 1.3366x | 1.3366x | 1.3378x |
| P2000 | 1.4006x | 1.4006x | 1.4014x |
| P2016 | 1.5229x | 1.5229x | 1.4819x |

- These are projections/records, not new wall measurements. No quiet pass or
  extra attribution is selected. The integration-28 profile follows on fixed
  code `7105e24d49`; retain compatible anchors and candidate profiles. The due
  rotated deep run follows #557. Profile26 gives P2016 13,271,006,581 loop
  instructions; the retained profile attributed 99% of its original gap to
  widely spread boundary glue, so ownership/storage remains the useful repair.
- Evidence: original coverage retains 330 of 449 completed traces; the bounded
  original-only clock preparation and 119 unfinished traces remain selected.
  #558's differing-road input is integrated. The earlier deep sanitizer found
  production noncanonical bool returns and a fixture lifetime bug, fixed in
  #552; the previous soak passed 521 scenarios. No new deep result is claimed.
  #568 has 20 focused scenario passes before helper adoption; #569 still has
  fixture failures. #554's new internal-call cleanup passes focused checks
  and requires a fresh scoped review before its #557 combination.
- CI: #569 remains a draft. The cadence retry passed Windows x64/x86, macOS Debug/Release, Linux
  Clang/Dedicated and prepared Windows/macOS semantic comparisons. The next
  platform cadence follows integration 30. Failed follower checks are on its
  draft, not a merged-branch regression. All open issues are audited again at
  this stocktake; user-parked NewGRF corpus work remains parked.

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite (AGENTS.md "Migration method"). Each free
slot takes ready work from the earliest phase, and later phases fill the rest
(user decision, 2026-10-28). The navigation established the phases; #564 now
provides the standard source-area table at code `7105e24d49`. Source areas are
an accounting classification and do not move drawing or executable header
logic into a different phase.

| Area | Original code lines | Retired | Left | Glue added | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 35,300 | 12,124 | 6,767 | 18,891 |
| Simulation storage | 13,724 | 2,299 | 11,425 | 1,310 | 12,735 |
| NewGRF | 10,785 | 5,218 | 5,567 | 423 | 5,990 |
| Save/load | 13,388 | 536 | 12,852 | 585 | 13,437 |
| Script API | 14,908 | 878 | 14,030 | 453 | 14,483 |
| Network | 12,713 | 211 | 12,502 | 182 | 12,684 |
| Support | 21,617 | 665 | 20,952 | 424 | 21,376 |
| Graphics and viewport | 10,985 | 1 | 10,984 | 1 | 10,985 |
| GUI | 52,179 | 117 | 52,062 | 165 | 52,227 |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 522 | 54,626 | 163 | 54,789 |
| Upstream tests | 2,248 | 0 | 2,248 | 4,491 | 6,739 |
| Boundary layer | 0 | 0 | 0 | 17,333 | 17,333 |
| **Whole fork** | **284,355** | **50,111 (17.62%)** | **234,244** | **32,299** | **266,543** |

The 1,305 exports and 3,957 callback slots include restricted/private fields.
The earlier navigation's physical-line and manual area/rate estimates are
superseded by this code-line inventory; retired original and added glue remain
separate. The source tool does not preprocess every platform define.

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
| 1. Simulation logic | 12,124 in the logic source bucket, including drawing; executable headers add selected phase-1 bodies | No non-drawing function body of the inventory or the command families compiles under `WITH_RUST` | Nearly done: #560, #562, #561 and the queued candidates; #559 is excluded with its return condition |
| 2. Simulation storage | 11,425 in the storage source bucket, including some executable headers; credit only moved records/operations and deleted glue | Every pool record and the map arrays are defined once in Rust and read in place by both languages. C++ calls Rust only for operations, and no per-field service or per-call-site reader remains for moved data | Starts with #563's design and #565 |
| 3. Simulation data readers | 41,158 after excluding script/network GUI: save/load 12,852, script 13,099, network 9,640, NewGRF 5,567 | These run in Rust; the Squirrel VM may stay C++ until phase 6 | First planning complete; #573–#575 and definition owners queued |
| 4. Support | 31,936 support/graphics bucket lines; data tables (12,931) retire with their consumers | Not planned yet | Not planned until it is the next phase (user direction, 2026-10-28) |
| 5. Interface | 67,796 GUI/platform lines including script/network GUI, plus drawing in other buckets | Not planned yet | As above |
| 6. Vendored | 54,626 | Not planned yet | As above |

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
| #564 metrics, orphan enforcement, cleanup and speed records | #567 | direct | `d8704269c7` | Code: 0 / 1,013 / 0 / 0; net glue 0 |
| #199 canonical live map helpers | #571 | direct | `7105e24d49` | Code: 72 / 0 / 20 / 0; net glue 4 |


## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **Finish existing components in readiness order.** #554's internal clock/
   setup export cleanup is pushed at `fe7c4ec429`; Cargo/build/clocks/session
   pass and a fresh scoped review follows. Preserve reviewed #557, combine the
   final heads on current base, inspect conflicts and run exact-head Full.
   #568 adopts integrated #571, then gets independent review and Full. #569
   finishes its two remaining native fixture failures, adopts #571, completes
   evidence, then gets review and Full. No reviewed owner gains new scope.
2. **#565 map fields in place.** The common helper is integrated; assign the
   next available component slot now. Convert the hottest road/water, pixel
   height and slope crossings and delete their wrappers/slots. Keep follower
   ownership in #560, reuse the integrated typed helpers, and coordinate shared
   files so no parallel service copy is built. Planning net glue -300 to -800.
3. **#561 company identity.** Start after the #554/#557 combination, preserving
   the session/startup boundary sequence. Updated code estimate: 350–430 retired,
   80–180 net glue, 80–160 evidence/tooling. Group livery and engine mapping
   already run in Rust and get no second retirement credit.
4. **#572 goals/story/league ownership.** Selected for the other free phase-1
   component slot while item 3 waits. Complete command/event/lifecycle policy;
   518 original source code lines, estimated 490–530 retired and 120–230 net
   glue. Required default command-result/GS-event evidence is in the issue.
5. **#168 clock/world audit batch**, after #554 integrates. Bound it to the
   current checklist; fix bounces, the duplicate company writer and obsolete
   exports with the integrated ratchet. Add #156's ordinary single-player
   completion observation if ready; it does not hold the batch.
6. **Queued phase-1 owners under #3.** Root issues each before implementation:
   world-input generation (heightmap input and town-data import, after item 5),
   complete NewGRF town-name storage/generation plus ActionF, clear-ground
   clearing, editor tree-group mutation, sign/depot naming and simulation-setting
   transitions. Measure real remaining bodies; no utility helper fallback.
   Vehicle base lifecycle is sequenced **after #570**, using its canonical
   movement/consist record contract; estimate 190–240 retired and 70–160 net
   glue. It never runs concurrently in the overlapping vehicle files.
7. **#566 drawing workload and #570 first shared vehicle records.** Assign
   phase 2 when earlier ready phase-1 items leave a slot. #570 follows the
   accepted world-state design and deletes replaced readers plus direct Tick
   bounces; estimated net glue -500 to -1,100, instruction credit 0.86% is a
   retained-profile model, not a measurement. Start #566 beside it and obtain
   its reference/candidate pixels and drawing Ir before storage integration.
   Drawing tooling estimate remains 150–250 lines.
8. **Queued later phase-2 pools.** Remaining vehicle pool, then stations/road
   stops, orders/lists, towns/industries, companies/groups/engines and smaller
   pools, then map allocation/definitions. Move coupled save adapters with each
   record owner, generate native declarations and verify layout, and delete
   displaced readers/services. Native inline accessors retire with consumers;
   source-header totals are not a retirement promise. Each gets an issue and
   design section when selected. Object instances are distinct from ObjectSpec.
9. **Phase-3 reader queue.** The fresh #564-based survey is complete. Queue
   #573 codec/chunk execution, #574 game network protocol/queues and #575 script
   host/commands/events, with explicit unsaved-output evidence in each issue.
   Respect earliest-ready phase order and stable affected save/session contracts;
   none is implementation authorization yet. Follow with complete NewGRF text,
   config/Action14 and definition ownership: #497 includes its canonical engine/
   feature binding payloads, #498 its ObjectSpec payloads. These are queued joint
   owners, not parsers deferred behind unrelated instance pools. Remaining
   afterload/legacy, chunk, script API/config and other network/NewGRF families
   remain reader selections under #3. Plan no phases 4–6 or Squirrel port.
10. **#168/#199/#500 completed-job flow publication; #501 probe typing.**
    Select the existing completed link-graph job service table and bounded
    tools/probe.py typing only. Zero retirement; estimated -30 to +80 net glue.
11. **#503 coverage continuation.** The bounded original-only clock-preparation
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

## Resume checkpoint (2026-10-28, integration 28)

Integrated code is `7105e24d49`; this stocktake selects #565/#572 for the two
free slots. The four existing components remain #554, #557, #568 and #569.
Root serializes final integration; ordinary issue/PR status carries detailed
validation and the single task comments carry running checks.

| Item | Saved state | Next action on resume |
| --- | --- | --- |
| #564 / #567 and #199 / #571 | Integrated 27/28 at `d8704269c7` / `7105e24d49`, reviewed and exact-head Full passed. CI_ON_DEMAND restored; watches off. | Verified merged-only cleanup; no further source work. |
| #554 | Root repair `fe7c4ec429` pushed; Cargo/build and clocks/session semantics pass. | Fresh scoped review, then combine its final head with reviewed #557. |
| #557 | Reviewed `9db71fb247`; no Full; behind base. | Preserve ordinary-build prerequisite and combine with final #554. |
| #554/#557 combination | Saved unpushed older branch/worktree remains. | Reassemble on current base; inspect source conflicts; integration PR and exact-head Full. |
| #560 / #569 | Root checkpoint `d9c4f42d2f`; Python checks/preflight pass; 214/216 native pass, two fixture cases still fail. Semantics unrun. | Finish fixture fidelity, adopt integrated #571, evidence, metrics and independent review. |
| #562 / #568 | Handoff `bd54247bee`; 20 affected scenarios and targeted checks pass. | Adopt #571, run touched checks, then independent review and Full. |
| #565 / #572 | Selected for the two available component slots. | Assign map-boundary maintenance and the complete objectives owner; keep at most six branches. |
| #561 | Corrected sizing and existing group-owner reuse; queued after #554/#557. | Start when that combination integrates and a slot is available. |
| #563 / #570 / #566 | Design accepted in world-state.md; #563 closed; records and drawing queued. | Assign under earliest-ready phase order; #566 before storage integration. |
| #573 / #574 / #575; #497 / #498 | Phase-3 issues queued from the fresh source-sized survey; definition issues include canonical payload ownership. | No implementation yet; select disjoint complete owners when earlier phases leave slots. |
| #503 | 330 of 449 original traces complete; bounded clock preparation unstarted. | Work order item 11; deep rotation after #557. |
| Profiles | Profile26 complete, all records below band; profile28 source fixed at `7105e24d49`. | Launch/read profile28; retain compatible anchors and candidate traces. No quiet pass due. |

Worktrees kept for the unfinished tasks and their evidence:

| Purpose | Retained worktree(s) |
| --- | --- |
| Unpushed combination | `.local/worktrees/integration554557` |
| #554 older implementation and reviews | `/home/fetch/.t3/worktrees/openttd-rust/t3-50116b81`, `t3-d8fb8d59` and `t3-32d82fdf`; root's older validation is in `.local/worktrees/repair554-callback`. Fetch the reviewed head rather than trusting an older local branch. |
| #557 implementation and review | `/home/fetch/.t3/worktrees/openttd-rust/t3-a73f1edd` and `t3-dfe15b32` |
| Current implementations | #554: `/home/fetch/.t3/worktrees/openttd-rust/t3-e40e2681`; #568: `t3-070c9aac`; #569: `t3-2eb3d6fd`. |
| Instruction profiles26/28 | `.local/worktrees/profile-after26-558` retains the completed profile; `.local/worktrees/profile-after28-571` is the next fixed source at `7105e24d49`. |
| Fixed original coverage | `/home/fetch/.t3/worktrees/openttd-rust/t3-117e7bcf` at `e52d475713` |
| Completed deep and instruction evidence | `.local/worktrees/deep-74cea95cf8`, `.local/worktrees/profile-after18-544`, `.local/worktrees/profile-after20-545`, and `/home/fetch/.t3/worktrees/openttd-rust/t3-1ef78f07` and `t3-e3e3aaeb` (profile24's six candidate profiles) |
| Merged-only cleanup | #555's final integration, implementation and review worktrees were removed by startup housekeeping. |

Relative project paths are under `/home/fetch/projects/openttd-rust`; shortened
T3 names share `/home/fetch/.t3/worktrees/openttd-rust/`. Other research
worktrees hold no evidence named here. Integrated housekeeping remains conservative
when foreign process working directories cannot be inspected; do not weaken it
to force removal.
