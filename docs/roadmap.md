# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-11, stocktake after merge 32)

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

**State.** Root restarted on 2026-11-11 after steering 34. Five PRs have merged through the queue since #894 (merge 27): #905 (28, `830b4d77cf`), #908 (29, `cdda5418f9`), #900 (30, `0f727600fb`), #910 (31, `ac0ed46a35`) and #915 (32, `985abab91f`). The tables below measure `985abab91f`. The merge-28 platform cadence went red on macOS Debug: a fork test fixture put water in the `Map::MaxX` column, and #894 had made the Rust call assert there as the original does. #908 fixed the fixture forward and merges resumed ([#903](https://github.com/fetch-laboratories/openttd-rust/issues/903#issuecomment-6806233274)); the pre-queue Linux tests had missed it because that build compiled assertions out (#911, fixed in #912). The next cadence follows merge 33, and the next stocktake and profile merge 44. No confirmed base regression is open.

**Reading.** Cleanup continues. Fork C++ fell 245 lines over merges 27 to 32: C++ under `tools/` fell 300, of which #915's industry lifecycle probe is 268 net, and `src/tests` rose 59, the growth #910 was allowed in order to restore the observations on #156. #734 stands at 99 of 319 files and 35.1% of lines, with `tile_map.rs` (#909) and six network files (#918) in review. #735's count has not moved since merge 20. The passes since then covered map and network leaves, and 70 of its 121 functions are FFI exports, which keep positional lists while their C++ callers remain; its movable inventory is the 51 helpers and commands, and all but two commands sit in files no pass has finished yet, so the count falls as the remaining worksets pass and no course change follows. Residue in finished files rose with the seven files finished since merge 20 (missing comment lines 135 to 161); #912's gate stops further growth and closure owns the rest. The #736 classification leaves about 1,430 movable lines of 20,053; the rest retires with pool and reader owners ([work order](#cleanup-work-order) item 4). Median last-ready-to-merge over merges 27 to 32 was about 2 hours 11 minutes, against 53 to 73 minutes before, from the merge stop, a GitHub Actions incident (20:22 to 21:11 UTC) and macOS runner capacity; no queue run failed. Full phase-3 planning remains untriggered, with 209 of #734's 319 files in no entry, and near-full reproduction is not met.

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric v2). The merge-20 interval analysis, process counts and earlier inventory columns are in this file at `6ac868ee17`.

| Measured quantity | Merge 8 | Merge 20 | Merge 26 | Merge 32 |
| --- | ---: | ---: | ---: | ---: |
| Original C++ remaining | 221,656 | 221,957 | 221,957 | 221,957 |
| Fork source glue, including generated records | 33,651 | 32,991 | 32,891 | 32,947 |
| Compiled source and generated records C++ | 255,307 | 254,948 | 254,848 | 254,904 |
| Tracked tools C++ evidence | 15,437 | 15,439 | 15,316 | 15,016 |
| ABI-generated non-record C++ evidence | 5,442 | 5,367 | 5,359 | 5,358 |
| Measured fork C++ series | 276,186 | 275,754 | 275,523 | 275,278 |
| Original C++ retired | 62,699 of 284,355 (22.05%) | 62,398 (21.94%) | 62,398 (21.94%) | 62,398 (21.94%) |
| Simulation C++ retired | 41,539 of 61,148 (67.93%) | 41,231 (67.43%) | 41,231 (67.43%) | 41,231 (67.43%) |
| Historical regex exports / callback slots | 1,536 / 3,298 | 1,508 / 3,219 | 1,508 / 3,211 | 1,508 / 3,210 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 87 / 279 / 30 | 48 / 245 / 27 | 47 / 244 / 27 | 47 / 244 / 27 |
| Exports held only by uncalled members (not enforced) | 53 | 23 | 23 | 23 |
| Rust / tooling code lines | 203,275 / 64,412 | 175,632 / 65,718 | 178,381 / 65,663 | 182,354 / 66,055 |

#872 moved 343 station-command lines from glue to original, which is why retirement reads 301 lower after merge 8; it restored no code. The finished-file figures at merge 32 use the merge-20 manifest plus `packet.rs` (#887), five files of #889 and `map_func.rs` (#894); #912 tracks the list as `migration/finished-files.txt` from its merge on.

| Source area / phase relationship | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 37,992 | 9,432 | 5,909 |
| Simulation storage (phase 2) | 13,724 | 3,239 | 10,485 | 1,385 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,074 | 38,720 | 3,354 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 548 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,787 |
| Boundary layer | 0 | 0 | 0 | 16,485 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps are on #156. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant and literal rows are analyzer estimates from `tools/code-standards-inventory.py`, not automatic source findings. "Finished" figures use the merge-20 manifest of 92 entries. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Overhaul | Integration 146 (steering 33) | Merge 20 | Merge 26 (steering 34) | Merge 32 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| #734 | Workset files with a finished pass | 0 of 319 | 51 of 319 | 92 of 319 | Next stocktake | 99 of 319 |
| #734 | Workset lines in finished files | – | – | 60,266 of 187,833 (32.1%) | Next stocktake | 66,913 of 190,751 (35.1%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 3,139 | 2,693; 133 in finished files | 2,304; 136 in finished files | 2,258; 135 in finished files | 2,250; 161 in finished files |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,957 | 1,772; 121 in finished files | 1,664; 170 in finished files | 1,663; 170 in finished files | 1,663; 171 in finished files |
| #734 | Transparent wrapper types / named constants | 69 / 2,181 | 287 / 3,482 | 303 / 3,662; 53 / 688 in finished files | 309 / 3,648; 53 / 688 in finished files | 309 / 3,662; 58 / 716 in finished files |
| #734 | Literal sites standing for a named value | – | – | 12; 0 in finished files | 12; 0 in finished files | 12; 0 in finished files |
| #735 | Functions over seven parameters | 150 | 132 (70 FFI, 39 helpers, 23 commands) | 121 (70 FFI, 25 helpers, 26 commands) | 121 (70 FFI, 25 helpers, 26 commands) | 121 (70 FFI, 25 helpers, 26 commands) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 71 / – | – | 48 / 11 | – | 48 / 10 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | – | 15,642 / 5,279 | 15,439 / 4,786 | 15,316 / 4,728 | 15,016 / 4,787 |
| #737 | Shared-accessor / rebound-callee / orphan-export rows | 117 / 314 / 42 | – | 48 / 245 / 27 | 47 / 244 / 27 | 47 / 244 / 27 |
| #737 | Exports held only by uncalled members | – | – | 23 | 23 | 23 |
| #738 | Rule corpus lines | 3,312 | 3,231 | 3,237 | 3,237 before this review | 3,266 |
| None | Ruff rows | 296 | – | 287 | – | 286 |

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset.
- **#735:** use the tracked long-signature inventory counter (#874/#882), `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, #737 and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork`, `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The [merge-32 profile](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6810087211) on clean `985abab91f` used `simulate --instructions --jobs 2`. All seven canonical exit saves and full logs match. The driver exited 1 on its anchor identity guard for every scenario: #908 added a `native-debug` feature to `rust/openttd-kernels/Cargo.toml`, a file the identity hashes, and nothing else in the identity differs from merge 20. The merge-32 projections below are therefore computed from the report's instruction counts and the retained anchors by the [formula](speed.md#instruction-projections), not printed by the driver; the next quiet pass re-anchors ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275)). The profile's output is retained in locked `t3-656b5a85-5eedda484b0f05a1` and replaces the merge-20 profile in `t3-9bfeabfd-b8f486fd2881369f` as the previous tip for attribution.

| Scenario | Original Ir | Candidate Ir (merge 20) | Projection (merge 8) | Projection (merge 20) | Candidate Ir (merge 32) | Projection (merge 32) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,774,994,754 | 1.163070x | 1.161260x | 2,789,433,667 | 1.173339x |
| Opus | 21,773,343,583 | 27,411,470,613 | 1.345115x | 1.345115x | 27,485,703,563 | 1.345115x |
| Grok | 20,969,727,163 | 26,174,670,001 | 1.346652x | 1.346652x | 26,243,562,014 | 1.346652x |
| P1996 | 3,600,502,896 | 4,505,830,818 | 1.352193x | 1.352193x | 4,514,871,657 | 1.352193x |
| P2000 | 6,049,922,238 | 7,408,317,235 | 1.444531x | 1.444531x | 7,434,777,978 | 1.444531x |
| P2016 | 11,343,424,651 | 13,711,755,287 | 1.382593x | 1.382593x | 13,729,352,305 | 1.382593x |
| Drawing (diagnostic) | 2,149,015,961 | 2,154,439,219 | 1.004635x | 1.004635x | 2,154,418,251 | 1.004635x |

No policy projection rose 3% or entered its band: generation rose 1.04% and stays below its 1.235x band and 1.3x ceiling, and the play records stay on their anchor floor, the highest at 1.444531x against the 1.52x band and 1.6x ceiling. No wall pass is due, and these instruction projections are not new wall-time measurements. `Map::tile_add` has no symbol of its own in the generation or Opus profiles, so it is inlined in release, which settles #903's instruction acceptance.

## Course (user, 2026-11-06)

- **Execution order:** the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([planning](planning.md#goal-and-phases)).
- **Yardstick:** net fork C++ removed, counting C++ under `tools/` and `src/tests`. Rust exports plus callback slots fall or hold; growth needs a reason the reviewer accepts ([cost decisions](planning.md#cost-decisions)).
- **Limits:** at most two implementation agents (user, 2026-11-10; [team](team.md#roles-and-models)); stocktake and profile every twelfth merge (user, 2026-11-08; every sixth from 2026-11-06, and stocktake 137 is skipped).
- **Enforcement:** checks enforce mechanical rules on every push (#733); reviewers work from the [review checklist](review-checklist.md); implementers read the [implementer brief](implementer-brief.md) and their work order.
- **Process (user, 2026-11-10):** #860 lists the changes from the development-process review: a merge queue, Windows x86, Clang and GCC Dedicated in Full, cached CI packages, one Clippy run, scenario comparison on CI, direct reviewer and implementer messages, and a checkpoint written at wind-down only.
- **Sizes, never durations:** plans, issues and briefs size work in counts (files, lines, call sites), never in time (user, 2026-11-06).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

Phase numbers name content; the execution order above sets the sequence. Phase 1 (simulation logic) exited at integration 63.

Position at code `61c6d2c598` (integration 146), docs `89b6807754`, 2026-11-09. Counts are code lines from `python3 tools/port-metrics.py --whole-fork` (metric version 2, implementation `79b7d225f6`): comments, blanks and preprocessor lines are excluded, and the scope is `src/` plus generated records for all platforms, not one executable. Parentheses compare the last navigation, `2d233f628b` (integration 88), recounted with the same metric. The recount matches its published figures, so there is no inventory correction. Areas are accounting buckets; the last column names what removes each.

| Area | Original | Retired (change) | Remaining (change) | Fork glue (change) | Compiled C++ (change) | Removed by |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Simulation logic | 47,424 | 38,329 (+114) | 9,095 (−114) | 6,458 (−172) | 15,553 (−286) | Phase 1 has exited; its residue goes with its consumers: about 2,400 lines of drawing functions in phase 5, startup (`openttd.cpp`) in phase 4, and 1,984 facade lines and other bodies with their phase-2 owners |
| Simulation storage | 13,724 | 3,212 (+288) | 10,512 (−288) | 1,387 (+16) | 11,899 (−272) | Phase 2 |
| Save/load | 13,388 | 1,435 (+100) | 11,953 (−100) | 1,070 (+88) | 13,023 (−12) | Phase 3, except about 4,300 lines of pool and map chunks and their compat tables, which go with phase 2 |
| Script API | 14,908 | 2,406 (+12) | 12,502 (−12) | 750 (+12) | 13,252 (0) | Phase 3; VM and binding shims in phase 6 |
| Network | 12,713 | 3,253 (+676) | 9,460 (−676) | 1,168 (+126) | 10,628 (−550) | Phase 3 |
| NewGRF | 10,785 | 5,973 (+5) | 4,812 (−5) | 372 (+1) | 5,184 (−4) | Phase 3 |
| Support | 21,617 | 973 (+20) | 20,644 (−20) | 547 (+77) | 21,191 (+57) | Phase 4 |
| Graphics and viewport | 10,985 | 7 (0) | 10,978 (0) | 4 (0) | 10,982 (0) | Phase 4 |
| Data tables | 17,295 | 4,367 (+1) | 12,928 (−1) | 14 (+1) | 12,942 (0) | Each table with its last C++ consumer, in phases 2 to 5. The largest are `town_land.h` (2,206), `build_industry.h` (1,468) and `sprites.h` (1,278) |
| GUI | 52,179 | 415 (+70) | 51,764 (−70) | 463 (+71) | 52,227 (+1) | Phase 5 |
| Platform | 11,941 | 0 (0) | 11,941 (0) | 2 (0) | 11,943 (0) | Phase 5 |
| Vendored | 55,148 | 2,317 (0) | 52,831 (0) | 0 (0) | 52,831 (0) | Phase 6 |
| Upstream tests and fork test glue | 2,248 | 14 (0) | 2,234 (0) | 5,279 (−406) | 7,513 (−406) | Fork tests in cleanup (#736). Upstream tests go with the code they test; Catch2 goes when the last C++ test leaves |
| Boundary layer | 0 | 0 (0) | 0 (0) | 16,759 (−822) | 16,759 (−822) | Each side's owner; the final exit |
| **Source total** | **284,355** | **62,701 (22.05%, +1,286)** | **221,654 (−1,286)** | **34,273 (−1,008)** | **255,927 (−2,294)** | |
| C++ under `tools/` | | | | 15,642 (−1,497) | 15,642 (−1,497) | Cleanup (#736), except named boundary checks |
| ABI-generated non-record C++ | | | | 5,475 (−310) | 5,475 (−310) | With the records and layouts it checks |
| **Fork C++ census** | | | | | **277,044 (−4,101)** | |

Of the 21,117 evidence lines, 19,465 are candidate or shared, none is reference-only and 1,652 are unresolved, which the census counts as candidate. Outside the census are the 936 raw lines of C++ in `COMMAND_SOURCE` (`tools/simulation/stations.py`), compiled into fixture games, and the reference-only extracted-original comparison fragments; #736 owns both (batch T and its scope follow-up). The boundary has 1,539 exports (−3) and 3,332 callback slots (−304). Rust has 191,899 code lines (+53,663), of which about 37,000 are oracle text added since the last navigation (`*-oracle.txt` under `rust/`); tooling has 61,896 (+3,227).

Rates use commit timestamps through each endpoint, in calendar days, not agent time:

| Period | Days | Original retired | Per day | Fork C++ census change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start `6ced7c097e` to navigation `8fb4499105` | 25.93 | +50,111 | 1,933 | +3,023 | +117 |
| To navigation `c05fdd45e6` | 4.83 | +6,065 | 1,256 | −1,578 | −327 |
| To navigation `2d233f628b` (integration 88) | 3.47 | +5,239 | 1,509 | −4,655 | −1,341 |
| To steering 31, `220e138f8c` | 0.54 | +735 | 1,358 | −277 | −512 |
| To the pause before cleanup, `e800717540` (integration 102) | 0.28 | +600 | 2,157 | −1,113 | −4,001 |
| To steering 32, `0cb2f766b9` (integration 129) | 0.78 | −40 | −51 | −796 | −1,018 |
| To integration 131, `305ac20c28` (paused for steering 32 and the overhaul evaluation) | 1.04 | 0 | 0 | 0 | 0 |
| To now (integration 146) | 0.43 | −9 | −21 | −1,915 | −4,476 |
| **Since the last navigation** | **3.07** | **+1,286** | **419** | **−4,101** | **−1,335** |
| **Cleanup stage so far (integrations 103 to 146)** | **2.25** | **−49** | **−22** | **−2,711** | **−1,204** |
| **Whole history** | **37.30** | **+62,701** | **1,681** | **−7,311** | **−196** |

What set the rate:
- Ownership ports retired 1,250 to 2,160 original lines a day through integration 102. The cleanup stage retires none by design (user, 2026-11-06). Its net −49 follows glue edits in original-derived files; root traced the −9 at integration 138 to that reclassification, not restored original code.
- The cleanup still cut the fork C++ census by 2,711 lines, close to the pace of the porting period before it. #736 supplied most of it: C++ under `tools/` fell 1,795 lines, and compiled C++, including `src/tests`, fell 869.
- The 33 vocabulary PRs (integrations 109 to 126) and the 1.04-day pause for steering 32 took the stage's first half. Once file passes began at integration 132, #734 finished 41 files and 32,080 lines in 12 integrations (stocktake 143).

| Order | Stage | Size now | Exit (end state) | Status |
| ---: | --- | --- | --- | --- |
| 1 | Cleanup | #734: 227 of 319 entries and 127,567 of 187,833 raw lines unfinished (merge 20). #735: 121 long functions, 48 debt keys and 11 allows. #736: 15,439 counted tools C++ lines and 4,786 fork test lines, plus separately inventoried embedded C++. #737: I and O's 18 evidence-held chains remain; W/V and the three scanner classifications are delivered, with five network exports retained for #730. #738 is delivered | #734: all 319 files finished and closure (work order item 6) done; #735 closes with it. #736: no file the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013) calls movable remains; each remaining C++ file under `tools/` and `src/tests`, and any embedded C++, either checks the calling boundary itself or reaches a live native pool, reader or production caller, and names the owner or boundary whose retirement removes it ([evidence](porting.md#evidence); root, 2026-11-11: the earlier wording required 65 pool and reader fixtures to check the boundary, which steering 34 keeps native until their owners retire them). #737: batches W, V, I and O delivered and its three withheld scanner rows classified (I and O added by root on 2026-11-10, from #839's review and #856's scanner rule). O is delivered when `--report-uncalled` is enforced against a baseline that only shrinks, each row naming the owner that retires its evidence caller (root, 2026-11-11). #738: its last item applied. Lint ratchets shrink per PR but set no exit, and the structural Clippy rows are outside it. Read each end state from the code and inventories, not from issue state | Current. #736 and #737 were reopened at this navigation: #810's "Closes #736 batch U" and #814's "It does not close #737" closed them on merge |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | About 37,800 compiled lines: script API 13,252, network 10,628, NewGRF 5,184, and save/load 13,023 less the pool and map chunks. The [queued owners](#after-cleanup), planned at integration 67, estimate 4,600 to 7,000 net C++ removed | Readers run in Rust with evidence for their observable outputs. Every remaining body in these buckets has a named later owner: VM and binding shims phase 6, script and network GUI phase 5, pool chunks phase 2 | Next. Plan it in full before cleanup's last items are dispatched ([goal and phases](planning.md#goal-and-phases)) |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | 11,899 compiled lines in storage, about 4,300 save/load chunk lines, the logic residue assigned to pool owners, and record field routes in the 16,759-line boundary layer | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in [accepted placement](#accepted-reader-and-pool-placement-2026-11-03-after-integration-67) |
| 4 | Support | About 32,200: support 21,191 and graphics and viewport 10,982, plus their data tables and startup | As in [planning](planning.md#goal-and-phases) | #3; plan deferred until adjacent (user, 2026-10-28) |
| 5 | GUI, drawing and platform | About 64,200: GUI 52,227 and platform 11,943, plus simulation and data-table drawing | As in [planning](planning.md#goal-and-phases) | As phase 4 |
| 6 | Vendored and final retirement | Vendored 52,831, plus whatever boundary, test and evidence C++ remains | Every original and fork-written C++ line retired and candidate native targets removed ([goal and phases](planning.md#goal-and-phases)) | As phase 4 |

Why the course changed at this navigation (cited at `89b6807754`):
1. The cleanup exit (`roadmap.md:124`) named no end state. "Lint and growth baselines shrinking" is a trend, and "streams done" could be read from issue state. GitHub had closed #736 and #737 on merges, leaving about 21,000 test C++ lines and two #737 batches. The exit now names end states, both issues are reopened, and the PR template no longer opens with `Closes #<issue>`.
2. Nothing planned phase 3 in full before cleanup ends: `start-development` step 3 planned only more cleanup. The queued owners cover about a sixth of phase 3's buckets. Planning and step 3 now plan it first.
3. Goal restatements had drifted. `AGENTS.md:4` retired only fork C++ "glue", against the 11-06 yardstick that counts tools and tests; `start-development` said "simulation first" and stocktakes "every sixth integration". All three now match the user's decisions.

Root's first actions on resume:
1. Follow the [resume checkpoint](#resume-checkpoint).
2. Drive each cleanup stream to its end state above. Check that #736's and #737's bodies list their remaining batches; record delivered batches there.
3. Once every remaining cleanup item is dispatched or in the ready queue, spawn the phase-3 planner ([start-development](skills/start-development/SKILL.md) step 3).

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. [Selection](planning.md#selection) ranks cleanup work by stream inventory. A #734/#735 pass owns the files it finishes whole. Another stream's PR may add separate test modules or one-line hooks to those files; it names the hook, and a conflict is resolved by base update (steering 34: #898 merged beside #894's overlapping parents without conflict, and no recorded reservation was shown to have prevented one). Before starting, the implementer checks its file set against open PRs' actual edits with `python3 tools/retrospective.py readiness`; root brokers no reservation. Files whose edits merge mechanically are exempt, including `lib.rs` module lists, `migration/*-debt.json`, `migration/harness-inventory.json` and generated registries; resolve those edits when updating from base. Cleanup PRs merge on their stream's proof, review and the merge queue's Full, growing neither fork C++ nor the boundary ([integration](root-workflow.md#integration)).

Slots (two implementers; user, 2026-11-10): while ready worksets remain, one slot runs #734/#735 file passes and the other runs items 2 to 4 in order. A running profile counts as one of the two. #738 runs as one docs agent outside the implementer count.

1. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, in one pass:
   - the original's wrapper types in internal signatures, return types and accessors. FFI exports, service tables and generated declarations keep base integers (#355);
   - named constants wherever a literal stands for a named value;
   - the original's explanatory comments at the matching branches;
   - #735's argument and context structs for the functions defined in those files;
   - the original's shared helpers, called where the original calls them. Where the helper has no Rust port yet, the pass adds it once in its header module (`Town::Iterate`, `Map::SizeY`, `ClampTo`, `CompanyMask`).

   **Bodies behind FFI exports** (steering 33, resolving root's [#735 deferral](https://github.com/fetch-laboratories/openttd-rust/issues/735#issuecomment-6726055806)). While its C++ caller remains, an FFI export keeps its positional base-integer list, so cleanup adds no generated declaration or C++ facade code. The command or helper body behind it moves into a Rust-internal function that takes the original's types and its argument or context struct, and the export becomes a shim that builds them, as #827's waypoint commands do. Rust callers call that typed function, never an `openttd_rust_*` export.

   A typed accessor replaces its raw form. A temporary raw adapter, delegating to the typed accessor, serves only callers in files no pass has finished yet; each adopts the typed form in its own file's pass. The pass converts callers in already-finished files to the typed form itself, as mechanical edits listed in its body. The narrow allowance's `_raw` renames apply only to held files that are not finished. An adapter retires with its last caller, or at #734 closure.

   **Finished-file residue.** Steering 33's lists were delivered by #871 (`94679169c9`). [Steering 34's list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702) joins the closure residue sweep unless a later pass touches the file; items that are macro sites are #892's. A finished file is one with no remaining finding of the [review brief](review-brief.md)'s every-line list, and implementers now check that before review ([implementer brief](implementer-brief.md#finishing-a-file-pass)).

   Split a workset only at a file boundary, and only when its diff would exceed about 2,000 changed lines. That size is provisional: the 33 vocabulary PRs averaged about 150 lines. Above about 1,000 changed lines, assign one reviewer per function group ([review](workflow.md#review)). Later PRs touch a finished file only for a mechanical shared-file edit or for a caller adopting a changed signature.

   Order (adopted 2026-11-08 from the overhaul thread's proposal, made at the user's request; it replaces closing passes for 01 to 09 and then the largest integer-site count first): pass a workset before its callers. A file passed after its callees adopts their typed forms in its one pass; one passed before them keeps raw calls and is touched again. Fan-in is the number of files outside the workset that reference it through `crate::` (measured at `5af17025a3`).
   - (a) Delivered: the 03 remainder, `station_cmd.rs` (#839) and `station_service.rs` (#868), followed by the finished-file residue #871.
   - (b) 13 Map/terrain (fan-in 90; `map_func` alone 85), with 23's type-bearing files: `bitmath_func` (39), `random_func` (21), `pool`, `math`, `kdtree`, `binaryheap`, `integer` and `container_func`. 23's fan-in of 137 is mostly `game_state`, `debug` and `witness` (45 to 46 each), which gain nothing from types; they and `raw_field` stay in 23's remainder.
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). The proposal put 04 before 06, but the calls between those two worksets run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25), and 23's remainder. The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Large files are not left for later. A file over about 1,500 lines gets its own single-file pass, placed by fan-in like everything else: before its workset's other files when they call it, and otherwise immediately after the workset pass that skipped it (root, 2026-11-09, after the planner showed that `station_cmd.rs` calls the station group and that nearly every 09 file calls `movement.rs`). Twenty-four workset files exceed that size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). A single file cannot split at a file boundary, so a single-file pass above 2,000 changed lines stays one PR with one reviewer per function group, as #819 did. The 03 remainder therefore becomes three passes: `station_cmd.rs`, `station_service.rs` (2,285), and `station.rs`, `station_map.rs`, `station_cmd/waypoint.rs` and `station_lifecycle.rs` together. #734 reports progress in lines finished as well as files.

   A pass's reviewer applies the review brief's every-line list to every line left in its files. Proof follows the [cleanup checklist](review-checklist.md#cleanup-prs). Skip the archive-equivalence attempt for a pass that changes signatures or adds argument structs: it cannot pass, and #801's attempt differed in 15 of 370 object groups.
2. **Tools maintenance.** Delivered: #855's Clippy ratchet (#858, `9b379f0d99`), #856 with #847 and #867 (#869: fork-only CTest name ratchet, instantiation-only orphan exports and the non-failing `--report-uncalled`), and #854's inventory tool (#870, `tools/code-standards-inventory.py`). Each pass reports its files' residue from that tool.

   **#899 retires Windows x86 MSVC and replaces its 32-bit Rust tests (user decisions, 2026-11-11), in one integration after in-flight NewGRF #898 reaches the queue.** It precedes further tools expansion and the F evidence family without interrupting #894 or #898. No Windows x86 job, target or Rust-only path remains, and a 32-bit Windows configuration is refused. `Rust tests (32-bit)` runs the workspace tests for `i686-unknown-linux-gnu`, a test proxy and not a game target. Windows x64, the Emscripten build and width-dependent source handling remain. Root owns workflow review/bootstrap and the replacement's width-dependent negative control.

   #897 delivered #896's Rust-only x86 Full change; the free-runner trial remains active. #896's author retains the one-day weighted-minutes and network-failure follow-up, distinguishing the later #899 cutover if it falls in the window; #897 alone cannot claim a mixed window's saving. Its Windows-specific acceptance is historical and superseded when #899 lands; #899 validates the replacement job, without duplicating the runner trial.

   #902 (`ci.py queue-wait`) and #904's growth-acceptance item are delivered (#905, merge 28); #900 carries #904's two workflow items. #901 (finished-file residue report and growth gate), #911 (assertions in the pre-queue Linux CTest build; root, 2026-11-11, after the merge-28 cadence went red on a `_DEBUG`-only fixture fault) and #737 batch O's last PR are one tools PR, #912. #891 is its own tools PR (#916); its sanitizer items 1 to 3 merge before #734 closure. #919 (Quick's base-relative gates compare against a stale base on pull-request events; root, 2026-11-11, from #909's false growth failure) is the next small tools entry and needs a bootstrap Full. Fold later small tools items into the next tools PR. #882 delivered #874's long-function counter, #875/#880/#881 and #736's embedded/extracted-original scope follow-up. The remaining #736 end-state classification still names each retained evidence boundary; new inventory visibility is not C++ deletion.
3. **#857 delivered** in #871: the native assertions dropped in the town, industry and station ports were repaired with the residue pass.
4. **#736 test evidence and #737 boundary debt**, one PR per batch where the batch fits the review bound above. Fold any item under about 150 counted C++ lines into another item of its batch. Delivered: #736 U (#810), P (#820), the W town, landscape and house families (#829, #833, #840), F cargo (#844), F vehicle scheduler and company economy (#853), oracle compaction (#877), scope follow-up (#882), W session, objectives and signs (#895) and W NewGRF (#898); #737 V (#814, #825, #837, #873, #876), O (#872, 21 chains), W with nine more O chains (#885) and I (#888). Remaining order (steering 34, [#736](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6789588232); classification bullets revised by root on 2026-11-11), replacing the 11-10 gross-line ranking:
   - **Restore the lost observations** in one small PR ([#156](https://github.com/fetch-laboratories/openttd-rust/issues/156#issuecomment-6799988195) items 1 to 7), preferring edits to kept native cases; it may add at most what the transfers removed, and only for this purpose is exempt from cleanup's no-growth rule. Records claim only what their tests observe.
   - **#736 classification, delivered.** One planner, no source change, seeded from [steering 34's table](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6799989049): each remaining file is movable now, retires with a named phase-2 pool or phase-3 reader owner, or is retained boundary evidence, ranked by counted C++ deleted per expected changed line. W-s realized 27 of 951 counted lines and NewGRF 154 of 836.
   - **Six movable items in the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s order** (delivered by `/root/plan_736_classification`, 2026-11-11; 13 files, 1,871 counted lines gross): the industry lifecycle probe, the link graph schedule probe, the Catch2 files that delete whole, the train and aircraft probes, the authentication probe and the ScriptList owner sections, each oracle with an `oracle-capture.py` recipe. A check of a live native binding, pool or production caller stays native until that boundary retires ([evidence](porting.md#evidence)).
   - **No further oracle transfers of console fixtures or of Catch2 files that reach live pools** (65 files, 14,273 counted lines): each retires inside the port that retires its pool or reader, counted in that port's C++ removed. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
   - **#737 batch O's last PR** (root, 2026-11-11) enforces `--report-uncalled`, which counts calls from upstream tests, against a baseline that only shrinks. #736's movable items free at most three of the 23 held exports, so the PR does not wait for them: each remaining row names the pool or reader owner that retires its evidence caller, and that owner removes the row. It follows #901 in the tools slot and may fold into that PR.
   - **#736's end-state inventory,** the stream's last PR, confirming the classification.

   Every #736 item follows the oracle rule on #736. T follows W and F only where a named remaining consumer requires it. Root re-checks actual open PR edits before commissioning each item.
5. **#738 rules delivered** at `c49bd95076`; the stream is closed at its end state. Keep the rule corpus measured, and consolidate later necessary process changes without reopening completed scope merely to fill a slot.
6. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. Seventeen Rust files (2,944 lines; `test_tables.rs` is 1,735 of them) are in no workset. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with #854's tool (item 2). #892 rides remaining file passes, then a finished-file residue sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

   #734 also owns the `unwrap`/`expect` and release-overflow fidelity gaps recorded in #156: remaining file passes handle their selected files, and closure handles recorded residue in already-finished files. Compare each with the original's preconditions and observable failure behaviour; an occurrence alone is not a defect. This ownership ruling does not expand #892's macro scope or add a census, lint rule or gate.

7. **#893 cleanup-exit evidence**, after closure and before the first phase-3 reader dispatch: run the coverage measurement, seeded-fault pilot and all-member deep check. Pause implementers for the two host measurements. Read the report and handle findings under [deep checks](deep-checks.md#policy) before dispatch. Phase-3 planning follows its normal trigger and does not wait for these results.

Corrections adopted on 2026-11-07 under [#746](https://github.com/fetch-laboratories/openttd-rust/issues/746), still in force:

- #734's proof uses the [cleanup checklist](review-checklist.md#cleanup-prs): unchanged operations and order, plus the merge queue's Full when compiled code differs. Per-PR Callgrind profiles and disassembly accounting are not required.
- #747 is integrated through #751/#752, and #855 narrows it further: the Clippy ratchet keeps parameter count and safety documentation only. Remove unnecessary existing cast suppressions on later base updates.
- #734 permits the original's integer uses through wrapper accessors and measures wrapper adoption rather than Rust enum counts. The implementer brief places helpers by their original file, including `.cpp`-only helpers (#738).

## After cleanup

Readers first, in this order, re-estimated by fork C++ when selected:

| Rank | Owner | Issue estimate | Notes |
| ---: | --- | ---: | --- |
| 1 | #669 afterload/legacy | 1,800 to 2,600 | Conditional rank: re-check all-pool writes against boundary falls or holds before selection; pool records remain C++-owned until phase 2, so this owner may need to follow storage |
| 2 | #730 network core/discovery/resource | 700 to 1,100 | Resume from closed draft #731's branch |
| 3 | #669 GRF configuration/common publication/actions | 500 to 800 | Remaining SpriteGroupPool lifetime |
| 4 | #573 continuation | 350 to 550 | Codec, corrupt input, LoadCheck and wrappers; pool chunks move with their pool |
| 5 | #668 coordinator/STUN/TURN | 400 to 600 | |
| 6 | #669 admin, then script discovery/configuration | 350 to 550; 500 to 800 | Admin includes its socket pool policy |

Then storage, each pool with its save/load chunks, redone from current base with each closed draft's branch as reference: #656/#598/#599 vehicle (#714), #634 station/road-stop/cargo (#700), #638/#498 small pools (#723), #635 town/industry/subsidy (#704), #637/#497 remaining engine pools, #717 attachment, #725 YAPF, #906 orders, #907 companies and groups, then the map (#565's remainder). #716 tracks the all-pools closure.

## Closed drafts

Closed 2026-11-06 with branches kept; heads and states are also on each PR and issue. Redo each from the cleaned base in the [after-cleanup order](#after-cleanup), using its branch as reference.

| PR | Issue | Head | State at close |
| --- | --- | --- | --- |
| #700 station | #634 | `0256b0bbdb` | Cargo, builds and 12 focused cases passed; selected-native result unverified; foreign Industry/Town/PSA/virtual-dispatch seams open |
| #704 town/industry | #635 | `60757916b4` | Unfinished 15-file foreign-consumer edit; Clippy failures; `OpenTTDIndustryTown` still has native news callers |
| #714 vehicle | #656 | `20c5f1bb99` | Cargo passed; native/runtime pending; engine field route open; conflicts with 102 |
| #723 small pools/ObjectSpec | #638 | `2ec78fb676` | Four checkpoints pushed at close, unvalidated; next was adopting `Shared.grf_price_shift` |
| #731 network | #730 | `655d8f8990` | Saved unfinished lifetime work, unvalidated; prior `595232f50c` passed Cargo and native |

## Accepted reader and pool placement (2026-11-03, after integration 67)

Agent: /root/place-after67-reader-plan | Model: gpt-6.1-sol | Reasoning effort: high

Root accepted these ownership boundaries. The current reader envelope is in
[Phases](#phases); #669 holds the remaining owners and their selection events,
not an additive retirement promise. #653's industry definitions are integrated;
specification files classified under simulation storage/logic add work outside
the NewGRF bucket. Retain the VM, registration glue, stack conversions and
exception/suspension shims until their phase-6 owner. Executable API header
methods belong to their API owners; whole headers and new forwarding methods
are not savings.

Pool briefs include allocation bookkeeping (`data`, `used_bitmap`, `first_free`,
`first_unused`, `items`, assertion credit `checked`, `cleaning` and applicable
allocation-cache state), allocation/indexed allocation, reuse, iteration,
reset/cleanup and construction publication/destruction hooks. The generic
native pool template earns no repeated deletion credit while any C++ pool
instantiates it. Every Rust-used name/container/cache field moves in phase 2;
only exact original-C++-only fields may await a named later owner. Record lookup,
coupled save descriptors and generated declarations follow integrated #570.

Exact remaining shell dispositions constrain briefs:

| Boundary | Phase-2 ownership and exact later-owned remainder |
| --- | --- |
| Vehicle/type shells | Combined #656/#598/#599 takes maintenance, cargo/identity/loading, resolver fields, names and Rust-used sprite/cache fields; existing controller/cargo/order/visual/occupancy owners stay canonical. `coord`, `bounds`, viewport hash links and `fill_percent_te_id` belong to phase-5 drawing/viewport; `old_orders` belongs to #669 legacy conversion. Identity/vtable/owner-pointer scaffolding retires with final callers. |
| Stations/road stops/cargo | #634 takes names and Rust-used caches, spec-reference vectors, custom road-stop data, catchment/loading/nearby-list facades, GoodsEntry cargo/flow attachments and CargoPacket/CargoPayment pool completion. Link-graph owner takes flow/completed-job publication. `sign` belongs to phase 5; spec definitions belong to #669. |
| Town/industry/subsidy | #635 takes town `psa_list`, Rust-used `cached_name`/caches, industry `psa`, town/station links and subsidy flags; existing Rust name/cargo/count/nearby owners stay canonical. Town `cache.sign`/`show_zone` belong to phase 5. Industry `cached_name` may await phase-4 formatting only without a Rust field route; presentation facade storage retires with phase-4 string callers. Persistent-storage payload/pool is #638; industry definitions #653 and HouseSpec #669 after #628. |
| Engine | #637/#497 takes info, vehicle-specific info, original image/type/list position, GRF/override/badge payloads, lifecycle and name access after adopting #655. Only `display_flags`/`display_last_variant` await phase-5 build-list GUI. |
| Orders/lists/backups | All current fields and pool state move in phase 2; `old_order_index` and `OldOrderSaveLoadItem::{index,next,order}` belong to #669 legacy conversion. |
| Company/group | Phase 2 takes settings, finance/statistics aliases, identity, names/livery and free-unit/free-group state used by Rust. AI instance/info/config payload belongs to #669 script orchestration; authorization-list payload to #669 network remainder. Rust-used links move with the record. Only `Group::folded` awaits phase-5 GUI. |
| LinkGraph/LinkGraphJob | Graph/settings snapshots, join/abort state, annotations, flows and publication move in phase 2, including #168/#199/#500 completed-job publication. A native thread handle may remain behind a named join/start operation until platform threading moves; no field-callback exception follows. |
| Small pools | #638 includes objects, depots, signs, goals, story pages/elements, league tables/elements and persistent storage, plus #498 payloads and Rust-used strings. Only `Sign::sign` awaits phase-5 drawing/viewport. |

All issue estimates are net compiled C++ removed, including retained/generated
declarations, facades and fixtures. Rewritten consumers receive no reduction
credit. #658's **+445 code-line .inc re-baseline** is reported separately from
its small cleanup savings and #657's deletion savings. Reader evidence compares
API return values/errors/command results/enumeration and protocol packets,
ordering, timeout/error/connection decisions; world-state equality alone misses
these observations. Reader owners use integrated interfaces only. Root supplies
a shared service once under [integration](workflow.md#integration); a service
need does not park the whole owner behind an unrelated draft. Real state and
lifetime dependencies retain their selection events.

## Thirty-fourth steering review (2026-11-11)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: xhigh

Root was paused at the user's wind-down. This review covers the Claude Code root of 11-10 08:34 to 17:20 and the Codex root of 11-10 20:52 to 11-11 06:54: 34 merged PRs since steering 33, 26 of them through the merge queue, and #894 merging as this review ran. Six fresh audits ran on `gpt-6.1-sol`, and then, at the user's direction, six on Opus; findings from both are kept. They compared the station passes and residue (#839, #868, #871); vehicle transactions and boundary work (#879, #883, #888, #885); map leaves and #894 (#878); network and text passes (#887, #889, #886, #884); #736 evidence transfers; and process cost. Claims acted on below were verified against the source; no game was run for them.

Findings:

- **Fidelity held in release builds with assertions on; the defects are in `_DEBUG`, crash form and assertions-off paths** ([#903](https://github.com/fetch-laboratories/openttd-rust/issues/903)). #894 routed `TileAdd`'s `_DEBUG` projection into helpers that earlier passes had substituted for the original's plain `tile + TileOffsBy*()`, at about 35 sites. Under native `_DEBUG` with freeform edges off, a buoy beside the north edge and the disaster submarine on row 0 abort where the original continues. No CI tier runs a `_DEBUG` game. Also: a malformed client auth response reaches `unreachable!()` where the original's `NOT_REACHED` gives the native crash report; an airport cost estimate with no towns dereferences null with assertions off; and #871's #857 repair covers one of three callers.
- **Finished files still leak residue.** Residue per finished file fell since steering 33 (missing comment lines 2.6 to 1.5, bare-integer sites 2.4 to 1.8). All four fidelity audits still found export calls, raw adapters, opcode dispatch, scratch pools, `execute: bool` projections, service-table constants and omitted assertions in finished files ([list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702), [addendum](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6799987657)). Of 135 initial review findings, 107 were standards findings, and up to 83 were in classes a grep or the inventory tool flags. The review brief already covers most of these, and they pass implementers: #883 reached review with ten standards findings, and seven finding classes recurred in at least three PRs.
- **Review grew a third stage.** Over 11 PRs from 11-10 20:52 the Codex root ran 18 review, 16 re-check and 11 acceptance sessions. Reviews found 10 behaviour or evidence defects and 30 standards findings; re-checks found one linkage defect (#884); acceptance found none. Acceptance came from two rules meeting: `1227a1becc` said only an acceptance needs the `native` result, and the T3 host requires a re-check to be a new delegate.
- **#736's ranking promised deletions the method cannot deliver.** Counted C++ fell 877 lines across #844, #853, #877, #895 and #898. W-s realized 27 of 951 lines and NewGRF 154 of 836; #895 slipped the existing 150-line fold rule. Six observations were lost or weakened and three records overstate their tests ([#156](https://github.com/fetch-laboratories/openttd-rust/issues/156#issuecomment-6789588762), [addendum](https://github.com/fetch-laboratories/openttd-rust/issues/156#issuecomment-6799988195)). Compaction (#877) was verified complete. Of the 20,044 counted lines left, 10,780 are console fixtures, which transfers retired at 3% and 18% of their gross lines; about 8% to 17% of the inventory can move by transfer now, and the rest leaves with its boundary ([#736](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6799989049)).
- **The ready queue carried PR state and whole-file holds.** Its planning content is its purpose. Two parts were not: PR state that GitHub owns, which went stale within hours (#894's retry state), and reservations on whole files or parents, none shown to have prevented a conflict while #898 merged beside #894's overlapping parents. Seven planner refreshes ran in ten hours, about 610,000 uncached tokens.
- **Root's turns went to routine work.** PR and queue watching, process scans for children's jobs, merge notices, CI-failure relays and file-reservation brokering took about 47% of the two roots' tool calls (Claude Code 622, Codex 1,501). The Claude Code root also ran 13 `tail --pid` waits and relayed review findings twice, against team's rules.
- **Three of four queue removals were not the PR's own defect** ([#904](https://github.com/fetch-laboratories/openttd-rust/issues/904)): a 32-bit-only Clippy lint and a C++ test warning that run only in Full, and a growth acceptance bound to exact totals that another PR's merge voided. They cost 227 queue minutes above the median and 240 weighted paid minutes.
- **The merge queue works.** Median final queue entry to merge was about 15 minutes, and median last Ready to merge was 73 minutes under the Claude root and 53 under the Codex root. Of 44 queue runs, 37 passed, six failed and one was running at the cutoff; three failures were one stale fixture initializer, caught by Quick.
- **The free-runner trial raises the network-failure rate.** Three of 11 first-attempt simulation tiers on free runners hit the #693 family, against 3 of 75 (4.0%) on Blacksmith in the same period and 11 of 201 (5.5%) in #896's baseline; three or more in 11 has a probability of 0.8% to 2.0% at those rates. The runner-kind decision is the user's (#896).

Standing corrections from steering 33:

| Correction | Verdict | Cost and what it caught |
| --- | --- | --- |
| Finished means finished | Partly slipped | #871 cost 4,262 changed lines and fixed the audited lists plus #857's assertions; the rule was right, but prevention was missing. Action 2 |
| #855 ratchet | Applied | Failures fell from 1.8 to 0.455 per PR, below the provisional 0.5 |
| #857 | Applied | Delivered in #871 |
| #736 and #737 follow-ups | Applied, two gaps | #877 removed 2.35 MB of oracle bytes; #853 left the two gaps now on #156 |
| Bound the roadmap | Entry count held; PR state and whole-file holds returned | Action 4 |

Rule changes outside steering: the merge queue, two-implementer cap, CI scenario comparison, wind-down-only checkpoint and direct messages (user, #860) are all applied and retained. Windows x86 in Full (#862) cost about half of the 11-10 increase in paid minutes and is being removed at the user's direction (#897, #899). Free runners are untested: compare queue failures per merge at stocktake 32.

Actions:

1. **At most two review stages** ([review loop](team.md#review-loop)): the source review, then one re-check only when a fix changed behaviour, evidence or a body's operations. No separate reviewer re-verifies a CI run; the implementer waits on its own `native` run, records it and queues. Keep direct reviewer and implementer messages. A re-check is requested through root because the host requires it to be a new delegate.
2. **Finish a pass before review.** The [implementer brief](implementer-brief.md#finishing-a-file-pass) adds a per-file completion step, and [#901](https://github.com/fetch-laboratories/openttd-rust/issues/901) gates growth of the mechanical classes in finished files. The residue list joins #734's closure sweep.
3. **#736 classifies before transferring more** ([work order](#cleanup-work-order) item 4), and live native boundaries stay native ([evidence](porting.md#evidence)).
4. **Ready queue.** PR state stays on the PR, reservations name actual overlapping edits, and the planner refreshes when no entry is ready for the next slot to free ([team](team.md#ready-queue-and-pipelining)).
5. **#903, one fidelity PR, comes first:** every caller of the tile-addition helpers matches its original, `RustTileAdd` goes, the `_DEBUG` body leaves the release hot path, the two reachable paths get `_DEBUG` tests, and the auth, airport and assertion items are fixed. The defects are `_DEBUG`-only or assertions-off, so #894 is fixed forward rather than rolled back.
6. **The implementer waits on its own queued PR; root watches none** (user, 2026-11-11). Root counts merges from `origin/rust-migration` and hears of removals from the implementer. #902 adds the background wait the host allows a delegated agent ([integration](root-workflow.md#integration)).
7. **Root's turns go to selection, rulings, briefs and the roadmap** ([delegation](team.md#delegation-user-2026-11-08)), at the user's direction (2026-11-11):
   - implementers read their own failures, check their own file overlaps, and start their own reviewers and re-checks from a fixed spawn message that never says what to look for;
   - root dispatches ready entries as the planner wrote them;
   - a [routine agent](team.md#routine-agent) on the routine model runs housekeeping, the platform cadence, bootstrap Full steps and the missed-resume check, and root decides on its results;
   - children check their own jobs before ending a turn; root makes no job checks or process scans.
8. **The queue removes a PR only for its own defect** (#904): growth acceptances bind to the PR's own added keys, and the C++ test warning check and 32-bit Clippy run before the queue.

No user direction is reversed. The re-check's trip through root is a host constraint on the 11-10 direct-messages rule, which otherwise stands. The planner trigger and #901's false-failure threshold are provisional. Steering 35 judges them, and whether defects escaped once the acceptance stage was gone.

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
fork-C++ change, decided before review. Rust-defined `extern "C"` functions
return 0-or-1 `u8`, not `bool` (#644). Read a failed Full to its last job before
recording its cause. The decided beside placement and its revisit conditions live in the world-state design. Per-message `Debug` and `GrfMsg` services are design faults
after #657 integrated. A change to Cargo, the toolchain or cmake runs the
platform profile once before Full. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence. Native fatal paths keep the native error
box and crash reason (#709). Native diagnostic subsets address a named failure
or uncovered fixture; exact-head Full is the final simulation tier. A completion
claim names only the delivered scope (#636). Release a quiet-host hold when the
timed games end; a rejected peripheral host action does not end the assignment.

Steering 31 (2026-11-06), still in force: an ownership port's checkpoint integrates once it meets the [integration](root-workflow.md#integration) rule and the owner continues in a fresh PR, while cleanup passes finish their files instead; a field moves in the same change that deletes its Rust-called services, before pool adoption, and no service routes through `Record()`; the whole-fork metric runs twice per PR; status comments change at milestones; long jobs start as `exec python3 tools/signal-witness.py -- <command>`, and a host-killed job (exit 143, no logged failure) reruns once incrementally (#724).

Steering 33 (2026-11-09), still in force:
- a pass adds missing shared ports, moves command and helper bodies out of FFI exports into typed internal functions, and converts finished files' calls to callees it types; `_raw` adapters serve only unfinished files ([work order](#cleanup-work-order) item 1);
- the Clippy ratchet keeps parameter count and safety documentation only, against the merge base (#855, #858), reversing the 11-06 approval of the length, nesting and complexity families on measured cost;
- the delegation model, report-only reviewers, implementer fixes and twelfth-merge stocktake stand; both providers use them;
- root commits the roadmap only at stocktakes, course changes and wind-down, with the planner's queue refresh the one exception.

Steering 32 (2026-11-07), still in force:
- cleanup work is ranked and reported by stream inventory;
- #734/#735 passes finish their files, with two implementer slots;
- #806's declared carries replace rename, fidelity and renewal contracts;
- #736 oracles store each distinct output once, under #736's budget;
- review reports and integration notes carry findings, fixing commits and the disposition, and a base update records only its applicable condition.

## Recent completed work

Evidence and review reports stay in the linked PRs. Metrics are actual reductions where stated, not sums of rewritten lines. Per-integration rows for 69 to 129 are in the roadmap at `a9ca01d657`.

| Integrations | Work | Result |
| --- | --- | --- |
| 69–102 | Porting through steering 31: settings, corpus, typing, shared records, GRF text, logging, vehicle records #636, script APIs, industry specs, game protocol, fatal reporting, accounting tools, map leaves, shared pool core/facade, content state #706, graph/job pool #719, engine records #722 | Compiled C++ −4,289 through 88; #722 −852 at 101. Original retirement reached 22.07%. |
| 103, 107 | #733 checks #742; lint narrowing #747/#751 | Quick CI runs Cargo checks and the lint, policy, growth and proof gates. Tooling +1,956 code lines. |
| 105, 109–126 | #734 foundation F #739 and 33 vocabulary PRs (#753–#798) | Rust +4,900 added lines; C++ and boundary hold. Worksets 02 and 03 checked off; 132 of 319 workset files touched. |
| 128 | #735 P1 Station helper contexts #801 (`5d43172624`) | Eight parameter-count rows retire; Rust +310/−200. |
| 104, 107, 108, 127, 129 | #736 R0 #740, cargo/engine #749/#750, MovementGap #748, Company finance #802 (`0c8a0f8a05`), Town/Industry policy #803 (`7b6887f43d`) | 127 and 129 retire 467 counted C++ lines for 1,273 net Rust test lines and 2,071,485 raw oracle bytes. All 40,320 finance and 21,128 policy cases preserved. |
| 104, 106, 108 | #737 C #741, H #745, R #744 | Compiled C++ −151, exports −10. |
| 130 | Housekeeping retention of unpublished detached commits #804/#805 (`b7b686b51a`) | Tooling +33 code lines; C++, Rust and boundary unchanged. |
| 131 | #806 declared lint carries, fidelity suppressions and content-bound acceptance #808 (`305ac20c28`) | Tooling +563 code lines; C++, Rust and boundary unchanged. Module-scoped rows still bind acceptance to the exact head (#806). |
| 132–146 | #734/#735 file passes: workset 01 (#807, #812, #819), 02 (#811, #816, #821, #828), 03 (#813, #827, #834) and 13/23 part a (#832). #736 U #810, P #820, W #829/#833/#840. #737 V #814. Maintenance #809 (#726), #808 (#806), #826, #818, #831, #842, #838 | 51 of 319 workset files finished, with residue listed on #734. Fork C++ −1,915 from 129 to 146 (Phases); exports 1,552 → 1,539. No rollback. |
| — | #858 Clippy ratchet for #855 (`9b379f0d99`) | Clippy debt rows 649 → 59: length, nesting and complexity retired; growth-only against the merge base. |
| 147 | #851: #849 coverage fix, #846 #835 timing tests, #825 #737 V part 1, #844 #736 F cargo, #843, #850 (`489f4ea161`) | Restores the 14 kept CTest cases that integrations 142 and 144 dropped (coverage regression; #849 guards the route). #844's income oracle misses #736's budget: 1,981 bytes per counted line against 1,500, left for oracle compaction. |
| — | #859 checker-only Full and unchanged growth acceptances (`ba7bc3fd44`) | A PR changing only Quick's checker scripts, their debt files and inert Markdown runs Quick alone for Full (about 8 minutes, not 22). Growth acceptances hold while the added source is unchanged. Inert Markdown narrowed to `AGENTS.md` and `docs/**.md` minus CMake-packaged docs, for carry and post-merge skips too. |
| Merges 1–8 (queue, 11-10) | #866 preflight parity (`048109075c`); #734 passes #852 `saveload.rs`, #839 `station_cmd.rs` and #868 `station_service.rs` (`c12561671f`); #737 V part 2 #837; #736 F vehicle scheduler and company economy #853; tools #869 (#856, #847, #867) and #870 (#854) | 54 of 319 workset files finished, worksets 01 to 03 complete. Fork C++ census 277,044 → 276,186 from 146; historical exports 1,539 → 1,536; callback slots 3,332 → 3,298. No rollback; #853 left the queue twice before merging. |
| Merges 9–20 (queue, 11-10) | #737 O/V/W (#872, #873, #876, #885); residue/assertions #871; file passes #878, #879, #883, #884, #886; oracle compaction #877; tools/scope #882 | 92 of 319 entries finished. Same-version fork C++ −432; shared/rebound/orphan baselines 87/279/30 → 48/245/27; thirty uncalled chains removed. #738 docs item delivered separately. #885's Clang queue finding was fixed before its successful retry. |
| Merges 21–27 (queue, 11-10/11) | Passes #887, #889, #894; #737 I #888; #736 W-s #895 and NewGRF #898; CI #897 | Fork C++ 275,754 to 275,523 at merge 26. Exports hold at 1,508; callback slots 3,219 to 3,211. #894's queue retry passed. No rollback. |
| Merges 28–32 (queue, 11-11) | Tools #905 (#902, #904 growth binding); fidelity #908 (#903); CI #900 (#899, #904 workflow items); #736 restore #910 (#156 items 1 to 7) and industry lifecycle probe #915 | Fork C++ 275,523 to 275,278; callback slots 3,211 to 3,210; exports hold at 1,508. Windows x86 retired with `Rust tests (32-bit)` in its place. The merge-28 cadence went red on a fixture that #894's `_DEBUG` assertion exposed; #908 fixed it forward, with no rollback. No queue run failed. |

## Recurring

- Merges, [one merged PR each](root-workflow.md#integration), count from the switch to the merge queue after integration 147.
- After every twelfth merge (user, 2026-11-08), take stock under `/start-development` step 4 and profile the merged tip in the background ([speed](speed.md#instruction-projections)). A running profile counts as one of the two implementers. The merge-32 stocktake is complete and its profile goes on #275; the next follow merge 44.
- Run the platform cadence after every fifth merge; the merge-28 cadence went red on macOS Debug and #908 fixed it forward, and the next follows merge 33. A new failure stops merges ([integration](root-workflow.md#integration)). Full retains Clang and GCC Dedicated (#860). The cadence runs macOS and the platform comparison. #900 (merge 30; #899, user, 2026-11-11) removed Windows x86 from Full and the cadence and added `Rust tests (32-bit)`, which runs in `native`, Full and the cadence. Windows x64 and the Emscripten build remain unchanged. Cadences 135 (Windows x86 oracle tests, fixed by #818) and 140 (`-Wmissing-field-initializers`, fixed by #831) each stopped integrations for about 75 minutes.
- All-member deep checks are requested at cleanup exit and phase-3 exit; other deep runs remain on request ([policy](deep-checks.md#policy)).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions and reports #892's remaining production macro counts beside the #734 inventory.
- #156 (coverage gaps), #275 (speed), #724 and #710 (host) stay open trackers. #500 is delivered by #734; #168 and #199 attach to #737 and the owners that touch those records. #693 retains the cancelled reconnect failure and two completed original/original runs that did not reproduce it; the cause remains unresolved and that two-run authorization is exhausted. Its loopback hang cost two Full reruns in root's 11-08/09 session (integrations 141 and 145); the implementer queues the PR again once ([integration](root-workflow.md#integration)). #736 P (#820) delivered without touching it, so diagnosis returns with #730's network owner after cleanup, or sooner if hangs exceed one rerun in ten Full requests at a stocktake (provisional; 2 in 26 measured). #503's coverage continuation runs last.

Excluded, with issue placement and return conditions rechecked at merge 20 (2026-11-11); near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Ready queue

Two or three undispatched work orders ([delegation](team.md#ready-queue-and-pipelining)). At most two implementers and six unmerged component branches. Entries 1 and 2 are the file-pass slot's and entries 3 and 4 are the other slot's ([work order](#cleanup-work-order)); a free slot takes its own top ready entry, and the other slot's when it has none. Entry 4 is a fourth entry, for the tools PR of work order item 2, added beside the three file-pass and evidence entries, which stay. In the other slot it goes ahead of entry 3 once root has ruled on its checkpoint rule, because item 2 precedes item 4; until then that slot takes entry 3. File sets were rechecked on 2026-11-11 at merge 35 (`0f5ac588af`) against the edits of the one open PR, #916, with `python3 tools/retrospective.py readiness`; each entry names the overlaps found.

1. **9m, #734/#735 movement file passes: `movement.rs`, `movement/tests.rs` and `signal_func.rs`.** This shared callee precedes the rest of workset 09 and its movement callers; leaf network passes do not displace it. Include #892's assertion class and #734's recorded fatal-path fidelity gaps in the selected files' every-line pass; closure owns already-finished residue.
   - Size: 1,840 raw lines, 1,574 of them in `movement.rs`. About 1,730 changed lines at 20c's realized 0.94 changed lines per raw line, replacing the earlier 748 to 1,348 ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6809207113)); refine it at the first checkpoint. Above about 1,000 changed lines it takes one reviewer per function group.
   - Typed conversions reach 68 external referencing files, 47 of which also reference the two map modules, so it waited for the `tile_map.rs` pass, which edited `movement.rs`, `native_test_support/movement.rs` and 46 other files. That pass merged (#909, merge 34), so this entry is ready now. The open PR, #916, edits none of its three files.
   - Preserve W-s's `signs_cmd.rs` hook when converting its two `movement::pixel_z` calls, and the test-module hooks NewGRF left in `newgrf_text.rs` and `newgrf_townname.rs` and the industry lifecycle probe left in `industry.rs`. The four `native_assert` calls from network files take no type this pass converts.
2. **20d, #734/#735 network file passes: `network_action.rs`, `network_socket.rs` and `network_client.rs`.** The first half of workset 20's last six files, split again at a file boundary with the most widely used files first, because 20c realized 0.94 changed lines per raw line against the 0.45 estimated ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6809207113)). It fills the file-pass slot only while entry 1 is not ready.
   - Size: 1,440 raw lines and 44 `extern "C" fn` items on 20c's head; about 1,350 changed lines at 0.94, to refine at the first checkpoint. The inventory tool reports 8 missing comment lines and no bare-integer site; `network_action.rs` holds two production sites of #892's macro class and `network_client.rs` two `native_assert` calls to settle in place.
   - Bodies behind the exports move to typed Rust-internal functions and the exports become shims. Calls into the unfinished `network_dispatch.rs` and `network_lifecycle.rs` keep their raw forms, and those two files and `session.rs` get a raw adapter only where a signature changes. The pass itself converts the callers in the files 20c finishes (`network_auth.rs`, `network_command.rs`, `network_connection.rs`, `network_frame.rs`) and in its new `network_server.rs`, as mechanical edits listed in the PR body. Leave `test_tables.rs` alone beyond a changed signature.
   - Ready now: the 20c pass merged (#918, merge 35). It edited all three files (109 changed lines) and defines the typed forms and header modules this pass adopts. #916 touches none of them.
3. **#736 link graph schedule probe** (work order item 4, the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s item 2): retire `tools/simulation/linkgraph-schedule-reference.cpp` and `scheduler_reference` in `tools/simulation/play_saves.py` into a Rust oracle test of `linkgraph_schedule.rs`.
   - Size: 187 raw C++ lines (161 counted) and about 70 driver Python lines deleted. The classification expects 975 changed lines outside oracle text; the industry lifecycle probe realized 1,903 against 1,590 expected, which puts this one near 1,170. It reports its realized share at the first checkpoint.
   - Files: the deleted probe; `play_saves.py`; a new `linkgraph_schedule` recipe in `tools/oracle-capture.py` with its patch under `tools/oracle-recipes/`, capturing the eleven extracted `LinkGraphSchedule` bodies with the fixture's shells; a separate Rust test module and its oracle text, with a one-line hook in `linkgraph_schedule.rs`; rows in `test_tables.rs`; a record under `migration/evidence-replacements/`; and the probe's line in `migration/harness-inventory.json`.
   - Follow the oracle rule on #736 and the industry probe's pattern (#915): freeze the original's observations before deleting the producer, keep a nonempty default-CI route for the save-play scenarios' `scheduler_reference` finding, and claim in the record only what the tests observe. `src/tests/linkgraph_schedule.cpp` (native LGRS staging, #717) and `tools/migration/windows-abi/linkgraph.inc` stay native.
   - In `tools/uncalled-export-baseline.txt`, remove the `openttd_rust_schedule_dequeue` row and narrow the `openttd_rust_schedule_queue` row to #634, whose `station-lifecycle-console.hpp` still calls `LinkGraphSchedule::Queue`; the baseline only shrinks.
   - Ready now: the tools PR that adds that baseline file merged (#912, merge 33). No open PR edits any file above; #916's edits to `tools/migration.py` are away from `rust_archive` and `rust_configuration`, which the deleted driver called. Entry 4 also edits `tools/oracle-capture.py` (its output writer, not the recipe list); whichever of the two merges second updates from base.
4. **Tools PR for #919, #920 and #923: Quick's base, enforced preflight and a probe compile check** (work order item 2). One PR, because all three edit `ci-quick.yml` or `tools/preflight.py` and share one bootstrap Full. #920's CI review counted 187 failed runs from 11-08 to 11-11, about 140 of them on gates that run locally, and 58 of the 133 failed pull-request Quick runs repeating the same step on the next push.
   - #919 goes first, in its own commit: every base-relative Quick gate takes the first parent of the tested merge commit as its base, on pull-request events and requested profiles. It failed #909 and #916 on 11-11, and each paid a base update and a second `native` run.
   - #920: `tools/preflight.py` records a pass stamp for the tree and base it checked; a pre-push hook, installed in every worktree as `post-checkout` is, refuses a non-Markdown push without the stamp and always runs a fast tier; a checkpoint push takes a logged override; Quick does not run on draft PRs and runs on `ready_for_review`; `ci.py request` requires the stamp; `tools/oracle-capture.py` strips trailing whitespace and writes tab-indented `capture.cpp`; a pre-commit hook runs the pinned checker's diff rules on staged files.
   - #923: when a change touches shared headers, the generated ABI, a probe or a runtime adapter, preflight compiles every comparison probe and runtime adapter without linking, and otherwise prints that it skipped the step. Four requested runs failed on such a break. `tools/probe.py` reads the compiler and Rust configuration from a configured build directory, so the step needs one and must say so when it is missing. Report the step's time if it adds more than about a minute.
   - Size: about 450 to 550 changed lines, about half of them seeded tests: under 30 for #919, about 300 to 400 for #920 and about 100 for #923. Refine it at the first checkpoint.
   - Files: `.github/workflows/ci-quick.yml`, and `ci-request.yml` only where a requested profile passes its base; `tools/preflight.py`, `tools/test-preflight.py`, `tools/ci.py`, `tools/test-ci.py`, `tools/oracle-capture.py` and a hook installer with its two hook scripts. The docs that state the checkpoint rule (`AGENTS.md`, `docs/workflow.md`, `docs/implementer-brief.md`, `docs/rust-migration.md`) change in the same PR, with the text root rules on. #916 edits none of these; #923 calls `tools/probe.py` and `tools/migration.py` as they are and does not edit `migration.py`, which #916 edits.
   - It changes `.github/workflows/`, so it queues only after root's workflow review and a bootstrap Full on its head ([CI administration](root-workflow.md#ci-administration)).
   - Ready now. Root ruled on #920's checkpoint rule on 2026-11-11 ([#920](https://github.com/fetch-laboratories/openttd-rust/issues/920#issuecomment-6810043988)): the logged override skips the stamp and runs nothing slower than a few seconds, and the growth gate belongs to the stamped tier.

**Later.** The rest of workset 09 follows 9m. 20e (`network_dispatch.rs`, `network_lifecycle.rs` and `admin_conversion.rs`; 1,421 raw lines, 68 `extern "C" fn` items, about 1,340 changed lines) follows 20d, under the same idle-slot rule. #736 follows the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s order after the link graph schedule probe: the Catch2 files that delete whole (326 counted lines), the train and aircraft probes with `src/tests/train.cpp` (384), the authentication probe (160) and the ScriptList owner sections (120 to 200), then the end-state inventory. Each needs a new `oracle-capture.py` recipe. No console fixture moves by transfer: the 65 files and 14,273 counted lines of class 2 retire with their pool or reader owners, and the 33 files and 3,909 lines of class 3 with their boundaries. Earlier analysis for 7a and 9m is on [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6787577538).

**Placement.** #896 stays with its author for the one-day weighted-minutes and network-failure follow-up; record any #899 cutover separately rather than attributing a mixed window to #897. #891's sanitizer items merge before #734 closure. The #891 tools PR (#916) changes no workflow file and is nearly at review, so #919 did not fold into it and joins entry 4. The four infrastructure issues the user filed on 2026-11-11 rank in the tools order by the failed runs each removes:
- First, #920 and #923 with #919, as entry 4.
- Second, #922 (`ci.py enqueue` refuses a head whose latest Quick or requested run is red, an ejected PR queues again only after a base update and a preflight pass, and stale-head refusals and negative controls stop counting as failures). Four of the six merge-queue Full failures of 11-10 and 11-11 were avoidable, at seven failed jobs each, and five more runs were stale-head refusals. It is its own tools PR of about 250 to 350 changed lines in `tools/ci.py`, `tools/test-ci.py`, `ci-request.yml` and the queueing steps of `docs/root-workflow.md`, `docs/workflow.md` and `docs/team.md`. It changes `ci-request.yml`, so it needs its own workflow review and bootstrap Full. It waits on entry 4 to merge: its re-queue rule reads #920's stamp, and both edit `tools/ci.py` and `tools/test-ci.py`.
- Third, #921 (carries and growth acceptances in a tracked file; malformed records fail in one line; growth awaiting acceptance reports as pending). About 16 of 23 lint-policy failures were carried suppressions, two were tracebacks and 17 were growth failures, some of them #919's class, which entry 4 removes first. It is not one PR with #919: #919 is a workflow fix that is ready now, and #921 changes the review contract. It is its own tools PR of about 350 to 500 changed lines in `tools/cleanup_contract.py`, `tools/lint-policy.py`, `tools/lint-ratchet.py`, `tools/growth-ratchet.py`, `tools/refactor-proof.py` and `tools/test-cleanup-checks.py`, with `docs/review-checklist.md`, `docs/workflow.md` and `docs/rust-migration.md`. It waits on root's design ruling on how a reviewer's acceptance stays attributable, and on #916 to merge, which edits three of those files. It needs a bootstrap Full only if the ruling makes Quick treat pending growth differently by event, which would change `ci-quick.yml`.

Entry 4 shares no file with #921, so #921 may take the other slot before #922 if its two conditions are met first.

#924 (root's question on dropping eight lapsed retained-worktree lines from the resume checkpoint, about 24 GB of SSD) is not work to dispatch. The user's answer brings it back; until then root leaves the listed worktrees and the checkpoint's list alone and reruns housekeeping after each merge.

#892 is macro-only and rides remaining passes and closure; recorded `unwrap`/`expect` and release-overflow gaps belong to #734's passes and closure. #893 runs after closure and before the first phase-3 dispatch. Full phase-3 planning remains untriggered: of #734's 319 files, 98 are on the finished-file list, seven are dispatched and five are in this queue, leaving 209 files in 20 worksets with no entry.

## Resume checkpoint

**Paused at the user's request (lenient wind-down, 2026-11-11).** Base code is `d0c5e058f30318816bf72730ab3c5c1973acdb4e` (#898, merge 26); this docs commit is the pause checkpoint. #898's queue Full passed and it merged at 06:40:27 UTC. No rollback was performed and no confirmed base regression is identified. No new implementation, repair, review, build, simulation, CI request or retry is authorized after the bounded handoff below. A later queue failure waits for resume. An independently confirmed base regression still follows the authorized automatic rollback rule.

Agent: /root | Model: gpt-6-astra | Reasoning effort: xhigh

| Remaining PR | Pushed head | Checks, jobs and state at pause | Next action |
| --- | --- | --- | --- |
| #894, complete `map_func.rs` and mechanical callers | `94e4088bc017604930186911e46bbd283c5f0ee3` | Merged as `cdbbf93af4` at 07:08:23 UTC, merge 27, on its one permitted retry. | Nothing further on the PR. Its `_DEBUG` tile-addition defects are #903, ready-queue entry 1. |
| #900 / #899, retire Windows x86 with replacement 32-bit Rust tests | `17d785d91f3fdcc375da71c6735369daefa69a54` | Clean, pushed draft, not queued. Existing preflight `bju990jme` passed 14 of 14 in 315 seconds; Quick passed. Worker stopped, no local job remains. Full status is pending without a requested runtime run. No review, bootstrap, Native/platform request or negative control occurred; `Rust tests (32-bit)` has never executed in CI. | On resume, source review, then root's workflow bootstrap Full, required CMake/platform evidence and the width-dependent negative control. The local i686 result used extracted libraries and does not validate CI's package installation. Do not land Windows x86 removal until replacement CI test execution is proved. |

#900's first draft has 230 changed workflow/CMake/Python lines plus 19 documentation lines, against the revised 150–300 estimate (original removal-only 100–200). Its local i686 run passed 162 tests with the three justified #818 exclusions; this is not CI acceptance. #899 has not landed, Windows x86 remains in base CI and #896's free-runner trial window is not yet mixed with that change. Leave the trial and its author's one-day follow-up alone; record any later #899 cutover separately.

Verified CI state at the pause: `CI_ON_DEMAND`, `CI_MERGE_QUEUE` and `MIGRATION_CCACHE` remain `true`; `CI_LINUX_SIMULATION` is `blacksmith-4vcpu-ubuntu-2404` again (user, 2026-11-11, steering 34: 3 of 11 free-runner simulation tiers hit the #693 family against 3 of 75 on Blacksmith). `CI_LINUX_COMPARISONS`, `CI_LINUX_PLATFORM` and `CI_MACOS_HEAVY` stay unset, so comparisons and Clang remain on free runners.

The old #898, #894 and #900 workers are stopped, and root released its PR watches before the requested steering thread. No tile-map continuation, F evidence, 9m, further #736 family, #891 item, planner or Ready refresh is started during this wind-down. Existing implementation selection on resume still uses `claude-opus-5-5`, medium; at most two implementers and six unmerged component branches.

| Other work | State and resume boundary |
| --- | --- |
| #903 fidelity batch | Undispatched; ready-queue entry 1. |
| Complete `tile_map.rs` (7a continuation) | Undispatched; ready-queue entry 2. Terrain/Clamp/town wrappers are not completed by #894. |
| #736 restored observations and classification | Undispatched; ready-queue entry 3. The F company/clocks/commands family is re-ranked by that classification. |
| 9m movement and further #736 families | Not started. 9m waits for the `tile_map.rs` pass; #736 follows the classification. |
| #891 tools, closure and later reader work | Not started here; existing work-order placement remains. #892 is macro-only; recorded unwrap/expect/overflow gaps stay with #734. #893 remains before the first phase-3 dispatch. |
| Platform cadence | Last scheduled cadence after merge 23 passed. Next due after merge 28, not due at this pause. #897's Windows x86 Rust-only Full/native-cadence policy remains until #899 lands. |
| Stocktake and profile | Last complete stocktake/profile at merge 20; next at merge 32. Keep the merge-20 artifacts and current speed anchors. No new census, profile, deep run or wall pass was launched for wind-down. |

Flakes and limits carried forward:

- #693's earlier checkpoint recorded 2 network-timing failures in 42 simulation-tier runs (the #839 loopback hang and #877 reference `content-protocol` miss). The merge-20 bounded interval later found no observed hang signature among 63 native/queue requests, with 38 reaching simulation shards; that workflow denominator does not prove each exercised network scenarios. These distinct windows are not combined into a current rate.
- After that stocktake, #887's reconnect hang passed its single unchanged retry; its cause remains unknown. #894's first queue Full failed a pinned-reference `content-protocol` timeout before candidate comparison, recorded in [#693](https://github.com/fetch-laboratories/openttd-rust/issues/693#issuecomment-6788785313). The single #894 retry described above is the entire wind-down retry allowance; a further queue failure waits for resume. No source/oracle change or new local network diagnosis is authorized. The prior two-run original/original authorization is exhausted. Keep `.local/retained-network-693` and the later linked artifacts; no candidate or main regression is inferred merely from a timeout.
- A flaky shard can be rerun with `gh run rerun RUN --failed` on a branch containing `db62aad692` (#881); a branch on older tools needs a fresh `native` request. This is a resume rule, not permission for another wind-down retry.
- A workflow-changing PR needs root's reviewed bootstrap Full before queueing. Existing exact-head acceptances retain their evidence limits; later combined trees are validated by queue Full. #900's pending Full status is not a running requested Full or runtime evidence.
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected. #898 is a partial evidence transfer; native ownership, loader/error observations, pool/ABI identity and retained fixture consumers remain as its accepted report states.
- Pass briefs retain `cargo check -p openttd-kernels --features native-test-support --all-targets --locked`; ordinary Rust checks omit that feature. Root attribution stays `Agent: /root | Model: ... | Reasoning effort: ...`. #710's missed-resume lesson still applies on resume; no background result reopens this wind-down.

Retained state for housekeeping:

- The optional wind-down housekeeping sweep was skipped. No deletion or fresh size measurement is claimed. Keep open-PR checkout `/home/fetch/.t3/worktrees/openttd-rust/t3-ad237b08-5d844ae8ffa954ad` (#900) and every dirty or unpushed tree. The merged #894 and #898 checkouts are removable by the next housekeeping run.
- Keep locked `t3-9bfeabfd-b8f486fd2881369f`, with the merge-20 #275 profile and `.local/merge20-facts.txt` plus named source artifacts. Preserve all current speed/deep/evidence anchors.
- The previous checkpoint's retained-state list follows unchanged as historical preservation guidance; it is not a new assertion that every old checkout still exists. Merge-25 cleanup already removed only its recorded eligible trees. `/steer` may reconcile stale references, preserving uncommitted/unpushed work and evidence.

- Keep the two implementer checkouts: `t3-8bdcce4f` (#883 and #884; its `.local/verification` holds #884's passing verify) and `t3-43581c15` (W; `.local/logs/w-manifest.md` is not in Git). Both are clean and pushed.
- Keep `t3-ee66f61c`, locked: profile 155's outputs (merge 8, `c12561671f`) for #275, until the next profile is recorded.
- `tools/housekeeping.py` found nothing removable at the wind-down. It kept three merged, clean checkouts only for a task active within 12 hours or an earlier roadmap reference: the #738 docs agent's, the planner's and #850's. This checkpoint does not name them, so a later run may remove them.
- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `t3-8ed2642d` (profile 96), `t3-62945724` (98) and `t3-a6181dc7` (100) with their comparisons and frozen runtimes. Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) and `t3-d3ff7564` (profile 143, clean `da020b52c7`) with their HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Never push stale root-docs guidance over current main. Keep the pinned original and main's uncommitted user-owned `.codex/config.toml` unchanged.
