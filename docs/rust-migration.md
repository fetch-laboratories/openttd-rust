# OpenTTD-Rust migration

OpenTTD-Rust is an independent experiment in incremental C++ to Rust migration.
The original game's observable behavior is the specification, including quirks
retained for historical fidelity. Record possible improvements in fork GitHub
issues, deferred until near-full Rust reproduction. Game-simulation code takes
priority, with Rust owning component state; `docs/roadmap.md` sets current work
and `AGENTS.md` states the selection rules.

## Repository and reference

The fork is https://github.com/fetch-laboratories/openttd-rust. `origin` points to the fork;
`upstream` points to https://github.com/OpenTTD/OpenTTD.git. The migration starts
from OpenTTD 15.3, commit `14ec60f248547d4d062a1160f0fc26d742319888`, pinned in
`migration/baseline.json`.

`rust-migration` is the integration branch. Create isolated task branches/worktrees
from it and target it with fork PRs; the pinned original revision remains the
behavioral reference throughout migration.

The verification driver creates `.local/reference/openttd` in the main checkout
as a detached Git worktree at that revision, shared by every worktree of the
clone. It rejects a changed reference revision or dirty reference source. The
shared reference builds into the main checkout's `.local/build-reference`; each
worktree builds its candidate into its own `build-rust`. Neither changes
reference source. Updating the baseline is a separate deliberate task.
Never change candidate behavior and expected results together to make checks pass.

Migrated code lives in the shared `rust/openttd-kernels` crate, linked into the
game, the unit tests and the native generators (strgen, settingsgen). Each port
keeps its original C++ interface; see "Ported components" below. None of these
ports completes its containing subsystem. Preserve the complete game, including
networking, saves, NewGRF mods, graphics, and shared random-number behavior.

## Build and verification

The CI-policy regression suite additionally uses Node.js to execute the actual
workflow scripts; CI requests use an authenticated `gh` CLI. The maintenance
tools work independently of the Codex host or T3 Code.

The current verification setup targets native Linux and needs a C++20 compiler,
CMake, Ninja, Python 3.11 or newer, SDL2 development files, and the normal OpenTTD
libraries described in `COMPILING.md`. OpenGFX supplies free graphics for regression
games; commercial game assets are unnecessary. The verification driver requires
the pinned Rust toolchain and always configures the candidate with `OPTION_RUST=ON`.
Ordinary CMake builds default to `OPTION_RUST=OFF`, preserving the original portable
C++ path. Rust linkage supports native GNU/Linux (64-bit x86 or ARM) and native macOS arm64.
The scoped native Windows MSVC mode is described below.
Other platforms, cross compilation, and macOS universal/Intel configurations reject
an enabled Rust option; their portable fallback remains migration work (#3).

On Ubuntu with administrator access:

```sh
sudo apt-get install build-essential cmake ninja-build python3 pkg-config \
  libsdl2-dev liblzma-dev libpng-dev libcurl4-openssl-dev libfreetype-dev \
  libfontconfig-dev libharfbuzz-dev libicu-dev liblzo2-dev openttd-opengfx
```

For this host, Ubuntu 26.04 on x86-64, `tools/bootstrap-local.py` offers setup without
administrator access:

```sh
python3 tools/bootstrap-local.py
python3 tools/migration.py verify
```

The bootstrap downloads Ubuntu packages into `.local/downloads`, extracts them into
`.local/deps`, and installs Rust into `.local/cargo` and `.local/rustup`. The bootstrap
changes no shell profiles or system packages. It relies on installed compiler/runtime
libraries and host-specific package names; it is not a portable installer.
`rust-toolchain.toml` pins Cargo compiler, formatting, and lint tools. CI installs
that same toolchain before verification. The crate has no external dependencies.

The driver's `verify` action requires all four Cargo checks below, builds both
graphical executables, runs both CTest suites (upstream unit and scripted game
tests), and requires the candidate to retain all reference test names. Empty test
inventories fail verification. The candidate Rust option is checked in the CMake
cache and recorded in the report; a C++ fallback cannot pass as a migrated candidate.
The `tools` action builds the native Rust generators alone into
`.local/build-tools-rust` (`OPTION_TOOLS_ONLY=ON`, `OPTION_RUST=ON`), using the
bootstrapped toolchain when present. `tools/run-comparisons.py` then runs every
reference comparison tool in parallel, including the company financial-command,
industry NewGRF-production, cargo income/destination and cargo queue-scaling probes;
CI runs all three steps.

```sh
python3 tools/migration.py rust-checks
python3 tools/migration.py build
python3 tools/migration.py verify
python3 tools/migration.py tools
python3 tools/run-comparisons.py
```

Each driver invocation retains command logs, test reports and the source revision under `.local/verification/<timestamp>/`.
Passing establishes only covered behavior. The driver does not itself compare
every game state or prove full game equivalence.

`rust-checks` runs the four Cargo checks without creating or building the
reference. `--jobs` (default: the free cores, `max(2, cores - round(1-minute
load))`, read once at start) also limits Cargo invoked by CMake. Every logged
command reports its phase, elapsed time and output path, with periodic updates;
lock waits identify live holders. POSIX commands run under a pipe supervisor;
driver death (including SIGKILL) closes the pipe and kills the command session. Reference and candidate configure/build/test
operations use separate locks, and simulation captures immutable runtime copies
under those locks. Standalone fixture games share the benchmark coordination lock.
Validation is established by the exact-head CI run and the commands in the PR.
Local logs and semantic reports help diagnose failures.

Windows Rust linkage is supported for the scoped native MSVC configurations below.

The fork executable is `build-rust/openttd-rust`; Cargo artifacts reside in the
ignored `build-rust/cargo` directory. CMake tracks the Rust sources, manifests,
lockfile, and toolchain file to rebuild the archive and affected executables.
The reference executable is `.local/build-reference/openttd` in the main
checkout. Most in-game branding remains original. When using this host's
extracted dependencies, launch with their library path:

```sh
LD_LIBRARY_PATH="$PWD/.local/deps/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  ./build-rust/openttd-rust -X
```

`-X` avoids global game folders. Use separate development configuration and saves.

### Generated ABI probes and fixture defaults (#284)

`layouts!` declarations in `abi.rs`, `company.rs`, `crypto_primitives.rs`, `blake2b.rs`,
`x25519.rs` and `auth.rs` name the C++ type, probe group and ordered Rust fields. Use
`rust_field => cpp_field` for aliases and `width field: RustType` for an extra
native-width check. Native-only groups remain in their owning unit tests.
CMake runs `tools/generate-abi.py` into the build directory; probes compare the
complete list and its `SIZE_MAX` terminator through the named metadata API.
Add declarations there, never a second C++ field list or numeric type ID.

Callback declarations inherit signature-correct abort defaults. Fixtures specify
only callbacks they actually use; data pointers and scalar carriers retain their
own initialization rules. Generated typed visitors check actual production
instances for null callbacks and abort defaults in the default native tests.
`tools/test-generated-abi.py` checks the historical conversion against the deleted
lists and tests current generation separately, allowing later ABI changes.
Cross-language signature assertions remain outside this maintenance change.

## Compiler cache, shared reference and CI time

Every worktree of a clone shares the main checkout's pinned reference checkout
(`.local/reference/openttd`), reference build (`.local/build-reference`), bootstrapped
toolchain/dependencies, and per-role ccache stores (`.local/compiler-cache/`).
A lock serializes reference configure/build/test across concurrent runs, so the
original is compiled once per clone rather than once per worktree. New worktrees
need no symlinks. The driver skips an explicit CMake configure when the existing
cache already holds every requested `-D` value; Ninja still reconfigures on any
CMake input change. Configure-time environment changes (for example a new
dependency prefix) need a fresh build directory or one manual configure.

The driver uses ccache whenever it is installed (`tools/bootstrap-local.py`
installs it locally). `migration/ccache.conf` keeps compiler-content checks and
empty sloppiness, and enables direct mode. The driver sets `base_dir` to each
role's root, so paths are relative and worktrees share entries. PCH is disabled
under ccache; `--no-ccache` gives an ordinary PCH build, and `--ccache-bypass`
keeps the no-PCH flags without reuse for measurement. The first local trial
(`07c7134451`) found warm-cache validation 58% shorter than ordinary PCH builds,
with no code or data differences in the objects.

CI cache compatibility prefixes use OS, architecture, baseline revision, compiler and ccache
versions, and the `migration/ccache.conf` hash. The cache action adds the commit SHA to the exact
key and restores by compatibility prefix. ccache hashes compiler content, arguments and included
files itself; ordinary workflow/driver edits do not change the prefix (#79).
Successful `rust-migration` pushes save the cache, and PRs restore it. Windows and
macOS keep vcpkg binaries in an actions/cache files store keyed by runner image;
vcpkg's ABI hashes make stale entries miss. Their Rust jobs also compile through
ccache 4.14.1 (checksummed download) with PCH off; MSVC uses embedded `/Z7` debug
info (CMP0141), which ccache requires. Platform CI also runs on `rust-migration`
pushes; queued pushes coalesce, so it checks the newest merged tip.

## Simulation comparison

`python3 tools/migration.py simulate` builds both games, then runs
`tools/simulate.py`; CI runs its default set after the comparisons. Shared decoding,
comparison, execution and reporting live in `tools/simulation/core.py`. Add each
port's scenario dictionaries and prepare/check hooks to its component module in
`tools/simulation/`; `scenario_modules()` assembles the families. Optional
install/game-argument and AI-selection hooks keep scenario setup in that module.
It is the
evidence for game-logic ports. Each scenario runs the reference and the
candidate headlessly in isolated directories under `.local/simulation/`, twice:
with `-d desync=3`, which writes an uncompressed snapshot every 32 economy days,
and plainly, because desync mode also rebuilds caches every tick and takes
YAPF's uncached rail path. Both write an exit save. Every chunk is decoded
(tables field by field from the stored header, other chunks byte by byte) and
the first differences are reported as `chunk/element/field: ref -> cand`.
Any `[desync:` warning (a cache mismatch), snapshot-list/end-moment mismatch,
or complete log/stdout difference fails. Both modes compare their exit saves.

- Scenarios: both regression saves with their AIs; generated maps (TGP and
  original, sizes, seeds, disasters on); and `play-*`, road networks built by
  LLM players (`migration/saves/README.md`), each loaded with manual
  distribution and with cargodist (short link graph intervals). The saves'
  `console` lines go to `scripts/game_start.scr`. `--soak` adds more; `--self`
  compares the reference with itself; names filter. Extend `scenario_list()`
  for ports. The owner-built #86 save exercises signalled trains and a ship
  (`tools/simulation/rails.py`), requiring movement on every run and carried
  cargo/delivery revenue at desync checkpoints. The later owner save
  (`play-padhattan-ridge-2000-*`, #179) retains the 1996 input
  and adds three trains, three ships, three helicopters and a plane, with
  movement/cargo/delivery checks and one cargodist crossing bar/release witness.
  Saved subsidies and PBS signals do not establish all multiplier/route branches.
  The supplemental reference-built
  aircraft fixture (`tools/simulation/aircraft.py`) observes a small plane and
  helicopter between two airports, requiring movement, loaded cargo, flying
  state and paid delivery. `tools/aircraft-scenario-ai/README.md` records its
  setup details, reproduction commands and limits.
- Masks (`MASKS`, with reasons and hit counts in the report): the random save
  id, build revision/NewGRF version, and `round_trip_time`, which the original
  saves uninitialized (#83); ports touching it need their own check.
- Port divergences go in `KNOWN_FAILURES` by first divergence and issue.
- Cited branch witnesses (#198): one Rust facility (`witness.rs`) holds named
  counters for every port; hits never cross into C++, and `OPENTTD_WITNESS`
  makes `src/rust/witness.cpp` write `branch-witnesses.json` at exit. A module's
  `game_environment` sets it when `scenario["witnesses"]` is set (distinct
  candidate, not `--benchmark`), and its `check` calls `core.branch_witnesses`
  with a counter prefix; a missing file or zero required branch fails. Rail and
  water scenarios do so.
- Games launch on Linux as an unprivileged user with child-only
  `RLIMIT_NPROC=0`. The original `StartNewThread` failure paths compute link
  graphs and write saves synchronously; graph scheduling, join dates and loaded
  LGRJ jobs remain unchanged. This prevents late workers from consuming null
  driver iterations while the simulation is paused. A thread-creation probe
  rejects hosts where privileges bypass the limit. Other platforms are not
  supported by this offline comparison policy. There are no timing retries or
  truncated logs. Active GameScripts remain outside the corpus.
- Both executable/runtime trees are copied before scenarios, dereferencing data
  symlinks; the reference copy holds its shared build lock. Keep the candidate
  build idle during its initial copy; later builds cannot change the test run.
  `--candidate` uses data beside that executable. Reports retain semantic
  comparisons; `candidate_commit`/`candidate_status` describe the initial
  checkout (#97).
  Evidence: `.local/simulation/<time>-<pid>/report.json`.

Simulation speed (#155) is reported for every scenario as `plain_speed`:
paired process wall seconds and candidate/reference ratios, plus their median.
Ratios are null when exit saves are missing, runs fail, end moments differ, or
semantic differences remain; timing never changes the pass/fail result.
Use profiling counters off, the same build settings and pinned manual-distribution
play save:

```sh
python3 tools/migration.py simulate play-opus-55-167-002-manual --benchmark 3 --jobs 2
```

The driver builds with two jobs, then runs one scenario worker. Benchmark mode
runs only plain pairs, shares the clone-wide coordination lock with other harness
games, and compares each exit save and full logs. Both roles use #154's serial
offline thread-failure fallback, recorded as `execution` in the report. These
are total offline times, not normal threaded frame latency. It retains the final
pair and runtimes; `plain_commands` supplies arguments for optional `perf record`
replay through `tools/simulation/game_launcher.py`. Preserve the run's HOME/XDG
directories and runtime libraries; keep profiling separate from timing samples.
Timings include startup, loading, serial link-graph work and save I/O. Benchmark
games wait directly for child exit with a timeout watchdog; ordinary runs retain
subprocess timeout polling, which can add about 50 ms.
Shared coordination does not reserve a quiet host. Wall medians are taken only
in speed passes, with no builds, links or other timed games running. Compile load
moves P2016 medians 2-7% and concurrent timed games about 10% either way. A speed
pass takes four medians per scenario separately from instruction profiles and
establishes complete anchors.

`python3 tools/migration.py simulate --instructions --jobs 2` profiles the six
speed scenarios with witnesses off, `-r 64x64`, and exact Callgrind selectors
`StateGameLoop()` and, for generation, `_GenerateWorld()`. Never use a wildcard:
it also toggles off collection in the nested link-graph pause helper. These
entries exclude loading/saving and early generation setup. The driver explicitly
reconfigures and verifies effective release `line-tables-only` debug information.
Completed profiles supply Ir by its `events:` position; canonical exit-state and
full-log comparisons still run, including on reference-count cache hits. Profile
elapsed times never enter `plain_speed` samples. Only reference Ir is cached;
identity covers executable, runtime files (not their CMake bookkeeping)/libraries,
resolved inputs, build/profile settings, architecture and Valgrind version. Candidate executable identity stays
separate from the compatibility identity checked against anchors.

`SPEED_ANCHORS` in `tools/simulation/roads.py` holds each measured anchor
`{head, ratio, Ir, identity}` and a separate `integrated` record with its
head and Ir. Empty anchors mean collection only, never a passing speed guard.
Each speed pass fills them from its same-head quiet medians. Compute each projection
as `R0 * (1 + 2 * max(0, I / I0 - 1))`, directly from the fixed anchor. The
ceilings are 1.5x play and 1.3x generation, with a speed-pass trigger within
5%. PRs run no speed check and never edit `SPEED_ANCHORS` or `SPEED_INTEGRATED`
(user decision, 2026-10-22). After every second integration root profiles the
integrated tip in the background and records the six projections in the
roadmap; a rise of more than 3% since the previous tip profile gets a
fix-forward maintenance PR (AGENTS.md). Quiet passes replace the whole anchor
and refresh `integrated` together. Never reset anchor Ir alone. Profiles run
alongside other agents. Factor 2 is provisional: at
`802ec7d6a4` the measured wall sensitivity to loop Ir was 0.75 TGP, 1.0-1.31
for Opus/Grok/P1996/P2000 and 1.54 P2016 (#356). Memory stalls, excluded work
and kernel/I/O costs remain outside the proxy. AGENTS.md schedules speed passes.

## Native macOS arm64 Rust linkage

CMake verifies the pinned `rustc -vV` host against the actual C++ platform,
architecture and 64-bit pointer width, then passes an explicit Cargo `--target`.
On macOS it requires exactly `CMAKE_OSX_ARCHITECTURES=arm64` and a deployment
minimum of 11.0 or newer, with the same resolved SDK and minimum supplied to Rust
through `SDKROOT` and `MACOSX_DEPLOYMENT_TARGET` (see Rust's
[Darwin target documentation](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html)).
Intel packaging, additional Windows CRT modes and Emscripten host/target builds
remain separate tasks.

Archives live at `<build>/cargo/<validated-target>/release/libopenttd_kernels.a`.
`tools/migration.py` exposes `rust_configuration(build)` and `rust_archive(build)`;
all comparison consumers use this cache-validated lookup. Imported `HOST_BINARY_DIR`
tools remain previously built executables and do not consume the target archive.

The pinned compiler's `--print=native-static-libs` output supplies final link flags
(see [static-library linkage](https://doc.rust-lang.org/reference/linkage.html#linkstaticlib)).
Configuration retains `rust-toolchain.txt` and `rust-native-libs.log`; archive builds
retain `rust-build.log`. A content-stable `rust-build-configuration.txt` dependency
records the compiler, target, SDK, minimum and relevant build flags. A changed
configuration invalidates only that target's release crate; unchanged
reconfiguration preserves the archive. Native CI checks minimum changes and
restoration explicitly.

The macOS ARM jobs activate Rust through the reusable workflow's `rust`
input. Debug enables `OPTION_USE_ASSERTS`: game/tests get `WITH_ASSERT`, while
generators keep ordinary C++ assertions with neither `WITH_ASSERT` nor `NDEBUG`.
Release defines `NDEBUG` for every consumer. Both run the four Cargo checks,
nonempty CTest inventories and scripted regressions, then build and execute fresh
native tools. The evidence artifact retains JUnit, compiler/SDK/target metadata,
compile commands, link scripts, native libraries, archive architecture, final
Mach-O symbols and fresh generated files. Whole-archive Apple `nm` inspection is
excluded: its LLVM21 reader cannot parse LLVM23 bitcode embedded by the pinned Rust
compiler. Architecture, exact archive linkage and final executable symbol checks
remain mandatory. Linux results alone do not establish Darwin support.

## Native Windows MSVC Rust linkage

Issue #33 adds one native Windows mode: VS 2022 MSVC, x86 or x64, single-config
Ninja, `RelWithDebInfo`, `OPTION_USE_ASSERTS=ON`, and the static release CRT.
The C++ compiler's architecture macros and pointer width select
`i686-pc-windows-msvc` or `x86_64-pc-windows-msvc`; the pinned Rust host is recorded
separately, so an x64 Rust host does not choose the game architecture. The
architecture jobs install the exact target standard library and run the four Cargo
checks with an explicit target for target-dependent commands.

`WindowsRust.cmake` sets `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded` before creating
any game, test or generator target, including the tools-only path; CMP0091 is NEW
before the first `project()` call (see the
[CMake runtime property](https://cmake.org/cmake/help/latest/prop_tgt/MSVC_RUNTIME_LIBRARY.html)).
The rustc native-library query and Cargo both use `-C target-feature=+crt-static`
(see Rust's [CRT documentation](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes)).
The archive is `<build>/cargo/<validated-target>/release/openttd_kernels.lib`; the
shared locator validates target, pointer width, CRT, flags and path. The
configuration stamp includes these settings, so a code-generation change and its
restoration rebuild the archive.

Debug/debug CRT, dynamic CRT, other build types, assertions disabled, non-MSVC,
multi-config/non-Ninja, ARM/UWP/MinGW, cross-OS and external `HOST_BINARY_DIR`
configurations remain unsupported for Windows Rust (#3). Conflicting runtime flags,
Rust target overrides, mismatch suppression or missing target std fail
configuration; the C++ fallback is never selected implicitly.

The unchanged CMake ordering gives Windows RelWithDebInfo game/tests both `NDEBUG`
and `WITH_ASSERT`, while generators have `NDEBUG` alone. The x86 build exposed six
existing narrowing assignments (station expansion, snow-line calculation,
map-height selection, old-save station loading); explicit casts to their existing
unsigned destinations keep the original modulo conversion. All exports keep
`extern "C"`, which is cdecl on i686
([MSVC target ABI](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)).
Rust copies foreign encoded descriptors with `read_unaligned` before taking
references, closing the [MSVC i686 alignment gap](https://doc.rust-lang.org/rustc/platform-support.html).

`windows-rust-evidence.py` checks PE machine headers, Ninja response files, exact
archive linkage, MSVC maps and static CRT imports, and runs the native ABI
executable: struct sizes, alignments and field offsets for every `abi.rs` layout
ID, high-bit scalars, by-value returns, sentinels, null/empty inputs, Rust
allocation/view/destroy paths, history-engine staging and a per-thread non-C
locale lowercase check (signed negative compare arguments stay outside the defined
C domain, #26). Fresh tools-only builds run the current string and settings
generators after deleting previous outputs.

## Team process and engineering standards

`AGENTS.md` is authoritative for agent roles, models and reasoning effort,
attribution, the issue/PR/review flow, and the evidence budget. This section adds
only repository facts. `.codex/config.toml` sets root to `gpt-6.1-sol` xhigh and
spawned agents to `gpt-6.1-sol` high by default, with at most five spawned threads;
explicit spawn settings select the required role, and a running host may impose a
lower limit.

`docs/skills/` holds three user-invoked skills: `/steer`, the user's steering
review; `/start-development`, root's restart prompt; and `/wind-down`,
which writes the user's message pausing root. Steering reviews put their
corrections in the roadmap and edit `/start-development` only to improve it;
root follows these skills and leaves them unedited.

For unattended provider handoff, establish an independently executable takeover
before the source provider exhausts its allowance. A scheduled prompt that needs
the exhausted provider to launch its replacement is not a fallback. Verify the
host's actual execution provider; naming another model in the prompt is not
enough. If the host cannot provide independent takeover, record that limitation
and preserve a checkpoint before stopping. This rule creates no schedule itself.

Shared append lists group by component (reserve sections for concurrent ports),
then sort entries by name: includes after stdafx/before safeguards, independent
ABI probe calls, and shared table fields/mirrors/initializers/layouts (one field per line).
Witness blocks are alphabetical; keep names and consecutive local indices
together in `witness_blocks!`; global indices/array lengths are derived.

The driver and CI enforce native build/tests, nonempty test inventories, reference test-name preservation, and Cargo checks listed in `AGENTS.md`.

Server-side protection on `rust-migration` requires `Full validation`, the cheap
commit/string/script-mode checks, and resolved conversations. Full validation is
an explicit commit status, published only after the native comparisons, simulation,
platform matrix except macOS, quick checks and annotation checks succeed for a captured
PR merge commit, with the status published on its head. Skipped or cancelled jobs cannot satisfy it. Workflow dispatch runs the
controller from protected `rust-migration`; candidate test jobs have read-only
permissions and the status publishers never execute candidate code.

Ordinary PR pushes run cheap checks. With `gh` authenticated to the fork, use:

```sh
python3 tools/ci.py request 123 --profile rust
python3 tools/ci.py request 123 --profile native
python3 tools/ci.py request 123 --profile platform
python3 tools/ci.py request 123 --profile full
python3 tools/ci.py wait .local/ci-requests/REQUEST.json
```

`rust` runs four Cargo checks; `native` adds the Linux matrix and full migration
comparison; `platform` runs the platform matrix including macOS. All include quick checks.
Only `full` satisfies the merge gate. Unmergeable or pending-mergeability PRs
are refused; a changed PR head needs a new full request. Before final Full,
update the PR from current `origin/rust-migration`; GitHub strict status checks
require it to stay up to date before merging. A passing status for an old base
does not validate the new combination. If base advances, update and request
Full again. Keep implementation/review parallel and batch ready changes in one
integration PR to avoid serial runs. Do not push routine docs while that final
run is in progress; docs advance base too. This replaces the #233 policy that
accepted unchanged PR heads and checked new combinations after merging.
The CLI records the SHA and request/run identity and refuses a changed PR head.
`CI_ON_DEMAND=true` enables this scheduling after root installs and verifies the
required status. With the variable absent, automatic full CI remains as a safe
bootstrap. For a workflow-changing PR, root reviews its workflow definitions,
temporarily unsets `CI_ON_DEMAND`, and requests `--profile full --bootstrap`
(the `ci:full` label path tests the PR definitions). Root restores the variable
to `true` after merge and records both changes in the PR; never use `--admin`.
At an integration opportunity, batch two or three ready, reviewed component
heads if available; integrate a lone ready PR without waiting to fill a batch.
Assemble an integration PR from current base, preserving component commits and
linking their reviews. Review only substantive conflict resolutions under
AGENTS.md; unchanged components do not need another review. Request Full on the
combined PR, without requiring separate component Full runs first.
Finish routine base-doc updates before requesting Full. Freeze batch membership
and coordinate root sessions to avoid base pushes during the run; implementation
and review continue. Any head, membership or base change needs fresh Full,
including removing a failing component. Merge only the passing, current head
with `--match-head-commit`, then record and close the included component PRs.
Keep batches small: the integration merge is the automatic rollback unit.
A stack organizes dependent PRs but does not itself combine validation runs.

Required checks must pass on the PR head and the branch must be up to date with
base (`required_status_checks.strict=true`). The publisher still records the
immutable tested merge tree separately from the head status; its green status
alone is insufficient when the branch is behind. Root uses ordinary protected
merging with the reviewed head pinned by `--match-head-commit`, never `--admin`.
No extra local integration build or full local simulation run is required.
Protected pushes queue without cancelling a running validation; a newer push
replaces a pending one, so a burst of merges validates the newest tip and the
intermediate pushes show as cancelled. Markdown-only pushes do not start these
workflows, so they cannot displace a pending code push. A merged push whose non-Markdown files match the tree marker from the same successful
full request refreshes native compiler caches without repeating tests or scenarios.
Covered protected pushes also rebuild the cached Linux GCC and Windows
targets to refresh their compiler caches. This path skips Cargo checks, CTest,
comparison/configuration probes, annotation checks and platform evidence; Clang
and Emscripten have no compiler cache to refresh. The path is derived only from
the protected push's matching non-Markdown tree entries, and requested PR validation always runs the
complete selected suite. A combined tree no full request
covered receives all native and platform checks. Tree comparisons must be complete;
missing or truncated GitHub inventories retain heavy validation. A Markdown-only push that still
starts (GitHub runs path-filtered workflows when it cannot compute the diff) skips heavy
builds only after complete classification of fewer than 300 changed files.
Post-merge checks remain a backstop for missed defects; they do not authorize
untested combinations. Ordinary force pushes and branch deletion are disabled.
The approving-review count is zero because agents
share credentials; the attributed independent review report is the process gate
before root integrates. Repository controls are enforced separately from these
documents.

### Automatic rollback (user-authorized, 2026-10-22)

A confirmed breaking merge with no later code is removed without asking, during
development or wind-down. This is an explicit exception to ordinary PR-only
code changes, not a repair PR or a new validation cycle. Runner/API errors alone
do not identify a breaking merge. Read the failing result and identify its code
cause; do not roll back an arbitrary latest commit because a check is red.

1. Fetch the current remote tip and identify the breaking merge and its first
   parent. Check that all later changes are docs-only. Preserve the old tip on
   an origin recovery branch and record the component PR's head on its issue
   before changing the shared branch. Do not discard dirty work in any checkout.
2. Prepare the replacement from that first parent in a clean checkout. Reapply
   later docs with their completion claims corrected for the removed work.
   Keep the new integration/rollback rules if they were among those docs.
3. Push the replacement with an explicit lease:
   `git push --force-with-lease=refs/heads/rust-migration:OLD_REMOTE_SHA origin REPLACEMENT_SHA:refs/heads/rust-migration`.
   If the lease fails, fetch and reassess the new tip; never use bare `--force`.
   Root is authorized to temporarily enable force pushes for this operation
   if protection requires it. Preserve all other settings, restore the prior
   force-push setting even on failure, and read it back. Administrative access
   also permits the documented direct docs pushes; it never permits bypassing
   validation for a new code integration.
4. Reopen the tracking issue if it was closed, record the recovery branch/head
   and next repair step, and report the reset afterwards. GitHub keeps the old
   PR marked merged; it cannot be reopened. Resume through a replacement PR
   linked to it. Wind-down records that next step without opening the repair PR
   or waiting for post-reset CI. Keep the recovery ref until the replacement
   work is safely retained; do not restore the rejected merge by pulling it
   back from a stale local branch.

If later code makes this bounded reset inapplicable, stop integrations and
record the recovery needed. During wind-down, hand it off without starting a
fix cycle. A failed recovery operation is reported as failed, never as a reset.

Native validation exports both frozen runtimes and once-built fleet/orders adapters,
then runs component comparisons and three simulation shards concurrently
(each about as long as the comparisons job). Recorded
scenario seconds assign work while keeping effects/disasters/aircraft fixture
families whole. The gate requires every job and exactly the current default
`simulate.py --list` inventory. GitHub's run-scoped artifacts carry the runtimes
and reports between jobs. Exports strip debug sections only on copies; original
verification binaries stay intact. Probe workers receive prebuilt
generators/archives and the widget probe's real link objects; build metadata is
relocated for the receiving checkout. Reports/logs and retained
failing saves are artifacts; build inputs expire after one day. Tests and their results are never cached. Cache writers trim native roles
to 750 MB, Linux flavours to 600 MB and macOS flavours to 650 MB. The trusted
cache-retention workflow keeps the latest entry per prefix and bounds total usage
to 8 GiB (below the 10 GiB repository limit).

Runner settings are `CI_LINUX_HEAVY`, `CI_LINUX_COMPARISONS`,
`CI_LINUX_SIMULATION`, `CI_LINUX_PLATFORM` (default `ubuntu-24.04`; Clang only,
the cached GCC flavours always use free `ubuntu-24.04`), and
`CI_MACOS_HEAVY` (default `macos-latest`) and `CI_WINDOWS_HEAVY` (default
`windows-2022`). Heavy native jobs use `nproc` slots.
One full request peaks around 13 jobs; overlapping requests may queue against
account/runner capacity. Runner size and account plan remain repository settings.
The repository belongs to the `fetch-laboratories` organization, where Blacksmith
serves the Linux jobs: `CI_LINUX_HEAVY` is `blacksmith-8vcpu-ubuntu-2404` and the
other Linux variables are `blacksmith-4vcpu-ubuntu-2404`. Windows uses
`blacksmith-4vcpu-windows-2025` (#252 accepted the image; #304 matched the
size to its `--parallel 4` build); macOS and light jobs stay
on GitHub-hosted runners (free for this public repository; Blacksmith
bills macOS at 20x). Blacksmith serves `actions/cache` from its own co-located
store on its runners, so the GitHub cache budget above covers only GitHub-hosted
jobs. Unsetting a variable returns that job to GitHub with no workflow change.
Profile CI time with the Blacksmith CLI (`blacksmith jobs list|steps|stats`,
`jobs aggregate`, `jobs diagnose rightsize`) before changing runner sizes or
sharding: `stats` gives each Blacksmith job's CPU and memory timeseries. Pass
`--job-name` to `aggregate` and `diagnose rightsize`: each full request names its
run `CI full PR N <hash>`, so grouping by workflow leaves one run per group
(`insufficient_data`). Size by the busy step, not the job average: compile and
scenario steps saturate every runner, and setup steps idle at about one core. Agents
authenticate with an org token (`blacksmith org-token create`, admin only), never
a copy of a person's `~/.blacksmith/credentials`.

macOS runs on `rust-migration`, not per PR (#357). After every fifth integration
root runs `gh workflow run ci-build.yml --ref rust-migration`, which builds and
tests only the two macOS jobs at the tip and refreshes their caches, and waits with
`gh run watch`; a red result is fixed before more integrations. An uncovered
protected push also runs them. After two hosted-capacity cancellations of a macOS
run, root runs `gh variable set CI_MACOS_HEAVY --body blacksmith-6vcpu-macos-26`,
reruns, and runs `gh variable delete CI_MACOS_HEAVY` when that run finishes;
setting the variable needs no Blacksmith credentials.

`python3 tools/preflight.py --base origin/rust-migration` runs the pinned inherited
commit checker. Its first run downloads the pinned hooks into ignored shared
storage; `--hooks PATH` uses an existing clean pinned checkout offline.
Record validation commands and limits in the PR; CI runs identify the tested head.

`python3 tools/shared-accessors.py --base-ref origin/rust-migration` enforces
#249 in quick CI. It checks fork-added functions/lambdas outside the named shared
service files for single shared-world reads or shared-helper forwarding. The
shared files include the canonical `engine_services.h` and
`presentation_services.h`; root selected these exact filenames for #248, and its
independent review verifies their consumer adoption and deletion of port copies.
Existing definition fingerprints are in `tools/shared-accessor-baseline.txt`: remove retired
entries, never add exemptions. CI compares that baseline with the PR base or prior
protected-branch commit. `tools/test-shared-accessors.py` runs CLI category and
ratchet regressions by default. This small syntactic check covers typed parameters,
cast/lookup/typed owner aliases, settings, timers, map reads and named helper calls;
it does not replace review of macros, derived-type fields or compound callbacks.

Original header helpers and constants live once in crate modules named after the
header (`slope_func`, `map_func`, `tile_type`, `company_type`, `command_type`,
`direction_type`, `economy_type`, `newgrf_callbacks`, `strings_type`,
`viewport_func`, `town_map`; #261). Ports import them instead of keeping copies,
and `src/rust/services.cpp` asserts every value. The checker cannot see Rust copies.

`python3 tools/port-metrics.py` reports the change in retired pinned-original
C++ lines separately from fork glue added/removed and signed net glue. Original
retirement can be negative when original lines are restored; removing fork glue
does not retire original C++. Ownership ports retire more original C++ than
net glue plus tooling added, or explain why; conversions report their net glue.
`--whole-fork` prints totals against the pinned original and the retired share of
simulation source lines. Its one inventory covers world-state owners, commands,
accessors, link graph and pathfinding, excluding GUI, platform, vendored code and
generic support. Mixed owner files, blank lines, comments and data tables count.
The diff/guard classification measures source lines, not C++ preprocessing or
behavioral completion. Historical roadmap rows keep their labelled legacy counts.

## Selecting components

Selection follows [AGENTS.md](../AGENTS.md) and the [roadmap](roadmap.md).

## Ported components

The entries below record the components ported so far. The earlier ones were
selected under the previous coverage-first rule and are mostly utility kernels.

Unless an entry says otherwise, these properties hold for every port:

- With `OPTION_RUST=ON` (`WITH_RUST`) the C++ facade calls Rust; with it off, the
  original C++ bodies compile unchanged. Rust on further platforms is #3.
- Rust panics and Rust allocation failure abort (`panic = "abort"` in both
  profiles); the C ABI never unwinds, and no C++ exception crosses a Rust frame.
  Identical resource-exhaustion timing is not claimed.
- Borrowed byte spans are initialized, readable bytes in one live allocation with
  length at most `PTRDIFF_MAX`, immutable during the call; empty spans may be
  null; read-only spans may overlap. Rust retains no pointer after returning.
- Evidence is the unchanged upstream tests first; named comparisons cover only listed gaps.
  C++ sanitizers do not instrument Rust accesses. These checks establish neither whole-game
  equivalence nor cryptographic security or complete secret erasure.


<!-- Subsystem: Support libraries and text. Add a new entry at the end of its group, not of the file. -->

### StringConsumer integer parsing

C++ keeps the public templates, cursor updates, optional/pair/string-view adapters and formatted
logging. Rust owns base selection, digit scanning, width-aware conversion, overflow/clamping and
lexical skipping. The ABI borrows arbitrary bytes and returns scalar parse metadata; C++ formats
diagnostics and commits the cursor.

Default CI runs the unchanged StringConsumer cases and `python3 tools/run-comparisons.py`, which
includes the integer parser and builder comparison. Covered inputs include width boundaries,
negative hexadecimal and malformed prefixes; fatal logging behavior is checked by the existing
comparison.

### Alternating-iterator traversal

Rust owns initial position/selectors, logical advancement, side selection, end
transitions and position comparison. C++ keeps the typed iterators, range identity
assertions, dereferencing and container lifetimes. Each increment recomputes the
live range distance; Rust requests a typed move, C++ applies it and queries only
the requested live boundary, then Rust completes the next-side state. Size is not
cached and random access is not required; stable noncontiguous iterators remain
supported after insertion.

`src/rust/alternating_ffi.h` maps size_t to usize and explicit uint8 selectors (0
before, 1 after). Nonnegative distances keep the original size_t conversion with no
added range cap. No pointer, allocation, element or ownership crosses the ABI.
Logical end skips movement/completion and keeps the last selected Base iterator,
while a separately constructed end keeps middle; they compare equal by position.

Evidence: the fifteen unchanged fixed sequences, plus public-interface tests for
empty/singleton, independent copies, postfix/prefix identity, position ordering,
distinct end Base values and stable-list insertion.

### StringConsumer byte algorithms

`consumer.rs` owns exact unsigned little-endian assembly; bounded read/skip lengths and
shortfall/cursor decisions; byte-prefix matching and conditional consumption;
substring/character-set search and membership; and separator result/consumption decisions. C++
keeps typed optional/default conversions, string_view construction, diagnostic formatting,
cursor commit and trivial accessors, and preserves empty view pointers through the original
substring at the current offset.

`src/rust/consumer_ffi.h` returns scalar `repr(C)` metadata by value. Nonempty
search/set/separator assertions remain preconditions; release behavior outside them (including
the original empty-separator loops) is not claimed.

The in-place pair, owning string/container code, allocators, escape parsing and encoded-string
transformation remain C++. SQFile resizes its own buffer and rebuilds the consumer; no Rust
borrow survives that boundary.

Default CI retains the unchanged consumer tests and runs `python3 tools/run-comparisons.py` for
all registered reference comparisons. The public tests cover byte reads, offsets and shortfalls;
release behavior outside original preconditions remains unspecified.

### StringBuilder numeric byte encoders

Rust extracts the bytes for `PutUint8` and `PutUint16LE`/`32LE`/`64LE`, and formats integral
`PutIntegerBase` values in bases 2 through 36: lowercase digits, a leading minus for signed
negatives, no base prefix, one digit for zero. Signed binary wrappers keep their modulo-width
unsigned casts; the signed magnitude uses unsigned negation, including `INT64_MIN`.

`src/rust/builder_ffi.h` takes scalars and returns `repr(C)` byte arrays by value; C++ passes a
span of the returned local array synchronously to its virtual `PutBuffer`, and sinks must not
retain it. Invalid bases are outside the original 2..36 `std::to_chars` precondition.

Evidence: the three unchanged StringBuilder cases, InPlaceReplacement and encoded-string tests,
plus `python3 tools/run-comparisons.py`, which runs the integer and builder comparisons in
default CI. Those comparisons cover base/width boundaries and the synchronous output sink.

### Encoded-string compatibility and parameter rewriting

Rust owns `FixSCCEncoded`, `FixSCCEncodedNegative`, `EncodedString::ReplaceParam` and the shared
`GetEncodedStringWithArgs` serialization. C++ keeps the save-version dispatch and its order
(legacy encoding before version 350, old markers before 169, negative repair before 353, then
sanitation), general decoding, rendering, ScriptText encoding and sanitation.

Retained quirks: legacy conversion is permissive (old E028/E02A normalize only with fix_code;
markers are recognized inside quotes; quotes toggle and disappear; quoted colons stay bytes;
numerics are not validated).

`src/rust/encoded_ffi.h` passes explicit tags (0 monostate, 1 uint64, 2 byte span), never C++
string/vector/variant layouts; StringID width and the RS/E000..E003 token contract are asserted.

Evidence: the four unchanged FixSCCEncoded/Negative and ReplaceParam tests, plus `python3
tools/encoded-comparison.py` (verbatim pinned functions, Rust and portable bodies, four
NDEBUG/WITH_ASSERT combinations, ASan/UBSan/LSan on the C++ side; the Rust archive is
uninstrumented).

### Byte-string utilities

Issue #21 moved case-insensitive compare/equal/prefix/suffix/contains, lowercase conversion,
uppercase hex encoding, sequential hex decoding and byte-set trim scanning into
`byte_strings.rs`. C++ keeps std::string owners, views, erases and the installed standard
library's equal-prefix length-comparison policy; Rust returns that C++-supplied scalar only when
the shared prefix is equal.

Rust calls native C `toupper` with the original char promotion (signedness supplied by C++) and
`tolower` with unsigned-byte promotion, keeping process-locale behavior. Negative-char uppercase
inputs other than EOF are outside portable C's domain; only observed native behavior is matched
(#26).

Lowercase takes an exclusive mutable span with offset at most size. Only the portable
natural-contains fallback uses these helpers; ICU, Windows/macOS collation, validation, in-place
replacement and StringIterator backends remain C++.

Evidence: the twelve unchanged utility tests, plus `python3 tools/byte-strings-comparison.py`
(3,699 records against full pinned string.cpp, length-result saturation above INT_MAX via mmap,
-funsigned-char builds).

### UTF-8 codec and byte positions

The full UTF-8 codec, `IsUtf8Part`, forward/backward iterator stepping and `GetIterAtByte`
normalization run in Rust. Inline DecodeUtf8/consumer PeekUtf8 handle leading ASCII (including
NUL) without an FFI call. The C++ view keeps its borrowed string_view and iterator facade, pair
adapters, comparison assertions, postfix copying and invalid-data `?` dereference.

Retained behavior: surrogates are accepted; overlong and out-of-range first sequences are
rejected; malformed trailing data after a valid first sequence is ignored; unused encoding bytes
are zeroed.

`src/rust/utf8_ffi.h` uses 32-bit codepoints and size_t lengths/offsets and returns `repr(C)`
data by value, with no allocations or output aliases.

Evidence: the three unchanged UTF-8 view tests and consumer/builder tests, plus `python3
tools/utf8-comparison.py` (assertion and NDEBUG builds; encoding boundaries, malformed runs,
embedded NUL, empty views, consumer-versus-view movement, all single bytes, and the `offset >=
size` end branch including SIZE_MAX). Game logging and Unicode rendering are outside it.

### Rounded square root and runtime integer saturation

`IntSqrt(uint32_t)` and runtime `SoftClamp` call `math.rs` through
`src/rust/math_ffi.h`. `IntSqrt` keeps nearest-integer rounding, including 65536 for UINT32_MAX.
`ClampTo` uses the original inline C++ template (#276); its hot callers avoid an ABI crossing.
`DivideApprox` remains C++: its potentially overflowing signed intermediates need separate work.

The ABI is scalar-only: modulo-2^64 value bits plus explicit width and signedness; C++20
integral conversion rebuilds the result.

Reversed signed 8/16-bit SoftClamp intervals convert min to unsigned and then promote to int, so
`SoftClamp<int8_t>(0, -1, -3)` returns 126.

Evidence: the unchanged IntSqrtTest Zero/FindSqRt, ClampTo and SoftClamp cases, plus `python3
tools/math-comparison.py` (every uint32 root square and rounding transition, width/signedness
extrema, bool and character/size aliases, adapters, unsigned 128-bit destinations, SoftClamp
intervals, GNU link-wrap call counts) and `python3 tools/math-extension-comparison.py` (wide
templates on the native library; macOS CI runs it with `--build build`).

### Generic history structural engine

Issue #29 moved descriptor-driven validity, rotation scheduling and query traversal into
`history.rs`. HistoryRange constexpr construction/layout, typed HistoryData storage, every
SumHistory specialization, graph fillers and GetAndResetAccumulatedAverage remain C++;
production averaging keeps its literal-0 int accumulators and nested reduction grouping.

Rust keeps an arbitrary-depth scalar frame stack and streams staged operations; C++ describes
immutable ranges by value and executes every typed operation after Rust returns. Unsigned index
arithmetic wraps at uint32, and GB's uint32 truncation remains for uint64 validity masks.

Retained quirks: update and rotation use explicit cur_month while queries read the live
TimerGameEconomy::month; children update/rotate first even for saturated or skipped parents;
no-prerequisite higher rotations still shift; query validity ORs all children while
IsValidHistory checks only the first; invalid children still contribute data; invalid query ages
keep the original fatal dispatch.

Evidence: the unchanged 288-month test (86 assertions in each standalone run), plus `python3
tools/history-comparison.py` (11,133 records at O0/O2: phases, masks, arbitrary chains, ages,
typed operation order, exceptions, aliasing, graph fillers, and the three verbatim production
reducers; a nested-year fixture yields 0 where a flattened reduction yields 1).

### Authentication and streaming owners

Issue #35 moved X25519 session and encryption-context ownership into Rust behind a versioned,
primitive-only host function table. Bundled Monocypher algorithms are unchanged. Rust owns
stable key/session allocations and vendor-context storage; C++ supplies each vendor context's
size/alignment and starts its trivial lifetime, so Rust mirrors no vendor struct and other
archive consumers gain no vendor symbols.

Secrets are initialized in their final allocation; copies go heap to heap and assignment
overwrites fixed storage, as in C++; rvalue copies preserve the source. Streaming contexts keep
the bundled successful-rekey behavior (the counter does not advance); failed authentication
leaves context and output unchanged.

Borrowed fixed-width views keep their address until owner destruction; callers serialize access.

Evidence: the five unchanged network cases, plus `python3 tools/auth-comparison.py`: pinned and
candidate session/Packet/vendor sources as separate endpoints, both mixed directions, Rust/Rust
and portable C++ against original transcripts (wire bytes, derived keys, RNG traces,
failure/retry, copies, aliasing, exception cleanup), with a C++ sanitizer run that also checks
Rust allocator leaks but does not instrument Rust memory accesses.

### Paired Script Admin conversion

`ScriptAdminMakeJSON` and `ScriptEventAdminPort::GetObject` keep their C++ interfaces. Rust
selects types, walks both conversion directions, propagates results and schedules the original
VM and JSON operations; an opaque per-invocation handle returns scalar actions that C++ executes
after each call returns.

C++ keeps the bundled VM, JSON objects, network framing and exception cleanup. No VM/JSON
layout crosses the boundary, and no Rust borrow survives a reentrant host operation.

Outgoing: `depth == 25` is checked before reading the VM type or changing JSON (also for an
explicit initial depth); live iteration, key stringification, duplicate-key overwrite order,
`index - 1` and `depth + 1` are kept; a failed child leaves completed root children, with the
original VM and iterator pops before cleanup. Incoming: object root only, floats rejected, no
depth limit; failure restores the stack top before logging and pushing null; malformed input
keeps the original diagnostic.

Evidence: the two unchanged `test_script_admin.cpp` cases (15 outgoing and 27 incoming checks),
plus `python3 tools/admin-conversion-comparison.py`, which uses the bundled VM and unchanged
pinned bodies at O0/O2: depth 25/26, incoming depth 40, integer extrema, byte keys, partial
failure, colliding keys, reentrant `_tostring`, allocation failures and a script allocation
limit. Game-log storage, arbitrary depths, network simulation and complete script equivalence
are outside this evidence.

### ScriptList storage and iteration

Issue #44 moved both item/value indexes, all four sort modes, live pending-cursor and end state,
mutation accounting, filters and list algebra into Rust. C++ keeps script identity/bindings,
pool enumeration, VM/error/operation charging and save/load adapters.

The scalar/pointer ABI avoids aggregate returns on 32-bit hosts; items/values are signed 64-bit
and modification tokens signed 32-bit. Rank arithmetic and modification tokens wrap like the
compiled original (#260). Owner borrows end before VM reentry; callbacks that invalidate
iterators while evading the modification check remain outside the evidence.

Evidence: unchanged `regression_regression` and `regression_stationlist`, plus `python3
tools/script-list-comparison.py` (original, Rust and portable binaries at O0/O2 with the bundled
VM: cursor swaps and insertions, self operations, pending removals, empty-list union, filters,
zero/negative/signed-limit ranks, callback failures, partial commits, operation charges,
List/TileList save/load and clones). A Rust unit test seeds the token-wrap boundary.

### ScriptList VM control

Issue #65 / PR #70 moves `Valuate`, VM-filtered `FillList`, `SaveObject`, `LoadObject`, `_get`,
`_set` and `_nexti` control into a scalar Rust controller. C++ executes VM actions, live typed
pool reads, diagnostics and native command/ operation-limiter RAII scopes.

Valuation touches before validation, checks type before modification, and commits before
pop/charge. Filtering keeps function/scope timing, validity-before-callback, live index rereads
after callbacks, bool-only results and its original throw type. Original failed-getter undefined
values (#67), unrepresentable conversions, signed overflow and invalidated iterators remain
outside defined evidence.

Evidence: both unchanged scripted suites and `python3 tools/script-list-comparison.py`: 1,118
observations per O0/O2 mode against pinned and portable C++, plus identical generated
AI/GameScript/template bindings. The fixture uses four typed
items; production pool/CPU-limit evidence comes from the game regressions.

### Nested widget descriptor parser

`MakeNWidgets` and `MakeWindowNWidgetTree` use a widget-specific pull parser. Rust owns the
descriptor cursor, attribute traversal, container decisions, recursive tree control, end
markers, complete-consumption policy and first/root/body/shade composition. C++ keeps the
single-part factory, attribute operations, widget classes, RTTI checks, `unique_ptr` owners,
Add/GetWidgetOfType, generator callbacks (which may run a nested parser), constexpr builders and
public signatures.

The scalar ABI carries uint64 descriptor offsets, owner slots, uint8 widget tags and capability
observations; no union, virtual object, RTTI layout or function pointer enters Rust. Retained
quirks: push-button bits are not masked for container classification; function-produced subtrees
never adopt following nodes; EOF inside a container is accepted; null generator results keep the
unconsumed cursor and end-marker assertion; the `WITH_ASSERT`-only trailing-parts exception
stays separate.

Evidence: all four unchanged `test_window_desc.cpp` bodies (every registered WindowDesc through
the production parser), plus `python3 tools/widget-parser-comparison.py` against unchanged
pinned parser bodies with real construction primitives (163 registered descs natively): types,
order, attributes, shade, ownership, callback/cleanup order, exceptions, and null generators
under three assertion policies. Rendering, layout, events and arbitrary malformed descriptors
are outside this evidence.

### ChaCha20, Poly1305 and AEAD primitives

Issue #48 moved the bundled Monocypher 4.0.2 ChaCha20/Poly1305/AEAD family. Its C interfaces and
caller-owned context types stay. C++ starts context lifetimes and passes
size/alignment/field-offset descriptors; raw field access reads only initialized fields,
preserves unwritten chunk/padding bytes at init and wipes the complete caller context at
finalization.

The nonthrowing wipe and constant-time verify16 come from a two-function explicit cdecl table;
Rust has no vendor imports or global callbacks. Other Monocypher algorithms and the portable
family remain C++.

ABI layout IDs 19 through 21 cover the two-leaf table and two field-layout descriptors.

Default CI runs `python3 tools/auth-comparison.py` against pinned vendor functions, Rust and
portable C++. Existing network tests and the comparison cover the primitive family; C++
sanitizer coverage does not instrument Rust memory accesses.

### BLAKE2b family

Bundled Monocypher BLAKE2b keeps all six public C functions. Rust owns the twelve compression
rounds, keyed/unkeyed initialization, word/block buffering, pending-final-block decisions,
digest serialization and final context wiping. C++ owns the context lifetime and supplies
size/alignment/field offsets (ABI descriptor ID 22); the archive borrows the existing
nonthrowing wipe leaf, imports no vendor symbols, reads no context padding and adds no heap
allocation.

The supplied hash size stays in the initial parameter; final output writes `min(hash_size, 64)`,
keeping defined source behavior at sizes zero and over 64. Keys of 65-128 bytes fit the original
padded block and keep source behavior (the documented interface limit is 64); keys over 128
overrun original storage and have no reproduction guarantee.

One-shot output may overlap message/key, including partially: all inputs are consumed before
output writes.

Default CI runs `python3 tools/auth-comparison.py` and the unchanged network tests. The
comparison covers keyed/unkeyed hashes, streaming partitions, context copies and final wiping
against the pinned vendor implementation.

### Packet framing, binary serialization and transfer state

Rust owns the native-width limit, the persistently narrowed uint16 cursor, binary
encoding/decoding, length-prefix sequencing, framing offsets and transfer planning/commit
decisions. C++ keeps vector/string storage and each original allocation operation, direct spans,
socket policy, assertions, external encryption/transfer callbacks, Send_string/Recv_string
collection and sanitation, and the exact Recv_bytes callback.

The facade keeps the original per-byte append order and advances each received result byte
before its potentially throwing C++ push; parsing commits position two only after resize
succeeds. Normal false decryption still skips its MAC; throwing decryption does not. Transfers
commit positive results to the live post-callback cursor with uint16 narrowing; native unsigned
arithmetic wraps as in the original.

Evidence: the five network/authentication cases, the 1,800-record mixed-endpoint corpus of
`python3 tools/auth-comparison.py`, both scripted regressions, and `python3
tools/packet-comparison.py` at O0/O2 (pinned, portable and Rust Packet sources: binary/buffer
bytes, suffix identity, copy independence, TCP/UDP framing, partial/zero/negative/throwing
transfers, reentrant live cursor changes, close policy, controlled encryption callbacks,
selected C++ allocation failures, and the original uint16 per-byte wrap with larger native
buffers and prefix narrowing); native ABI smoke calls exercise the exports.

### Normal X25519 and Montgomery ladder

`crypto_x25519`, `crypto_x25519_public_key` and `crypto_eddsa_trim_scalar` keep their public
interfaces; one coarse `scalarmult` adapter serves the unchanged dirty-small/inverse callers.
Rust owns the complete 255/256-bit ladder and its private ten-limb field decode, encode,
add/subtract, masked swap/copy, multiplication, squaring, carry, inverse-square-root and
inversion. C++ shared-field helpers remain for untouched Edwards, conversions, Elligator and
other families; that internal duplicate is unfinished migration, not shared-field closure.

A separate synchronous immutable cdecl table (ABI ID 25) holds only the original nonthrowing
wipe and constant-time verify32 leaves; no field object, callback registration, application
operation, vendor import or heap crosses the ABI. Original explicit wipe points and order
remain; compiler spills/copies are not erased.

The fixed 32-byte output may overlap secret/scalar/point, including partially: inputs are
consumed before serialization. The public-key top bit is ignored, noncanonical field encodings
are accepted and raw zero outputs remain primitive outputs; authentication rejection policy and
RNG/protocol behavior are unchanged.

Evidence: the five network tests and `python3 tools/auth-comparison.py`, whose direct fixture
compiles the vendor's actual private ladder with one coarse wrapper as the oracle: scalar clamp
bits, zero/one/base/noncanonical/top-bit points, output/input overlap, forward trim, both raw
ladder bit counts with bit 255 set, and deterministic dirty-small/fast/inverse callers.

### Coupled string validation and borrowed in-place replacement

Rust owns the shared StrMakeValid scan/policy decisions, the StrValid fixed-span scan and
InPlaceBuilder write progression/copy. Each sanitation decision carries a consumed count and at
most one existing UTF-8 encoded character; C++ advances the actual StringConsumer before its
original output operation.

The historical codec stays authoritative, including accepted surrogate values. The default UTF-8
comparison covers malformed sequences and byte offsets; game logging and Unicode rendering
remain outside its scope.

C++ keeps allocation, string owners, cursor commit and fatal dispatch. Rust retains no buffer
pointer; raw forward reads/writes preserve defined left overlap without overlapping references.

Evidence: the unchanged InPlaceReplacement case (26 assertions) and the UTF-8, consumer and
builder tests, plus `python3 tools/utf8-comparison.py` in its O2 assertions/NDEBUG modes against
unchanged pinned string.cpp bodies, portable and Rust candidates: all 16 flags with ignored
unknown bits, malformed/truncated/ overlong sequences, surrogates, printable/SCC boundaries, NUL
and C-string suffix bytes, missing terminators, empty/unchanged inputs,
copy/assignment/self/rvalue copies, live consumer rewinding, defined left overlap and overtake
fatal state.

<!-- Subsystem: Landscape, water and map structures. Add a new entry at the end of its group, not of the file. -->

### TGP terrain generation

Rust owns the temporary height map and every TerraGenesis stage, including
height estimation, noise, histogram, smoothing and regional curves. #350 also
moves advance/apply loops into this owner, writing through typed map services
without a native height-output copy. C++ extracts named settings and retains
shared RNG, canonical map storage and native progress/abort-hook operations.

The process-wide abort owner is borrowed only between progress calls. Both
progress operations catch AbortGenerateWorldSignal in the native handoff; cleanup
runs with no live generator borrow, then the landscape facade rethrows after
Rust returns. Default generation cases and the native cancellation probe compare
both TGP progress events and single cleanup. Original bodies remain portable-only.
Maximum-map performance and Windows/macOS runtime parity remain untested.

### Landscape scheduler, ground and flooding

The #212 owner ([PR #255](https://github.com/fetch-laboratories/openttd-rust/pull/255))
controls `RunTileLoop`, clear/rail/road/tunnel ground evolution, flooding behaviour,
half-tile rail removal, dry-up, neighbour wakeups and live flood-victim traversal.
The process-global saved cursor commits after the complete source batch; DATE,
legacy loading, reset and zero repair retain their widths/order. Original selected
bodies compile only without `WITH_RUST`; packed map/pool storage remains native.

The in-entry LFSR keeps native unsigned map-log widths and reads each live type
byte from audited allocation metadata. All seven Rust tile owners dispatch directly,
void tiles straight to the water owner, which reuses the dispatched type and reader
and derives `IsValidTile` from the map size; only house/station/object have native
scalar handlers with local strong-index construction; the five unused facade slots
are null.
Flooding calls the Rust signal flush through the canonical signal context.
Leaves shared by several
owners (clear/tree ground, snow, shore, sound, foundation) live in shared tables.
Tree/industry water loops, tree wakeups, rail flooding queries and road town-type
selection call Rust directly; canonical rail leaves have one binding. Ground
boundaries use actual native enums, unrestricted transparent
Rust carriers, exact counters and typed station handles. Legacy protocols adapt
explicitly. Each native entry owns independent lazy stack command-result storage.
No hot call allocates a protocol object or copies a complete native tile/vehicle.
Vehicle iterators advance after crash/events/news/effects; airport membership,
bridge endpoint order and company restoration follow source. No ordinary tick
callback unwinds; generation-progress cancellation stays outside tick.

`verify`, the default scheduler comparison and `simulate landscape roads-flooding`
exercise real ABI calls, declared endpoints and required named branch witnesses;
`--self`/`--soak` extend evidence. Missing witnesses fail. The unchanged scheduler
probe covers dimensions/cursor/reentry phases unreachable through ordinary saves.
Remaining matrix/editor/NewGRF/threaded gaps are in #156.

#274 extends the canonical allocation descriptor with typed base/extended pointers,
count and native assertion policy. The scheduler's clear/water/tree reads use
header-named Rust helpers; shore-tree flooding shares that reader. Field-sized raw
loads finish before callbacks, and observations reload after mutation. The audited
scheduler no-reset/no-yield interval alone permits descriptor metadata to survive
callbacks. C++ retains arrays, allocation and all writes. Standalone water/tree,
generation, planting/command and unrelated owner reads retain canonical typed
callbacks; no descriptor is carried into these unaudited entries.
Generated named probes pin both layouts. Default native tests compare field/slope
encodings, extended fences, live mutation, reallocation and assertion-policy
fallbacks with unchanged native services. Assertions-off raw reads add no bounds,
type or field rejection. The PR retains the historical same-base TGP profile;
speed is checked on the integrated branch, without per-PR counts or a speed gate.
This maintenance moves no storage or whole owner.

### Landscape partial-pixel height

`GetPartialPixelZ`, the scalar landscape height kernel, runs in Rust behind its
original C++ interface. The ABI takes two `int32_t` coordinates and one `uint8_t`
slope; a `uint64_t` result carries the complete `uint32_t` height domain plus
`UINT64_MAX` for the original unsupported-base-slope `NOT_REACHED` dispatch.
No pointers, allocations, shared state or ownership cross it. C++ asserts widths,
tile dimensions and slope/corner encodings. Rust preserves half-tile returns
before base validation, upper-bit clearing, signed arithmetic shifts, unsigned
TILE_SIZE/TILE_HEIGHT promotions and wrapping, and signed-to-unsigned returns.
Coordinates outside the tile remain accepted, including legitimate `UINT32_MAX`
heights. Original signed-overflow inputs have no equivalence guarantee; Rust
adds no overflow panic. Unsafe code is denied except for the export attribute.

Evidence: 32 unchanged upstream cases through the C++ entry point, Rust boundary
cases, native ABI sentinel/height probes, and `python3 tools/math-comparison.py`
against the unchanged pinned body, portable C++ and Rust: all 256 slope bytes on
an extended -32..32 grid and selected full-width coordinate pairs, with C++ UBSan.
The comparator excludes only executed signed-overflow expressions; it exercises
the out-of-tile arithmetic gap that the simulation harness's callers cannot reach.
The harness checks game state; the direct corpus is bounded, not exhaustive over
all coordinate pairs.

### Spiral tile traversal

Rust owns square/hole initialization, position initialization, per-direction
movement, shell jumps, outside-map skipping, end detection and coordinate-only
equality. C++ keeps the typed iterator/sequence facade, TileXY dereference,
copies/postfix wrappers and map storage. Map dimensions are passed at each
constructor and prefix increment; Rust caches none. Orthogonal and diagonal
tile-area algorithms remain C++.

`src/rust/spiral_ffi.h` has five pointer-free by-value operations and a copyable
40-byte state (nine uint32_t fields counting the four extents, then a uint8_t
direction at byte 36; alignment 4), asserted on both sides. Directions are
NE(-1,0), SE(0,1), SW(1,0), NW(0,-1), with west shell jumps (+1,-1). Coordinate,
extent, position and radius arithmetic wraps explicitly at 32 bits, including
temporary outside-map coordinates; there is no new clamp or diameter cap. Positive
diameter/radius and increment-before-end remain original preconditions. End is
radius equality with a non-invalid direction; equality compares x,y only.

Evidence: the five unchanged ordered tests (217 assertions) plus four public cases
(clipped sequences on 128x64/64x128 maps, live map dimensions, copy/postfix/equality
and sentinel state, UINT32_MAX hole-initialization wrapping), compiled against
pinned reference algorithms, Rust facades and portable C++ bodies. The huge
inherited perimeter after a wrapped extent is not traversed. Native generators do
not use spiral traversal.

### Tree generation, simulation and planting

Rust owns generation/scattering/grove geometry, placement policy, tile growth, climate loops,
tick seeding and the persisted byte counter, plus command traversal, limits, costs and errors
(#99/#107). Original bodies compile only in portable builds. Map/pools, rendering and the editor
forest-brush loop remain C++; the brush calls Rust placement.

Each entry (tile loop, tick, generate, scatter, place, plant, command, clear) is one plain call
with a by-value settings record (#202). It reads typed plant/tree records and calls typed
`noexcept` services from a designated `OpenTTDTreeServices` table and the shared map predicates
(`GetTileZ` only where the original calls it) at the original points. Water flooding, the NewGRF
ambient callback and nested landscape clears may reenter; no Rust borrow survives them and the
tile is read again afterwards. World-generation abort is the only handoff: the progress wrapper
keeps the exception, Rust returns with no further draw or write, and the facade rethrows.
Save/load reaches only counter-address/reset exports; DATE/TTD/TTO keep the original byte and
DATE LoadCheck omission.

`python3 tools/migration.py simulate trees` covers climates, tree placement, growth and counter
reloads. Cargo tests and the native ABI probe cover explicit/editor policy, Money bounds,
bitpattern 7, abort return order, reentry and `GetTileZ` call points. Editor interaction,
diagonal traversal, legacy saves, NewGRF ambient callbacks and the abort rethrow are unreached.

### Water-region cache and graph service

Rust owns the map-lifetime cache, validity, optional tile labels, edge masks,
patch counts and aqueduct flags, flood scratch, lazy rebuild, invalidation and
ordered neighbour traversal (#104). C++ keeps canonical map storage, shared
water track/follower queries, formatting and the water_regions.h facade. The
original implementation compiles only in portable builds; WRGN legacy skip is
unchanged. No persistent cache mirror or RNG enters this port.

Direct `noexcept` queries return copied scalars. A visitor cursor keeps only
progress and per-side labels, returning before arbitrary C++ visitors with no
cache borrow. Subsequent sides observe live changes; the aqueduct flag is read
without a forced rebuild after side visitors, with original per-tile label
rebuild points preserved. C++ RAII destroys the cursor when a visitor throws.
Map replacement during a visitor violates original reference lifetimes.

`python3 tools/migration.py simulate water --jobs 2` compares every semantic
chunk in plain/desync runs: manual/cargodist cargo, paid delivery, lock/canal
closure and recovery, cross-region aqueduct, nearest depot, and reference-built
live-path reload. Native ABI
probes cover visitor invalidation and owner replacement that ordinary YAPF
visitors cannot trigger. `tools/water-scenario-ai/README.md` gives preparation,
negative-probe and optional crossing/timing measurement commands. Arbitrary
maps/NewGRFs and exhaustive region/path limits remain evidence limits.

### Water construction, clearing and restoration (#294)

Rust (`water_cmd.rs`, shared `water.rs`/`ground.rs`) owns ship depot/lock/canal
commands/removal, water clearing/restoration, river/desert traversal and owner/terraform/
build-above policy. Original command bodies and `MakeLock` remain portable-only.
Lock checks/build/removal order, depot partial clears and post-clear accounting,
conditional RNG draws, narrowing and wrapping retain the original sequence.

C++ keeps packed map primitives, depot identities, drawing, native stack
`CommandCost` objects and a lazy stack area iterator. Generic landscape commands
retain the complete native `Command::Do` lifecycle. Rust helpers call the
integrated clear-square, flooding/neighbor, region-cache and economy owners directly.
Shared restoration retains class/accounting/RNG order; docking calls its Rust owner.
Shared command/depot/environment services replace touched rail/town/industry
copies. Named generated layouts, aborting unused fixture slots and actual
production table checks cover the boundary; no native world borrow spans reentry.

`python3 tools/migration.py verify --jobs 2`, default `water-commands` and existing
water traffic/mutation/reload scenarios compare semantic saves and require
restoration/clear witnesses. The console fixture compares results/parameters,
map/depot fields, infrastructure, docking membership and RNG across both axes,
all lock directions/parts, occupied/bankrupt queries, force/partial clearing,
editor/desert and restoration inputs. Pool exhaustion, construction under bridges,
water objects, arbitrary NewGRF/legacy layouts and GUI remain #156 coverage gaps.

### Tunnel and bridge commands and ownership (#293)

Rust (`tunnelbridge_cmd.rs`) owns complete bridge/tunnel construction, removal
permission and clearing, ownership, terraform and build-above policy, including
ordered replacement, partial effects, reservations and infrastructure updates.
Header-named modules share bridge prices/foundations/heights, length and track
helpers, and source constants. The hidden tunnel end tile is one process-wide
Rust atomic, shared with GUI readers; no component world borrow crosses reentry.

C++ keeps canonical map/spec/pool storage, rendering, vehicle entry and native
stack command results. Direct typed noexcept services retain shared operations;
ordinary play reaches no throwing script VM or save service. Bridge height calls
Rust foundation directly, including existing station/water/object consumers.
Slope/z has one canonical shared observation. Tunnel exit terraform uses the
integrated object scratch owner's stable index across recursive clears.

Default `roads-commands` and its reload compare all bridge transports/both axes,
road/rail tunnels, test/execute/query (including AI), replacement, ownership,
refusal, occupied clearing, bankruptcy, autoslope, recursive exit terraforming,
hidden failure end tiles and price thresholds at 25/50/100 tunnel tiles.
The existing default object probe checks scratch growth/restoration. Live train
re-reservation, rectangular-map tunnel intersection choices, external NewGRFs
and legacy layouts remain #156 coverage gaps. PR #352 records checks and limits.
### Object construction, generation and HQ lifecycle (#295)

Rust (`objects.rs`) owns build/area commands, clear and ownership policy,
terraform/build-above, HQ growth/acceptance/production, colour updates,
object tile loops and generation including lighthouse/transmitter searches.
It also owns object NewGRF variables, nearby/sister queries and animation triggers;
native interpreter, badge, override and generic animation-frame services remain.
Process-wide wrapping type counts and ordered cleared-area command scratch
have one Rust owner. Tunnel clients retain indices over nested append, and
industry transactions snapshot/restore the same scratch at the original points.
C++ retains the native object pool/specs/map/save/render records, stack command
results and typed shared services; original bodies remain portable-only.

Generation progress cancellation returns before native rethrow. Other leaves
are synchronous noexcept calls. Borrows end before callbacks; HQ passenger/mail
production shares one random word and lazily discovers stations. Command calls
use the shared `command::nested` lifecycle. Command, finance, clearing and water
restoration reuse the integrated shared owners; ownership enters from and
redispatches through the shared Rust dispatcher (#333). Named ABI
metadata and typed abort defaults cover the object table and consumed records.

Default `objects` semantic scenarios cover four climates/borders, HQ relocation,
cargo, owned-land area construction, statues, ownership, clearing back to
canal/river/sea with water redispatch, ID reuse and afterload
counts. Compact unchanged-body probes cover count wrapping/destruction, scratch
growth, cancellation, query availability/register/nearby/count decisions and callback/RNG
ordering, including unmasked `SB` refresh and reentry. External NewGRF/legacy,
autoslope and generation branches remain tracked in #156.

### Shared tile ownership dispatch (#331)

Rust owns the general `ChangeTileOwner` tile-kind dispatch and directly calls the
integrated rail, road, station, water and industry owners. Native clear, house,
tree, void, tunnel/bridge and object handlers remain separate typed leaves; the
original general dispatcher remains portable. Company transfer/control/buy and
native landscape/station entries own call-local constructed costs and the full
canonical road context. Nested station removal returns before scratch reuse;
AI startup still returns to its native outer frame before it can throw.
The default `water-commands` scenario compares captured ownership errors,
restored-tile transfer/bankruptcy, native objects, depot bankruptcy, map state,
both water balances, docking and RNG. This is shared boundary maintenance, with
no new subsystem ownership or retirement completion claim.
The default `ownership-dispatch` scenario builds company-owned road stops and
waypoints, level crossings, rail stations, an airport, dock, buoy, lock, ship depot,
tunnels and bridges, and sets industry founder/exclusivity, then runs a merger and a
bankruptcy through `ChangeOwnershipOfCompanyItems` (#156), with required witnesses.

<!-- Subsystem: Rail and road infrastructure. Add a new entry at the end of its group, not of the file. -->

### Rail signal blocks and fixed work sets (#219)

Rust owns complete signal-block exploration, immediate signal update/propagation, first-segment
classification and all three fixed 64/256/128 work sets, including counts, sticky overflow
flags, private direction tables and last owner. State has process lifetime under the game-state
mutex; loading adds no new reset.

C++ keeps public signal constants, shared map/pools/spatial queries, commands and rendering.
Native assertion configuration remains authoritative.

The default `signals` simulation compares movement, commands, pending state, reservations,
reload and ownership barriers. Real tbd/global propagation overflow, threaded play and arbitrary
mod layouts remain #156 limits.

### Rail track, depot and signal commands (#265)

Rust (`rail_cmd.rs`) owns single-track build/remove and drag, depot build/removal,
single/drag signal commands and autofill, rail conversion, and rail tile clear,
owner-change, autoslope/terraform and build-above policy. Test/execute partial
effects, error precedence, costs and wrapping remain source-ordered; none draws RNG.
Original bodies remain portable-only.

C++ retains stack `CommandCost` slots, nested commands, crossing/depot map writes,
PBS, and the conversion iterator/ordered affected-train list. Shared command checks
serve rail, fleet and industry; canonical road/rail map services supply reads.
Shared services also provide slope settings, `DoClearSquare` for trees/rail,
reservation release/path requests and one safe-waiting service for train/YAPF/rail.
Native enum return types retain their widths through transparent Rust carriers.
No owner borrow spans reentry. Rail tile loops, track status and drawing stay native.

`python3 tools/migration.py simulate signals-rail-commands --jobs 2` compares
command results, semantic saves and required `rail_cmd.*` witnesses: obstacle drags,
crossings/depots/slopes, signal cycling/conversion/copy/autofill through a bridge,
conversion/reroutes, clearing/terraforming, company transfer/deletion and reservations.
Coast/half-tile water and PBS path-end removal remain #156 gaps. PR #270 records
exact-head checks and outstanding full-simulation, speed and CI gates.

### Road track, depot and conversion commands (#266)

Rust (`road_cmd.rs`) owns road removal permissions and slopes, single and long
build/removal, depots, clear/owner/nearest-town updates, terraform, conversion and
build-above. Portable originals remain. Native road-type allocation, drawing,
track-status/vehicle-enter and tile loops stay outside this command port.
The boundary reuses shared command checks and native stack costs, road/rail map
leaves, company counters, depots, crossings and the rail conversion cursor.
Rust road/station helpers share the ordinary nesting, test/execute, rating and
finance lifecycle. Outer native town, founding, controller and tile-loop frames
provide one stack scratch and two result slots for direct Rust owner calls.
Shared Rust also owns road prices and tile clearing; native facades retain portable
originals and typed storage/error leaves. Town/tram builds enter the Rust road
owner directly; ground removal, nearest-town and bankrupt rail calls are direct.
Town authority and rating policy call the integrated Rust owner directly.
Crossing reservation reads/writes now have one canonical definition shared with
trains. The canonical `MayHaveRoad` predicate preserves rail-station/head guards.
The nearest-town map scan bundles only its reached original predicates per tile.
No world-state borrow spans nested commands or vehicle/town callbacks.
The default `roads-commands` and `roads-commands-reload` scenarios compare test and
execute outcomes, costs/extra cash, semantic maps, company/town state and RNG,
including drags, one-way/tram coexistence, reserved crossings, stops, bridges,
depots, slopes, town removal, owner changes and low-funds removal.
The fixture declares builtin-like road-type inputs; external NewGRF programs and
legacy save variants remain corpus gaps. PR #290 records validation and speed.

### Road stop bays and drive-through entries (#354)

Rust (`road_stop.rs`) owns each RoadStop's status byte and the shared
drive-through entries: bay admission and freeing, entrance status, `GetByTile`,
continuation, both `Enter`/`Leave` levels, joining (alone, parent, join, merge)
and splitting (last, head, tail, split with rebuild), `Rebuild`,
`CheckIntegrity`, destruction and both `AfterLoadRoadStops` passes. One Rust
allocation per shell holds the status and entries pointer; the BaseEntry stop
owns the entries, allocated and freed where the original does. `status` stays a
native reference alias for the ROAD descriptor; `GetEntry` is a const reader.
The road controller, station commands and road YAPF call the owner directly.
C++ keeps the shell, pool, `xy`/`next` links and station stop lists
(`OpenTTDRoadStopServices`), two compound vehicle reads, the tile hash and
`GetNextRoadStop`, whose only caller is the native YAPF area leaf. Native
vehicle entry, PreDestructor, cache check and load call typed entries.
`stations-commands-road-stops` and `-road-stops-reload` compare every stop's
status and entry length/occupancy through joins on both axes, occupied merges,
interior/head/tail/last removal, bay admission and reload, and require every
`road_stop` branch (`integrity` in desync runs). Articulated bay refusal and the
unreachable `include` duplicate remain #156 gaps.

### Waypoint construction and command policy (#366)

Rust owns deleted-waypoint reuse in pool order, rail/road axis selection and tile
validation, complete rail/road/buoy construction, buoy removal and waypoint name
uniqueness, rename and sign-move policy. It calls integrated station joining,
expansion, road-cost and removal policy directly, and water restoration directly.
C++ retains canonical Waypoint/BaseStation shells, spec allocation, map setters,
presentation and caller-stack `RailStationTileLayout`/iterator and command costs.
No native world borrow spans callbacks; serialized synchronous leaves are noexcept.
The last native road-infrastructure caller is replaced and its facade/export removed.

`python3 tools/migration.py rust-checks` covers Cargo4. Focused semantic evidence:
`python3 tools/migration.py simulate stations-commands-waypoints
stations-commands-waypoints-reload stations-commands-commands
stations-commands-reload water-commands` compares commands/errors, infrastructure,
map/STNN, names/reload and RNG, with required default waypoint branch witnesses.
Both axes, adjacent/distant joining, replacement, deleted reuse/equal-distance ties,
occupied refusals and buoy water-class restoration/bankruptcy are exercised.
External station/road-stop GRFs, exhausted pools/spec lists and legacy saves remain
coverage gaps under #156. Independent review and exact-head Full are root's gates.


### Animated-tile scheduling and shared animation control (#385, draft)

Rust owns the process-wide queue, scheduling/control, six typed bindings,
trigger traversals and dispatch; existing owners call it directly. Native map
frames/bits, road records, pools/specs/PSA and stack resolvers remain. Borrows end
before callbacks. Generated visitors include all six version8 fields.
ANIT retains native Sl* frames, nested active pointers, append/resize and partial
publication. Shared station lookup/membership, IsAirport and orthogonal iteration
preserve original callees; unused native animation table/facades are portable-only.
`python3 tools/migration.py simulate objects-gaps objects-reload effects-controllers-1`
requires bindings240/144/90, traversal72, airport5, visits46/21/9, zero-frame road
insertion, six actual ANIT load/error processes and positive immediate reload of
queue/frame/bits/road records/RNG. Airport SB retains unmasked OR.
The56 keyed sound/sentinel cases delegate real sound/setter/dirty calls (20 each),
comparing frames, records, ordered queue and sequence;36 version cases require
six actual version8-0100 diagnostics with valid native GRFConfig lookup.
Five rejection cases compare RNG/callback/queue outcomes and retain mask mutation.
Area/spec/member/mask reads and initial-RNG-before-area are source-compared, not
independently traced; this precise dynamic limit remains on #156 under root's
revised evidence scope. Thirteen successful owner pairs require two live survivors
and exact swap/deferred order; the catcher pair requires exact queue append.
Purpose cost348 implementation/81 native evidence/550 tooling including CMake
versus unchanged415 basis;57 retired scaffolding lines add no behavior. Root accepted
the reviewed cost; scoped conflict review/Full remain. Historical/pre6/TTO saves, external GRFs,
GUI threads and raw>=64 outside pinned Linux compilation remain #156 limits.

<!-- Subsystem: Vehicles, pathfinding and orders. Add a new entry at the end of its group, not of the file. -->

### Effect vehicles

Rust owns all twelve controllers, movement tables and private animation bytes (#101/#107). Each
C++ EffectVehicle shell owns one zero-created opaque state, also on indexed load; RAII destroys
it on ordinary deletion and pool cleanup.

Copied call-scoped observations and immediate scalar writes keep original mutation order. Rust
calls common Random/map and effect-specific sound/viewport noexcept leaves
directly, plus the Rust animated-tile owner; the native leaves cannot reenter Rust. The immutable shared service table is borrowed only
until the controller returns (#299); no full table is copied.

`python3 tools/migration.py simulate effects` compares save chunks across subtypes, expiry,
reload and natural creation. Viewport pixels, audible output, full legacy fixtures and every
caller remain limits.

### Disaster scheduling and vehicles

Issue #103 moves all fifteen subtype controllers, eight initializers, eligibility,
countdown/reset, industry construction reset and square-clearing policy, movement, shadow/rotor
updates and both target-release hooks into Rust. Rust owns each shell's state, flags, image
override and destroyer target, plus the persisted global delay. Original bodies compile only in
portable builds.

Copied observations and direct noexcept shared services preserve original ordering and shared
RNG draws. Private scalar addresses remain stable through modern and legacy save staging; flight
helpers borrow only the flags scalar for their call. World clones share an immutable
service-table borrow tied to the Run/future lifetime (#299), ending on destruction,
including cancellation. Production uses the process-wide const table; Rust fixtures
own separate tables through destruction, and native partial tables stay in scope.
Callbacks may mutate their separate context, never the borrowed table. #377 adds
a borrowed typed occupancy context: movement resolves each shell once, updates
the Rust index and calls the base viewport leaf; clear retains native costs.

The default `disasters` simulation checks scheduling, crashes, removal, industry reset,
submarine movement and reload. Legacy save fixtures, viewport pixels, sound output, allocator
failure timing and exhaustive NewGRF combinations remain limits.

### Road vehicle control and private state

#121 moves tick/day control, movement, blocking/overtaking, reversal, station/depot transitions,
crash expiry, servicing and cache policy to Rust. Each shell owns seven scalars and the
canonical path; #124 YAPF writes that same path. Modern, split-vector and TTD/TTO adapters
retain field widths through C++ staging.

#199 adds shared typed vehicle and GroundVehicle services (`vehicle.rs`, `vehicle_ffi.h`,
`vehicle_services.{h,cpp}`). No owner, path or world borrow spans callbacks, reentry or
deletion.

#311 shares one lean max-speed calculation across road control and native queries. Road
control supplies that limit between the shared native visual-effect gate and tail, then
observes movement/speed inputs again after effects. Default native/ABI checks cover the
front-state/curve/bridge domains and pre/post-effect ordering; train callers keep the shared policy.

#330 folds tick-front capture into the native tick entry. Named read-only scalar places
are loaded at the prologue branches, then discarded before controller callbacks; counter
writes stay native and wrap explicitly in Rust. Default native fixtures cover real address
identity, fresh loads, status resets, front/nonfront policy and counter boundaries.

The default `roads-` simulation compares acceleration, servicing, path, RNG, crash expiry and
reload. Legacy saves, custom NewGRFs, articulated/tram turns, sounds and viewport pixels remain
limits tracked by #156.

### Ship controller and private state (#146)

Rust owns the complete ship tick/movement, locks, acceleration, rotation/reversal,
track/depot/destination policy, economy-day/service/cache/build initialization, and
nearest-depot region BFS with ordered pool selection. Four canonical scalars belong to each Rust
ship owner; #119 remains the sole path/search owner. C++ keeps shared Vehicle/map/pool storage,
renderer bounds/sprites, allocation, Money accounting and modern/legacy save staging.

Entries are synchronous typed calls without task/future/mailbox allocation or operation
dispatch. Services are individually named `noexcept` functions; narrow getters fetch only used
fields, typed results carry positions and search choices, and one reusable game-thread Rust
set/deque owns depot scratch. Tile/depot, orders/loading, viewport, breakdown, effect/sound,
cache/YAPF and NewGRF callbacks remain direct: none can unwind a script VM or save/load error
during ordinary play.

`python3 tools/migration.py verify --jobs 2` includes the four Cargo checks and native ABI
checks for scalar widths, native C++ transient coordinate narrowing, direct callback order and
nested owner mutation. `python3 tools/migration.py simulate water --jobs 2` requires the
candidate's `ship.*` witness branches in the water corpus for locks, aqueducts, rotation/reload,
service, buoy/loading, depot restart, and moving build/sell/ID-reuse owners. Legacy save
versions, viewport pixels and arbitrary custom NewGRF combinations remain coverage limits.

### Rail vehicles and controller state (#130)

Rust owns train consist/cache/curve/speed policy, both tick passes, movement, reversal, crossing
control, crash/deletion, servicing and day handling. It also owns reservation extension/rollback
and temporary order lookahead; #122 retains YAPF search ownership. C++ train shells hold one
Rust allocation for the controller state, while pools, map storage and shared services remain
native.

Typed calls preserve the original callback and RNG order, and no owner borrow crosses reentry or
deletion. The default rail simulation and native checks cover movement, reservations, signals,
service, commands and reload. Arbitrary NewGRFs, legacy saves and unexercised controller
branches remain limits tracked in #156.

### Aircraft controllers and airport movement

#136 owns both tick passes, movement/speed/flight, shadow/rotor, FTA traversal, terminal/helipad
reservations, service/range, crash cleanup and airport lifecycle in Rust. Fixed allocations own
private counters/cache and airport masks; trivial C++ save aliases preserve their addresses.

#237 borrows shared vehicle/map/settings/RNG services directly. No owner borrow crosses
ProcessOrders/loading, depot/refit, animation, crash or deletion. Runtime dimensions are
captured per entry; wrapping arithmetic/RNG order remain.
#325 gives the controller one stack descriptor of named canonical native x/y/z,
speed, Direction and subtype addresses; position alone acquires only subtype/z.
Raw reads end before callbacks, with fresh loads after native mutations. Geometry,
map, speed/dirty, direction/new-position, NewGRF cache/image, sound and crash-marking
services preserve the shell; pool growth moves slots, not vehicles. Descriptors end
before airport state handlers/depot work, deleting crash cleanup, replacement or
load/reset. Standalone flight/disaster entries retain their narrow services.

The default `aircraft` and `disasters` scenarios exercise traffic, service, removal, crash and
reload. Real NewGRF/refit/airport rotations, threaded GUI, legacy-save fixtures and presentation
remain gaps tracked on #156; these checks do not establish full subsystem equivalence.

### Fleet grouping and replacement lifecycle (#147)

Rust owns group metadata/names/ordered children, group/company statistics and engine counts,
renewal rules/links and company renewal heads, plus the pending tick-end map. It runs every
group command, traversal/statistics/livery update, renewal-rule command, replacement
eligibility/refit choice, speculative build, rearrangement, rollback, execution and the
ascending-VehicleID drain. Original bodies compile only in portable builds.

C++ keeps pool identities, group numbers, GUI folding, save adapters and `Vehicle::group_id`.
Rust reads/writes that shell field through services; no owner borrow spans a nested command.

Three named-slot noexcept tables (ABI 393-396, with owner prefixes 390-392) serve Rust; native
stack CommandCost objects keep AddCost first-error and Money saturation, and original assertion
sites run in assertion builds.

Checks: `python3 tools/migration.py verify --jobs 2` and the default `fleet-` simulation. NewGRF
refit/attach callbacks and articulated parts remain #156.

### Common vehicle transactions and nonrail construction (#314)

Rust (`vehicle_transactions/`) owns the `vehicle_cmd.cpp` commands: build, sell, refit, clone,
start/stop, mass start/stop, depot sell/replace, send-to-depot dispatch, naming and service
intervals, with vehicle-set selection and depot/sort lists. It also owns the road vehicle and
aircraft constructors (`construct.rs`, shadow and helicopter rotor included), articulated
assembly and its consistency check. Speculative refit builds restore both RNG streams; clone
cleanup sells partial consists. Commands nest through `command::nested` and call ship, orders,
cargo, fleet, road and aircraft owners directly. Original bodies compile only portably.

C++ keeps pools, shells and their field stores, specs/NewGRF callbacks, unit allocation,
native stack `CommandCost`s and UI leaves. Rail construction, moves and sale are #315 (below).

Checks: `python3 tools/migration.py verify --jobs 2` and the default `fleet-` scenarios. The
`transaction-build-road`/`-aircraft` and `transaction-clone` scenarios require
`vehicle_transactions` witnesses for road, aircraft, rotor, RNG restoration and clone join
failure. Articulated road vehicles (NewGRF only), wrong-depot road types, airports refusing an
engine and clone build failure after the front part are #156 gaps.

### Rail construction and depot consist transactions (#315)

Rust (`vehicle_transactions/rail.rs`) owns the `train_cmd.cpp` bodies from `GetFreeWagonsInDepot`
through `CmdSellRailWagon`: wagon, engine and rear-head construction, free-chain joining and
`NormalizeTrainVehInDepot`, backup/restore lists, arrangement, dual-head and subtype
normalisation, attachment/limit validation (an error leaves the chain split until the caller
restores, as the original), head promotion/demotion with order, configuration and unit transfer,
and rail sale. `CmdMoveRailVehicle` is a facade; build, clone and fleet autoreplace nest the Rust
body directly. Original bodies compile only portably.

C++ keeps shell allocation and source-ordered field stores (including the rear head's `Money`
halving), the attach callback, unit-ID pools and presentation leaves. Chain links stay native and
change only through `SetNext`; rail moves call the Rust cargo capacity owner directly. Shared
leaves in `vehicle_services.h`/`engine_services.h` serve the train controller, cargo capacity and
this owner. `vehicle_base.rs`/`ground_vehicle.rs` hold the
GVSF values and subtype helpers.

Checks: `python3 tools/migration.py simulate fleet-transaction-rail --jobs 2` requires the
`vehicle_transactions.rail_*` witnesses: free-chain join, normalisation, front demotion and head
transfer, new front, move and sale length refusals with restoration, multihead build, dual-head
relocation and sale order transfer, plus semantic chain/unit/group/order/money dumps and ID reuse.
NewGRF attach-callback results, the GRF-version branches and order-pool exhaustion are #156 gaps.

### Engine lifecycle and availability (#220)

Rust owns each engine's lifecycle scalars and name, the process-wide aging-stop state,
startup/reliability, daily/monthly preview and availability policy, variant synchronization and
control/name/visibility commands. Native Engine shells expose canonical scalar aliases;
temporary owners survive relocation and TTO replacement. Original bodies compile only portably.
Pool/configuration/spec, NewGRF/articulated, company/type availability and presentation remain
native through typed synchronous noexcept services shared with fleet/company consumers. No owner
borrow spans a callback; script events enqueue without running a VM. RNG draws, narrowing and
command/effect order retain the original behavior.

Modern ENGN scalar descriptors address aliases directly; only opaque names use a native string
scope, preserving partial decoding on failure. Historical loaders address canonical prefixes
without live mirrors. Panics/environmental exceptions abort; native save/load errors remain
outside Rust. `python3 tools/migration.py verify --jobs 2` checks Cargo and native
ABI/lifetimes. `python3 tools/migration.py simulate engines- --jobs 2` requires default
preview/acceptance/join/no-build, commands, retirement/sync, never-expire and 2050-cutoff
witnesses and compares modern reload. The identical adapter declares variant and cutoff spec
inputs for unchanged reference bodies; branch results, command costs and semantic saves must
match. Counters record native entries, service crossings and returned bytes, disabled for
timing. External NewGRF programs/retire_early, ENGS and TTO/TTD fixtures, legacy preview
conversion and threaded GUI/presentation remain #156 gaps. #248 records the fixing reviews,
source comparisons and final-head CI/speed results.

### Orders lifecycle ownership

Issue #138 / PR #176 moves order storage, links, backups and consist timetable/unbunching state
into Rust, including traversal, command edits, sharing, depot execution and backup lifecycle.
C++ retains pool identity, vehicle shells, command execution, VM callbacks and save/load
adapters; original bodies remain in portable builds.

Typed synchronous services preserve source ordering. Rust releases owner borrows before
callbacks, commands or deletion, and native exceptions remain in C++. The default orders
simulation and existing tests cover commands, sharing, backups, reload and reentry. NewGRF
callback variants and legacy saves remain coverage limits tracked in #156.

### Ship YAPF searches and canonical path cache

Rust owns both region and track searches, arena indices/parent chains, open and closed
membership, the original binary-heap tie/removal rules, destination estimates, region corridor,
two attempts, random lost paths and cache extraction (#119). Search arenas/maps/heaps retain
process-wide capacity (#306); active searches own storage, with no cached world pointers.
Each Ship owns one opaque Rust path
with copied controller access; modern VEHS stages one byte across C++ load errors and commits it
on unwind, while the pre-struct-list loader retains its reversed path order.

C++ keeps ship/world pools, track following, station/dock queries, engine/settings observations
and canonical map arrays. No ordinary throwing/reentrant world callback occurs in this search;
save errors remain outside Rust frames.

The existing `python3 tools/migration.py simulate water --jobs 2` corpus compares all chunks for
ferry manual/cargodist, canal/lock loss and recovery, aqueducts, depot reversals and
reference-produced live-path reload. Arbitrary maps/NewGRFs and complete legacy saves are not
exhaustive.

### Rail YAPF search, caches and reservation

[#133](https://github.com/fetch-laboratories/openttd-rust/pull/133) moves all four searches,
node/segment arenas, exact-order heap, costs, lookahead, limits and reconstruction
into Rust. Rust owns six specialization-specific cache banks, rail-change
invalidation, reservation traversal and ordered signal rollback. C++ retains
the train controller, shared track follower, PBS and canonical world services.
Explicit platform/waypoint station triggers return to C++ with borrows released;
waypoint track reservation also retains the original synchronous PBS station
triggers while search/reservation borrows remain active, before its second trigger.
Original search bodies remain portable-only; diagnostic dumps use temporary
views and the original format, with no canonical C++ search mirror.

The default `padhattan` and `rail-reservation` simulations exercise cache reuse, reversal,
depot/safe-tile search, reservation, busy targets, rollback and reload under both 90-degree
policies.
Native unchanged-base/heap probes cover finite limits, live cost invalidation
and ordered opposing PBS signal restoration absent from the supplied network.
They do not prove full rail-cost semantics for arbitrary maps, NewGRFs, legacy
saves or every signal family. Controller reservation extension belongs to #130.

### Road YAPF search and path construction

Issue #124 moves track/depot searches, exit-direction keys, node arena, lookup
maps, exact-order heap, segment traversal/cost, heuristic, limits and intermediate
fallback into Rust. Reconstruction and station-area trimming write #121's sole
canonical path. Original specialization/node types compile only portably.
C++ retains map/pools/settings, station/stop observations and `CFollowTrackRoad`
behind copied scalar `noexcept` leaves. None runs scripts, mutates/reenters the
search/road owner or throws in ordinary play. Short owner access ends before
leaves. Neither search draws RNG; wrapping, exit-key replacement, strict ties
and follow-before-limit timing retain source behavior. Panics/OOM abort.
Arena/maps/heap retain process-wide scratch capacity (#278); active searches own
it, with no cached native pointers. Inputs/destinations/counters reset per call.
#288 shares one tile observation across segment cost/destination/depot checks.
Endpoint predicates invalidate on advance; startup and >4096 exits remain
unevaluated until needed, without stop-cost lookups. Only accepted continuations
read ordered current/next ground-vehicle center Z, vehicle speed fields and
old-tile follower limits; arithmetic stays Rust and Follow is never repeated.

`python3 tools/migration.py simulate roads --jobs 2` compares plain/desync state,
station trimming and competing penalties. Native tests compare heap/origin masks
and depot transcripts with unchanged C++ at node limits 0/1/2/7 and cost bounds
0/70/71/100; a transient trace checks rejected/accepted queries, costs and long
segment cache invalidation. Native ABI checks cover every field. Arbitrary maps,
NewGRFs, articulated/tram, waypoints, exhaustive stop/loop/segment branches and
legacy saves remain #156 gaps; shared follower logic remains native.

<!-- Subsystem: Stations, cargo and economy. Add a new entry at the end of its group, not of the file. -->

### Station cargo-list queries

Issue #51 reuses the Rust ScriptList owner. Rust owns the four selector/filter rules, pending
run keys and unsigned 32-bit totals, positive flush decisions, add-versus-set merges, per-origin
cumulative-share decoding, and query-plan selection (waiting/all versus equal_range, planned/all
versus find). C++ keeps station/cargo validation, pool/GoodsEntry access, HasData checks,
packet/flow iterators and scalar extraction.

A caller-owned 16-byte collector (uint32 amount/previous, uint16 station IDs including 0xFFFF,
byte selector/finalized) is asserted in C++ and ABI layout ID 18; Windows x86 uses explicit
cdecl. Rust borrows the destination only during feed/finalize, and the destination outlives the
collector; no world pointer, STL/VM layout, iterator, callback or C++ exception enters or
survives a Rust call. Fresh collector totals are bounded by packet-pool/origin limits;
modification tokens use the shared owner's wrapping arithmetic.

Evidence: the unchanged stationlist script (24 constructions, 41 output rows), both scripted
suites, the upstream unit inventory, and the bounded cargo mode of `python3
tools/script-list-comparison.py` (pinned loop bodies, portable and Rust feeds, FlowStat origins,
wrapping, sentinel keys, repeats, reentry, a C++ exception between feeds).

### Station cargo service

Issue #125 moves the ordered loading queue, station service counters, acceptance mask and cargo
rating/pickup/speed/age/waiting/fraction metadata into one opaque Rust owner per BaseStation.
Native scalar references and the queue facade share it; ordinary/indexed construction,
retirement, reuse, STNN/STNS and legacy fixups share it without persistent native mirrors.
Packet, flow, link-graph, map and geometry storage remain C++.

Issue #214 replaces numeric dispatch with borrowed typed noexcept tables (ABI 110-115), narrow
observations (500-515), and caller-owned native iterator/container frames. Portable bodies
remain unchanged.

`python3 tools/migration.py simulate stations economy` uses the existing road/CAPY inputs for
loading queues, fractional admission, acceptance loss, full/gradual loading, rating
expiry/decay/capping, stale-link refresh/removal/restriction/release and active reload. Legacy, NewGRF and refit
gaps remain in #156.

### Station construction, joining and removal (#267)

Rust owns the station command policy for rail platforms and waypoints, bay and
through road stops, airports, docks and oil rigs, plus sign/name/owner, terrain,
bridge clearance, joining, deletion/reuse and docking-neighbour decisions.
`water_cmd.cpp` docking eligibility and registration belong to this owner.
Original command bodies compile only in portable builds. The existing #125/#214
station state remains canonical; there is no second station owner or save layout.

C++ retains shell/spec/road-stop allocation, NewGRF resolvers, spatial indices,
rendering/description and the hot tile-loop/animation/vehicle-entry/track/slope
services. Entries use borrowed typed synchronous tables (ABI 1060-1072), shared
Rust command pre/post checks, prices, clear-square/water restoration and native
result slots. No ordinary-play script VM or save/load exception crosses the boundary.
Rust ends owner access before reentry and calls owned aircraft/station/reservation
policy directly. Reservation lookahead saves the existing layout-checked order
value in each Rust stack guard; native pool/container allocation follows the original.

`python3 tools/migration.py simulate stations-commands aircraft-controller-removal
water-structures-mutate --jobs 2` compares command costs/errors, RNG, STNN/STNS,
map, town and company state, including partial removal, joins, delete/reuse and
modern reload. The station module includes unchanged-reference private callbacks
in fixture executables and requires `station_command` witnesses in default CI.
Default #320 cases require removed-destination path clearing, `TileAddWrap` border
refusals and type-63 overbuilding through `AllocateRailType`. Retargeting uses virtual
`SetDestTile`; shared native traversal serves stations and flooding. Builtin tram,
airport noise and reload cases remain required. External NewGRF programs, legacy
and unusual occupied/reservation branches remain tracked by #156.

### Cargo storage and movement (#139)

Rust owns packet fields, ordered station/vehicle lists, count/age/feeder/action/ reservation
caches, cumulative flow shares and origin maps. It runs packet operations, cargo
movement/staging/rerouting, live-flow reconciliation, capacity redistribution and autoreplace
cargo transfer. Stable arena slots and linked ordering give constant-time list operations;
destination endpoints bound station lookups to the selected queue. Freed slots are reused; arena
capacity is retained. C++ retains packet pool identity/allocation/hooks, payment/delivery
(#117), loading policy (#125), link-graph computation (#74), world access and thread join.
CAPA saves the repr(C) packet in place; STNN/VEHS/flow adapters export call-local containers
or scalar-key readers; pointer fixups
and save errors finish outside Rust. Payment getters reenter after owner borrows end;
ConsistChanged runs after Rust returns. Portable bodies stay.

Checks: `python3 tools/migration.py verify --jobs 2`, and the default cargo-storage, economy,
station, Padhattan and aircraft scenarios. Active CAPY scenarios check saved packet
references/actions, aging, feeder credits and forced policies. Rust tests cover partial
movement, payment getter reentry, same-list rerouting and capacity changes. `python3
tools/cargo-storage-comparison.py` checks unreachable pool-limit Split failure against the
unchanged pinned body. Legacy conversions and complete refit/autoreplace transactions lack
additional play witnesses (#156).

### Cargo payment and delivery

Issue #117 moves CargoPayment state/lifetime, final/transfer payment and settlement, industry
acceptance, statistics/payment ordering, destination deduplication and the complete production
flush into Rust. C++ retains pool shells, canonical world and packet storage,
loading/reservation, subsidy checks, NewGRF resolution and UI.

ABI IDs 49-51 and 60 describe cargo observations, saved fields, synchronous services and the
repr(C) payment, which CAPY saves in place through `CargoPayment` reference aliases.
Cargo/refusal/production/randomisation/animation and vehicle-sound resolution uses the NewGRF
evaluator, without script VM entry or payment/destination reentry; wrappers are noexcept. Money
saturates arithmetic but retains native shifts; callback unsigned products, BigMulS narrowing,
stockpile limits and ordered randomisation/animation are explicit.

The default `economy` simulation compares delivery, transfer, active CAPY reload and partial
acceptance. NewGRF resolver execution, legacy CAPY files and every industry layout are not
exhaustive in the semantic corpus; the narrow fixture supplies callback results.

### Company finances and economy lifecycle

Issue #137 moves cash/fraction accounting, loans, histories, rating/value,
construction budgets, maintenance/interest/year rotation, bankruptcy/recovery,
ranked offers, creation/deletion, acquisition and the complete ownership-transfer
traversal into Rust. Rust also owns periodic scheduling, prices/multipliers,
inflation/recession, score parts and the company tick cursor. Selected bodies
compile only portably except StartupCompanies/InitializeCompanies: their native
timer/reset control remains, and InitializeCompanies writes Rust's tick cursor.
Identity, settings, networking, UI, AI and shared world services remain C++.
One Rust allocation per CompanyProperties holds finance/history fields with C++
field lifetimes and stable addresses; economy/prices/scores have process lifetimes.
#192 makes every entry a named synchronous call taking borrowed designated tables
(`OpenTTDCompanyFinanceServices`, `OpenTTDCompanyServices`) of typed noexcept
services; facades resolve owners and settings once. Pool scans filter owner, then
type, before detail reads; tile-owner changes, deletions, allocation, the nested
service-interval command, the bankruptcy Post and `StopAI` run directly. Only
`AI::StartNew` (script memory policy can throw before ScriptInstance's catch) and
the `CCA_NEW_AI` Post return through caller-owned stack frames (startup, control,
tick, competitor timeout). ABI 210-214 and 400-417 check layouts.
`python3 tools/migration.py simulate companies economy` compares all chunks/RNG;
Native CTest covers lifetimes, network deferred deletion, posting
order and abandoned startup frames. Subsidies, exclusive rights, goals/story
pages, hostile purchase, multiplayer runtime and NewGRFs remain evidence limits.

### Link graph job computation

Issue #74 moves demands, both MCF passes, cycle elimination, path lifetime and flow mapping into
Rust. Rust owns copied graph/settings, private demand/edge annotations, a reusable path arena
and ordered cumulative-share maps. C++ keeps worker launch/join and abort flags, LGRP/LGRJ
save/load and the existing station flow join.

ABI IDs 34-37 cover scalar node/edge/settings/share records. Its sole callback reads the job's
atomic abort flag and cannot throw/reenter. C++ allocations happen outside Rust.

Evidence: the unchanged regressions/comparisons and `python3 tools/migration.py simulate` with
manual/cargodist road saves, symmetric/asymmetric scalers, boundary
distance/size/accuracy/saturation settings and reload from a reference snapshot with outstanding
LGRJ records (GLOG history is reset only in prepared road inputs so both loaders log their
revision). Graphs, allocator failures and abort timing are not exhaustive; resource exhaustion
timing differs with the allocation strategy.

### Link graph scheduler ownership (#373, #380 via #391)

One process-wide Rust owner holds ordered queued/running pointers and controls
rotation, spawn/join/requeue, pool-wide date shifts and pause policy. Native pools,
job snapshots, worker launch/join, abort and acquire/release completion publication
remain unchanged; existing job computation is not counted as new retirement.
A constant-initialized member table supplies the graph owner and 18 typed native leaves; Time is
one narrow entry observation. Visits and cargo admission call queues directly in Rust. Shared
graph owner/ID/validity/get helpers serve the job reader and existing consumers.
No queue borrow crosses native callbacks, including synchronous fallback and deletion.
LGRS keeps two native scoped vectors with the original reference field names/order.
Exported spans are copied without callbacks/yield/reset under game-state serialization
and discarded before SlObject. SlSaveChunks precedes asynchronous compression.
Native RAII publishes partial staging on return/unwind; recursive SlNullPointers uses
and publishes the same active vectors before returning, preventing stale restoration.
Default symmetric/asymmetric cargodist reloads require both queues, LGRP/LGRJ refs
and queue/spawn/join/requeue/reload/import witnesses. Their module compiles unchanged
pinned scheduler bodies for worker/pause, duplicate rotation, recycled-ID, clear and
pool-shift gaps, failing absent completion markers. Native tests exercise ABI,
partial-state unwind and real SlError recursive cleanup. Actual concurrent worker,
multiplayer/GUI timing and historical LGRS versions remain #156 gaps. Independent
source/conflict reviews, root cost disposition and exact-combination Full passed in #391.

### Live link graph contents and mutation (#378)

Rust owns cargo, compression date, nodes and sorted edge vectors in one owner per
native shell. Lifecycle/copy/init, node and edge mutation, supply/demand/location,
restriction/release, merge/scaling, compression, dates and queries run in Rust.
C++ keeps pool IDs/reuse and station membership leaves; Rust owns the scheduler.
Each job deep-copies the same representation; native annotations borrow canonical
station scalars and immutable edge references until the existing worker join.
The computation and flow-application ports are unchanged ownership, not new retirement.

LGRP/LGRJ retain field names, widths/order and legacy matrix/list conversions.
Native SlObject writes canonical scalar addresses and appends edges before decoding;
partial effects remain visible. Legacy matrix staging retains its original scope.
SlErrorCorrupt/SlNullPointers and all exception unwinding remain C++ frames. LGRP/J
have no pointer-fixup handler; graph borrows end before station writes or deletion.
Read-only job access never creates an exclusive Rust borrow during worker execution.

Default evidence extends the station fixture with merge, last/nonlast replacement,
current acceptance, fresh timestamps, compression thresholds, unsigned travel sums,
Scale narrowing and date sentinels against unchanged reference bodies. Required owner
witnesses cover those mutations and real stale restriction/release. Native tests pin
scalar layouts/addresses and distinct job snapshot storage/lifetime. Focused station
stale/removal/reload and symmetric/asymmetric LGRJ reload compare graphs, station flows
and full semantic state. Legacy/corrupt-load matrices and real-thread timing remain #156.

<!-- Subsystem: Towns, industries and subsidies. Add a new entry at the end of its group, not of the file. -->

### Built-in town-name generation

Issue #96 moves all 21 built-in generators, seed selection, English replacements, Finnish vowel
handling, Czech grammatical matching and private constants into Rust. C++ keeps NewGRF routing,
settings and save/load; #246 owns retries/uniqueness.

One coarse call owns complete output behind an opaque result. C++ appends the immutable byte
view with RAII destruction, including on append exceptions; no builder/world pointer, callback
or shared RNG enters Rust.

Evidence: the semantic harness observes actual built-in rendered names through read-only AI logs
and compares every saved field in plain/desync modes. This samples seeds; it is not exhaustive
coverage of 21 * 2^32 combinations, every allocator failure or NewGRF town-name behavior.

### Town growth and lifecycle state

Issue #120 moves town tick traversal, road/build choices, terrain retries, house
selection/placement and expansion into Rust. The partial #148 extension owns town metadata,
histories, ratings and building counts; #256 moves house tile loops and deletion
commands. Native field aliases expose one Rust owner, with process-wide state
under the game-state mutex.

Rust ends owner borrows before commands or NewGRF callbacks that can reenter. The default
`town-growth` simulation and native tests cover growth, layouts, reload, reentry and aliases.
Natural tunnel construction, custom NewGRF callbacks and complete historical saves remain
evidence limits; #148 is not a complete lifecycle port.

### Town founding, generation and names

#246 extends the sole town owner with canonical custom-name bytes and owns founding,
placement/coastal/grid search, initial growth, generation counts/retries, layout, rename and
automatic/custom-name uniqueness. C++ keeps map/pool shells, house specs, rendering/NewGRF
resolvers, spatial/noise/news services and native save decoding. A separate native custom-name
presentation string retains the original `GetCachedName()` reference identity through
rename/reset/re-rename; commands, validation and save authority read Rust bytes. Generated-name
cache stays separate. All descriptor-addressable town scalars/growth fields load directly
through their aliases. Only the opaque Rust name uses scoped native string staging, committing
partial decoded state on unwind. Legacy compatibility scratch remains native. Named typed
services return native enum/scalar widths; allocation resolves the canonical owner once. Borrows
end before growth, rendering, deletion and reentry. Only
`SetGeneratingWorldProgress`/`IncreaseGeneratingWorldProgress` return cancellation to a native
frame; Rust drops uniqueness scratch before native rethrow. Portable original bodies remain
unchanged in non-Rust branches. `python3 tools/migration.py simulate town-founding- --jobs 2`
compares climate/layout, no-city generation, coastal generation, manual/deity/editor commands, exact
cost/error/extra-money/ID, zero-growth deletion/reuse, rename/cache lifetime, name exhaustion
and reload. Default module-owned adapters also compare cancellation against unchanged source,
full-width count clamps/RNG and otherwise unreachable name bytes; missing markers fail. Real
NewGRF, historical saves, GUI threading and final 10,000-attempt fallback remain coverage limits
tracked in #156; house lifecycle belongs to #256.

### House lifecycle and tile simulation (#256)

`town/house.rs` owns `TileLoop_Town` (construction stages, rebuild/replacement, original and
binomial cargo with recession halving), builtin lift animation, produced/accepted cargo,
north-part normalisation, `ClearTile_Town`/`ClearTownHouse`/`RemoveNearbyStations`,
`TerraformTile_Town`, `CmdPlaceHouse`, cache rebuild/repair traversals, yearly aging and the
NewGRF house tile loop, `CanDeleteHouse`, animation triggers and rerandomisation. It writes the
sole Rust town owner directly and ends each owner access before any callback, command or station
service. #409 extends ownership to house/shared-town resolution and shares its
north-part helper with lifecycle. C++ keeps stack virtual facades, common Action2,
`StationFinder`/`MoveGoodsToStation`; original bodies compile only portably. Narrow phase observations replace whole
tile/spec copies; projections compose canonical spec-field readers. Each entry reuses its town
and map context, including construction triggers and rebuilds. Town count, recession, autoslope,
foundation price, tick counter and animated-tile removal are shared services; house clearing
calls the canonical Rust ground owner directly.
Cargo scaling, translation, label lookup and delivery, plus NewGRF boolean conversion and
inspect-window deletion, share canonical typed native leaves with industry consumers.
`python3 tools/migration.py simulate town-houses- --jobs 2` places every original house through
`CmdPlaceHouse`, compares error histograms, occupied/replace, protected clears, autoslope
terraform, company clearance until refusal and aged-house rebuild in both cargo modes, and
requires candidate `house.*` witnesses. #409 adds real house resolver groups; station
removal on house clearance remains a #156 gap.

### House and shared-town NewGRF resolution (#409, PR #416)

`town/resolver.rs` owns HouseScope/HouseResolver and TownScope/TownResolver query
policy, root/scope selection, process-wide ordered house classes and count reset.
Count/history/radius and north-part queries call canonical Rust town/house owners.
Lifecycle, construction, foundation and animation enter the resolver directly;
displaced table callbacks and native helper bodies are excluded with WITH_RUST.
C++ retains stack virtual facades, Action2, specs/override managers, map/pool
shells and one ordered call-local StationFinder. Var64 reads canonical Rust cargo
status and uses the shared GoodsEntry conversion. No new map view is exposed.
Native leaves are noexcept; no ordinary-play throwing service is used. Save/load
exceptions stay native. Contexts end before load/reset/yield; owner/container
borrows end before evaluator, storage and station reentry.
Default `town-houses-` scenarios install real deterministic/randomized groups:
construction over existing houses, production/prototype, parent readonly/null,
class reset/reuse, count clamps, signed var64 offsets/acceptance histories and
var65 multipart-origin exclusion/nearest match. Reload reinstalls transient
groups. Callback/register outcomes, CITY/map/cargo state, logs and RNG compare.
Focused founding/growth, storage reload, native callback reentry and ABI checks
cover the changed routes. External-GRF badge/content combinations, historical
save adapters and GUI-thread integration remain tracked under #156. Independent
whole-owner review and current-base exact-combination Full remain root gates.

### Industry periodic production and builder ownership (#129)

Rust owns the produced/accepted slots, optional accepted histories, production fields, sound
countdown and industry-builder records. C++ industry shells expose nonowning views of these
allocations; INDY, IBLD, ITBL, ECMY and TTO/TTD load adapters preserve the original names,
widths, version gates and ordering. INDY, IBLD, ECMY and TTO/TTD industry scalars load and
save through the reference aliases; only the Rust-owned industry text is staged.

Rust runs production/transport ticks, farm and lumber updates, NewGRF production application,
monthly statistics/closure, daily changes and builder targets/retries. Borrows end before reentry.

Pool/map/spec storage, construction and landscape-clear transactions, presentation and bounded
NewGRF resolution stay in C++ behind typed `noexcept` slots (#203). CargoPayment delivery
remains #117.

The default `industry-` scenarios exercise production, closure, builder retries and resets. The
corpus does not exhaust legacy saves, NewGRF resolver programs or every industry layout; native
tests check ABI/lifetimes.

### Industry construction and tile lifecycle (#150)

Rust owns industry metadata/text, ordered type registries, builder scratch,
destruction, initial generation, full validation/funding/prospecting transactions,
initialization, construction-stage completion, tile loops, builtin animation,
acceptance, clear/terraform policy and NewGRF tile/parent trigger traversal.
Native aliases and scoped load adapters preserve saved names/reference fixups,
partial initialization and ID reuse. Water destruction retains the tile-type recheck.
Packed map/pool shells, rendering, specs, resolvers and persistent storage stay C++.
Only progress cancellation (`AbortGenerateWorldSignal`) returns to the native
exception frame after source-order restoration. Direct synchronous services use
native stack costs; temporary resolver shells borrow a Rust stack owner through
the original constructor/destructor scope. No state borrow survives reentry.
`python3 tools/migration.py simulate industry-command-fund industry-command-prospect
industry-command-clear industry-command-terraform --jobs 2` compares command tests,
execution, ID reuse and autoslope; default industry scenarios also cover four
climates, lumber, generation and builder/farm cleanup. Default industry comparisons
check absent NewGRF acceptance/triggers and rare animation against unchanged bodies;
native tests check cancellation/restoration, callback policy and borrowed lifetimes.
Default `industry-water-check-*` scenarios require oil-rig funding refusal over
real depots/lower lock parts and acceptance after removal (#234).
Shared map/RNG leaves use the canonical typed services; stable dimensions are
captured once per consuming entry. [PR #218](https://github.com/fetch-laboratories/openttd-rust/pull/218) records integration evidence.
Legacy saves, resolver programs and exhaustive industry layouts remain #156 corpus limits.

### Subsidy lifecycle, route search and awards (#316)

Rust owns each subsidy’s canonical scalars, all route searches, creation/deity
checks, source/destination cache rebuild and deletion, monthly generation/expiry,
delivery awards and company transfer/deletion. Native SUBS descriptors and
GUI/script readers alias the sole Rust allocation; original bodies stay portable.
C++ retains pool identity/allocation order, map/catchment and cargo specifications,
news formatting and event enqueueing through typed noexcept leaves. Payment,
company and industry owners call Rust directly; no owner borrow spans a callback.
The deity command retains duplicate/self/distant routes, and monthly policy keeps
RNG short circuits, 1001 attempts, u16 wrapping/narrowing and expiry’s bypass of
both disabled-generation early returns. Cache flags use the original Set/Test
operations; house cargo scans call the integrated Rust house owner directly.
`python3 tools/migration.py verify --jobs 2` checks Cargo and native ABI.
Default `subsidies-control` and `subsidies-reload` compare commands, every endpoint
class, multi-awards/rivals, actual final-delivery payments for all four multipliers,
searches, deletion/transfer and modern SUBS reload, requiring Rust branch witnesses.
The existing console adapter compiles the unchanged reference monthly lambda for
zero-countdown, full-pool retry and disabled-expiry RNG gaps; scalar address checks
cover the reference-alias boundary. External NewGRF programs, legacy conversion
fixtures and threaded GUI/presentation remain coverage gaps tracked in #156.

### Complete PBS reservation service (#367)

Rust (`pbs.rs`) owns reservation reads/writes, complete platform scans, traversal,
loop termination, train selection and waiting policy. `follow_track.rs` owns
reserved-track masking between native Follow and GetSpeedLimit calls, removing
the YAPF/PBS bounce. Native map bits remain
canonical. The named `CFollowTrackRail::Follow` leaf carries all partial failure
observations in a call-local typed result; spatial enumeration retains native
hash order, while Rust selects the lowest front-train ID. Crossing updates and
train state/trackdir call their Rust owners directly. Header-matched track/PBS
helpers replace integrated copies; displaced native reservation bindings retire.
Station randomisation, animation and redraw retain source order and the same
native station identity. No world borrow spans a trigger or shared service.
Default rail-reservation, reload, controller and signal scenarios compare map
bits, movement/orders and RNG and require PBS branch witnesses. Their adapter
compiles the pinned algorithm bodies, adapting only migrated field reads, for
transient endpoints, train_on_res and trigger order; controlled platforms, loops,
stray/one-way paths, lowest-ID ties and occupied structure ends are restored
before play. Native runtime export/import carries both once-built PBS adapters
into CI shards. Native ABI/table checks and existing YAPF/controller tests remain.
External NewGRF trigger reentry, custom railtype turn flags, legacy conversion
and threaded GUI coverage remain #156 gaps. PR #370 records commands and metrics.
### Tile movement dispatch (#371)

Rust owns tile track status, admission, pixel height (including outside-map),
foundation dispatch and every tile policy, rail/road foundations, depot distances,
portal frames and bridge entry preparation. Commands, drawing and existing tile
loops retain their owners. Native map arrays, pools and vehicle shells remain
canonical; typed noexcept leaves carry consumed scalars or stable identities.
Owner loads/stores end before depot, road-stop, sound, viewport and NewGRF calls.
Train stopping, crossing occupancy, order stopping, slope/pixel and bridge helpers
call their integrated Rust owners directly. Road pathfinder track bits and
road/ship YAPF plus water-region movement queries also call Rust directly; header
preconditions follow native assertion policy, without valid-input FFI. Native
facades serve remaining native callers; portable original bodies remain excluded from WITH_RUST.
`python3 tools/migration.py rust-checks` runs Cargo4; `build` and native ABI/tests
check the boundary. Existing rail/road/station scenarios require movement counters,
including all depot directions, all train tunnel portal directions, station stopping,
road-stop admission/refusal/reload, road bridges and Padhattan lock/coast queries.
The road fixture queries adjacent barred crossings, edge-water masks and four half-tram directions.
`python3 tools/landscape-comparison.py` adds 19,608 unchanged-body foundation and
signed-coordinate cases to the existing default scheduler comparison.
Remaining NewGRF foundations, train/ship bridges, road tunnels, partial-depot
reversal timing, half-water and extreme bridge-speed branches are tracked in #156.

### Vehicle station/depot visits and route-link prediction (#349)

`rust/openttd-kernels/src/vehicle/visits.rs` owns `BeginLoading`, `HandleLoading`,
`LeaveStation`, `CancelReservation`, `ResetRefitCaps`, `CalcPercentVehicleFilled`,
`VehicleEnterDepot` with depot-entry recording, all of `LinkRefresher` and both
`IncreaseStats` overloads. It calls the Rust orders, station loading
(`prepare_unload`), cargo storage, refit command, train/ship state, maintenance
and pending-autoreplace owners directly, including aircraft hangar entry.
Train, ship, aircraft, road, orders,
station-command and loading callers reach it through `VehicleServices::visits`;
the C++ member functions are thin facades for the remaining native callers
(destruction, station removal, saves).

C++ keeps link-graph pool identity, allocation, node/merge/schedule and edge
updates, station/vehicle shells, NewGRF triggers, window/news/sound leaves,
and two `CommandCost` stack slots for the nested
depot refit. Prediction frames are entry-local (`Vec`, `BTreeSet`); temporary
refit cargo/subtype writes are restored in source order. No ordinary-play
callback unwinds; `AI::NewEvent` queues. Direct Rust entries preserve original
assertions through the shared native assertion policy.

`orders-visits` (padhattan-ridge-2000, cargodist) gives the running fleet refit,
autorefit, conditional, full-load and cross-graph orders and requires the
`vehicle_visits.*` prediction witnesses; the station scenarios require the
arrival/refresh/full-load ones. Link-graph pool exhaustion is a #156 gap.

### Landscape generation and rivers (#350)

Rust owns initialization, complete GenerateLandscape orchestration, original
terrain templates, coverage lines, desert/rainforest, springs, lakes/wetlands,
recursive river flow/widening, and the complete river YAPF state/search. It also
owns heightmap greyscale conversion/loading, flat worlds, slope repair and ground
watering. C++ keeps decoding, map storage/writes, sprite lookup and presentation.
Native helpers remain callees; existing tree, water, commands and scheduler
owners are called directly. The scheduler alone acquires its audited descriptor
for each loop call; generation reads use typed map services throughout.

Sea BFS/recursion remains Rust-owned. One stack optional unordered_set preserves
native flattening order; each search reconstructs it and ends before recursive
flow. Both generation progress operations use generation_services.hpp, returning
before native exception rethrow; no generator borrow survives a progress call.
Local queues, lookup-only marks and YAPF nodes/heaps retain source traversal,
admission, tie, RNG and narrowing order, including native search diagnostics.

Default generated.py/landscape.py cases compare both generators/all climates,
rectangular/freeform maps, coverage extrema, short/main rivers, sea cutoff and
flattening, lake/wetland/tree terminals, widening and bounded YAPF. Early map/RNG
snapshots, BMP rotation/aspect/padding/extreme-grey probes and TGP cancellation
are required, as is the native sea-order unit test. External heightmap formats,
maximum dimensions, alternate graphics and late/GUI aborts remain #156 gaps.

### Common ground-vehicle dynamics and canonical cache (#372)

`ground_vehicle.rs` owns train/road power and cargo cache rebuilds, acceleration,
whole-chain depot checks, crash slope-flag clearing, inclination/Z updates and
speed accumulation, including the selected train/road physics callees. Each
native shell holds one Rust cache/flags allocation with stable native aliases;
normal/indexed construction and destruction share that owner. `gv_flags` keeps
its uint16 save field and old-loader access; the cache remains rebuilt on load.
C++ keeps shell/pool identity, engine/cargo specifications, NewGRF properties,
viewport updates, typed map operations and the shared base crash leaf. Controllers
call the Rust owner directly; original dynamics/helpers compile only portably.
No owner borrow spans callbacks, and links are read at the original loop points.
Default road original/realistic/reload/service/collision/flooding, rail acceleration/
collision/service/reload and fleet build/consist scenarios require ground witnesses
in plain and desync runs. The existing fleet console compares unchanged reference
bodies over 30 arithmetic/default-drag inputs per type; native tests check canonical
aliases on normal/indexed shells and flags cleared before the base crash callback.
`python3 tools/migration.py verify` checks Cargo4, both native suites and ABI layouts;
PR #375 records the focused simulation commands and metrics. External powered/
articulated NewGRFs, maglev traffic, legacy saves and threaded GUI remain #156 gaps.

### Vehicle tile occupancy and live spatial queries (#377)

`vehicle/occupancy.rs` owns the process-wide tile buckets, complete on-tile/near
cursors, hash update/reset, and ground/track/tunnel occupancy command checks.
Each stable native shell carries the sole intrusive link record; Rust owns its
mutations. Native pools, IDs, coordinates and viewport indexing stay in C++.
Original bodies compile only portably. Rust road/train collision, signal,
flooding, road-stop rebuild, ship and fleet/command scans enter the owner directly;
position operations update the index before calling native viewport leaves.
No query/update/advance allocation or mirrored occupancy state is introduced.
Reset retains stale links and rebuild order; iterators read live next after
callbacks. No owner borrow spans crash/flood reentry, and no generation guard
changes source admission or revisitation. Shared native command costs preserve
local error messages; occupancy consumes no random draws.
Default road/crash/flood/reload, train collision, road-stop and fleet scenarios
require occupancy witnesses alongside their semantic vehicle/cargo/RNG checks.
The existing fleet console compares unchanged public iterator bodies at the
full-scan threshold, coordinate extremes and current-shell mutation; native
checks cover canonical link identity, stale removal, copied cursors and pixel-Z
command refusal. The existing ABI/range fixtures exercise their direct routes.
Shared geometry/bit/track modules supply the original header callees.
Disaster movement and ground-clear enter occupancy directly through borrowed
typed context; the exact base viewport and native cost lifetime stay shared.
GUI/external articulated-GRF gaps remain #156.

### Cargo-monitor accounting and ordered maps (#386, #388 via #391)

Rust owns both process-wide ordered pickup/delivery maps, shared bit helpers,
activation/query/reset/stop, selective/global cleanup and delivery accounting.
Payment and company owners call Rust directly. Payment resolves stations through
its existing subsidy service at each original observation; independent stack
cursors preserve nested industry scans. Arrivals update registered keys only,
without allocating, and unsigned amounts narrow to signed32 before saturation.
Native facades retain script validation, canonical stations/pools and assertions.
CMDL/CMPU retain two-scalar TempStorage, ascending save order, native SlObject and
SlError frames, historical key conversion and incremental first-insertion wins;
no Rust map borrow spans a native callback or decoding operation.
Default economy scenarios require nonempty changed maps for manual/cargodist and
pending unload; stockpile also checks decoded positive maps immediately on reload.
Company deletion/acquisition compare encoded keys and surviving amounts. The
existing engine console tests controls, destination membership, signed bounds,
InitializeEconomy reset and actual native chunk-handler header/partial errors and
duplicate rows. Production monitor-table completeness runs in native ABI tests.
Focused evidence: `python3 tools/migration.py simulate economy-manual economy-cargodist economy-stockpile companies-delete companies-acquire engines-monitors`;
`python3 tools/cargo-income-comparison.py`, Cargo4 and native company/payment/ABI tests.
Historical monitor-key conversion and external GameScript callers remain #156
coverage gaps. Independent fixing review and the separate root cost disposition
are recorded on #388; #391 integrated the frozen owners on the routing base after
scoped reconciliation review and exact-combination Full passed.

### Vehicle NewGRF resolver and canonical query cache (#392, PR #397)

`newgrf_vehicle.rs` owns the five-value cache/validity allocation and lifetime,
cache fill/invalidation, stack self/parent/relative/root selection, wagon overrides,
real-group selection, complete vehicle/purchase variable control, callback/property/
probability entries and trigger recursion. Native virtual facades keep the portable
originals excluded from `WITH_RUST`; common Action2 evaluation, specifications,
pools, saved vehicle random/waiting fields and rendering stay native.
Existing train/road/aircraft/ship, cargo, command and loading owners enter Rust
properties/callbacks/randomisation directly. Typed observations read consumed
fields only; neither cache/state borrows nor references into native storage cross
common evaluation. Invocation paths allocate nothing; each native shell owns one
canonical cache allocation with native aliases, including indexed construction.
Default fleet resolver evidence uses actual deterministic/procedure/randomized/
real groups, cache miss/hit/caller-v7/v8 translation, wide badge IDs, unavailable
variables, signed properties, probability RNG, output registers, parent/relative
reentry, all five triggers and zero-bit reseeding. Stock road/ship/aircraft fixtures
compare public vehicle/purchase variables; aircraft mappings use all byte states
at the fixture airport's positions. Native GRF reload destroys the synthetic groups
and reinstalls fresh groups; it compares transient cache observations separately
from nonempty random/waiting fields decoded from a saved fleet and loaded again.
PR #397 records focused commands and metrics. Arbitrary external GRF programs,
historical saves, GUI scheduling and remaining branch combinations are #156 gaps;
the fixtures do not claim persistent synthetic sprite groups or whole-game equivalence.

### NewGRF transactional storage (#393)

`newgrf_storage.rs` owns process-wide modes, the pointer-ordered changed registry,
temporary registers/generation keys and canonical 16/256-word payloads/backups.
Native pool shells retain IDs and feature associations; save/debug aliases address
the Rust arrays directly. Every mode switch restores temporary changes in shell
pointer order. PSAC saving restores payloads without removing registry membership;
destruction removes pending entries, and the registry outlives native destructors.
Rust also owns industry/airport/town StorePSA and town GRFID selection/traversal.
Native typed leaves retain pool allocation/publication and ordered town associations;
town publishes after StoreValue, while industry/airport publish before it.
Original bodies remain portable. Native SlObject, reference fixup, legacy copies
and SlError frames keep their original order, with no live Rust frame or borrow
across save errors. Compression workers receive already serialized bytes.
Default `town-storage-reload` uses real resolvers and PSAC: all creation policies,
GRFID reads/writes, command-test/save rollback, deletion/reuse, nonempty modern
save/reload and native header/partial-load plus 16-word compatibility probes.
Native storage tests cover alias identity, legacy copying, all six modes,
save-registry retention, pending destruction and temporary-key rollover.
External NewGRF programs, complete pre-161 conversion saves and GUI thread
scheduling remain #156 limits. PR validation and metrics carry the exact evidence.

### Vehicle NewGRF resolver runtime (#392, #397 via #415)

Rust owns the canonical stack cache, self/parent/relative scopes, root and real-group
selection, complete variable dispatch including 0x61 and purchase contexts,
callbacks/properties/probability, subtype/aircraft helpers and randomisation.
Native virtual facades retain common Action2 evaluation, immutable specs, shells
and saved random/waiting fields. Direct Rust callers replace displaced callback
bounces. Typed shared services preserve metadata observation, native indexing and
GetSpec fallback behavior; no invocation allocation or mirrored world state.
Cache aliases and lifetime follow the resolver object. Owner borrows end before
common evaluation, storage and reentry. Original bodies remain portable.
Default fleet resolver/reload, rail and aircraft fixtures use real deterministic
and randomized groups, require variable/register/trigger witnesses and compare
semantic vehicle/cargo/save state plus both RNG streams. Native ABI and existing
aircraft comparison probes supplement these cases. Exact commands, inventories,
necessary-cost disposition and two function-group fixing reviews are on #397.
Unreached external-GRF, historical and GUI paths remain on #156. Final shared
adoption/current-base combination passed exact-head Full in #415.

### Industry and industry-tile NewGRF resolvers (#398, #407 via #415)

Rust owns both stack feature contexts, roots/lazy town, variables, count/search
and tile-relative helpers, probability/refusal and feature resolution policy.
Existing industry/registry/cargo owners supply canonical state directly; PSA
reads and writes use the integrated storage owner. Small copied invocation state
ends native context borrows before evaluator/storage reentry. Native adapters
retain common Action2 frames, virtual facade construction, specs/map/pool identity
and consumed result copying; no per-resolution allocation or world-record view.
Original bodies remain portable. Displaced probability/refusal caller bounces,
dead query facades and orphan bindings are removed.
Default industry-resolver uses actual deterministic/randomized production, parent,
self and tile reseed groups. Required witnesses exercise live/null/temporary
contexts, unavailable-variable/register results, var67/68 counting and wrapping,
caller-file cargo, legacy version/location rules and tile identities. Semantic
INDY/map/save/log and RNG comparisons accompany existing production/lifecycle/
animation/acceptance and cargo probes. Commands, metrics, accepted boundary cost
and fixing/adoption reviews are on #407; external/historical limits are on #156.
The final exact-current-base combination passed Full in #415.

### Rail-station and waypoint NewGRF resolution (#399, PR #411)

Rust owns stack scope/root/cargo/lazy-town state, complete station/waypoint
variables, real-group selection, platform/end/continuation, slope callbacks and
randomisation. All six optional caches follow the resolver object's lifetime,
including repeated DoResolve. Native specs/association vectors, map/station shells,
saved random/waiting bits, common Action2 and optional TownScope frames remain.
Direct cargo/station/movement owners replace displaced command/animation/PBS
callback bounces. Shared goods conversion, scope fallback, container/airport and
metadata helpers retain native types, narrowing and original GetSpec behavior.
No owner/cache/container borrow spans evaluation/storage; setters remain immediate.
Original portable bodies and native callback CommandCost/error frames retain order.
Default resolver/reload fixtures install actual deterministic/randomized groups,
require all variables/availability, cache reuse/fresh-owner contrasts, roots/parent
readonly/retry, slope outcomes, trigger gating and lower/upper/combined reseeding.
Lower-only reseeding proves six tile draws plus one station draw. Semantic STNN/
map/cargo/save/register/ordered effects and both RNG streams compare in both modes.
Transient groups are reconstructed after native load; arbitrary external-GRF,
historical and GUI paths remain #156. Commands, accepted cost, fixing reviews
and root's final current-base combination evidence are linked from #411.

### Road-stop NewGRF runtime and associations (#410, #422 via #427)

Rust owns road-stop/road-waypoint stack context, root/cargo selection, lazy parent
and variable policy, callback and randomisation control, spec-slot selection,
assignment/removal and trigger-cache reconstruction. Native station shells retain
shared spec/tile vectors, saved bits and serialization; specs, parsed Action2 and
rendering remain native. Thin virtual/evaluator frames keep portable callers.
Station variables, existing-cargo counts, empty masks and storage use their Rust
owners directly; station/spec metadata has one shared native definition. Displaced
command/animation/cargo callback slots and orphan declarations are removed.

Ordinary road-stop randomisation constructs CBID_NO_CALLBACK in the original;
real groups preserve waiting publication/retention, gates and zero new draws.
Explicit CBID_RANDOM_TRIGGER fixtures establish resolver masks, without claiming
ordinary execution of the otherwise-unreachable reseed/consumption branches.
Default command/reload cases compare actual groups, slot/cache lifecycle, full
semantic saves, map/STNN/registers and both RNG streams. Transient spec/group
reattachment after load does not establish arbitrary external-GRF loading.
Build/Cargo4, affected native/ABI and default scenarios, callee/duplicate/orphan
review and exact-current-base Full are recorded in #427. Remaining historical,
external-GRF and named unreachable branches remain on #156. Necessary typed
native vector/spec/evaluator boundary cost was accepted once at the checkpoint.
