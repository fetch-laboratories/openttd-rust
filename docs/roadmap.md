# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-28, code at `06ed8c4a6f`)

- 79 ownership ports. Whole-fork inventory: 72,102 of 470,612 original lines
  retired (15.32%), including 56,403 of 103,451 simulation lines (54.52%). Rust,
  tooling and glue total 129,354, 51,058 and 46,170 lines.
- What the remaining 47,835 simulation-inventory lines are: 26,113 blank,
  comment or preprocessor lines; 10,464 header code (class layouts and inline
  accessors, which retire only when their data and C++ callers move); 11,258
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
  scenarios. #511's mutation study detected 31 of 40 mutants; #558 covers one
  survivor. Original coverage recollection reached 330 of 449 traces.
- CI: the fifth-integration cadence run `40719341427` at `06ed8c4a6f` failed on
  infrastructure, not code. Both Windows jobs failed downloading libogg for
  vcpkg (HTTP 502 from gitlab.xiph.org), and the macOS jobs got no hosted
  runner. No rollback applies.

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
3. **Design the first shared-storage port (#563).** A planning agent writes the
   world-state design note now and opens the implementation issue. Per-tick
   vehicle records and the `X::Tick` dispatch shells come first, because their
   glue dominates P2016's gap. This applies world-state.md's existing condition
   ("when measured coupling warrants it"); it is not a new direction.
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
8. **Cadence.** Re-dispatch the cadence run on the current tip and read it
   before the next integration. If macOS gets no runner again, that is its
   second hosted-capacity cancellation and the `CI_MACOS_HEAVY` rule applies. A
   repeat of the vcpkg 502 selects a vcpkg binary cache, not retries.

AGENTS.md changes:
- The play-save ceiling is 1.6x (user decision); ceiling and band decisions
  use the mean of the last three passes, carried forward by growth.
- Spread glue ranks the crossing-removing port above repair batches.
- No C++ frame is kept for attribution.
- The review report cites #564's orphan check once it integrates.

The restart prompt now plans when fewer than three ownership items remain,
includes storage ports, and gives the planner no preferred candidate.

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

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **Cadence re-dispatch.** Dispatch `ci-build.yml` on the current tip and read
   its Windows/macOS semantic and platform results before the next
   integration. Fix real failures forward. Infrastructure failures follow
   correction 8.
2. **#554 boundary fixes, then the #554/#557 integration.** Apply the PR
   comment's four items. A fresh fixing review checks only those fixes. Update
   the saved combination (`integrate/session-and-deep-rotation`) from current
   base, open its integration PR and run exact-head Full. Nothing else is added
   to its scope.
3. **#558 differing-road-type town growth.** Fresh fixing review and final Full;
   the 100 tooling lines are already accepted.
4. **#560 track follower ownership.** Start now. Planning range: about 430
   retired, net glue -50 to +80. It removes the road YAPF follow crossing
   (1.29% of P2016; `RoadTile` and `RoadTraversal` are 3.6% of Grok).
5. **#562 station naming, tile loop and lifecycle.** Start now, concurrently
   with #560. Planning range: 450 to 500 retired, 150 to 220 net glue. If its
   first checkpoint shows costly destructor glue, swap the slot to engine runtime
   (queued below) rather than shrinking scope.
6. **#563 vehicle storage design.** A planning agent at high effort produces the
   design note and the first storage implementation issue. No branch.
7. **#561 company identity.** Start when a slot frees after item 2. Planning
   range: 450 to 550 retired, 120 to 200 net glue.
8. **#168 audit batch**, after #554 integrates: the checklist in the #168
   comment of 2026-10-28.
9. **#564 tooling batch:** the orphan-export check, T3 task-worktree cleanup,
   and the 1.6x ceiling with the three-pass record in `speed_budget`.
10. **#168/#199/#500 completed-job flow publication; #501 `tools/probe.py`
    typing.** Unchanged: select only the completed link-graph job service
    table. Zero retirement, 120 to 250 Rust, -30 to +80 net glue.
11. **#503 coverage continuation.** The bounded original-only clock-preparation
    fix (at most 30 production and 50 test lines), then the 119 unfinished
    traces with provenance checks. Preserve the fixed checkout and failed
    attempts.

Queued ownership candidates. Root creates each issue when it selects the
candidate; ranges come from the steering planning audit:
- Engine runtime: `Engine::Engine`, `GetCost`/`GetRunningCost`, `GetPower`/`GetRange`, `EngineOverrideManager`. 250 to 330 retired, 80 to 150 net glue. The constructor's `src/table/engines.h` copy stays behind a typed accessor or out of scope.
- Town helpers: `ClosestTownFromTile`, `UpdateTownMaxPass`, road-type introduction, `GetTownRadiusGroup`, `CleanUpRoadBits`, `GetClosestWaterDistance`. 250 to 330 retired, 60 to 120 net glue.
- Vehicle base lifecycle: `Crash`, `HandlePathfindingResult`, `PreDestructor`/`~Vehicle`, unit numbers, `SendToDepot`, shared-order links, the yearly timer. 450 to 550 retired, 200 to 300 net glue; sequence it with #563.

Recurring:
- The next instruction profile follows the twenty-sixth integration.
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
- #497 Action3 and #498 object definitions remain parked until a selected
  simulation component needs their data in Rust; none of #560 to #563 does.
- #69's remaining tile-list/support work is deferred until a selected simulation
  component needs it; catchment's rectangle/bitmap part is complete in #522.
- #64 and #68 are vendored cryptography, deferred until after simulation
  ownership. New GUI/platform ownership under #3 follows simulation; #509's
  completed validation and recurring cadence are not excluded by that policy.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until
  near-full reproduction under #513's original-default setting policy.
  #513 needs implementation when the first such improvement is accepted.

## Resume checkpoint

Paused at the user's request. Code remains at `06ed8c4a6f` (#555, the
twenty-fifth integration). No confirmed code regression or rollback target
exists. No subagent, build, simulation, deep check or instruction profile is
running.

| Item | Saved state | Next action on resume |
| --- | --- | --- |
| Cadence | Run `40719341427` at `06ed8c4a6f` failed on infrastructure: vcpkg libogg 502 on both Windows jobs, and no macOS runner. | Re-dispatch (work order item 1) and read it before any further integration. |
| #554 | Reviewed head `6058f4b898`; no Full at any head; branch is behind. The steering audit's four boundary items must be fixed first. | Apply the PR comment's items, run a fresh fixing review, then go to the combination row. |
| #557 | Reviewed head `9db71fb247`; local checks and fixing review pass. No Full at any head; branch is behind. | Keep it with disjoint #554 in the combination. Keep the same-revision ordinary-build prerequisite for sanitizer-only rounds. |
| #554/#557 combination | Unpushed branch `integrate/session-and-deep-rotation`, head `e9b72a04ec`, in `.local/worktrees/integration554557`. | After #554's fixes, update from current base, inspect conflicts, open the integration PR and run Full for its final head. |
| #558 | Draft head `c919a1a298`. The ordinary semantic case and isolated M23 sensitivity pass. No fresh fixing review and no Full yet. | Fresh fixing review, current base, exact-head Full. |
| #560, #561, #562, #563 | Selected by this review; not started. | Follow the work order. |
| #503 | Coverage has 330 of 449 complete traces; the clock-preparation fix is not started. | Work order item 11. |
| Profiles and deep | Profile24 complete. The rotated deep run is due after #557. The next profile follows the twenty-sixth integration. | Keep due work pending and preserve anchor and runtime dependencies. |

Worktrees kept for the unfinished tasks and their evidence:

| Purpose | Retained worktree(s) |
| --- | --- |
| Unpushed combination | `.local/worktrees/integration554557` |
| #554 implementation and reviews | `/home/fetch/.t3/worktrees/openttd-rust/t3-50116b81`, `t3-d8fb8d59` and `t3-32d82fdf`; root's older validation is in `.local/worktrees/repair554-callback`. Fetch the reviewed head rather than trusting an older local branch. |
| #557 implementation and review | `/home/fetch/.t3/worktrees/openttd-rust/t3-a73f1edd` and `t3-dfe15b32` |
| #558 implementation, passing case and retained failures | `.local/worktrees/town-road-input` |
| Fixed original coverage | `/home/fetch/.t3/worktrees/openttd-rust/t3-117e7bcf` at `e52d475713` |
| Completed deep and instruction evidence | `.local/worktrees/deep-74cea95cf8`, `.local/worktrees/profile-after18-544`, `.local/worktrees/profile-after20-545`, and `/home/fetch/.t3/worktrees/openttd-rust/t3-1ef78f07` and `t3-e3e3aaeb` (profile24's six candidate profiles) |
| Merged-only cleanup | #555's `.local/worktrees/final555`, `/home/fetch/.t3/worktrees/openttd-rust/t3-44edd90d` and `t3-c2c51eb8`; housekeeping removes them on resume. |

Relative project paths are under `/home/fetch/projects/openttd-rust`; shortened
T3 names share `/home/fetch/.t3/worktrees/openttd-rust/`. Other research
worktrees hold no evidence named here; #564 automates their removal.
