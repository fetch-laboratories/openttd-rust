# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-02, twenty-eighth steering at integration 65)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

- Code is `6afdcbf390` (integration 65). Root has been paused since the wind-down at 16:20Z. The 11-02 session integrated #648 (61), #645 (62), #640 (63, phase 1's exit), #642 (64) and #651 (65), with no rollbacks.
- Whole fork at `6afdcbf390`, as in the Phases chart:
  - 56,176 of 284,355 original code lines retired (19.76%);
  - compiled C++ 262,065 (92.16%) and glue 33,886;
  - 1,443 exports and 4,064 callback slots;
  - Rust 127,673 and tooling 53,611.
  Stocktake 64 (`5e414c5a64`) is in Git history (`dca42d6c02`); stocktake 66 has not run.
- Compiled C++ is undercounted: `tools/port-metrics.py` skips `.inc` files, and three `src/rust/*.inc` files hold 487 physical lines of fork glue. #658 fixes the count; record the step as a re-baseline.
- Integrations 61 to 65 removed 2,437 compiled lines, 2,310 of them from #642's vendored swap. The other four removed 127 lines and added 1,507 Rust and 826 tooling lines.
- Open drafts at their heads (`python3 tools/port-metrics.py --head HEAD`, steering measurement):

| PR (issue) | Head | Retired | Net glue | Compiled C++ | Rust | Tooling |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| #655 (#652) | `54c44c68c5` | 752 | +113 | -639 | 1,756 | 584 |
| #636 (#570) | `6191d5f9b0` | 742 | +365 | -377 | 1,710 | 517 |
| #649 (#604, #199) | `ba07c85bb1` | 453 | +428 | -25 | 437 | 118 |
| #628 (#574) | `2711496991` | 1,940 | +2,002 | +62 | 5,291 | 60 |
| #647 (#644, #646) | `9e0d5af362` | 0 | +173 | +173 | 870 | 214 |

- Speed: the table is profile 64 at `5e414c5a64`, flat against profile 62 (-0.11% to +0.07% Ir). Every scenario stays 0.12% to 0.90% under its anchor count. The drawing workload (#566) runs at 1.0027x reference Ir. The instruction harness rejects the anchors because #642 changed `rust/openttd-kernels/Cargo.toml`, part of the build identity. Projections use the speed formula by hand until a quiet speed pass re-anchors (recurring list).

| Scenario | Candidate Ir, profile 64 | Projection | Carried three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 2,731,301,085 | 1.111722x | 1.154106x |
| Grok | 26,898,981,923 | 1.387147x | 1.385653x |
| Opus | 28,297,683,609 | 1.393949x | 1.394556x |
| P1996 | 4,542,468,375 | 1.336551x | 1.337764x |
| P2000 | 7,400,813,786 | 1.400577x | 1.401446x |
| P2016 | 13,400,911,478 | 1.522911x | 1.481910x |

- **The platform cadence after integration 65 is red** (run 41466312349 at `6afdcbf390`, both attempts).
  - macOS Debug (attempt 1) and macOS Release (attempt 2) fail `tools/check-rust-platform.py:146-156`: "rust-build.log: native libraries differ from the configure query". The Windows and annotation failures cascade from it.
  - The steering read the Release job log. The build prints `native-static-libs: -lc -lm -liconv -lSystem -lc -lm`.
  - The configure probe (`cmake/Rust.cmake:137-156`) compiles a staticlib with no dependencies and writes its line to `rust-native-libs.log`, which the job log does not show.
  - Root's diagnosis stands: `libc` reached `Cargo.lock` through `cpufeatures` in #642's merge `5e414c5a64`.
  - #651's code merge follows it, so the bounded rollback is unavailable. The repair comes first; no integration happens before it.
- Coverage: #156 holds the gaps, including those from this review's #642, #651 and #655 audits. #503 comes last.
- Unintegrated branches: five (#647, #636, #628, #649 and #655).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite ([goal and phases](planning.md#goal-and-phases)). Each free
slot takes ready work from the earliest phase, and later phases fill the rest
(user decision, 2026-10-28). Position at code `6afdcbf390` (integration 65) in
code lines from `python3 tools/port-metrics.py --whole-fork`, with the change
since the previous navigation's position at `2a89fb023f` (integration 47) in
parentheses. Source areas are an accounting classification: they do not move
drawing or executable header logic into a different phase. "Retired" counts a
rewritten consumer line as retired and its replacement as glue, so compiled C++
(original plus glue) is the distance.

| Area | Original | Retired | Left | Glue added | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 37,640 (+69) | 9,784 | 6,460 (-514) | 16,244 (-583) |
| Simulation storage | 13,724 | 2,767 (+50) | 10,957 | 1,313 (-22) | 12,270 (-72) |
| NewGRF | 10,785 | 5,201 (-79) | 5,584 | 349 (-79) | 5,933 (0) |
| Save/load | 13,388 | 1,075 (+539) | 12,313 | 727 (+142) | 13,040 (-397) |
| Script API | 14,908 | 1,202 (+324) | 13,706 | 542 (+89) | 14,248 (-235) |
| Network | 12,713 | 561 (+350) | 12,152 | 234 (+52) | 12,386 (-298) |
| Support | 21,617 | 930 (-52) | 20,687 | 452 (-61) | 21,139 (-9) |
| Graphics and viewport | 10,985 | 4 | 10,981 | 1 | 10,982 |
| GUI | 52,179 | 115 (-2) | 52,064 | 162 (-3) | 52,226 (-1) |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 2,317 (+1,795) | 52,831 | 0 (-163) | 52,831 (-1,958) |
| Upstream tests | 2,248 | 0 | 2,248 | 5,292 (+174) | 7,540 (+174) |
| Boundary layer | 0 | 0 | 0 | 18,352 (+198) | 18,352 (+198) |
| **Whole fork** | **284,355** | **56,176 (19.76%, +2,994)** | **228,179** | **33,886 (-187)** | **262,065 (92.16%, -3,181)** |

The boundary has 1,443 Rust exports (+66) and 4,064 callback slots (-142).
Rust totals 127,673 code lines (+3,448) and tooling 53,611 (+2,301).
Simulation-source retirement is 40,407 of 61,148 (66.08%, +119). The duplicate
baselines at `5e414c5a64` hold 418 rebound-callee rows (581), 126
shared-accessor rows (128) and 54 orphan exports (55). #607 (integration 48)
re-baselined the count: its sweep removed 812 retired and 813 glue lines that
`WITH_RUST` fallbacks had counted on both sides, leaving compiled C++ unchanged.

| Period (steering and navigation commits) | Days | Retired | Per day | Compiled C++ change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start (`6ced7c097e`, 10-02) to 10-27 | 24.7 | 49,536 | 2,010 | -17,908 | -730 |
| 10-27 to the navigation at integration 47 (10-31) | 4.1 | 3,646 | 890 | -1,201 | -290 |
| Navigation to the twenty-sixth steering (11-01) | 1.12 | 13 (825 without #607) | 10 (730) | -488 | -430 |
| Twenty-sixth to twenty-seventh steering (11-02) | 0.59 | 197 | 330 | -256 | -430 |
| Twenty-seventh steering to now | 0.22 | 2,784 | | -2,437 (-479 without #642's vendored swap) | |
| **Since the last navigation** | **1.93** | **2,994** | **1,550** | **-3,181 (-1,223 without the swap)** | **-1,650 (-630)** |
| **Whole history** | **30.75** | **56,176** | **1,830** | **-22,290** | **-730** |

Days are calendar days. Integrations 48 to 65 landed in three root sessions
(10-31 19:47Z to 21:33Z, 11-01 16:42Z to 11-02 00:17Z, 11-02 12:00Z to 14:52Z);
the gaps were Claude session limits, user pauses and steering. What set the rate:
- #642 replaced bundled Monocypher with maintained crates: 1,958 of the
  3,181-line fall. Without it, compiled C++ fell 630 lines per day, twice the
  previous period's 290 but under the whole-history 730. At that rate the
  209,000 non-vendored compiled lines take about 330 days.
- The other ownership ports moved compiled C++ between +28 (#645) and -224
  (#633) each. Phase-3 owners delivered under their estimates: #575 planned
  1,000 to 1,350 retired and delivered 354 with +220 net glue (compiled -134).
- #570, phase 2's template, has not integrated: its checkpoint stands at
  compiled C++ -377.

Why the course changed:
1. Phase 2 could exit with C++ kept indefinitely. Its exit criterion here
   required "no per-field service ... for moved data", and
   [storage ports](porting.md#storage-ports) deleted only "replaced" services.
   #563's design keeps "Money/date fields and unselected vehicle fields in the
   shell" (`docs/design/world-state.md`, #563 records), yet Rust's maintenance
   owner reads reliability, age, value, profit and breakdown fields through
   `OpenTTDVehicleMaintenanceServices` and the shared vehicle table's
   `write_profit`, `write_reliability` and service-date slots. No later phase
   moves simulation fields, so those services and the shells would never
   retire. A pool is now moved only when no Rust code reaches any of its
   fields through a service, and #656 owns the vehicle remainder.
2. Planning ranked and costed owners by retired lines everywhere except phase 2
   ([selection](planning.md#selection)). Retired lines count rewritten lines
   and ignore glue, so #575's +220 net glue never counted against it. Every
   phase now ranks and decides cost by compiled C++ removed.
3. The planning trigger in `/start-development` (step 3) counted unstarted
   owners, including blocked ones. Phase 2's unstarted owners all wait for
   #570, and phase 3's queued #653 and #573 slice 3b wait for drafts, so
   planning would not run while no owner is ready and about 35,000 phase-3
   lines have none. It now counts ready owners.

| Phase | Left (code lines) | Exit criterion | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | Its bucket's 9,784: about 3,780 drawing and description (phase 5), 490 startup in `openttd.cpp` (phase 4), 1,975 facade shells that retire with their callers, 540 file scope and about 3,000 non-drawing bodies | Every remaining non-drawing body belongs to the phase-2 owner of its file | Exited at integration 63 (#640, #559) |
| 2. Simulation storage | 10,957 in the storage bucket, the phase-1 bodies above, and the per-field services within the boundary layer's 18,352 glue lines and 4,064 callback slots | No Rust code reads or writes a pool or map field through a C++ service or per-call-site reader ([storage ports](porting.md#storage-ports)). Records and map arrays are defined once in Rust, keep the original pool, lifetime and save behavior ([world-state design](design/world-state.md#transferring-shared-storage)), and are read in place by both languages; C++ calls Rust only for operations. Fields only unported C++ touches stay in shells, each named with the phase that moves it. Each pool owner also ports the remaining non-drawing bodies in its files | Current. #570 (#636) is at checkpoint 2 (compiled C++ -377). Then #656, the remaining vehicle fields (it may join #598 and #599 as one owner); #634 with #635; orders; #637; companies and groups; link graph; #638; then the map with the rail, road, water, tunnel, landscape and tile bodies |
| 3. Simulation data readers | 39,962: save/load 12,313, script 12,782, network 9,283, NewGRF 5,584. Script and network GUI (3,793) are phase 5 | These run in Rust. The Squirrel VM and the C++ binding shims it calls may stay until phase 6 | Next, beside phase 2. #575 integrated (65). Open owners cover about 5,000: #573, #574 (#628), #652 (#655) and #653. About 35,000 have none: `afterload.cpp` and the old loader, chunk handlers, script API classes, content, admin and coordinator protocols, network core, and NewGRF configuration and actions |
| 4. Support | 45,100: support 20,687, graphics and viewport 10,981, startup 490, and data tables 12,931, which retire with their consumers | Not planned yet | Planned when phase 2 exits and phase 4 becomes the next phase (user direction, 2026-10-28) |
| 5. Interface | 71,570: GUI 52,064, platform 11,941, script and network GUI 3,793, and about 3,780 drawing lines in the logic bucket | Not planned yet | As above |
| 6. Vendored | 52,831. Only Squirrel (8,774) and md5, ICU and the social API (452) need a port or crate. nlohmann JSON (14,452), fmt (11,986), Catch2 (11,469) and the OpenGL headers (5,604) retire with their last C++ user | Not planned yet | As above |

Upstream unit tests (2,248) and fixture glue retire with the code they test.
Boundary-layer glue retires as its C++ callers do; phase 2 deletes its per-field part.

Root's first actions from this chart, after the resume checkpoint's cadence repair:
1. Run a planning round now: the corrected trigger fires, because no phase-2 or
   phase-3 owner is ready. Size phase 3's unowned 35,000 lines and rank owners
   by compiled C++ removed ([selection](planning.md#selection)), leaving the
   Squirrel VM and its binding shims. Place #656.
2. Finish #570's checkpoint 2, then #656 (alone or with #598 and #599, decided
   at planning). Every later pool owner's issue lists the shell fields Rust
   still reaches through services and moves them, as #634 and #635 already do,
   and names each field it leaves with the phase that moves it.
3. Record each owner's compiled-C++ estimate in its issue and its actual change
   at integration, and compare them at each stocktake.

## Twenty-eighth steering review (2026-11-02: missed child resumes and compiled C++ on open drafts)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

**What was audited.** Five independent read-only audits compared source with the pinned original, with no builds or games:
- #642 and #651 (integrated) and #655 (draft), by Codex `gpt-6.1-sol` at high effort, because the Claude session window was projected near its limit;
- #636 and #628 (drafts), by Claude Opus at high effort.

The steering checked every claim acted on below against the source and the CI logs.

**Root's track.** The 11-02 Claude Code session ran from 11:18Z to 16:22Z.
- It applied the twenty-seventh corrections in its first turn.
- It integrated five PRs (61 to 65) with eight Full runs. Five passed and three failed: #651's first and #649's two. Two of the failures were generated-ABI include paths.
- It routed implementation to Codex while the Claude session window was projected over its limit. After the user's correction that only the 5-hour window paces (user, 2026-11-02), it returned to the normal Claude Code root setup.
- The navigation after the wind-down (`c05fdd45e6`) moved every phase to compiled-C++ ranking, required every Rust-used field to move, and made the planning trigger count only ready owners.

**No reachable divergence** in #642, #651, #655, #636 or #628. #642's crate composition matches Monocypher 4.0.2 for X25519, BLAKE2b, the incremental AEAD and cofactored Ed25519, and it draws random bytes in the original order.

1. **Codex children are not always resumed when their job exits.**
   - Four times on 11-02, a T3-delegated Codex child ended its turn on a named job that then exited without waking it:
     - #647's reviewer, from 11:55Z to 14:10Z;
     - the same reviewer again, from 14:11Z until root cancelled it at 15:56Z;
     - #636, from 15:08Z to the wind-down;
     - #655, from 15:55Z to the wind-down.
   - That is about 6 agent-hours. It is why #647, ranked first in maintenance, did not integrate while five other PRs did. Root noticed the first stall after 2 h 15 min.
   - Correction: a child checks its job once before ending a turn (`AGENTS.md`). Root checks every child idle on a named job at each of its own wakes ([team](team.md#spawning), `/start-development` step 1).
2. **Ports add Rust and glue faster than they remove compiled C++.**
   - Excluding #642's vendored swap, integrations 61 to 65 removed 127 compiled lines and added 1,507 Rust and 826 tooling lines.
   - #628 adds compiled C++ (+62) after 5,291 Rust lines. About 24% of its glue is native fixtures, and 172 lines are per-message log slots.
   - The navigation's compiled-C++ rule had not yet reached open drafts.
   - Correction: #628 makes the bounded repair in its steering comment before review, for an expected -240 to -360 once #657 lands. #655 and #636 record compiled-C++ cost decisions in their PRs.
3. **Ports add one log service per message.**
   - `rust-migration` has 166 such services (41 `Debug`, 125 `GrfMsg`), each a table field, a declaration and a lambda. #628 adds about 86 more.
   - Correction: #657 ports both macros once. [Boundary](porting.md#boundary) names a per-message service a design fault once #657 integrates.
4. **Compiled C++ omits `.inc` glue** (487 physical lines in three files). Correction: #658, which also carries one boundary leftover from #642 and two from #651.
5. **A dependency change reached the platform matrix only after integration.** #642 added crates, the cadence after integration 65 went red, and it now holds every integration. Correction: a change to `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` or `cmake/` runs the platform profile once before Full ([checks](workflow.md#checks)).
6. **#636 is on course.**
   - Every saving so far comes from deleted services (about -420). The roughly 740 rewritten consumer lines net zero.
   - Finishing checkpoint 2 should reach about -900 to -1,250 compiled C++.
   - Before integration, `train::Handle` must carry the record: today 42 sites in `train.rs` call `record_from_shell` on each use, every train tick.
   - The steering comment on #636 lists the remaining inventory and the dependent-load table for the placement decision.
7. **#655 has two Rust-to-C++-to-Rust bounces and a broken comparison fixture.** The bounces are `MapGRFStringID` → `GetGRFStringID` → Rust, and `HandleChangeInfoResult` → `DisableGrf` → Rust. Because of the fixture, no scenario covers its head yet.
8. **#647's resume round is scoped** under the infrastructure review rule:
   - its four gate gaps are hypothetical forms; fix the first if it is cheap and list the rest as limits;
   - one fresh reviewer finishes checks 5 to 7.

No user direction is reversed. On 2026-11-02 the user reselected #507's NewGRF corpus, which #654 had asked about. The steering's first answer wrongly treated #526's GPL source work as a prerequisite; the resume plan on #507 already kept the sets uncommitted. [Evidence](porting.md#evidence) and `AGENTS.md` now state that rule, and #526 is closed. The restart prompt gains one standing step (finding 1).

| Twenty-seventh correction | Verdict and cost/result |
| --- | --- |
| 1. #644 and #646 take the first free slot | Slipped, through finding 1. Root started #647 in its first turn, but its reviewer stalled twice, so five integrations went first. Non-test `bool` returns grew by one, and #647's base merge converts them. No divergence was observed. |
| 2. Phase-2 cost by compiled C++; placement from the post-#570 profile | Applied to #636, which reports compiled C++ -377. The navigation extended the rule to every phase. The placement decision is not due yet. |
| 3. Read failed Fulls to the end | Applied: #645 integrated at its first Full after the packing fix. Generated-ABI include paths still failed #651's and #649's Fulls. #649's `128c1b9c96` adds `build-rust/generated-abi` to `CPATH` for every driver compile, which ends that class of failure when it integrates. |
| 4. Issue placement | Applied: #604 is in #649, and #497's Action 3 joined #652. |

| Change since the twenty-seventh review | Verdict and next observation |
| --- | --- |
| Navigation `c05fdd45e6`: move every Rust-used field, rank every phase by compiled C++, and count only ready owners in the planning trigger | Untested. Watch whether the resume's planning round ranks by compiled C++, whether #656 is sized from #636's final inventory, and whether open drafts record a compiled-C++ cost decision before review. |
| Only the 5-hour session window paces Claude agents (user, 2026-11-02, in the user's global instructions) | Root applied it at once. |

New since this review, for the next steer to judge:
- Whether any child stays idle on an exited job across more than one root wake.
- Whether #628 reaches review with compiled C++ below zero and a named end for each remaining bounce.
- Whether #657 lands before #628 and #655 integrate, and how much compiled C++ it removes.
- Whether #636 finishes checkpoint 2 near -900 to -1,250, with `train::Handle` carrying the record.
- Whether the cadence repair and the platform-profile rule keep the next cadence (after integration 70) green.

## Earlier steering, still in force

Ownership, original behavior, process scope, named callees, typed shared services
and exact random-draw order come first. Preserve default semantic evidence,
record gaps on #156 and keep retirement distinct from glue. No hot allocation,
opcode dispatch, whole-record copies or owner bounces; no C++ frame is kept only
for attribution. Rust reaches the map by a load through `map_func::World`, with
native assertions and no C++ fallback, and descriptor reads stay within
`docs/design/world-state.md`. The rebound-callee, shared-accessor and
orphan-export baselines only shrink. An owner does not port support code whose
data stays in C++ behind one service per field. Ready owners proceed beside
speed work; only timed collection holds builds and games. Ceiling and band
decisions use the three-pass record; the projection factor and speed thresholds
remain provisional, and 0.1% to 0.3% speed candidates get no attribution
research. Testing is maintenance: scenarios arrive with the ports that need
them. Six branches, batched maintenance, fixing reviews, strict exact-combination
Full, bounded rollback and action-based wind-down remain. A historical mention
is not placement; CI-found behavior fixes get a scoped review; deep checks run
only on the user's request; active speed dependencies are preserved. Third-party
test inputs stay out of Git and CI artifacts ([evidence](porting.md#evidence)). Every agent runs
its own long jobs and ends a turn only on a named job ID; root checks any child
that ends a turn without one, and any child idle on a job that has exited. A
base update that meets a review condition needs only CI. Each new scenario,
fixture or checker names the behavior existing checks cannot distinguish; this
rule is provisional, on a basis of 1.5 tooling lines per retired line. Roadmap
commits happen only at stocktakes, course changes and wind-down. #627's
per-container typed leaves for STL containers are a phase-3 necessity; they do
not justify per-field leaves in phase-2 records. Every owner's cost is its
compiled-C++ change, decided before review. Rust-defined `extern "C"` functions
return 0-or-1 `u8`, not `bool` (#644). Read a failed Full to its last job before
recording its cause. Before briefing #634 and #635, root decides record
placement from the post-#570 profile. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence.

## Recent completed work

Rows cover the period since the twenty-sixth steering review; earlier rows are
in their PRs and Git history. Metrics are `tools/port-metrics.py` code lines:
Rust / tooling / original C++ retired / net glue.

| Integration | Issues / PR | Merge commit | Metrics |
| --- | --- | --- | --- |
| 57 | #629 multiplayer comparison isolated from host loopback probes / #632 | `46a2f240d7` | 0 / 8 / 0 / 0 |
| 58 | #622 later batch: shared NewGRF spec and GRF-file bindings / #639 | `83c62ef441` | net glue -34 / rebound rows 523 to 420 |
| 59 | Shared-record declaration generator (#570, #641) / #643 | `2d4a063e33` | 48 / 236 / 0 / +8 |
| 60 | #69 tile-area geometry and the diagonal iterator / #633 | `b97004e599` | 529 / 5 / 197 / -27 |
| 61 | Platform cadence: landscape fixture `shared` initializer / #648 | `cdfc2de482` | 0 / 0 / 0 / 0 |
| 62 | #641 packet state and byte storage in a generated Rust record / #645 | `bf2c4632a2` | 718 / 175 / 52 / +80 |
| 63 | #559 command invocation lifecycle (phase 1's last owner) / #640 | `7940e700c2` | 403 / 287 / 136 / +115; rebound rows 420 to 418, orphan exports 55 to 54 |
| 64 | #64 maintained crypto crates and Rust network authentication / #642 | `5e414c5a64` | 2,027 / 480 / 2,242 / -68 |
| 65 | #575 script host, command context and event lifecycle / #651 | `6afdcbf390` | 386 / 364 / 354 / +220 |
| 66 | #659 carried Full validation across pushes that keep the tested non-Markdown tree / #660 (CI only; the user allowed it ahead of the cadence repair) | `f47064fc57` | 0 / tooling only / 0 / 0 |
| 67 | #661 Emscripten compiler cache / #662 (CI only; the user allowed it ahead of the cadence repair) | `d93afabee9` | 0 / tooling only / 0 / 0 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

1. **Repair the red platform cadence first** (resume checkpoint). Rerun the cadence on the merged fix. No integration happens until it is green.
2. **Integrate the reviewed maintenance in readiness order** (one reviewer and one Full each):
   - #649 (#604 settings access with #199's disjoint items): fixture unit tests, a scoped review of `ba07c85bb1`, Full and merge. Its `128c1b9c96` ends the generated-ABI include failures behind two of the 11-02 failed Fulls.
   - #647 (#644 `bool` returns with #646 `TileArea::expand`): the scoped round in the twenty-eighth steering comment on #647, then Full and merge. Its gate then rejects new `bool` returns.
3. **#507 NewGRF corpus** (user reselected, 2026-11-02, closing #654; #526 closed as unneeded), in the first free slot. It is maintenance with one reviewer and one Full.
   - Open a new PR from `rust-migration`. Reuse #525's scenario module (`tools/simulation/newgrf_sets.py`) and its scenario AI from `testing/corpus507-newgrf` at `ccc28e2ed0`, but not its committed `.grf` files or source archive.
   - Pin the three OpenGFX+ sets (Trains 0.3.0, Road Vehicles 0.4.1, Industries 0.3.5) by name, version and SHA-256, and fetch them from the BaNaNaS CDN into ignored `.local/` storage ([evidence](porting.md#evidence)).
   - Extending `tools.simulation.community_ai` and its cached manifest avoids a workflow change. Keep the sets out of every uploaded artifact.
   - The default slice is `newgrf-sets-arctic` (about 7 s of game time); `newgrf-sets-arctic-long` runs in soak. The same-NewGRF larger save follows as a later #507 PR.
   - Integrate it before #655 if it is ready first; #655 does not wait for it.
4. **#570 vehicle records (#636) to checkpoint 2**, then review and integration. It is phase 2's first owner and the template for every later pool.
   - Before integration, apply the twenty-eighth steering comment on #636: `train::Handle` carries the record; delete every remaining per-field route for a field already in a Rust record (the comment's inventory); fix the `disaster.rs` `shell()` naming; run the scenarios at the head; update the PR body.
   - Expected compiled C++ at checkpoint 2 is about -900 to -1,250; judge it at review against that estimate.
   - Then #656, the fields still in the shell, alone or with #598 and #599 as decided at planning. Remeasure the inventory after #570.
5. **Run the planning round from the Phases chart** in the first free agent slot. It needs no branch. Size phase 3's unowned 35,000 lines, rank by compiled C++ removed, and place #656, #657 and #658.
6. **Maintenance batch: #657 and #658** (one reviewer, one Full), built by root from `rust-migration` once #647 and #649 have integrated. #657 deletes about 166 per-message log services, so it touches many service files. Open drafts adopt it at their next base update and do not build their own helper. #658 changes the compiled-C++ count; record that as a re-baseline at the next stocktake.
   - #565 item 2, after #570 integrates, because both change `movement.rs`. Delete the map fallbacks and convert the remaining rail, crossing, depot, stop, water and tunnel leaves, `tile_slope_z`, and world generation's per-tile services (issue body), with #168's unfinished world-generation items. Production net glue -300 to -600. It also deletes the native `bridge_above` and `rail_tile_type` services that #616 and #620 duplicated with Rust map readers (`road_cmd.rs:932`, `road_cmd.rs:1443`, `station_cmd.rs:447`).
   - Later: #501 probe typing. #168/#199/#500's completed-job publication moves to the link-graph pool owner.
7. **The next phase-2 pools after #570 integrates** (ranked by plan-after56, 2026-11-01; the planning round re-ranks them by compiled C++). Each needs #570's generator, save descriptors and record lookup.
   - Before briefing #634 or #635, root decides from the merged-tip profile after #570, and from #566's drawing workload, whether later records sit beside their shells, as #570's do, or inside them. Use the dependent-load table and the profile checklist in the twenty-eighth steering comment on #636. If the cost is material, record the decision in the [world-state design](design/world-state.md) before the briefs.
   - #634 stations and road stops: 450 to 650 retired, -300 to -800 net glue, about 110 to 160 station-field slots. It removes the per-station `SharedForEachStation` bounce and #612's two C++ calls per k-d tree coordinate (`station/spatial.rs:12`).
   - #635 towns, industries and subsidies: 350 to 550 retired, -150 to -450 net glue, about 40 to 70 field slots. Disjoint from #634; the two may run together.
   - Then:
     - orders and order lists (441 left; read on every vehicle tick);
     - #637 engines (545, with #497's engine bindings);
     - companies and groups (510);
     - link graph (607);
     - #638 the smaller pools (615, with #498's ObjectSpec payloads);
     - the map (3,988 storage), with the rail, road, water, tunnel, landscape and tile bodies.
   - Pools without an issue get one and a design section when selected. Each moves its coupled save adapters with each record, generates native declarations, verifies the layout, and deletes the displaced readers, services and their baseline rows.
8. **Phase 3 fills every other free slot:**
   - #628 (#574 network protocol): make the bounded cost repair in the twenty-eighth steering comment on #628 before review, beside slice 5. That means:
     - fix the `SendNet` and accept bounces;
     - move about 150 fixture lines to Rust tests;
     - write one handler-wrapper template;
     - delete the unreachable virtual defaults;
     - record the compiled-C++ decision and a named end for each remaining bounce.
     Then one round review. Integration needs slice 5's default two-client observations.
   - #652 (#655): the six items in the twenty-eighth steering comment on #655, including the two bounces and the reference-side fixture. Rerun the loading scenarios, record the cost decision (+113 net glue against +50), then review by function group. Formatting-time GRF string code (about 170 lines) stays C++ as phase 4. `Engine::grf_prop` and `overrides` stay with #637, and #498 with #638.
   - #573's next PR, after #570 integrates: slice 3b changes the `SaveLoad` base and the `saveload.h` macros that #570's vehicle descriptors use. Its brief keeps the save-descriptor macros unchanged so it can run beside #634 and #635; otherwise whichever starts second waits. Then slice 4, the corrupt-input and LoadCheck probes, and the native `CopyFromOldName` helper, as #627's status comment lists.
   - #653 industry and industry-tile specifications, after #647 and #649 integrate (both edit the industry files). Compiled C++ about -270 to -400; the spec records' vector fields are decided at its first checkpoint.

   Size each owner to repay its review and Full ([selection](planning.md#selection)). Plan no Squirrel port.
9. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect just to fill a slot.

Recurring:
- After every second integration (next after 66), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip with the pinned `LD_LIBRARY_PATH` from [speed](speed.md#instruction-projections).
- Run the platform cadence after every fifth integration. The cadence after 65 is run 41466312349; the next is after 70. Fix a red cadence before further integrations.
- A quiet speed pass is owed in the next quiet window ([speed passes](speed.md#speed-passes)), because #642 changed `rust/openttd-kernels/Cargo.toml` and `simulate --instructions` now rejects the anchors' identity. It pauses builds and games for timed collection, so run it when no agent is building, and at the latest before the post-#570 record-placement decision.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions checked by this review:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-02, wind-down after integration 65)

Paused at the user's request: wind-down at 16:20Z. Code is `d93afabee9` (integration 67: the CI-only #660 and #662 merged after the pause at the user's request; the cadence repair below still gates every other integration). `CI_ON_DEMAND` is `true`. No open head has a requested or passing Full. Each PR's status comment holds its detail. The user reselected #507's NewGRF corpus on 2026-11-02 (work order 3). Agents were stopped or told to stop after recording state; no local job result below is known unless stated.

Due first on resume, in order:
1. **Repair the red platform cadence** (run 41466312349) before any integration. Confirm the cause from the macOS `rust-native-libs.log` (evidence artifact or a local configure for `aarch64-apple-darwin`), then make the configure probe and the build agree on native libraries now that `libc` is a dependency, in a small root PR with Full and a platform run. Rerun the cadence on the merged fix.
2. Stocktake 66 (owed after integration 65's successor; take it at the next even integration), the quiet speed pass (recurring list) and the next planning round stay owed.

| Issue / PR | Head | Next action | Running job or CI |
| --- | --- | --- | --- |
| #644, #646 / #647 boundary `bool` returns and `expand` overflow | `9e0d5af362` | Base merge over integrations 61 to 65 done and validated (77 Cargo tests, 332 CTests, packet and auth comparisons, 16 simulations, cheap CI). /root/bool_review2 (Claude) was told to post its report and stop; it found gaps in the gate. Next: the scoped round in the twenty-eighth steering comment on #647 (fix the first gate gap if cheap, list the rest as limits, one fresh reviewer for checks 5 to 7), Full, then merge after the cadence repair | Review report: check #647. No Full |
| #604, #199 / #649 settings access batch | `ba07c85bb1` | Full at `20eaf59ebb` failed `native / build` (generated-ABI include; fixed in `c48cdcd89e` and `128c1b9c96`); Full at `128c1b9c96` failed six scenarios. Cause: `SessionFixture::Run` advanced the company tick cursor after `19520dd248` deleted the editor stub. Fixture fix `ba07c85bb1`: the six scenarios and their family run (56 `landscape-*`, `stations-commands-*` and `stations-cargo-*` scenarios, including both `-reload` variants) are equal, 60 of 60 (status comment). Next: the fixture unit tests including the `company_economy` cursor check, a scoped review of the fix, Full at the exact head, merge | Quick checks green at `ba07c85bb1`. No Full |
| #570 / #636 vehicle records | `6191d5f9b0` | Bounce removed, handle identity switched, ground and train records moved; build and 236 of 236 CTests passed; compiled C++ -377. Session 73637 left no result. Next: rerun the train-edge comparison and the 17 focused train and fleet scenarios, then finish checkpoint 2 with the must-change items in the twenty-eighth steering comment on #636 (`train::Handle` carries the record, the inventory's moved-field routes deleted, `disaster.rs` naming, scenarios at the head), then review | None |
| #574 / #628 network protocol | `2711496991` | All four steering faults fixed (close, send, receive and command-post bounces; #645, #640 and #642 adopted); 33 obsolete exports retired; fixture clock fix pushed. Validation session 51484 result unknown. Compiled C++ +62 at this head (steering measurement). Next: rerun the scoped validation at `2711496991`, then the bounded cost repair in the twenty-eighth steering comment on #628 beside slice 5, then one round review (Claude) | None |
| #652 / #655 GRF text and NewGRF Actions 3, 4 and 13 | `54c44c68c5` | Text owner, translation and all three action handlers in; net glue +113 against the +50 estimate. Session 87540 failed 0 of 2: `newgrf-loading-bindings` and `-reload` do not compile the reference side of `tools/simulation/newgrf-loading-console.cpp` (a `CargoLabel` passed where `GetAiPurchaseCallbackResult` takes a `CargoType`). Next: fix that fixture and the other must-change items in the twenty-eighth steering comment on #655 (two bounces, the one-call-site language reader, the duplicate road and tram service, the choice-flush copy), rerun the loading scenarios, record the cost decision, then review by function group | None |

PR worktrees (preserve): #647 `t3-c4634554`, `t3-17a0c009` and the cancelled reviewer's `t3-2efde48a`; #649 `t3-13298c8f`, `t3-6e7160e6` and `t3-b4561a0d`; #636 `t3-c67aada9` and `t3-41b02f9e`; #628 `t3-eb8e9c59`, `t3-1e4ffc20`, `t3-3208e14e`, `t3-5aa8a17f`, `t3-fc38354a` and `t3-e0d0c168`; #655 `t3-986b5854`. The merged and finished review checkouts are T3-locked; housekeeping keeps them until T3 releases them.

Keep unchanged: `t3-120ef117` (old #603), `t3-431c84b4` (cancelled #565 source with 58 dirty entries), `t3-d78d5f8b` (native fixture evidence), `.local/reference/openttd` at `14ec60f248` and the instruction-profile worktrees under the main checkout's `.local/worktrees/`.
