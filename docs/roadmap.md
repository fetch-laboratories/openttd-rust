# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-11, stocktake after merge 32)

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

**State.** Root restarted on 2026-11-11 after steering 34. Five PRs have merged through the queue since #894 (merge 27): #905 (28, `830b4d77cf`), #908 (29, `cdda5418f9`), #900 (30, `0f727600fb`), #910 (31, `ac0ed46a35`) and #915 (32, `985abab91f`). The tables below measure `985abab91f`. The merge-28 platform cadence went red on macOS Debug: a fork test fixture put water in the `Map::MaxX` column, and #894 had made the Rust call assert there as the original does. #908 fixed the fixture forward and merges resumed ([#903](https://github.com/fetch-laboratories/openttd-rust/issues/903#issuecomment-6806233274)); the pre-queue Linux tests had missed it because that build compiled assertions out (#911, fixed in #912). The next cadence follows merge 33, and the next stocktake and profile merge 44. No confirmed base regression is open.

**Since the stocktake (merges 33 to 44, 2026-11-11/12; the tables below still measure merge 32).** Twelve PRs merged through the queue: #912 (33, `c303578d53`; residue gate, assertions in the pre-queue Linux tests), #909 (34, `62abbf2524`; `tile_map.rs` pass), #918 (35, `0f5ac588af`; network pass 20c), #916 (36, `9f558a0704`; #891 evidence tooling), #927 (37, `8f00af1983`; #920 preflight stamps, hooks, base-parent growth gate and #923 compile check), #928 (38, `b9d61ef53f`; network pass 20d), #930 (39, `6283cf04ce`; #921 tracked cleanup records and #925), #929 (40, `24c0fc242b`; #736 link graph schedule probe, 168 counted C++ lines deleted against 21 recipe lines), #926 (41, `c5f1fde840`; 9m `movement.rs` and `signal_func.rs` passes), #934 (42, `85eb1dba0d`; network pass 20e, which finishes workset 20), #932 (43, `b5c7bc03a6`; #922 `ci.py enqueue` and cancelled stale requests) and #935 (44, `93eedd81b2`; #933 hook, compile-check and Clang pass). No rollback. The platform cadences after merges 33 and 38 passed (the latter run 43139564180). `migration/finished-files.txt` lists 116 files at merge 44, against 99 at merge 32. #931 (Catch2 transfers) closed unmerged under root's movable rule, and #904, #911, #919 to #925 and #933 closed as delivered or decided; #737 closed at its end state. The cadence after merge 43 and the stocktake and instruction profile after merge 44 are due and were not run at the pause ([resume checkpoint](#resume-checkpoint)); no census was taken, so every other figure in this section is merge 32's.

**Reading.** Cleanup continues. Fork C++ fell 245 lines over merges 27 to 32: C++ under `tools/` fell 300, of which #915's industry lifecycle probe is 268 net, and `src/tests` rose 59, the growth #910 was allowed in order to restore the observations on #156. #734 stands at 99 of 319 files and 35.1% of lines, with `tile_map.rs` (#909) and six network files (#918) in review. #735's count has not moved since merge 20. The passes since then covered map and network leaves, and 70 of its 121 functions are FFI exports, which keep positional lists while their C++ callers remain; its movable inventory is the 51 helpers and commands, and all but two commands sit in files no pass has finished yet, so the count falls as the remaining worksets pass and no course change follows. Residue in finished files rose with the seven files finished since merge 20 (missing comment lines 135 to 161); #912's gate stops further growth and closure owns the rest. The #736 classification left about 1,430 movable lines of 20,053; after two delivered probes and root's movable rule of 2026-11-12 about 600 remain, and the rest retires with pool and reader owners ([work order](#cleanup-work-order) item 4). Median last-ready-to-merge over merges 27 to 32 was about 2 hours 11 minutes, against 53 to 73 minutes before, from the merge stop, a GitHub Actions incident (20:22 to 21:11 UTC) and macOS runner capacity; no queue run failed. Full phase-3 planning remains untriggered, with 209 of #734's 319 files in no entry, and near-full reproduction is not met.

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

The [merge-32 profile](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6810087211) on clean `985abab91f` used `simulate --instructions --jobs 2`. All seven canonical exit saves and full logs match. The driver exited 1 on its anchor identity guard for every scenario: #908 added a `native-debug` feature to `rust/openttd-kernels/Cargo.toml`, a file the identity hashes, and nothing else in the identity differs from merge 20. The merge-32 projections below are therefore computed from the report's instruction counts and the retained anchors by the [formula](speed.md#instruction-projections), not printed by the driver; the next quiet pass re-anchors ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275)). The profile's output is retained in locked `t3-656b5a85-5eedda484b0f05a1`, the previous tip for attribution at the next profile.

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
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | 11,899 compiled lines in storage, about 4,300 save/load chunk lines, the logic residue assigned to pool owners, and record field routes in the 16,759-line boundary layer | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in the [reader and pool placement](design/reader-pool-placement.md) design |
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
   - **Six movable items in the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s order** (delivered by `/root/plan_736_classification`, 2026-11-11; 13 files, 1,871 counted lines gross): the industry lifecycle probe (delivered, #915), the link graph schedule probe (#929), the train edge and aircraft range probes, the authentication probe and the ScriptList owner and cargo sections, each oracle with an `oracle-capture.py` recipe. An item is movable only if its counted C++ deleted clearly exceeds its counted recipe C++ added, and its first checkpoint reports both (root, 2026-11-12, after #931 measured the Catch2 files at net +29). The Catch2 files that delete whole, `src/tests/train.cpp` and the ScriptList persistence section fail that rule and stay native: the Catch2 files retire with the C++ they exercise, `train.cpp`'s PBS and reservation cases with #725 and its speed case with #656. A check of a live native binding, pool or production caller stays native until that boundary retires ([evidence](porting.md#evidence)).
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

Undispatched work orders ([delegation](team.md#ready-queue-and-pipelining)). At most two implementers and six unmerged component branches; five are out at merge 40 (#926, #932, #934, #935 and the authentication probe's `cleanup/736-auth-probe`), so one more dispatch fills the limit until one merges. Entries 1 and 2 are the file-pass slot's and entries 3 and 4 are the other slot's ([work order](#cleanup-work-order)); a free slot takes its own top ready entry, and the other slot's when it has none. Each entry names the one merge it waits on, or none; entries 1 and 3 wait on none. File sets were checked on 2026-11-12 at merge 40 (`24c0fc242b`) against the edits of the open PRs with `python3 tools/retrospective.py readiness 926 932 934 935`, and against the authentication probe's file list as its work order gave it, since it has no PR yet; each entry names the overlaps found. #932 edits only `tools/ci.py`, its test, one workflow and docs, so it overlaps no entry. Sizing, fan-in and held callers are on [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6812366384), with the file table in the [merge-35 analysis](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6811109267); the #736 estimates are on [#736](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812356028). Every entry keeps its carries and proposals in `migration/cleanup-records/<branch>.json` ([cleanup records](rust-migration.md#quick-cleanup-checks)), not in the PR body, and the pre-push hook wants a preflight stamp before a ready PR's push (#927).

Both file-pass entries carry three rules that cost the 9m pass (#926) review rounds:
- **Boundary design faults are fixed in the pass.** A pass fixes every design fault under [boundary](porting.md#boundary) in a file it finishes: a carrier written for one call site, or a Rust-to-C++-to-Rust bounce into code Rust owns. It ports the decision once into the Rust module named after its original file and calls that typed function; adding a function to an unfinished module does not make the pass own that file. Such a fault cannot be listed as residue or deferred, and the PR still grows neither fork C++ nor the boundary, net ([ruling](https://github.com/fetch-laboratories/openttd-rust/pull/926#issuecomment-6810796828)). Look for these before the first checkpoint and count them in its size.
- **`cfg(test)` never changes a production body.** A test observes the production path through a recording mock of its service, never through a trap compiled into the body.
- **Run the fixtures that call the pass's functions before review.** These are the comparison probes and the ABI probe fixtures under `tools/migration/windows-abi/`; `python3 tools/preflight.py` compiles them when headers, probes or adapters change. Two fixtures in #926 relied on a service read that the pass replaced with a map read, and only their run showed it. Repair a fixture's input, never its expected result. Each entry names its fixtures on #734.

1. **8a, #734/#735 vehicle definition file passes: `airport.rs`, `airport_definitions.rs`, `base_consist.rs`, `disaster_vehicle.rs`, `effectvehicle_func.rs`, `road/availability.rs`, `roadveh.rs` and `train_state.rs`.** The eight workset 08 files that call nothing in workset 09, so the order rule (a workset before its callers) does not hold them behind the 9m pass. The other ten workset 08 files all call `movement.rs` and are their callers, so these eight pass first within the workset.
   - Size: 953 raw lines (537 of them `airport_definitions.rs`), 10 exports (two of them generated by a macro in `airport_definitions.rs`) and 34 service-table pointer types. About 665 changed lines at 20d's realized 0.70 and about 895 at 20c's 0.94. The files are definitions with light residue, so the leaf ratios apply, not 9m's 1.45 (about 1,380), which is the bound if field types ripple into callers. One reviewer; refine the size at the first checkpoint.
   - Residue the inventory tool prints: 10 matched bodies, 14 missing comment lines (`get_vehicle_pos_on_build` 4, `has_road_type` 4, `openttd_rust_add_date_road_types` 6) and 7 bare-integer sites (`tile: u32` in `airport.rs`, `company: u8` four times in `road/availability.rs`, `compatible_railtypes: u64` and `track: u8` in `train_state.rs`). `airport_definitions.rs` calls the exports `openttd_rust_override_allocate` once and `openttd_rust_override_add` twice: port each decision once as a typed function in `override_manager.rs` and call that, under the first rule above; `override_manager.rs` is workset 15's and stays unfinished. `road/availability.rs` has one assertion-macro site for #892's class.
   - Ready now, with the reason from the call graph: none of the eight references `movement.rs`, `signal_func.rs` or any other workset 09 file. Their callees are `station_map.rs` (finished) and files of worksets 07, 15 and 16, which pass after 08 in the adopted order whenever 08 runs. No open PR and no file of the authentication probe edits any of the eight.
   - The pass converts the callers in finished files itself, as mechanical edits listed in the PR body: `station_cmd.rs`, `town/commands.rs`, `industry/tile.rs`, `road_stop.rs` and `vehicle_transactions/construct.rs`, and `movement.rs` once #926 has merged. Unfinished callers keep a raw adapter only where a signature changes: `aircraft.rs`, `disaster.rs`, `effect.rs`, `ground.rs`, `road.rs`, `train.rs`, `train_reservation.rs`, `vehicle_effect.rs`, `orders.rs`, `vehicle/visits.rs`, `vehicle/maintenance.rs`, `vehicle_func.rs`, `vehicle_base.rs`, `cargo_storage.rs`, `newgrf_airport.rs`, `newgrf_vehicle.rs`, `newgrf_loading.rs`, `landscape_cmd.rs`, `water.rs` and `script_road.rs`.
   - #926 edits ten of those callers (`aircraft.rs`, `disaster.rs`, `effect.rs`, `industry/tile.rs`, `movement.rs`, `orders.rs`, `road_stop.rs`, `station_cmd.rs`, `train.rs` and `train_reservation.rs`). Keep this pass's edits in them to the lines a changed signature forces, and resolve the rest by base update when #926 merges. #934 shares only `lib.rs` and `migration/finished-files.txt`, which merge mechanically.
   - Add the eight files to `migration/finished-files.txt`.
   - Before review, run the fixtures that call these files' exports, which keep their positional lists: `src/tests/engine.cpp` (road type availability), and `src/tests/train.cpp`, `src/tests/follower_fixture.hpp` and `tools/migration/train-edge-comparison.cpp` (`openttd_rust_train_state_new` and `_destroy`). No fixture calls the two airport property exports directly; `src/rust/airport_definitions.cpp` reaches them when a NewGRF loads, so run a NewGRF loading comparison that defines an airport.
2. **9a, #734/#735 pathfinding file passes: `pathfinder_func.rs`, `follow_track.rs`, `node_hash.rs`, `yapf_node.rs` and `water_regions.rs`.** The callees of `pbs.rs` and the three YAPF files, so they pass first in what is left of workset 09. Include #892's assertion class and #734's recorded fatal-path fidelity gaps in the every-line pass: one macro site in `node_hash.rs`, and three `unwrap`/`expect` sites each in `follow_track.rs` and `water_regions.rs`.
   - Size: 1,259 raw lines, 11 `extern "C" fn` items and 78 primitive-integer declarations. About 1,825 changed lines at 9m's realized 1.45 changed lines per raw line, the ratio of a shared callee with outside callers; about 1,180 at 20c's 0.94. With `pbs.rs` (771 raw lines) the pass would be about 2,945 at 1.45, so `pbs.rs` leaves at the file boundary. Refine the size at the first checkpoint. Above about 1,000 changed lines it takes one reviewer per function group.
   - Waits on the 9m pass (#926) to merge: these files call `movement.rs` in 12 places and adopt its typed forms. At its head `20357960f4`, #926 edits none of the five files. It does edit eight files that call them (`station_cmd.rs`, `town.rs`, `train.rs`, `train_reservation.rs`, `ship_control.rs`, `rail_cmd.rs`, `road_yapf.rs` and `test_tables.rs`), which is one more reason to start from its merge. #934 shares only `lib.rs` and `migration/finished-files.txt`, which merge mechanically; #932, #935 and the authentication probe edit none of the five files or their callers. Entry 1 touches none of them either: both entries keep a raw adapter in `train.rs` and `train_reservation.rs`, on different lines.
   - The pass converts the callers in finished files itself, as mechanical edits listed in the PR body: `station_service.rs` (one `follow_track::platform_length_full` call) and the test module of `town.rs` (`water_regions::Context` and `Regions`, and two export calls that move to the typed functions). Unfinished callers keep a raw adapter only where a signature changes: `pbs.rs`, `rail_yapf.rs`, `road_yapf.rs`, `ship_yapf.rs`, `train.rs`, `train_reservation.rs`, `ship_control.rs`, `water.rs`, `landscape_generation.rs` and `native_test_support.rs`. Leave `test_tables.rs` alone beyond a changed signature.
   - `follow_track.rs` drops its ten raw adapter calls (`reverse_diag_dir_raw` nine times and `station_rail_raw` once) for the typed forms the finished `direction_func.rs` and `station_map.rs` define. `water_regions.rs` has seven bare `tile: u32` sites, and the bodies behind its ten exports move to typed Rust-internal functions.
   - `src/tests/water_regions.cpp` and `water_regions_protocol.hpp` stay native ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812253795)) and call this pass's exports, which keep their positional lists. Run them, the follower and YAPF Catch2 cases and `tools/simulation/pbs-console.cpp` before review.
3. **#736 ScriptList owner and cargo sections** (work order item 4, the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s item 6 less the persistence section): move the `OwnerGaps` and `CargoGaps` sections of `tools/migration/script-list-comparison.cpp` into Rust oracle tests. The file and its driver stay for the valuation, persistence, filter, load, metamethod and nested sections, which open a VM.
   - It passes root's movable rule on current counts. The probe is unchanged since the [estimate](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812356028) (last edit `7843e5b780`): `OwnerGaps` is 59 non-blank, non-comment lines, and the cargo block between the prelude and the shared record helpers is 145, of which `ListToken` (8) stays for the VM sections; the estimate counted 119 of it as the section. So 178 to about 200 counted C++ lines deleted against an estimated 5 to 25 counted recipe lines: both sections drive the public `ScriptList` API or the extracted collector on the pinned original as they stand, and the comparison is `probe.compare` in Python. These are line counts, not `port-metrics.py`'s; report its two counts at the first checkpoint. If under 150 counted lines move, stop and tell root.
   - Size: about 1,100 to 1,500 changed lines outside oracle text, from the delivered probes' 6.3 to 8.5 changed lines per counted line (#929 realized 8.5 at acceptance: 1,420 for 168); the classification estimated 1,560. One new recipe, `script_list_owner`, which also captures the cargo records.
   - Files: `tools/migration/script-list-comparison.cpp` (the two sections, `CargoProbeList`, `GapCargoCollector`, `CargoToken`, the private-counter access and the owner-only helpers `keys`, `Fill` and `Drain` leave; `Hex`, `Content`, `Record`, `Require`, `TestList` and `ListToken` stay) and `main`; `tools/script-list-comparison.py`, whose `cargo-reducer.hpp` extraction (about 70 lines) leaves with the cargo section; `tools/oracle-capture.py` and one patch under `tools/oracle-recipes/`; separate Rust test modules with their oracle text, with a one-line hook in the finished `station_cargo.rs`, so that the residue gate stays clean, and in `script_list.rs` (workset 19, unfinished); a record under `migration/evidence-replacements/`. The probe's row in `migration/harness-inventory.json` stays.
   - Held callers: none leaves. The cargo collector's six exports (`openttd_rust_cargo_init`, `_finish`, `_packet`, `_origin`, `_share` and `_plan`) are called by production `src/script/api/script_stationlist.cpp`, and `openttd_rust_list_token` by the VM sections that stay, so no export becomes Rust-internal and `tools/uncalled-export-baseline.txt` has no row for this item.
   - Both sections observe the candidate through the fork's C++ `ScriptList` and `ScriptStationList_Cargo` adapter classes; a Rust test calls the owner beneath them. The VM sections that stay still construct those classes. If the first checkpoint finds an adapter observation that no retained section carries, that case stays native and names its boundary (the phase-3 script API reader); record it, and claim only what the test observes.
   - Follow the oracle rule on #736 and the delivered probes' pattern (#915, #929): freeze the original's observations before deleting the producers. `docs/components/economy.md` and `network-script.md` name this comparison's cargo mode as a check; update those lines to the Rust tests.
   - Ready now. No open PR edits the probe, `script_list.rs` or `station_cargo.rs`. #935 edits `tools/script-list-comparison.py` in one hunk at the binding-generation loop, away from the cargo extraction; it merges by base update. #935 also changes preflight's compile check for this comparison: at this base the compile-only run stops where the driver reads generated bindings that only a full run writes, so run `python3 tools/script-list-comparison.py` in full instead; once #935 merges, compile-only mode skips the bindings, compiles what is left of the fixture and compiles it with Clang too, so the shrunken file must be clean under `-Wmissing-field-initializers`. The authentication probe and entry 4 add recipes to `tools/oracle-capture.py`; those additions merge by base update.
4. **#736 train edge and aircraft range probes** (work order item 4, the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s item 4 less `src/tests/train.cpp`): retire `tools/migration/train-edge-comparison.cpp` with `tools/train-edge-comparison.py`, and `tools/migration/aircraft-range.cpp` with `finite_range_probe` in `tools/simulation/aircraft.py`, into Rust oracle tests.
   - Both pass root's movable rule ([estimates](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812356028)): 296 counted C++ lines deleted (154 and 142) against an estimated 25 to 45 counted recipe lines, because each probe's reference shell and case loop run on the pinned original as they stand and only the printing is new. `src/tests/train.cpp` fails the rule and stays native: do not move it.
   - Size: 352 raw C++ lines and about 100 lines of Python deleted; two new recipes, `train_edge` (four extracted bodies, 20,000 curve and 20,000 reversal trials) and `aircraft_range` (the event bodies `aircraft.py` extracts, 400 cases). About 1,900 to 2,500 changed lines outside oracle text, from the two delivered probes' 6.3 to 8.5 changed lines per counted line. Report counted C++ deleted, counted recipe C++ added and changed lines at the first checkpoint. If it projects past about 2,000, split at the file boundary, the train edge probe first.
   - Files: the two probes and their drivers; `tools/oracle-capture.py` and two patches under `tools/oracle-recipes/`; separate Rust test modules with their oracle text, each with a one-line hook in `train.rs` and `aircraft.rs`; rows in `test_tables.rs`; a record under `migration/evidence-replacements/`; `tools/uncalled-export-baseline.txt`. The probes' `native_sources` rows in `migration/harness-inventory.json` stay, as the two delivered probes kept theirs. `aircraft-boundary.hpp`, `controller-orders.hpp` and `fixture_map.hpp` stay: the ABI probe includes them.
   - Follow the oracle rule on #736 and the delivered probes' pattern (#915): freeze the original's observations before deleting the producers, and claim in the record only what the tests observe. The train edge probe draws 16 random vehicles for each trial, so print only what the swap changed and the event order, not every vehicle's state, and report the oracle's bytes at the first checkpoint.
   - With the probe gone, `Train::GetCurveSpeedLimit` has no caller. Delete the member, its declaration and its baseline row, and make `openttd_rust_train_curve_limit` Rust-internal if nothing else calls it, as the link graph probe did for `LinkGraphSchedule::Dequeue`.
   - Waits on the 9m pass (#926) to merge: at its head `20357960f4` it edits `train-edge-comparison.cpp`, `train.rs`, `aircraft.rs` and `test_tables.rs`. Capture from the probe as #926 leaves it. The authentication probe adds a recipe to `tools/oracle-capture.py` and narrows rows of `tools/uncalled-export-baseline.txt`, and entry 3 adds a recipe; those additions merge by base update. #935 edits `tools/probe.py` and `tools/simulation/console_adapter.py`, which the drivers import and this entry does not edit; #932 and #934 edit none of its files. Entries 1 and 2 each keep a raw adapter in `train.rs`, and entry 1 leaves `openttd_rust_train_state_new` and `_destroy`, which the probe calls, with their positional lists; the hook line merges beside them.

**Later.** In the file-pass slot no pass of workset 09 can start before #926 merges: 9a and 9s (`signal.rs`, 1,271 raw lines) call `movement.rs` and `signal_func.rs`, which 9m is retyping, and #926 itself edits `signal.rs` and `road_yapf.rs`; 9p (`pbs.rs`, 771 raw lines, about 1,120 changed lines at 1.45), the pass for `ship_yapf.rs` and `road_yapf.rs` (1,947 raw lines, split or not by 9a's realized ratio) and the single-file pass for `rail_yapf.rs` (1,903 raw lines) wait on 9a, and the last on 9p too ([order](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6811109267)). In workset 08 only entry 1's eight files are free of 09: `aircraft.rs`, `disaster.rs`, `effect.rs`, `ground.rs`, `ground_vehicle.rs`, `road.rs`, `ship_control.rs`, `train.rs`, `train_reservation.rs` and `vehicle_effect.rs` (16,020 raw lines) each call `movement.rs`, #926 edits seven of them, and `train.rs`, `train_reservation.rs` and `ship_control.rs` also call 9a's, 9p's and 9s's files. They follow workset 09, the four over 1,500 lines (`train.rs`, `road.rs`, `aircraft.rs` and `train_reservation.rs`) as single-file passes. In the other slot no tools issue is undispatched, so after entries 3 and 4 #736 has only its end-state inventory left. `src/tests/train.cpp` (88 counted lines), the ScriptList persistence section (25) and the six Catch2 files of the earlier ruling stay native. No console fixture moves by transfer: the 65 files and 14,273 counted lines of class 2 retire with their pool or reader owners, and the 33 files and 3,909 lines of class 3 with their boundaries.

**Placement.** #896 stays with its author for the one-day weighted-minutes and network-failure follow-up; record any #899 cutover separately rather than attributing a mixed window to #897. No infrastructure issue is undispatched: #921 with #925 merged as #930, #922 (`ci.py enqueue` refuses a head whose latest Quick or requested run is red; [root's ruling](https://github.com/fetch-laboratories/openttd-rust/issues/922#issuecomment-6810044527)) is #932, and #933 (the pre-commit hook checked nothing, because git's hook environment leaks into the pinned-checker checkout; root, 2026-11-12) is #935, which also carries the Clang `-Wmissing-field-initializers` pass for preflight's compile check, since #885, #926 and #929 each lost a `native` run to that warning. Fold later small tools items into the next tools PR.

#892 is macro-only and rides remaining passes and closure; recorded `unwrap`/`expect` and release-overflow gaps belong to #734's passes and closure. #893 runs after closure and before the first phase-3 dispatch; its all-member deep check needs #925. Full phase-3 planning remains untriggered: of #734's 319 files, 108 are finished, five are dispatched (9m's two and 20e's three) and 13 are in this queue, leaving 193 files in 19 worksets with no entry, and #736's end-state inventory has none either.

## Resume checkpoint

**Paused at the user's request (lenient wind-down, 2026-11-12).** Base code is `93eedd81b2` (#935, merge 44); this docs commit is the pause checkpoint. The merge queue drained and is empty: #926, #934, #932 and #935 merged as merges 41 to 44. #932 and #935 were each removed once and requeued once, unchanged, as the wind-down allowed: #932 for a `network-protocol` hang in run 43147704245 ([#693](https://github.com/fetch-laboratories/openttd-rust/issues/693#issuecomment-6813852347)), #935 for a reference-side `content-protocol` miss in run 43150720812. No rollback was performed and no confirmed base regression is identified. The push runs on `93eedd81b2` (Quick 43153736985, baseline validation 43153737112, `CI - Build` 43153737108) were still running at the pause; their results are unknown here. No new implementation, repair, review, build, simulation, CI request or retry is authorized until resume. An independently confirmed base regression still follows the automatic rollback rule.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

| Remaining PR | Pushed head | Checks, jobs and state at pause | Next action |
| --- | --- | --- | --- |
| #936, #736 authentication probe (draft) | `86a683d198` | Returned to draft. `/root/review-936-all` raised two findings (adapter observations; the uncalled `GetAuthenticationMethod` chain); their fixes are `041f3a1280`, `b9d1369f49` and `86a683d198`, pushed as a logged checkpoint with no preflight stamp. Preflight passed 15 of 16 at `b9d1369f49`; the failing growth step passed alone once the proposal was added. `native` 43149400253 passed, on the old head `5363b6f9c3` only. No job runs. Checkout `t3-bc6721f0`, clean and pushed. | Merge base (merges 41 to 44), full preflight, `python3 tools/ci.py ready 936`, `native` at the head, then one re-check of findings 1 and 2 naming the three commits. That report must accept the 22 lines of test growth proposed in `migration/cleanup-records/cleanup-736-auth-probe.json`. |
| #937, 8a vehicle definition file passes (draft) | `bd44b53777` | Code for all eight files, the override-manager boundary fix and the caller conversions are pushed: 1,386 changed lines, one lint carry in its record file, growth holding. Cargo checks, 184 workspace tests and the Clippy, lint-policy, growth and residue gates passed locally. Nothing else ran. No job runs. Checkout `t3-c294893f`, clean and pushed, with `openttd_test` and `openttd_rust_abi` built at the head. | Merge base and convert `movement.rs`'s calls into these files, now that #926 has merged; run the entry's fixtures; full preflight; `native`; review in the two groups its PR body lists; queue. |

All implementers, planners and routine agents of this session are stopped, and root holds no PR watch. Implementation resumes on `claude-opus-5-5`, medium, and planning on `claude-fable-5-2`, medium; at most two implementers work at once and six component branches stay unmerged. A waiting implementer gets no second work order ([team](team.md#ready-queue-and-pipelining)).

Verified CI state at the pause: `CI_ON_DEMAND`, `CI_MERGE_QUEUE` and `MIGRATION_CCACHE` are `true`; `CI_LINUX_SIMULATION` is `blacksmith-4vcpu-ubuntu-2404`; `CI_LINUX_COMPARISONS`, `CI_LINUX_PLATFORM` and `CI_MACOS_HEAVY` stay unset. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

| Other work | State and resume boundary |
| --- | --- |
| Ready queue | The section above was written at merge 40 and was not refreshed. Its entry 1 (8a) is dispatched as #937. Entry 2 (9a pathfinding passes) and entry 4 (train edge and aircraft range probes) waited only on #926, which has merged, so they are ready once a planner re-checks their file sets at merge 44. Entry 3 (the #736 ScriptList owner and cargo sections) was ready and is undispatched. None was started. Refresh the section before dispatching. |
| Next tools PR | Three small `tools/ci.py` defects found by #932's own first use: `enqueue` prints "queued queued"; its requeue check ignores a merge-group run that ended `cancelled` by a job timeout, so it applied no one-retry check and logged nothing; and `queue-wait` reported #932 as removed (status 3) when it read the PR between leaving the queue and `MERGED`. No issue is filed; file one when the PR is dispatched. |
| Platform cadence | Passed after merges 33 and 38. Due now, after merge 43; not started. |
| Stocktake and instruction profile | Last at merge 32. Due now, after merge 44; not started. The profile driver exits 1 on the anchor identity guard until a quiet pass re-anchors (#275), so the merge-32 medians were computed by hand. Notes kept for the stocktake: #926 took five reviewer sessions after its source review and five `native` failures; fixture churn was first found by CI; reviewers worked in an implementer's shared checkout; `t3_background_wait` returned at once (#710); Windows x64 took 17 minutes in Full run 43120192348; the #923 compile step took 163 to 289 seconds on a loaded host and needs an idle figure. |
| Phase-3 planning | Not triggered. |

Flakes and limits carried forward:

- #693: this session saw three network-timing failures with no code cause, each cleared by one unchanged retry: a `network-protocol-loopback` hang on #916, the `network-protocol` hang that removed #932 (run 43147704245) and the reference-side `content-protocol` miss that removed #935 (run 43150720812). Earlier windows recorded 2 such failures in 42 simulation-tier runs, then none among 63 native and queue requests at the merge-20 stocktake; the windows are not combined into a rate. No source or oracle change and no new local network diagnosis is authorized by this checkpoint. Keep `.local/retained-network-693` and the linked artifacts; no candidate or base regression is inferred from a timeout.
- A flaky shard can be rerun with `gh run rerun RUN --failed`; a removed PR is requeued with `python3 tools/ci.py enqueue PR --retry REASON`, once, and a second failure of the same job goes to root.
- A workflow-changing PR needs root's reviewed bootstrap Full before queueing. The routine agent then reruns any "Invalidate previous validation" run skipped for a PR push inside the variable window; a push in that window otherwise has no `Full validation` status (#928, once).
- A push to a ready PR, `ci.py ready` and `ci.py request` need a preflight stamp; a checkpoint push to a draft takes `PREFLIGHT_CHECKPOINT`. Carries and proposals live in `migration/cleanup-records/<branch>.json` (#930).
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected. #898 is a partial evidence transfer; native ownership, loader and error observations, pool and ABI identity and retained fixture consumers remain as its accepted report states.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran at this wind-down and removed 21.8 GB: 23 checkouts merged into `origin/rust-migration`, with nothing dirty or unpushed among them. The SSD has 53 GB free.
- Keep #936's checkout `t3-bc6721f0` and #937's `t3-c294893f`: each is its open PR's one checkout.
- Keep the locked merge-32 profile checkout `t3-656b5a85` until the merge-44 profile is recorded on #275. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`.
- The profile checkouts of merges 8 and 20 and of integrations 96, 98, 100 and 143 (`t3-ee66f61c`, `t3-9bfeabfd`, `t3-8ed2642d`, `t3-62945724`, `t3-a6181dc7`, `t3-d3ff7564`) were released on 2026-11-12 (#924) and survive only by the tool's 12-hour activity rule; a later run may remove them.
- The list below is carried forward unchanged as preservation guidance from earlier checkpoints; it does not assert that every old checkout still exists. `/steer` may reconcile stale references, preserving uncommitted or unpushed work and evidence.

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Never push stale root-docs guidance over current main. Keep the pinned original and main's uncommitted user-owned `.codex/config.toml` unchanged.
