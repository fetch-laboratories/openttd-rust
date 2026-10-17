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
python3 tools/migration.py rust-checks --jobs 2
python3 tools/migration.py build --jobs 2
python3 tools/migration.py verify --jobs 2
python3 tools/migration.py tools
python3 tools/run-comparisons.py
```

Each driver invocation retains command logs, test reports, executable hashes,
source revision, and local changes under `.local/verification/<timestamp>/`.
Passing establishes only covered behavior. The driver does not itself compare
every game state or prove full game equivalence.

`rust-checks` runs the four Cargo checks without creating or building the
reference. The default `--jobs 2` also limits Cargo invoked by CMake. Every logged
command reports its phase, elapsed time and output path, with periodic updates;
lock waits identify live holders. POSIX commands run under a pipe supervisor;
driver death (including SIGKILL) closes the pipe and kills the command session. Reference and candidate configure/build/test
operations use separate locks, and simulation captures immutable runtime copies
under those locks. Standalone fixture games share the benchmark coordination lock.
Successful candidate game builds write a versioned executable identity containing
source and configuration digests plus the executable hash. Receipts associate
that identity only with a matching executable and configuration; external or
otherwise unrecorded builds retain unknown provenance.

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
pushes, which checks each merge.

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
  setup provenance, reproduction commands and limits.
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
  `--candidate` uses data beside that executable. Reports hash the executed
  copies before running; `candidate_commit`/`candidate_status` describe the
  initial checkout, not proof of an arbitrary binary's source revision (#97).
  Evidence: `.local/simulation/<time>-<pid>/report.json`.

Simulation speed (#155) is reported for every scenario as `plain_speed`:
paired process wall seconds and candidate/reference ratios, plus their median.
Ratios are null when exit saves are missing, runs fail, end moments differ, or
semantic differences remain; timing never changes the pass/fail result.
Parallel runs are noisy. Use an idle host, profiling counters off, the same
build settings and pinned manual-distribution play save:

```sh
python3 tools/migration.py simulate play-opus-55-167-002-manual --benchmark 3 --jobs 2
```

The driver builds with two jobs, then runs one scenario worker. Benchmark mode
runs only plain pairs, isolates each timed game from other clone-wide harness
games, and compares each exit save and full logs. Both roles use #154's serial
offline thread-failure fallback, recorded as `execution` in the report. These
are total offline times, not normal threaded frame latency. It retains the final
pair and runtimes; `plain_commands` supplies arguments for optional `perf record`
replay through `tools/simulation/game_launcher.py`. Preserve the run's HOME/XDG
directories and runtime libraries; keep profiling separate from timing samples.
Timings include startup, loading, serial link-graph work and save I/O. Benchmark
games wait directly for child exit with a timeout watchdog; ordinary runs retain
subprocess timeout polling, which can add about 50 ms.
The lock excludes harness games, not unrelated host activity. Port PRs record
before/after ratios and commits; the roadmap sets the regression budget.

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

The required macOS ARM jobs activate Rust through the reusable workflow's `rust`
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

`docs/skills/` holds three user-invoked skills: `/steer`, the user's Claude Code
steering review; `/start-development`, root's restart prompt; and `/wind-down`,
which writes the user's message pausing root. Steering reviews put their
corrections in the roadmap and edit `/start-development` only to improve it;
root follows these skills and leaves them unedited.

The driver and CI enforce native build/tests, nonempty test inventories, reference test-name preservation, and Cargo checks listed in `AGENTS.md`.

Server-side protection on `rust-migration` requires `Full validation`, the cheap
commit/string/script-mode checks, and resolved conversations. Full validation is
an explicit commit status, published only after the native comparisons, simulation,
entire platform matrix, quick checks and annotation checks succeed for a captured
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
comparison; `platform` runs the platform matrix. All include quick checks.
Only `full` satisfies the merge gate. Unmergeable or pending-mergeability PRs
are refused; a changed PR head needs a new full request. A later base advance
does not invalidate an unchanged head; independent reviewed PRs can validate concurrently.
The CLI records the SHA and request/run identity and refuses a changed PR head.
`CI_ON_DEMAND=true` enables this scheduling after root installs and verifies the
required status. With the variable absent, automatic full CI remains as a safe
bootstrap. For a workflow-changing PR, root reviews its workflow definitions,
temporarily unsets `CI_ON_DEMAND`, and requests `--profile full --bootstrap`
(the `ci:full` label path tests the PR definitions). Root restores the variable
to `true` after merge and records both changes in the PR; never use `--admin`.
For capacity batching, test a concrete integration PR and merge it once. A stack
organizes dependent PRs but does not itself combine validation runs.

Required checks must pass on the PR head; branches need not be up to date with the
base. The publisher records the immutable tested merge tree separately from the
head status.
Protected pushes have per-SHA concurrency, so a later docs push cannot cancel
the code merge run. A merged push whose tree matches the tree marker from the same successful full
request refreshes native compiler caches without repeating tests or scenarios.
Covered protected pushes also rebuild the cached Linux GCC, macOS and Windows
targets to refresh their compiler caches. This path skips Cargo checks, CTest,
comparison/configuration probes, annotation checks and platform evidence; Clang
and Emscripten have no compiler cache to refresh. The path is derived only from
the protected push's matching tree, and requested PR validation always runs the
complete selected suite. A combined tree no full request
covered receives all native and platform checks. Markdown-only pushes skip heavy
builds after complete file classification. A post-merge failure is fixed forward first. Force pushes and branch deletion are disallowed,
including for administrators. The approving-review count is zero because agents
share credentials; the attributed independent review report is the process gate
before root integrates. Repository controls are enforced separately from these
documents.

Native validation exports both frozen runtimes and once-built fleet/orders adapters,
then runs component comparisons and six simulation shards concurrently. Recorded
scenario seconds assign work while keeping effects/disasters/aircraft fixture
families whole. The gate requires every job and exactly the current default
`simulate.py --list` inventory, with matching executable and manifest hashes.
Export requires canonical binary/configuration identity matching clean current
source. Exports strip debug sections only on copies, proving identical
loadable/allocated code and data and retaining parent/derived hashes; widget
objects also preserve resolved symbols, relocations and COMDAT groups.
Original verification binaries stay intact; `tools/evidence.py` joins explicit
derived simulation identities to their verified parent hashes. Probe
workers receive prebuilt generators/archives and the widget probe's real link
objects; build metadata relocation has its own receipt. Reports/logs and retained
failing saves are artifacts; build inputs expire after one day. Tests and their results are never cached. Cache writers trim native roles
to 750 MB, Linux flavours to 600 MB and macOS flavours to 650 MB. The trusted
cache-retention workflow keeps the latest entry per prefix and bounds total usage
to 8 GiB (below the 10 GiB repository limit).

Runner settings are `CI_LINUX_HEAVY`, `CI_LINUX_COMPARISONS`,
`CI_LINUX_SIMULATION`, `CI_LINUX_PLATFORM` (default `ubuntu-24.04`), and
`CI_MACOS_HEAVY` (default `macos-latest`) and `CI_WINDOWS_HEAVY` (default
`windows-2022`). Heavy native jobs use `nproc` slots.
One full request peaks around 16 jobs; overlapping requests may queue against
account/runner capacity. Runner size and account plan remain repository settings.
The repository belongs to the `fetch-laboratories` organization, where Blacksmith
serves the Linux jobs: `CI_LINUX_HEAVY` is `blacksmith-8vcpu-ubuntu-2404` and the
other Linux variables are `blacksmith-4vcpu-ubuntu-2404`. Windows uses
`blacksmith-8vcpu-windows-2025` after #252 compatibility/warm acceptance; macOS and light jobs stay
on GitHub-hosted runners (free for this public repository; Blacksmith
bills macOS at 20x). Blacksmith serves `actions/cache` from its own co-located
store on its runners, so the GitHub cache budget above covers only GitHub-hosted
jobs. Unsetting a variable returns that job to GitHub with no workflow change.
Profile CI time with the Blacksmith CLI (`blacksmith jobs list|steps|stats`,
`jobs aggregate`, `jobs diagnose rightsize`) before changing runner sizes or
sharding: `stats` gives each Blacksmith job's CPU and memory timeseries. Agents
authenticate with an org token (`blacksmith org-token create`, admin only), never
a copy of a person's `~/.blacksmith/credentials`.

`python3 tools/preflight.py --base origin/rust-migration --verification REPORT`
runs the pinned inherited commit checker and checks explicit candidate compiler
logs for warnings in branch-changed files and diagnostics without source locations;
unchanged upstream warnings remain outside this scan. Its first run downloads the pinned hooks into ignored shared storage;
`--hooks PATH` uses an existing clean pinned checkout offline. Without supplied
logs it explicitly leaves warnings unchecked. `tools/evidence.py` exports explicit
verification and optional repeated simulation receipts with committed port metrics
and agent attribution; unsupported, failed or mismatched receipts fail. Narrow
checks, legacy receipts, dirty source and unknown provenance remain labelled.

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

### TGP terrain generation

Rust owns the temporary height map and every TerraGenesis generation stage, including
height estimation, noise, water histogram, smoothing and regional curves. `src/tgp.cpp` keeps
settings extraction, shared RNG callbacks, progress and final tile writes; the original
implementation compiles only without `WITH_RUST`.

An opaque owner advances through generation and normalization, returning progress actions to
C++. Settings are copied; RNG draw order and fixed-point conversions follow the source. Abortion
frees the owner after Rust returns, so the progress exception never crosses a Rust frame.

Default CI verifies the build and runs `python3 tools/migration.py simulate` with four curated
settings cases spanning all four climates and selected terrain, smoothness, variety, sea/border,
height and map-shape values. Maximum-map performance and Windows/macOS runtime parity remain
untested.

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
scalar handlers with local strong-index construction. Leaves shared by several
owners (clear/tree ground, snow, shore, water loop, sound, foundation) live in the
shared table. Ground boundaries use actual native enums, unrestricted transparent
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
Remaining matrix/editor/NewGRF/threaded gaps are in #156. `TreeObservation` remains;
the scheduler gate does not remove its full facade cost.

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

### Link graph job computation

Issue #74 moves demands, both MCF passes, cycle elimination, path lifetime and flow mapping into
Rust. Rust owns copied graph/settings, private demand/edge annotations, a reusable path arena
and ordered cumulative-share maps. C++ keeps job scheduling/thread/abort flags, LGRP/LGRJ
save/load and the existing station flow join.

ABI IDs 34-37 cover scalar node/edge/settings/share records. Its sole callback reads the job's
atomic abort flag and cannot throw/reenter. C++ allocations happen outside Rust.

Evidence: the unchanged regressions/comparisons and `python3 tools/migration.py simulate` with
manual/cargodist road saves, symmetric/asymmetric scalers, boundary
distance/size/accuracy/saturation settings and reload from a reference snapshot with outstanding
LGRJ records (GLOG history is reset only in prepared road inputs so both loaders log their
revision). Graphs, allocator failures and abort timing are not exhaustive; resource exhaustion
timing differs with the allocation strategy.

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

### Effect vehicles

Rust owns all twelve controllers, movement tables and private animation bytes (#101/#107). Each
C++ EffectVehicle shell owns one zero-created opaque state, also on indexed load; RAII destroys
it on ordinary deletion and pool cleanup.

Copied call-scoped observations and immediate scalar writes keep original mutation order. Rust
calls common Random/map and effect-specific sound, viewport and animated-tile noexcept wrappers
directly; none can reenter Rust.

`python3 tools/migration.py simulate effects` compares save chunks across subtypes, expiry,
reload and natural creation. Viewport pixels, audible output, full legacy fixtures and every
caller remain limits.

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

### Disaster scheduling and vehicles

Issue #103 moves all fifteen subtype controllers, eight initializers, eligibility,
countdown/reset, industry construction reset and square-clearing policy, movement, shadow/rotor
updates and both target-release hooks into Rust. Rust owns each shell's state, flags, image
override and destroyer target, plus the persisted global delay. Original bodies compile only in
portable builds.

Copied observations and direct noexcept shared services preserve original ordering and shared
RNG draws. Private scalar addresses remain stable through modern and legacy save staging; flight
helpers borrow only the flags scalar for their call.

The default `disasters` simulation checks scheduling, crashes, removal, industry reset,
submarine movement and reload. Legacy save fixtures, viewport pixels, sound output, allocator
failure timing and exhaustive NewGRF combinations remain limits.

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

### Ship YAPF searches and canonical path cache

Rust owns both region and track searches, arena indices/parent chains, open and closed
membership, the original binary-heap tie/removal rules, destination estimates, region corridor,
two attempts, random lost paths and cache extraction (#119). Each Ship owns one opaque Rust path
with copied controller access; modern VEHS stages one byte across C++ load errors and commits it
on unwind, while the pre-struct-list loader retains its reversed path order.

C++ keeps ship/world pools, track following, station/dock queries, engine/settings observations
and canonical map arrays. No ordinary throwing/reentrant world callback occurs in this search;
save errors remain outside Rust frames.

The existing `python3 tools/migration.py simulate water --jobs 2` corpus compares all chunks for
ferry manual/cargodist, canal/lock loss and recovery, aqueducts, depot reversals and
reference-produced live-path reload. Arbitrary maps/NewGRFs and complete legacy saves are not
exhaustive.

### Town growth and lifecycle state

Issue #120 moves town tick traversal, road/build choices, terrain retries, house
selection/placement and expansion into Rust. The partial #148 extension owns town metadata,
histories, ratings and building counts; C++ retains house tile loops and remaining
authority/deletion commands. Native field aliases expose one Rust owner, with process-wide state
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

### Road vehicle control and private state

#121 moves tick/day control, movement, blocking/overtaking, reversal, station/depot transitions,
crash expiry, servicing and cache policy to Rust. Each shell owns seven scalars and the
canonical path; #124 YAPF writes that same path. Modern, split-vector and TTD/TTO adapters
retain field widths through C++ staging.

#199 adds shared typed vehicle and GroundVehicle services (`vehicle.rs`, `vehicle_ffi.h`,
`vehicle_services.{h,cpp}`). No owner, path or world borrow spans callbacks, reentry or
deletion.

The default `roads-` simulation compares acceleration, servicing, path, RNG, crash expiry and
reload. Legacy saves, custom NewGRFs, articulated/tram turns, sounds and viewport pixels remain
limits tracked by #156.

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

Issue #124 moves both track/depot searches, road exit-direction keys, node arena,
lookup maps, exact-order heap, segment traversal/cost, heuristic, limits and best
intermediate fallback into Rust. Reconstruction and station-area trimming write
#121's sole canonical path directly; cache ownership is not counted twice.
Original road specialization and node types compile only in portable builds.
C++ keeps map/pools/settings, closest station-area and road-stop observations,
and shared `CFollowTrackRoad` behind copied scalar `noexcept` leaves. None runs
scripts, mutates/reenters the search or canonical road owner, or throws during
ordinary play. Short owner access ends before leaves. RNG is unchanged (neither
search draws); integer narrowing/wrapping, exit-key replacement, strict ties and
follow-before-limit timing retain source behavior. Panics/OOM abort.
Arena, lookup maps and heap retain process-wide scratch capacity between searches
(#278). Each active search owns the scratch; the cache keeps no native pointers.
Inputs, destination state and counters reset per call under the game-state mutex.

`python3 tools/migration.py simulate roads` covers the controller's original eight
cases, actual station-area prefix trimming and a forced cache miss with competing
penalties; paired reference/candidate runs compare saved fields in plain/desync modes. Native
tests compare actual heap and origin masks with unchanged C++ and depot-search transcript/counters
with `CYapfBaseT` at node limits 0/1/2/7 and depot cost bounds 0/70/71/100
(loaded node settings clamp to >=500). Native ABI checks
cover every field on supported hosts. Arbitrary maps/NewGRFs, articulated/tram,
road waypoints, exhaustive occupied-stop/loop/segment-length branches and actual
legacy saves remain limits; shared follower logic is deliberately unported.

### Orders lifecycle ownership

Issue #138 / PR #176 moves order storage, links, backups and consist timetable/unbunching state
into Rust, including traversal, command edits, sharing, depot execution and backup lifecycle.
C++ retains pool identity, vehicle shells, command execution, VM callbacks and save/load
adapters; original bodies remain in portable builds.

Typed synchronous services preserve source ordering. Rust releases owner borrows before
callbacks, commands or deletion, and native exceptions remain in C++. The default orders
simulation and existing tests cover commands, sharing, backups, reload and reentry. NewGRF
callback variants and legacy saves remain coverage limits tracked in #156.

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

### Aircraft controllers and airport movement

#136 owns both tick passes, movement/speed/flight, shadow/rotor, FTA traversal, terminal/helipad
reservations, service/range, crash cleanup and airport lifecycle in Rust. Fixed allocations own
private counters/cache and airport masks; trivial C++ save aliases preserve their addresses.

#237 borrows shared vehicle/map/settings/RNG services directly. No owner borrow crosses
ProcessOrders/loading, depot/refit, animation, crash or deletion. Runtime dimensions are
captured per entry; wrapping arithmetic/RNG order remain.

The default `aircraft` and `disasters` scenarios exercise traffic, service, removal, crash and
reload. Real NewGRF/refit/airport rotations, threaded GUI, legacy-save fixtures and presentation
remain gaps tracked on #156; these checks do not establish full subsystem equivalence.

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
