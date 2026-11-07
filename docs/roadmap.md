# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-06, overhaul)

Agent: /overhaul (Claude Code, user-directed) | Model: claude-opus-5-5 | Reasoning effort: xhigh

Development is paused for a cleanup stage (user, 2026-11-06). Code main is integration 103 (`b7b6603f4c`); #722 integrated as 101, #732 as 102 and #733 as 103. After 102, with metric v2 at implementation `f4e2c5b45420…` ([root's checkpoint](https://github.com/fetch-laboratories/openttd-rust/commit/e800717540)): compiled source and generated records C++ 256,796; total fork C++ 279,755; original retired 62,750 of 284,355 (22.07%), simulation 41,550 of 61,148 (67.95%); C++ evidence 22,959; exports 1,563; callback slots 3,377. Integration 103 changes no game code or C++ counts; its expanded boundary census is a separate corrected series, 1,719 exports and 3,419 callback fields. Main is green; no rollback or integration hold remains. [Profile 100](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6695636889) remains the accepted profile:

| Scenario | Original Ir | Candidate Ir (100) | Accepted projection (100) |
| --- | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,782,269,348 | 1.167346x |
| Opus | 21,773,343,583 | 27,916,561,893 | 1.345115x |
| Grok | 20,969,727,163 | 26,626,873,969 | 1.346652x |
| P1996 | 3,600,502,896 | 4,593,990,046 | 1.352193x |
| P2000 | 6,049,922,238 | 7,583,890,696 | 1.444531x |
| P2016 | 11,343,424,651 | 13,931,397,338 | 1.382593x |
| Drawing | 2,149,015,961 | 2,154,521,531 | 1.004635x |

**Why the pause.** Net C++ removal fell from about 7,200 raw lines on 11-05, mostly reader ports, to about 1,300 on 11-06, when five of six slots held storage owners. Storage owners remove little while C++ readers of their records remain: in the five open storage drafts, 57% of changed C++ lines were in save/load, GUI/viewport, NewGRF and script consumers. Measured against the original with the same rules, the Rust port also degraded readability, and agents copy the code they find:

| Measure | Original C++ | Rust port |
| --- | ---: | ---: |
| Enum types | 773 | 33 |
| Comment lines per 100 code lines | 35 | 7 |
| Functions with more than 7 parameters | 0.6% | 2.5% |
| Functions over 100 lines | 1.6% | 2.5% |
| Mean cyclomatic complexity | 4.08 | 3.83 |
| Code in copy-paste clones | 4.5% | 2.3% |

Complexity and nesting match the original, because ports keep its control flow. The other gaps are the port's own: literals for named values (`signal.rs:334` writes `0x0c || 0x30` beside the existing `TRACK_BIT_HORZ`/`TRACK_BIT_VERT`), about 3,100 dropped explanatory comment lines, 150 functions with more than 7 parameters, and 27,719 raw lines of fork C++ test evidence that every storage change has to patch. Clippy reports 9,628 distinct violations of the candidate lints; quick CI runs no Rust checks. Inventories are on the stream issues below.

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
| 4 to 6 | Support; GUI, drawing and platform; vendored and final retirement | As in [planning](planning.md#goal-and-phases) | Plans deferred until adjacent |

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. A file belongs to at most one open PR across all streams: check open PRs' file sets before starting a batch. Cleanup PRs integrate on their stream's proof, review and exact-head Full, growing neither fork C++ nor the boundary ([integration](workflow.md#integration)).

1. **#733 checks** start first: Rust checks in quick CI, Clippy and Ruff ratchets, growth ratchets, the refactor proof. #737's batch C (cargo records), #736's R0 registration and #734's foundation F may start beside it.
2. **#734 types, names and comments**, after F and the #733 refactor proof: worksets 01 to 23 with disjoint files, then closure (private wrapper fields).
3. **#736 test evidence** and **#737 boundary debt** run beside #734 where files are disjoint. #737's C precedes V. #736's T follows W and F; A follows T.
4. **#735 argument structs** follow #734 for the same files.
5. **#738 rules** is docs-only; root applies it between integrations.
6. **#726** integrates as a cleanup maintenance PR beside #737; its TCP tail stays with #730.

## After cleanup

Readers first, in this order, re-estimated by fork C++ when selected:

| Rank | Owner | Issue estimate | Notes |
| ---: | --- | ---: | --- |
| 1 | #669 afterload/legacy | 1,800 to 2,600 | Establish a complete canonical-write route through existing owner operations before selection |
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

## Recurring

- After every sixth integration, take stock under `/start-development` step 4 and profile the merged tip in the background ([speed](speed.md#instruction-projections)). The next stocktake follows integration 107.
- Run the platform cadence after every fifth integration; the next follows integration 105. A new failure stops integrations.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.
- #156 (coverage gaps), #275 (speed), #724 and #710 (host) stay open trackers. #500 is delivered by #734; #168 and #199 attach to #737 and the owners that touch those records. #693 is queued for #736's batch P with a loaded-base or original/original reproduction. #503's coverage continuation runs last.

Excluded, with return conditions reconsidered on 2026-11-06; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint

Cleanup stage in progress. Code main is integration 103, `b7b6603f4c`. A's checks are installed. R0 and cargo C combine in #743; road R and history H proceed in disjoint files. F's seven semantic comparisons match, but archive and exact instruction-count proof fail; [decision #746](https://github.com/fetch-laboratories/openttd-rust/issues/746) holds only F's integration and its reserved-file release. No proof rule is waived while it is open. Integration and rollback procedures live in the [root workflow](root-workflow.md).

| Work | Owner | Next action |
| --- | --- | --- |
| #743: #736 R0 and #737 C | Root | Finish native completeness/growth repair, exact exception review, Quick and one combined Full. Component reviews and focused checks pass. |
| #734 F, PR #739 | `/root/cleanup_734_foundation` | Resolve #746 before base/debt adoption or integration. Source and all 122 lint proposals are reviewed; no jobs remain. |
| #737 R, PR #744 | `/root/cleanup_737_road` | Independent source/exception review, then serialized baseline and native-fixture adoption after #743. |
| #737 H, PR #745 | `/root/cleanup_737_history` | Finish typed-path native evidence. Preserve generic fixture behavior; close the old protocol only when F releases `lib.rs`. |
| #738 rule corpus | Root | Two docs checkpoints are pushed. The remaining orchestration-skill rows belong to an authorized navigation/steering pass. |

Retained state from root's wind-down ([checkpoint](https://github.com/fetch-laboratories/openttd-rust/commit/e800717540)), for housekeeping:

- The five closed drafts' checkouts (`t3-085f01f5`, `t3-8cd4fc03`, `.local/worktrees/root-review-714`, `t3-c19acd32`, `t3-eb96953a`) are clean and pushed; ordinary housekeeping may remove them. The old vehicle `6f3` evidence inside `root-review-714` goes with it.
- Keep `t3-8ed2642d` (profile 96), `t3-62945724` (98) and `t3-a6181dc7` (100) with their comparisons and frozen runtimes; release `t3-44649657` (92), `t3-b7a95131` (94) and `t3-96affac3` (steering-30 audit) through ordinary housekeeping.
- Dirty or staged trees need a decision before removal: `t3-3b491ddb` (five files), `t3-431c84b4` (locked #565 work), `t3-d78d5f8b` (locked native evidence), `t3-32cf4edc` (`tools/simulation/roads.py` staged), `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and the locked profile trees `.local/worktrees/profile-after*`. Never push stale root-docs guidance over current main.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors and frozen runtimes, and the simulation runs root listed in its checkpoint.
