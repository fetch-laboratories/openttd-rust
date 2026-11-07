# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-07, integration 129; paused)

Agent: /root | Model: gpt-6.1-sol | Reasoning effort: ultra

Development is paused at the user's request in the cleanup stage. Code main is integration 129, `7b6887f43d` (#803); pre-pause main is `5df157bfb1`. The source/evidence review, corrected Industry case inventory and exact-head Full/tested-tree verification pass. The delivery update below uses retained reports for 127–129, not a new measurement or regular profile. No confirmed source regression or rollback is known. Post-merge Quick/CI - Build on `7b6887f43d` and Quick on `5df157bfb1` are pending at the recorded 16:28 UTC pause state; later results do not resume work. [Profile 125](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6707980382) remains accepted on clean `46864a207c`, with seven nonempty canonical exit comparisons and all six policy records below band. Profiles 113/119 remain retained. The user skipped 107. Platform cadences 120 and [125 passed](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42380211065). Next regular stocktake/profile is 131; platform cadence 130 follows that integration's eventual delivery.

| Measured quantity | Integration 129 |
| --- | ---: |
| Original C++ remaining | 221,645 |
| Fork source glue, including generated records | 34,923 |
| Compiled source and generated records C++ | 256,568 |
| Tracked tools C++ evidence | 16,869 |
| ABI-generated non-record C++ evidence | 5,522 |
| Measured fork C++ series (scope below) | 278,959 |
| Original C++ retired | 62,710 of 284,355 (22.05%) |
| Simulation C++ retired | 41,550 of 61,148 (67.95%) |
| Expanded Rust exports / callback fields | 1,709 / 3,418 |
| Historical regex exports / callback slots | 1,553 / 3,376 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 117 / 314 / 40 |
| Retained Clippy keys / diagnostics | 664 / 757 |
| Ruff keys / diagnostics | 296 / 296 |
| Raw GLOBAL Rust / tooling inventories (scope below) | 175,659 / 63,272 |

Fork-added C++ under `src/tests` is 5,684 lines, already within source glue. The measured ABI-generated C++ is 6,137 lines: 615 record declarations within source glue and 5,522 non-record evidence lines included above. Version 2 does not yet inventory extracted-original comparison fragments or C++ embedded in Python strings. Their volume is unknown; the 278,959-line series covers only the listed native inventory. This scope-completeness follow-up belongs to #736; unmeasured support earns no deletion credit. Historical regex and expanded boundary series have different scopes. The one shared final report was collected at the Full-tested 129 candidate, whose merged tree is identical. Raw GLOBAL Rust/tooling inventories also include oracle text and native support; implementation source and frozen data are separate below. Shared-accessor baselines retain 117 rows, rebound callees 314 and orphan exports 40. Clippy retains five approved families. Integration 128 retires eight actual parameter-count rows with three individually reviewed line-count carries, leaving 664 keys / 757 diagnostics. Ruff remains at 296 keys / 296 diagnostics. Unrelated rows, provenance and thresholds remain intact.

| Delivered checkpoint | Rust executable code +/− | Python code +/− | Tracked C++ code +/− | Frozen oracle bytes |
| --- | ---: | ---: | ---: | ---: |
| 127: Company finance #802 (`0c8a0f8a05`) | 268 / 0 | 4 / 53 | 0 / 127 | 976,283 |
| 128: eight Station helper contexts #801 (`5d43172624`) | 310 / 200 | 0 / 0 | 0 / 0 | 0 |
| 129: Town/Industry policy #803 (`7b6887f43d`) | 895 / 0 | 16 / 44 | 3 / 343 | 1,095,202 |

These three deliveries retire **467 counted fork C++ code lines**, within the two evidence families' earlier 437 to 517 estimate, while compiled production C++, original/simulation retirement and boundary counters hold. Their combined Rust executable cost is 1,473 added / 200 removed (net +1,273), Python net −77, and frozen oracle data 2,071,485 bytes. Finance preserves all 40,320 original cases; policy preserves all 21,128 cases and 55,056,680 ordered Industry resolver pairs. Actual native calling-boundary, tunnel, history and lifetime evidence stays. The exact integrating diff corrects the prior policy Python table from 18/46 to 16/44 without changing net −28 or source/raw reports. Its raw Rust additions of 22,029 split into 895 executable lines and 21,134 classified oracle-text lines. No new census was run for this pause.

The preceding 119–125 vocabulary window had zero net C++ removal; 126 also held C++ while restoring Train/Cargo vocabulary. The cleanup pivot to complete evidence families and real parameter-count retirement now supplies the reductions above. The earlier reference content-protocol failure and inconclusive static diagnosis remain retained after 126's one unchanged-code Full retry; no source repair or causal claim is made. #805's reviewed housekeeping safety fix is paused before delivery. Its ranked successors are unstarted static estimates, not commissioned work. No phase, anchor or lint threshold is reset. Locked #565 WIP remains retained.

| Source area / phase relationship | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,338 | 9,086 | 6,556 |
| Simulation storage (phase 2) | 13,724 | 3,212 | 10,512 | 1,388 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,067 | 38,727 | 3,360 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 551 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 5,684 |
| Boundary layer | 0 | 0 | 0 | 16,905 |

These are source-area subtotals, not new ownership or phase-exit claims; pool save chunks remain with their storage owner. Phase 1's exit does not assert zero original source in its area. Null-driver games, bounded NewGRF/historical-save corpora and retained native fixtures leave the recorded limits on #156; entry witnesses are not branch coverage. The inherited missing CargoPacket precondition assertions remain a fidelity difference, rather than merely a coverage gap. #503 deep checks remain user-requested. The source-operation reviews and Full gates do not close those broader limits.

The instruction counts below measure profile 125 on clean `46864a207c`, the documentation successor of code integration 125. Candidate compatibility identities match retained 119 and the Ubuntu 24.04 anchors within each role; reference identities/counts match retained evidence. The selected Callgrind/LibVEX is 3.26.0 (dispatcher 3.22.0), with GCC 13.3.0 and Rust 1.99.0. Historical manifests retain opaque compatibility digests rather than complete per-component diagnostics; the report records that provenance limit.

| Scenario | Original Ir | Candidate Ir (125) | Projection (125) |
| --- | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,783,244,380 | 1.168161x |
| Opus | 21,773,343,583 | 27,864,737,336 | 1.345115x |
| Grok | 20,969,727,163 | 26,586,132,618 | 1.346652x |
| P1996 | 3,600,502,896 | 4,590,881,782 | 1.352193x |
| P2000 | 6,049,922,238 | 7,557,248,562 | 1.444531x |
| P2016 | 11,343,424,651 | 13,926,293,082 | 1.382593x |
| Drawing | 2,149,015,961 | 2,154,353,683 | 1.004635x |

All seven nonempty canonical exit-save comparisons pass, including drawing; all full logs match, with every stdout/stderr pair empty. The latter supplies no nonempty branch evidence. An independent cross-check passes 180 assertions. Unchanged-anchor projections use the same-host available quiet-pass mean and speed formula. TGP rises 0.132989% since 119, below the 3% attribution trigger; all six policy records remain below band and ceiling. Five play counts remain below anchor Ir, so their projections stay at the anchor floors. Drawing has no policy ceiling. No wall pass or attribution repair is required, and no anchor/history was reset. Simulation took 875.0 seconds and verification 878.889 seconds; neither is a quiet wall median. Frozen manifests/build-list records remain after completed loose logs were reclaimed. Historical diagnostic gaps, matched tool advisories, canonical masks and cleaned drawing bitmap bytes remain disclosed in the report. Next regular stocktake/profile is 131.

**Why the pause.** Net C++ removal fell from about 7,200 raw lines on 11-05, mostly reader ports, to about 1,300 on 11-06, when five of six slots held storage owners. Storage owners remove little while C++ readers of their records remain: in the five open storage drafts, 57% of changed C++ lines were in save/load, GUI/viewport, NewGRF and script consumers. Measured against the original with the same rules, the Rust port also degraded readability, and agents copy the code they find:

| Measure | Original C++ | Rust port |
| --- | ---: | ---: |
| Comment lines per 100 code lines | 35 | 7 |
| Functions with more than 7 parameters | 0.6% | 2.5% |
| Functions over 100 lines | 1.6% | 2.5% |
| Mean cyclomatic complexity | 4.08 | 3.83 |
| Code in copy-paste clones | 4.5% | 2.3% |

Complexity and nesting match the original, because ports keep its control flow. The overhaul's initial gaps were literals for named values (`signal.rs:334` writes `0x0c || 0x30` beside the existing `TRACK_BIT_HORZ`/`TRACK_BIT_VERT`), about 3,100 dropped explanatory comment lines, 150 functions with more than 7 parameters, and 27,719 raw lines of fork C++ test evidence that storage changes had to patch. Those historical inventories remain on the stream issues. Quick CI now runs all four Cargo checks and the lint, policy, growth and claimed-proof gates. The ratchet retains the five approved complexity/safety-documentation families; its 757 current diagnostics are not comparable to the initial broader lint inventory.

## Course (user, 2026-11-06)

- **Execution order:** the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([planning](planning.md#goal-and-phases)).
- **Yardstick:** net fork C++ removed, counting C++ under `tools/` and `src/tests`. Rust exports plus callback slots fall or hold; growth needs a reason the reviewer accepts ([cost decisions](planning.md#cost-decisions)).
- **Limits:** at most four implementation agents ([team](team.md#roles-and-models)); stocktake and profile every sixth integration.
- **Enforcement:** checks enforce mechanical rules on every push (#733); reviewers work from the [review checklist](review-checklist.md); implementers read the [implementer brief](implementer-brief.md) and their work order.
- **Sizes, never durations:** plans, issues and briefs size work in counts (files, lines, call sites), never in time (user, 2026-11-06).

## Phases

Phase numbers name content; the execution order above sets the sequence. Phase 1 (simulation logic) exited at integration 63.

| Order | Stage | Exit outcome | Status |
| ---: | --- | --- | --- |
| 1 | Cleanup | Streams #733 to #737 done, #736 except boundary checks tied to a later phase, #738 applied; lint and growth baselines shrinking | Current |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | Readers run in Rust with evidence for their observable outputs; VM and binding shims are named for phase 6 | Next; planned in the [work order](#after-cleanup) |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in [accepted placement](#accepted-reader-and-pool-placement-2026-11-03-after-integration-67) |
| 4 to 6 | Support; GUI, drawing and platform; vendored and final retirement | As in [planning](planning.md#goal-and-phases) | #3; plans deferred until adjacent |

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. A file belongs to at most one open PR across all streams: check open PRs' file sets before starting a batch. Files whose edits merge mechanically are exempt, including `lib.rs` module lists, `migration/*-debt.json`, `migration/harness-inventory.json` and generated registries; resolve those edits when updating from base. Cleanup PRs integrate on their stream's proof, review and exact-head Full, growing neither fork C++ nor the boundary ([integration](workflow.md#integration)).

1. **#733 checks** start first: Rust checks in quick CI, Clippy and Ruff ratchets, growth ratchets, the refactor proof. #737's batch C (cargo records), #736's R0 registration and #734's foundation F may start beside it.
2. **#734 types, names and comments**, after F and the #733 refactor proof: worksets 01 to 23 with disjoint files, then closure (private wrapper fields).
3. **#736 test evidence** and **#737 boundary debt** run beside #734 where files are disjoint. #737's C precedes V. #736's T follows W and F; A follows T.
4. **#735 argument structs** follow #734 for the same files.
5. **#738 rules** is docs-only; root applies it between integrations.
6. **#726** integrates as a cleanup maintenance PR beside #737; its TCP tail stays with #730.

Corrections adopted on 2026-11-07 under [#746](https://github.com/fetch-laboratories/openttd-rust/issues/746), without changing the course:

- #734's proof uses the [cleanup checklist](review-checklist.md#cleanup-prs): unchanged operations and order plus exact-head Full when compiled code differs. Archive equivalence remains the first attempt; per-PR Callgrind profiles and disassembly accounting are not required. F's existing evidence is accepted.
- #747 is integrated through #751/#752: the Clippy ratchet retains complexity, parameter count, nesting and safety documentation, preserving retained debt. Remove unnecessary existing cast suppressions on later base updates, after F's integration.
- #734 permits the original's integer uses through wrapper accessors and measures wrapper adoption rather than Rust enum counts. The implementer brief places helpers by their original file, including `.cpp`-only helpers (#738).
- #734/#737 may merge mechanical shared-file edits on base updates; H's protocol closure does not wait for F's unrelated `lib.rs` declarations.
- Before selecting #669 afterload, re-check its all-pool writes against the boundary yardstick and current storage ownership, as noted below.

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

Then storage, each pool with its save/load chunks, redone from current base with each closed draft's branch as reference: #656/#598/#599 vehicle (#714), #634 station/road-stop/cargo (#700), #638/#498 small pools (#723), #635 town/industry/subsidy (#704), #637/#497 remaining engine pools, #717 attachment, #725 YAPF, orders, companies, then the map (#565's remainder). #716 tracks the all-pools closure.

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

Steering 31 (2026-11-06), still in force: an owner's checkpoint integrates once it meets the [integration](workflow.md#integration) rule and the owner continues in a fresh PR; a field moves in the same change that deletes its Rust-called services, before pool adoption, and no service routes through `Record()`; the whole-fork metric runs twice per PR; status comments change at milestones; long jobs start as `exec python3 tools/signal-witness.py -- <command>`, and a host-killed job (exit 143, no logged failure) reruns once incrementally (#724).

## Recent completed work

Individual evidence and review reports stay in the linked PRs. Metrics below
are actual compiled-C++ reductions where stated, not sums of rewritten lines.

| Integration | Work | Integration commit | Result |
| --- | --- | --- | --- |
| 69–88 | Settings, corpus, typing, shared records, GRF text #655, logging #679, vehicle records #636, script APIs #691/#697, CI repairs, industry specs #692 and game protocol #628 | `e76c94b306` through `5168023c13` | Compiled C++ −4,289 on the consistent series (navigation at `bc6c727c48`); #636 alone −1,952. Rows are in the thirtieth steering commit `7c4a448327`. |
| 89 | Native fatal reporting #709, repair #711 | `6426473252` | Production-path fatal probe and unrepaired-binary negative control; original/glue C++ unchanged. |
| 90 | Typing, TSan startup and network diagnostics #705 | `8fc7620ce5` | All-tools typing and precise one-use network allowances; platform workflow passed; C++ unchanged. |
| 91 | Accounting, housekeeping, identity and fact tools #712 | `561ec04c42` | Versioned census, ordinary cleanup and compatible profile guard; no ownership-retirement credit. |
| 92 | Map leaves #698 and harness consolidation #713, combined #715 | `f5f16377ce` | Historical C++ -178; total fork C++ -247; callback slots -57; original retirement unchanged. |
| 93 | Shared Rust pool core #716/#718 | `d5cb6a424e` | Coupled support; Rust +682, generated C++ +20; no production pool or retirement claim. |
| 94 | Content metadata/dependency/download state #706/#667 | `36a3681ee0` | Exact-head Full/tree match; combined 93–94 source C++ -431, total fork -214, original retirement +676. Seven profile comparisons equal, all projections below bands. |
| 95 | Shared native pool facade and first vehicle allocation #720 | `5391a8ee49` | Both source reviews and exact-head Full/tree match; eleven local games/64 snapshots and seven affected native tests pass. Measured +181 C++ coupled support, no full-vehicle or generic deletion credit. Platform cadence passed. |
| 96 | Map-reader maintenance #275/#721 | `01dcc7419f` | Both source reviews and exact-head Full/tree match. PR Grok Ir -3.966178% versus retained 92; Rust +77 added/net +27, C++/tooling zero. Combined 95–96 source C++ +181, fork +184; profile 96 passes seven comparisons and all six projections below bands. |
| 97 | Host signal witness #724/#727 | `4cc6ee4b20` | Independent source review and exact-head Full/tree match. Four signal/process tests; future long jobs use the integrated wrapper. C++ and Rust unchanged; no live sender identified yet. |
| 98 | Graph/job indexed pool checkpoint #719 | `9867694d02` | Both steering fixes/source review/focused/native and exact Full/tree pass. Source C++-153, fork-151; actual station publication remains717. |
| 99 | Shared nearby StationList #728 | `8faa77f296` | Source review/exact Full/tree pass. Source C+++6, evidence+1; necessary shared support, no completion/retirement credit. |
| 100 | Const nearby StationList views #729 | `dbbb502073` | Two qualifiers, same descriptor/lifetime. Source review/exact Full/tree pass; counters unchanged. Profile100 accepted; cadence initializer repair integrates in101. |
| 101 | Engine records/GRF/direct consumers #722 | `31478f303a` | Scoped reviews and exact Full42302553842/tree pass. Compiled C++−852, fork−973, original retired+380/net glue−472; callback slots−229. Actual pools/native consumers remain637/497. Main platform recovery42304912050 runs. |
| 102 | Shared owned-bytes assignment and GRF price metadata #732 | `a0337694c4` | Review and exact-head Full passed; compiled C++ +2, fork +4. Necessary shared support, no retirement credit. |
| 103 | Cleanup CI, lint/growth ratchets and refactor proof #733/#742 | `b7b6603f4c` | Three reviews and exact-head Full/tree match. Tooling +1,956 code lines; production Rust and C++ unchanged. |
| 104 | Evidence registration and direct cargo records #740/#741, combined #743 | `49f7ac951e` | Reviews and exact-head Full/tree pass. Compiled C++ -9, fork C++ -6, exports -3 and callbacks -1. The initial original-client reconnect stall is recorded on #693; no source regression was established. |
| 105 | Canonical native type foundation #734/#739 | `7c3fb4455a` | Source and exact lint reviews, accepted #746 proof disposition and exact-head Full/tree pass. C++ and boundary counts hold. Platform cadence passes. |
| 106 | Three live typed history queries and protocol closure #737/#745 | `1e109c47a9` | Source/closure reviews and exact-head Full/tree pass. Compiled C++ -47, fork C++ -71 and exports -3; generic native evidence restored, so original retirement decreases by 40. |
| 107 | Narrow lint debt and cargo/engine evidence #747/#751/#749/#750, combined #752 | `c540d0788f` | Source/evidence reviews and exact-head Full/tree pass after bounded CI repairs. Compiled C++ -77, outside evidence -35, fork C++ -112; boundary holds. Original retirement is unchanged. |
| 108 | Direct road records and MovementGap evidence #744/#748, combined #755 | `b2e1f4ba` | Reviews and exact-head Full/tree pass. Compiled C++ -95, total fork -140 and exports -4; 19,608 movement observations preserved. |
| 109 | Czech town-name vocabulary #754 | `3cc1d1593f` | Source/exact contract reviews and unchanged-head Full retry pass. Rust +404 added code lines; C++ and boundary hold. Initial inherited reconnect timeout remains disclosed. |
| 110 | Industry, station and cargo-monitor vocabulary #753/#756/#757, combined #758 | `726c8c18f8` | Source/union reviews and exact-head Full/tree pass. Rust +2,077 added code lines; C++ and boundary hold. Platform cadence passes. |
| 111 | House flags and property vocabulary #759 | `82e1166868` | Source/exact contract reviews and exact-head Full/tree pass. Rust +178 added code lines; C++ and boundary hold. |
| 112 | Vehicle-list and manager-face vocabulary #760/#762, combined #763 | `dd457ef961` | Source/union reviews and exact-head Full/tree pass. Rust +131 added code lines; C++ and boundary hold. |
| 113 | Cargo packet and Town policy vocabulary #761/#764, combined #765 | `1b5be29b7d` | Source/union reviews and exact-head Full/tree pass. Rust +167 added code lines; C++ and boundary hold. Stocktake/profile accepted; seven comparisons equal and six projections below bands. |
| 114 | Original order helpers and Livery vocabulary #766/#767, combined #769 | `2a341845d6` | Source reviews, clean union and exact-head Full/tree pass. Rust +138 added code lines; C++, tooling and all guarded counters hold. |
| 115 | Cargo flow and Vehicle headers #770/#771, combined #773 | `bf1bd30b12` | Source/union reviews, eight exact contracts and Full/tree pass. Rust +301 added/net +110; C++, tooling and guarded counters hold. Platform 115 passed. |
| 116 | Town naming/import and Signal vocabulary #768/#772, combined #776 | `980c6f0da8` | Source/union reviews, seven contracts and Full/tree pass. Rust +251 added/net +76; C++ and boundaries hold. Earlier cancelled Town Full and inconclusive diagnostics remain retained. |
| 117 | Vehicle renewal/servicing and Cargo payment #774/#775, combined #777 | `43b913f4a5` | Source/union reviews, exact Cargo identity carry and Full/tree pass. Rust +172 added/net +48; tooling/C++ and guarded counters hold. |
| 118 | Company and Town generation #778/#779, combined #780 | `2d05ea2ca6` | Five owner blobs preserved, source reviews and Full/tree pass. Rust +187 added/net +60; tooling/C++ and boundaries hold. |
| 119 | Vehicle breakdown and Cargo registry setup/labels #781/#782, combined #785 | `03392da0d4` | Source/union reviews, three exact contracts and Full/tree pass. Rust +133 added/net +24; C++ and boundaries hold. Stocktake/profile accepted: seven comparisons equal, six records below band. |
| 120 | Town creation and engine livery #783/#784, combined #786 | `1f8c1139d0` | Four owner blobs and exact ledger union preserved; two contracts and Full/tree pass. Rust +151 added/net +11; C++, tooling and guarded counters hold. Platform cadence passed. |
| 121 | Cargo sorting and Vehicle aging/value #787/#788, combined #789 | `34ce361ff2` | Both reviewed blobs and complete ledgers preserved; clean union and exact-head Full/tree pass. Rust +68 added/net +16; tooling/C++ and all guarded counters hold. |
| 122 | Town placement and coastal vocabulary #790 | `ac1616b56c` | Source review, unchanged depth-6 identity carry and exact-head Full/tree pass. Rust +50 added/net +14; tooling/C++ and all guarded counters hold. Earlier Quick predates exact acceptance; Full's fresh Quick passes. |
| 123 | Company rename and Cargo finalisation/translation #791/#792, combined #794 | `b8fe5094d9` | Both reviewed blobs and complete ledgers preserved; clean union and exact-head Full/tree pass. Rust +57 added/net +9; tooling/C++ and all guarded counters hold. |
| 124 | Vehicle tile-hash, cursors and occupancy vocabulary #793 | `dc02d1340c` | Reviewed owner blob and complete ledgers preserved; conflict-free base adoption and exact-head Full/tree pass. Rust +97 added/net +22; tooling/C++ and all guarded counters hold. |
| 125 | Town found/rename and engine-replacement rules #795/#796, combined #799 | `78d4234849` | Six reviewed blobs, exact ledger union and four renewed contracts preserved; scoped review and exact-head Full/tree pass. Rust +113 added/net +47; tooling/C++ and all guarded counters hold. Source stocktake and profile 125 accepted; platform 125 passed. |
| 126 | Train limits and Cargo resolver/specification vocabulary #797/#798, combined #800 | `7d35010d71` | Nine reviewed blobs and exact ledger union preserved; three renewed contracts and exact-head Full/tree pass. Rust +214 added/−122 removed/net +92; tooling/C++ and guarded counters hold. The earlier reference preparation failure remains retained after one unchanged-code validation retry. |
| 127 | Complete Company financial-command evidence #802 | `0c8a0f8a05` | Independent source/evidence review, all 40,320 original cases and exact-head Full/tree pass. Tracked C++ −127; Rust implementation +268, Python net −49, oracle data 976,283 bytes. Raw collector data classification is reported separately; production C++ and boundary hold. |

| 128 | Eight Station helper contexts #801 | `5d43172624` | Complete source/order review, three renewed exact contracts and exact-head Full/tree pass. Eight parameter-count rows retire; Clippy is 664 keys / 757 diagnostics. Rust +310/−200 code lines, net +110; C++, tooling and all guarded counters hold. |

| 129 | Complete Town/Industry policy evidence #803 | `7b6887f43d` | Independent evidence review and root summary fix verified; all six production replacements, default comparison inventory and exact-head Full/tree pass. Tracked C++ −340; Rust tests +895, Python net −28, oracle data 1,095,202 bytes. Retained tunnel/native ABI/lifetime evidence and boundary counters hold. |

## Recurring

- After every sixth integration, take stock under `/start-development` step 4 and profile the merged tip in the background ([speed](speed.md#instruction-projections)). The user skipped stocktake/profile 107 after the notes above were collected. Integration 125 stocktake/profile are accepted; profiles 113/119 remain retained for comparison. The next regular stocktake/profile follows integration 131.
- Run the platform cadence after every fifth integration; 115, 120 and 125 passed. The next follows 130. A new failure stops integrations.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.
- #156 (coverage gaps), #275 (speed), #724 and #710 (host) stay open trackers. #500 is delivered by #734; #168 and #199 attach to #737 and the owners that touch those records. #693 retains the cancelled reconnect failure and two completed original/original runs that did not reproduce it; the cause remains unresolved and that two-run authorization is exhausted. Further work stays with #736's batch P, without a selected repair or extra diagnostic run. #503's coverage continuation runs last.

Excluded, with issue placement and return conditions rechecked on 2026-11-07; near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint

Paused at the user's request. Normal work stopped at the recorded 2026-11-07 16:28 UTC state. Code main is integration 129, `7b6887f43d`; pre-pause main is `5df157bfb1`. This docs-only pause moves main without advancing code. There is no merge or rollback to perform during wind-down, no running child, and no newly commissioned implementation, branch or brief. Later CI or background completion does not restart normal work; explicit user resume is required.

| Work / state at pause | Next action on explicit resume |
| --- | --- |
| #804 / #805, `root/804-detached-housekeeping`, head `7fac379c436cb7d879c409c38d420776afd92daf` over `5df157bfb1` | Keep `.local/worktrees/root-804-detached-housekeeping`. Both Python blobs equal reviewed `df984cab30`; all 23 housekeeping tests and prior Python/gate receipts pass. Review carries. The pause commit moves base under strict protection: adopt the new main tip preserving those blobs, then obtain exact-head Full and the final report before protected merge. Platform cadence 130 follows delivery. |
| #805 Full `42391323158` and Quick | In progress at the recorded pause state; do not wait or infer merge readiness from later success. Keep the request at `.local/ci-requests/130-full.json`. Read results only after resume and update base before final integration validation. |
| Metrics job `24514`, `.local/130-final-measure.sh` | Running for pre-pause head `7fac379c43`; do not wait. Keep all outputs. This is not #805's final integrating report; the actual resume head needs that report. Completion does not restart wind-down. |
| Main CI | Post-merge Quick validation and CI - Build on `7b6887f43d`, and Quick on `5df157bfb1`, are pending at the recorded pause state. No red run is known. CI on this new pause commit is unobserved; no wait or new request is authorized. |
| Integrations 127–129 | Delivered with independent source/evidence review, exact-head Full and matching tested/merged trees. #736 F/W and #735 P1 remain open for their other families. No rollback occurred. |
| Next cadence | Platform 130 after its delivery; next regular stocktake/profile 131. Profile/platform 125 remain accepted, with profiles 113/119 retained. |

Unstarted ranked queue from the private after-128 static plan, retained at `.local/cleanup-after128-plan.json` and in `t3-20fec9ce`. No implementation, branch or brief was started for any item:

| Rank | Queued family | Static estimate / condition |
| ---: | --- | --- |
| 1 | #736 F: complete cargo-income probe transfer | 80 to 90 counted C++ lines retired; preserve all 185,285 cases and actual native payment/lifetime checks. Estimated oracle data 1 to 6 MB; actual cost requires its first concrete checkpoint. |
| 2 | #737 V: eight Orders field callbacks | 58 to 70 counted C++ lines retired and eight callback fields removed, using existing canonical records. Native shell conversion and protected fixtures stay. |
| 3 | #736 F: Engine runtime evidence | 40 to 75 net counted C++ lines retired after preserving native ABI, alias, width and lifetime observations. Least certain estimate; fold into a larger same-file checkpoint if the native remainder erases the benefit. |

The child slots were idle because the preceding owners and reviews had finished, while root serialized integration/report bookkeeping and the routing correction instead of refilling independent work. There was no quiet-host hold or resource/dependency block. That scheduling gap is recorded; the user's wind-down now expressly prohibits refilling. Mid-task messages use the child's verified `parentThreadId`, not a root found by title (`docs/team.md`).

Remaining placement and limits:

- Metric scope completeness stays with #736: account for unlisted extracted/embedded C++ and separate executable source, oracle text and native support. Preserve raw reports/provenance and distinguish coverage corrections from deletions.
- Keep #693 / #736 P's failed and passing network traces and bounded original/original nonreproduction. The initial #800 content failure differs from prior reconnect/multiplayer stalls. No stacks/connector state proved a cause; the two-run authorization remains exhausted. No repair, retry or new diagnostic is selected.
- The remaining #734–#738 cleanup stream checklists, wrapper closure, evidence/boundary remainder and authorized rule-corpus work remain open. No later phase is selected. Future root must verify source boundaries and live reservations before commissioning the static queue.

Retained state for housekeeping:

- Keep #805's one checkout `.local/worktrees/root-804-detached-housekeeping`, its pushed branch/head above, and the planning checkout `t3-20fec9ce` with its private plan. Preserve every checkout with uncommitted or unpushed work.
- Only the two user-named finished W checkouts, `t3-366d4384` and `t3-e935c6f1`, were removed in this wind-down, after proving their exact heads are merged or remotely retained and checking live-output guards. No broad housekeeping apply or ad hoc-log deletion followed. Until #805 integrates, manually verify Git retention before any removal; keep origin `preserve/housekeeping-foundation-6ab9c20e5d`.
- Keep job `24514` outputs: `.local/130-whole-fork.json`, `.local/130-source.json`, `.local/130-growth.json`, `.local/130-final-measure.log`, `.local/130-whole-fork.stderr.log`, `.local/130-source.stderr.log`, `.local/130-growth.stderr.log`, and the script/request/metadata. Keep the source-head label; none is promoted to a final integration receipt during wind-down.
- Keep `t3-8ed2642d` (profile 96), `t3-62945724` (98) and `t3-a6181dc7` (100) with their comparisons and frozen runtimes. Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep `.local/retained-network-693` with the failed Full artifact, inconclusive diagnostics, passing traces and bounded original/original evidence. Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Leave dirty/staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged), and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Never push stale root-docs guidance over current main. Keep the pinned original and main's uncommitted user-owned `.codex/config.toml` unchanged.
