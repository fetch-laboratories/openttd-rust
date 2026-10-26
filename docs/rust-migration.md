# OpenTTD-Rust migration

OpenTTD-Rust is an independent experiment in incremental C++ to Rust migration. The original
game's observable behavior is the specification, including quirks retained for historical
fidelity. Record possible improvements in fork GitHub issues, deferred until near-full Rust
reproduction. Game-simulation code takes priority, with Rust owning component state;
`docs/roadmap.md` sets current work and `AGENTS.md` states the selection rules.

## Repository and reference

The fork is https://github.com/fetch-laboratories/openttd-rust. `origin` points to the fork;
`upstream` points to https://github.com/OpenTTD/OpenTTD.git. The migration starts from OpenTTD
15.3, commit `14ec60f248547d4d062a1160f0fc26d742319888`, pinned in `migration/baseline.json`.

`rust-migration` is the integration branch. Create isolated task branches/worktrees from it and
target it with fork PRs; the pinned original revision remains the behavioral reference
throughout migration.

The verification driver creates `.local/reference/openttd` in the main checkout as a detached
Git worktree at that revision, shared by every worktree of the clone. It rejects a changed
reference revision or dirty reference source. The shared reference builds into the main
checkout's `.local/build-reference`; each worktree builds its candidate into its own
`build-rust`. Neither changes reference source. Updating the baseline is a separate deliberate
task. Never change candidate behavior and expected results together to make checks pass.

Migrated code lives in the shared `rust/openttd-kernels` crate, linked into the game, the unit
tests and the native generators (strgen, settingsgen). Each port keeps its original C++
interface; see "Ported components" below. None of these ports completes its containing
subsystem. Preserve the complete game, including networking, saves, NewGRF mods, graphics, and
shared random-number behavior.

## Terms

- **Owner**: the Rust state that holds a component's data and runs its control flow, such as one
  allocation per vehicle or a process-wide static. An "owner borrow" is a Rust reference into
  that state; it ends before any callback that can reenter Rust or mutate the state.
- **Shell**: the C++ object (usually a pool item such as a `Vehicle`, `Station` or `Town`) that
  keeps its pool identity, unmigrated fields and save aliases while its migrated state lives in
  a Rust owner the shell holds.
- **Canonical state**: the single authoritative copy of a datum, in Rust or C++. Ports read and
  write it in place and keep no mirror; "canonical native" means the authoritative copy remains
  in C++ (for example the map arrays).
- **Facade**: the C++ function or method that keeps the original signature and forwards to Rust
  under `WITH_RUST`; the original body stays in the `#else` branch for portable builds.
- **Leaf**: a typed `noexcept` C++ service that Rust calls for a shared operation (map access,
  `Random`, sound, viewport) and that returns without reentering the calling owner.
- **Service table**: a designated-initializer C++ struct of typed leaf function pointers (for
  example `OpenTTDTreeServices`) that Rust borrows for a call; each field is one named service,
  never an opcode.
- **Witness**: a named branch counter in `witness.rs`. Simulation scenarios that set `witnesses`
  require their listed counters to be nonzero, so a scenario fails when it stops reaching the
  branch it claims to cover.
- **Anchor**: a `SPEED_ANCHORS` record in `tools/simulation/instructions.py` holding a speed pass's
  measured wall ratio and game-loop instruction count for one scenario; speed projections scale
  from it.

## Build and verification

The CI-policy regression suite additionally uses Node.js to execute the actual workflow scripts;
CI requests use an authenticated `gh` CLI. The maintenance tools work independently of the Codex
host or T3 Code.

The current verification setup targets native Linux and needs a C++20 compiler, CMake, Ninja,
Python 3.11 or newer, SDL2 development files, and the normal OpenTTD libraries described in
`COMPILING.md`. OpenGFX supplies free graphics for regression games; commercial game assets are
unnecessary. The verification driver requires the pinned Rust toolchain and always configures
the candidate with `OPTION_RUST=ON`. Ordinary CMake builds default to `OPTION_RUST=OFF`,
preserving the original portable C++ path. Rust linkage supports native GNU/Linux (64-bit x86 or
ARM) and native macOS arm64. The scoped native Windows MSVC mode is described below. Other
platforms, cross compilation, and macOS universal/Intel configurations reject an enabled Rust
option; their portable fallback remains migration work (#3).

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
`.local/deps`, and installs Rust into `.local/cargo` and `.local/rustup`. The bootstrap changes
no shell profiles or system packages. It relies on installed compiler/runtime libraries and
host-specific package names; it is not a portable installer. `rust-toolchain.toml` pins Cargo
compiler, formatting, and lint tools. CI installs that same toolchain before verification. The
crate has no external dependencies.

The driver's `verify` action requires the four Cargo checks listed in `AGENTS.md`, builds both
graphical executables, runs both CTest suites (upstream unit and scripted game tests), and
requires the candidate to retain all reference test names. Empty test inventories fail
verification. The candidate Rust option is checked in the CMake cache and recorded in the
report; a C++ fallback cannot pass as a migrated candidate. The `tools` action builds the native
Rust generators alone into `.local/build-tools-rust` (`OPTION_TOOLS_ONLY=ON`, `OPTION_RUST=ON`),
using the bootstrapped toolchain when present. `tools/run-comparisons.py` then runs every
reference comparison tool (`tools/*-comparison.py`) in parallel and prints each one's case
inventory; CI runs all three steps. The tools share `tools/probe.py` (pinned-reference guard,
body extraction, build-rust archive linkage and pass line).

```sh
python3 tools/migration.py rust-checks
python3 tools/migration.py build
python3 tools/migration.py verify
python3 tools/migration.py tools
python3 tools/run-comparisons.py
```

Each driver invocation retains command logs, test reports and the source revision under
`.local/verification/<timestamp>/`. Passing establishes only covered behavior. The driver does
not itself compare every game state or prove full game equivalence.

`rust-checks` runs the four Cargo checks without creating or building the reference. `--jobs`
(default: the free cores, `max(2, cores - round(1-minute load))`, read once at start) also
limits Cargo invoked by CMake. Every logged command reports its phase, elapsed time and output
path, with periodic updates; lock waits identify live holders. POSIX commands run under a pipe
supervisor; driver death (including SIGKILL) closes the pipe and kills the command session.
Reference and candidate configure/build/test operations use separate locks, and simulation
captures immutable runtime copies under those locks. Standalone fixture games share the
benchmark coordination lock. Validation is established by the exact-head CI run and the commands
in the PR. Local logs and semantic reports help diagnose failures.

The fork executable is `build-rust/openttd-rust`; Cargo artifacts reside in the ignored
`build-rust/cargo` directory. CMake tracks the Rust sources, manifests, lockfile, and toolchain
file to rebuild the archive and affected executables. The reference executable is
`.local/build-reference/openttd` in the main checkout. Most in-game branding remains original.
When using this host's extracted dependencies, launch with their library path:

```sh
LD_LIBRARY_PATH="$PWD/.local/deps/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  ./build-rust/openttd-rust -X
```

`-X` avoids global game folders. Use separate development configuration and saves.

### Generated ABI probes and fixture defaults (#284)

`layouts!` declarations in `abi.rs`, `company.rs`, `crypto_primitives.rs`, `blake2b.rs`,
`x25519.rs` and `auth.rs` name the C++ type, probe group and ordered Rust fields. Use
`rust_field => cpp_field` for aliases and `width field: RustType` for an extra native-width
check. Native-only groups remain in their owning unit tests. CMake runs `tools/generate-abi.py`
into the build directory; probes compare the complete list and its `SIZE_MAX` terminator through
the named metadata API. Add declarations there, never a second C++ field list or numeric type
ID.

Callback declarations inherit signature-correct abort defaults. Fixtures specify only callbacks
they actually use; data pointers and scalar carriers retain their own initialization rules.
Generated typed visitors check actual production instances for null callbacks and abort defaults
in the default native tests. `tools/test-generated-abi.py` checks that every generated list and
visitor names exactly its declared fields, and parses a synthetic tree.
Cross-language signature assertions are not generated.

## Compiler cache, shared reference and CI time

Every worktree of a clone shares the main checkout's pinned reference checkout
(`.local/reference/openttd`), reference build (`.local/build-reference`), bootstrapped
toolchain/dependencies, and per-role ccache stores (`.local/compiler-cache/`). A lock serializes
reference configure/build/test across concurrent runs, so the original is compiled once per
clone rather than once per worktree. New worktrees need no symlinks. The driver skips an
explicit CMake configure when the existing cache already holds every requested `-D` value; Ninja
still reconfigures on any CMake input change. Configure-time environment changes (for example a
new dependency prefix) need a fresh build directory or one manual configure.

The driver uses ccache whenever it is installed (`tools/bootstrap-local.py` installs it
locally). `migration/ccache.conf` keeps compiler-content checks and empty sloppiness, and
enables direct mode. The driver sets `base_dir` to each role's root, so paths are relative and
worktrees share entries. PCH is disabled under ccache; `--no-ccache` gives an ordinary PCH
build, and `--ccache-bypass` keeps the no-PCH flags without reuse for measurement.

CI cache compatibility prefixes use OS, architecture, baseline revision, compiler and ccache
versions, and the `migration/ccache.conf` hash. The cache action adds the commit SHA to the
exact key and restores by compatibility prefix. ccache hashes compiler content, arguments and
included files itself; ordinary workflow/driver edits do not change the prefix (#79). Successful
`rust-migration` pushes save the cache, and PRs restore it. Windows and macOS keep vcpkg
binaries in an actions/cache files store keyed by runner image; vcpkg's ABI hashes make stale
entries miss. Their Rust jobs also compile through ccache 4.14.1 (checksummed download) with PCH
off; MSVC uses embedded `/Z7` debug info (CMP0141), which ccache requires. Platform CI also runs
on `rust-migration` pushes; queued pushes coalesce, so it checks the newest merged tip.

## Simulation comparison

`python3 tools/migration.py simulate` builds both games, then runs `tools/simulate.py`; CI runs
its default set after the comparisons. Shared decoding, comparison, execution and reporting live
in `tools/simulation/core.py`. Add each port's scenario dictionaries and prepare/check hooks to
its component module in `tools/simulation/`; `scenario_modules()` assembles the families.
Optional install/game-argument and AI-selection hooks keep scenario setup in that module. It is
the evidence for game-logic ports. Each scenario runs the reference and the candidate headlessly
in isolated directories under `.local/simulation/`, twice: with `-d desync=3`, which writes an
uncompressed snapshot every 32 economy days, and plainly, because desync mode also rebuilds
caches every tick and takes YAPF's uncached rail path. Both write an exit save. Every chunk is
decoded (tables field by field from the stored header, other chunks byte by byte) and the first
differences are reported as `chunk/element/field: ref -> cand`. Any `[desync:` warning (a cache
mismatch), snapshot-list/end-moment mismatch, or complete log/stdout difference fails. Both
modes compare their exit saves.

- Scenarios: both regression saves with their AIs; generated maps (TGP and original, sizes,
  seeds, disasters on); and `play-*`, road networks built by LLM players
  (`migration/saves/README.md`), each loaded with manual distribution and with cargodist (short
  link graph intervals). The saves' `console` lines go to `scripts/game_start.scr`. `--soak`
  adds more; `--self` compares the reference with itself; names filter. Extend `scenario_list()`
  for ports. The owner-built #86 save exercises signalled trains and a ship
  (`tools/simulation/rails.py`), requiring movement on every run and carried cargo/delivery
  revenue at desync checkpoints. The later owner save (`play-padhattan-ridge-2000-*`, #179)
  retains the 1996 input and adds trains, ships, helicopters and a plane, with
  movement/cargo/delivery checks. Saved subsidies and PBS signals do not establish all
  multiplier/route branches. The supplemental reference-built aircraft fixture
  (`tools/simulation/aircraft.py`) observes a small plane and helicopter between two airports,
  requiring movement, loaded cargo, flying state and paid delivery.
  `tools/aircraft-scenario-ai/README.md` records its setup details, reproduction commands and
  limits.
- Masks (`MASKS`, with reasons and hit counts in the report): the random save id, build
  revision/NewGRF version, and `round_trip_time`, which the original saves uninitialized (#83);
  ports touching it need their own check.
- Port divergences go in `KNOWN_FAILURES` by first divergence and issue.
- Cited branch witnesses (#198): one Rust facility (`witness.rs`) holds named counters for every
  port; hits never cross into C++, and `OPENTTD_WITNESS` makes `src/rust/witness.cpp` write
  `branch-witnesses.json` at exit. A module's `game_environment` sets it when
  `scenario["witnesses"]` is set (distinct candidate, not `--benchmark`), and its `check` calls
  `core.branch_witnesses` with a counter prefix; a missing file or zero required branch fails.
- Games launch on Linux as an unprivileged user with child-only `RLIMIT_NPROC=0`. The original
  `StartNewThread` failure paths compute link graphs and write saves synchronously; graph
  scheduling, join dates and loaded LGRJ jobs remain unchanged. This prevents late workers from
  consuming null driver iterations while the simulation is paused. A thread-creation probe
  rejects hosts where privileges bypass the limit. Other platforms are not supported by this
  offline comparison policy. There are no timing retries or truncated logs. Active GameScripts
  remain outside the corpus.
- Both executable/runtime trees are copied before scenarios, dereferencing data symlinks; the
  reference copy holds its shared build lock. Keep the candidate build idle during its initial
  copy; later builds cannot change the test run. `--candidate` uses data beside that executable.
  Reports retain semantic comparisons; `candidate_commit`/`candidate_status` describe the
  initial checkout (#97). Reports go to `.local/simulation/<time>-<pid>/report.json`.

Simulation speed (#155) is reported for every scenario as `plain_speed`: paired process wall
seconds and candidate/reference ratios, plus their median. Ratios are null when exit saves are
missing, runs fail, end moments differ, or semantic differences remain; timing never changes the
pass/fail result. Use profiling counters off, the same build settings and pinned
manual-distribution play save:

```sh
python3 tools/migration.py simulate play-opus-55-167-002-manual --benchmark 3 --jobs 2
```

The driver builds with two jobs, then runs one scenario worker. Benchmark mode runs only plain
pairs, shares the clone-wide coordination lock with other harness games, and compares each exit
save and full logs. Both roles use #154's serial offline thread-failure fallback, recorded as
`execution` in the report. These are total offline times, not normal threaded frame latency. It
retains the final pair and runtimes; `plain_commands` supplies arguments for optional `perf
record` replay through `tools/simulation/game_launcher.py`. Preserve the run's HOME/XDG
directories and runtime libraries; keep profiling separate from timing samples. Timings include
startup, loading, serial link-graph work and save I/O. Benchmark games wait directly for child
exit with a timeout watchdog; ordinary runs retain subprocess timeout polling, which can add
about 50 ms. Shared coordination does not reserve a quiet host. Wall medians are taken only in
speed passes, with no builds, links or other timed games running. Compile load moves P2016
medians 2-7% and concurrent timed games about 10% either way. A speed pass takes four medians
per scenario separately from instruction profiles and establishes complete anchors.

`python3 tools/migration.py simulate --instructions --jobs 2` profiles the six speed scenarios
with witnesses off, `-r 64x64`, and exact Callgrind selectors `StateGameLoop()` and, for
generation, `_GenerateWorld()`. Never use a wildcard: it also toggles off collection in the
nested link-graph pause helper. These entries exclude loading/saving and early generation setup.
The driver explicitly reconfigures and verifies effective release `line-tables-only` debug
information. Completed profiles supply Ir by its `events:` position; canonical exit-state and
full-log comparisons still run, including on reference-count cache hits. Profile elapsed times
never enter `plain_speed` samples. Only reference Ir is cached; identity covers executable,
runtime files (not their CMake bookkeeping)/libraries, resolved inputs, build/profile settings,
architecture and Valgrind version. Candidate executable identity stays separate from the
compatibility identity checked against anchors. Identity hashes the `ldd` libraries, so the
current anchors match only with the main checkout's pinned zlib and lzma runtime: prefix profiles
and speed passes with
`LD_LIBRARY_PATH=<main>/.local/tip-profile-anchored-runtime-zlib-20261023/runtime/usr/lib/x86_64-linux-gnu`.

A speed pass runs `tools/simulation/speed.py` from a clean checkout of the head it anchors.
Build and profile with `simulate --instructions`, then run `python3 tools/simulate.py speed
prepare DIR --profile REPORT` (frozen runtimes and identity proof, no games) and `python3
tools/simulate.py speed collect DIR`. Collection stops other builds, links, profiles and games
only while its 144 interleaved timed games run, records load and process observations without a
CPU threshold, and prints the `SPEED_PASS` rows after release.

`SPEED_ANCHORS` in `tools/simulation/instructions.py` holds each measured anchor `{head, ratio, Ir,
identity}` and a separate `integrated` record with its head and Ir. Empty anchors mean
collection only, never a passing speed guard. Each speed pass fills them from its same-head
quiet medians. Compute each projection as `R0 * (1 + 2 * max(0, I / I0 - 1))`, directly from the
fixed anchor. The ceilings, profiling cadence and speed-pass trigger are in AGENTS.md "Speed".
PRs run no speed check and never edit `SPEED_ANCHORS` or `SPEED_INTEGRATED` (user decision,
2026-10-22). Quiet passes replace the whole anchor and refresh `integrated` together. Never
reset anchor Ir alone. Profiles run alongside other agents. Factor 2 is provisional: measured
wall sensitivity to loop Ir ranges from 0.75 (TGP) to 1.54 (P2016) (#356). Memory stalls,
excluded work and kernel/I/O costs remain outside the proxy.

## Native macOS arm64 Rust linkage

CMake verifies the pinned `rustc -vV` host against the actual C++ platform, architecture and
64-bit pointer width, then passes an explicit Cargo `--target`. On macOS it requires exactly
`CMAKE_OSX_ARCHITECTURES=arm64` and a deployment minimum of 11.0 or newer, with the same
resolved SDK and minimum supplied to Rust through `SDKROOT` and `MACOSX_DEPLOYMENT_TARGET` (see
Rust's [Darwin target
documentation](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html)). Intel
packaging, additional Windows CRT modes and Emscripten host/target builds remain separate tasks.

Archives live at `<build>/cargo/<validated-target>/release/libopenttd_kernels.a`.
`tools/migration.py` exposes `rust_configuration(build)` and `rust_archive(build)`; all
comparison consumers use this cache-validated lookup. Imported `HOST_BINARY_DIR` tools remain
previously built executables and do not consume the target archive.

The pinned compiler's `--print=native-static-libs` output supplies final link flags (see
[static-library linkage](https://doc.rust-lang.org/reference/linkage.html#linkstaticlib)).
Configuration retains `rust-toolchain.txt` and `rust-native-libs.log`; archive builds retain
`rust-build.log`. A content-stable `rust-build-configuration.txt` dependency records the
compiler, target, SDK, minimum and relevant build flags. A changed configuration invalidates
only that target's release crate; unchanged reconfiguration preserves the archive. Native CI
checks unchanged reconfiguration and restoration around a deployment-minimum change on macOS
or a Cargo profile change on Windows.

The macOS ARM jobs activate Rust through the reusable workflow's `rust` input. Debug enables
`OPTION_USE_ASSERTS`: game/tests get `WITH_ASSERT`, while generators keep ordinary C++
assertions with neither `WITH_ASSERT` nor `NDEBUG`. Release defines `NDEBUG` for every consumer.
Both run the four Cargo checks, nonempty CTest inventories and scripted regressions, then build
and execute fresh native tools. `tools/check-rust-platform.py` requires WITH_RUST and the
original assertion flags in every game, test and generator compile command, matching
configure-time and archive native libraries, and the freshness trial above. A WITH_RUST consumer
cannot link without the archive, so linkage and architecture follow from the build. Linux
results alone do not establish Darwin support.

## Native Windows MSVC Rust linkage

Issue #33 adds one native Windows mode: VS 2022 MSVC, x86 or x64, single-config Ninja,
`RelWithDebInfo`, `OPTION_USE_ASSERTS=ON`, and the static release CRT. The C++ compiler's
architecture macros and pointer width select `i686-pc-windows-msvc` or `x86_64-pc-windows-msvc`;
the pinned Rust host is recorded separately, so an x64 Rust host does not choose the game
architecture. The architecture jobs install the exact target standard library and run the four
Cargo checks with an explicit target for target-dependent commands.

`WindowsRust.cmake` sets `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded` before creating any game,
test or generator target, including the tools-only path; CMP0091 is NEW before the first
`project()` call (see the [CMake runtime
property](https://cmake.org/cmake/help/latest/prop_tgt/MSVC_RUNTIME_LIBRARY.html)). The rustc
native-library query and Cargo both use `-C target-feature=+crt-static` (see Rust's [CRT
documentation](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes)).
The archive is `<build>/cargo/<validated-target>/release/openttd_kernels.lib`; the shared
locator validates target, pointer width, CRT, flags and path. The configuration stamp includes
these settings, so a code-generation change and its restoration rebuild the archive.

Debug/debug CRT, dynamic CRT, other build types, assertions disabled, non-MSVC,
multi-config/non-Ninja, ARM/UWP/MinGW, cross-OS and external `HOST_BINARY_DIR` configurations
remain unsupported for Windows Rust (#3). Conflicting runtime flags, Rust target overrides,
mismatch suppression or missing target std fail configuration; the C++ fallback is never
selected implicitly.

The unchanged CMake ordering gives Windows RelWithDebInfo game/tests both `NDEBUG` and
`WITH_ASSERT`, while generators have `NDEBUG` alone. The x86 build exposed six existing
narrowing assignments (station expansion, snow-line calculation, map-height selection, old-save
station loading); explicit casts to their existing unsigned destinations keep the original
modulo conversion. All exports keep `extern "C"`, which is cdecl on i686 ([MSVC target
ABI](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)). Rust copies foreign
encoded descriptors with `read_unaligned` before taking references, closing the [MSVC i686
alignment gap](https://doc.rust-lang.org/rustc/platform-support.html).

`tools/check-rust-platform.py` also requires only `/MT` in every Windows Rust consumer's compile
command and no dynamic C/C++ runtime import, and runs each unsupported configuration above as a
refusal smoke test. It checks the ordered native library arguments, including duplicates, in
each Ninja link command and its response files. CTest runs the native ABI executable
(`rust-abi-layout-and-calls`): struct
sizes, alignments and field offsets for every `abi.rs` layout, high-bit scalars, by-value
returns, sentinels, null/empty inputs, Rust allocation/view/destroy paths, history-engine staging
and a per-thread non-C locale lowercase check (signed negative compare arguments stay outside the
defined C domain, #26). Fresh tools-only builds run the current string and settings generators
after deleting previous outputs.

## Continuous integration

Server-side protection on `rust-migration` requires `Full validation`, the cheap
commit/string/script-mode checks, and resolved conversations. Full validation is an explicit
commit status, published only after the native comparisons, simulation, platform matrix except
macOS, quick checks and annotation checks succeed for a captured PR merge commit, with the
status published on its head. Skipped or cancelled jobs cannot satisfy it. Workflow dispatch
runs the controller from protected `rust-migration`; candidate test jobs have read-only
permissions and the status publishers never execute candidate code. Required checks must pass on
the PR head and the branch must be up to date with base (`required_status_checks.strict=true`).
The publisher still records the immutable tested merge tree separately from the head status; its
green status alone is insufficient when the branch is behind. Ordinary force pushes and branch
deletion are disabled. The approving-review count is zero because agents share credentials; the
attributed independent review report is the process gate before root integrates. Repository
controls are enforced separately from these documents.

Ordinary PR pushes run cheap checks. With `gh` authenticated to the fork, use:

```sh
python3 tools/ci.py request 123 --profile rust
python3 tools/ci.py request 123 --profile native
python3 tools/ci.py request 123 --profile platform
python3 tools/ci.py request 123 --profile full
python3 tools/ci.py wait .local/ci-requests/REQUEST.json
```

`rust` runs four Cargo checks; `native` adds the Linux matrix and full migration comparison;
`platform` runs the platform matrix including macOS. All include quick checks. Only `full`
satisfies the merge gate. Unmergeable or pending-mergeability PRs are refused. The CLI records
the SHA and request/run identity and refuses a changed PR head; a changed head, or a base that
advanced, needs a new full request. `CI_ON_DEMAND=true` enables this scheduling after root
installs and verifies the required status. With the variable absent, automatic full CI remains
as a safe bootstrap. For a workflow-changing PR, root reviews its workflow definitions,
temporarily unsets `CI_ON_DEMAND`, and requests `--profile full --bootstrap` (the `ci:full`
label path tests the PR definitions). Root restores the variable to `true` after merge and
records both changes in the PR; never use `--admin`.

Protected pushes queue without cancelling a running validation; a newer push replaces a pending
one, so a burst of merges validates the newest tip and the intermediate pushes show as
cancelled. Markdown-only pushes do not start these workflows, so they cannot displace a pending
code push. A merged push whose non-Markdown files match the tree marker from the same successful
full request refreshes native compiler caches without repeating tests or scenarios. Covered
protected pushes also rebuild the cached Linux GCC and Windows targets to refresh their compiler
caches. This path skips Cargo checks, CTest, comparison/configuration probes, annotation checks
and platform evidence; Clang and Emscripten have no compiler cache to refresh. The path is
derived only from the protected push's matching non-Markdown tree entries, and requested PR
validation always runs the complete selected suite. A combined tree no full request covered
receives all native and platform checks. Tree comparisons must be complete; missing or truncated
GitHub inventories retain heavy validation. A Markdown-only push that still starts (GitHub runs
path-filtered workflows when it cannot compute the diff) skips heavy builds only after complete
classification of fewer than 300 changed files. Post-merge checks remain a backstop for missed
defects; they do not authorize untested combinations.

Native validation exports both frozen runtimes and once-built fleet/orders adapters, then runs
component comparisons and three simulation shards concurrently (each about as long as the
comparisons job). Recorded scenario seconds assign work while keeping effects/disasters/aircraft
fixture families whole. The gate requires every job and exactly the current default `simulate.py
--list` inventory. GitHub's run-scoped artifacts carry the runtimes and reports between jobs.
Exports strip debug sections only on copies; original verification binaries stay intact. Probe
workers receive prebuilt generators/archives and the widget probe's real link objects; build
metadata is relocated for the receiving checkout. Reports/logs and retained failing saves are
artifacts; build inputs expire after one day. Tests and their results are never cached. Cache
writers trim native roles to 750 MB, Linux flavours to 600 MB and macOS flavours to 650 MB. The
trusted cache-retention workflow keeps the latest entry per prefix and bounds total usage to 8
GiB (below the 10 GiB repository limit).

Runner settings are `CI_LINUX_HEAVY`, `CI_LINUX_COMPARISONS`, `CI_LINUX_SIMULATION`,
`CI_LINUX_PLATFORM` (default `ubuntu-24.04`; Clang only, the cached GCC flavours always use free
`ubuntu-24.04`), and `CI_MACOS_HEAVY` (default `macos-latest`) and `CI_WINDOWS_HEAVY` (default
`windows-2022`). Heavy native jobs use `nproc` slots. One full request peaks around 13 jobs;
overlapping requests may queue against account/runner capacity. Runner size and account plan
remain repository settings. The repository belongs to the `fetch-laboratories` organization,
where Blacksmith serves the Linux jobs: `CI_LINUX_HEAVY` is `blacksmith-8vcpu-ubuntu-2404` and
the other Linux variables are `blacksmith-4vcpu-ubuntu-2404`. Windows uses
`blacksmith-4vcpu-windows-2025`, sized to its `--parallel 4` build; macOS and light jobs stay on
GitHub-hosted runners (free for this public repository; Blacksmith bills macOS at 20x).
Blacksmith serves `actions/cache` from its own co-located store on its runners, so the GitHub
cache budget above covers only GitHub-hosted jobs. Unsetting a variable returns that job to
GitHub with no workflow change. Profile CI time with the Blacksmith CLI (`blacksmith jobs
list|steps|stats`, `jobs aggregate`, `jobs diagnose rightsize`) before changing runner sizes or
sharding: `stats` gives each Blacksmith job's CPU and memory timeseries. Pass `--job-name` to
`aggregate` and `diagnose rightsize`: each full request names its run `CI full PR N <hash>`, so
grouping by workflow leaves one run per group (`insufficient_data`). Size by the busy step, not
the job average: compile and scenario steps saturate every runner, and setup steps idle at about
one core. Agents authenticate with an org token (`blacksmith org-token create`, admin only),
never a copy of a person's `~/.blacksmith/credentials`.

macOS runs on `rust-migration`, not per PR (#357). After every fifth integration root runs `gh
workflow run ci-build.yml --ref rust-migration`, which builds and tests only the two macOS jobs
at the tip and refreshes their caches, and waits with `gh run watch`; a red result is fixed
before more integrations. An uncovered protected push also runs them. After two hosted-capacity
cancellations of a macOS run, root runs `gh variable set CI_MACOS_HEAVY --body
blacksmith-6vcpu-macos-26`, reruns, and runs `gh variable delete CI_MACOS_HEAVY` when that run
finishes; setting the variable needs no Blacksmith credentials.

`python3 tools/preflight.py --base origin/rust-migration` runs the pinned inherited commit
checker. Its first run downloads the pinned hooks into ignored shared storage; `--hooks PATH`
uses an existing clean pinned checkout offline.

## Shared-code checks and port metrics

`python3 tools/shared-accessors.py --base-ref origin/rust-migration` enforces
#249 in quick CI. It checks fork-added functions/lambdas outside the named shared
service files for single shared-world reads or shared-helper forwarding. The shared files
include the canonical `engine_services.h` and `presentation_services.h` (#248); review verifies
their consumer adoption and deletion of port copies. Existing definition fingerprints are in
`tools/shared-accessor-baseline.txt`: remove retired entries, never add exemptions. CI compares
that baseline with the PR base or prior protected-branch commit.
`tools/test-shared-accessors.py` runs CLI category and ratchet regressions by default. This
small syntactic check covers typed parameters, cast/lookup/typed owner aliases, settings,
timers, map reads and named helper calls; it does not replace review of macros, derived-type
fields or compound callbacks.

Original header helpers and constants live once in crate modules named after the header
(`slope_func`, `map_func`, `tile_type`, `company_type`, `command_type`, `direction_type`,
`economy_type`, `newgrf_callbacks`, `strings_type`, `viewport_func`, `town_map`; #261). Ports
import them instead of keeping copies, and `src/rust/services.cpp` asserts every value. The
checker cannot see Rust copies.

`python3 tools/port-metrics.py` reports the change in retired pinned-original C++ lines
separately from fork glue added/removed and signed net glue. Original retirement can be negative
when original lines are restored; removing fork glue does not retire original C++. Ownership
ports retire more original C++ than net glue plus tooling added, or explain why; conversions
report their net glue. `--whole-fork` prints totals against the pinned original and the retired
share of simulation source lines. Its one inventory covers world-state owners, commands,
accessors, link graph and pathfinding, excluding GUI, platform, vendored code and generic
support. Mixed owner files, blank lines, comments and data tables count. The diff/guard
classification measures source lines, not C++ preprocessing or behavioral completion. Historical
roadmap rows keep their labelled legacy counts.

## Team process and integration

`AGENTS.md` is authoritative for agent roles, models and reasoning effort, attribution, the
issue/PR/review flow, the evidence budget, the exact-head Full gate and integration rules. This
section adds only repository facts. `.codex/config.toml` sets root to `gpt-6.1-sol` xhigh and
spawned agents to `gpt-6.1-sol` high by default, with at most five spawned threads; explicit
spawn settings select the required role, and a running host may impose a lower limit.

`docs/skills/` holds three user-invoked skills: `/steer`, the user's steering review;
`/start-development`, root's restart prompt; and `/wind-down`, which writes the user's message
pausing root. Steering reviews put their corrections in the roadmap and edit
`/start-development` only to improve it; root follows these skills and leaves them unedited.

For unattended provider handoff, establish an independently executable takeover before the
source provider exhausts its allowance. A scheduled prompt that needs the exhausted provider to
launch its replacement is not a fallback. Verify the host's actual execution provider; naming
another model in the prompt is not enough. If the host cannot provide independent takeover,
record that limitation and preserve a checkpoint before stopping. This rule creates no schedule
itself.

Shared append lists group by component (reserve sections for concurrent ports), then sort
entries by name: includes after stdafx/before safeguards, independent ABI probe calls, and
shared table fields/mirrors/initializers/layouts (one field per line). Witness blocks are
alphabetical; keep names and consecutive local indices together in `witness_blocks!`; global
indices/array lengths are derived.

At an integration opportunity, batch two or three ready, reviewed component heads if available;
integrate a lone ready PR without waiting to fill a batch. Assemble an integration PR from
current base, preserving component commits and linking their reviews. Review only substantive
conflict resolutions under AGENTS.md; unchanged components do not need another review. Request
Full on the combined PR, without requiring separate component Full runs first. Finish routine
base-doc updates before requesting Full. Freeze batch membership and coordinate root sessions to
avoid base pushes during the run; implementation and review continue. Any head, membership or
base change needs fresh Full, including removing a failing component. Merge only the passing,
current head with `--match-head-commit`, then record and close the included component PRs. Keep
batches small: the integration merge is the automatic rollback unit. A stack organizes dependent
PRs but does not itself combine validation runs. No extra local integration build or full local
simulation run is required.

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

## Selecting components

Selection follows [AGENTS.md](../AGENTS.md) and the [roadmap](roadmap.md).

## Ported components

The entries below record the components ported so far. The earlier ones were selected under the
previous coverage-first rule and are mostly utility kernels. Each entry names what Rust owns,
what stays in C++, open limits and the PR that records its checks; branches the harness does not
reach are tracked in #156.

Unless an entry says otherwise, these properties hold for every port:

- With `OPTION_RUST=ON` (`WITH_RUST`) the C++ facade calls Rust; with it off, the original C++
  bodies compile unchanged. Rust on further platforms is #3.
- Rust panics and Rust allocation failure abort (`panic = "abort"` in both profiles); the C ABI
  never unwinds, and no C++ exception crosses a Rust frame. Identical resource-exhaustion timing
  is not claimed.
- Borrowed byte spans are initialized, readable bytes in one live allocation with length at most
  `PTRDIFF_MAX`, immutable during the call; empty spans may be null; read-only spans may
  overlap. Rust retains no pointer after returning.
- Unchanged upstream tests cover each port first; named comparisons cover only listed gaps. C++
  sanitizers do not instrument Rust accesses. These checks establish neither whole-game
  equivalence nor cryptographic security or complete secret erasure.


<!-- Subsystem: Support libraries and text. Add a new entry at the end of its group, not of the file. -->

### StringConsumer integer parsing

C++ keeps the public templates, cursor updates, optional/pair/string-view adapters and formatted
logging. Rust owns base selection, digit scanning, width-aware conversion, overflow/clamping and
lexical skipping. The ABI borrows arbitrary bytes and returns scalar parse metadata; C++ formats
diagnostics and commits the cursor. Checks: the unchanged StringConsumer cases and `python3
tools/run-comparisons.py`, which includes the integer parser and builder comparison (width
boundaries, negative hexadecimal, malformed prefixes and fatal logging). PR #9.

### Alternating-iterator traversal

Rust owns initial position/selectors, logical advancement, side selection, end transitions and
position comparison. C++ keeps the typed iterators, range identity assertions, dereferencing and
container lifetimes. Each increment recomputes the live range distance; Rust requests a typed
move, C++ applies it and queries only the requested live boundary, then Rust completes the
next-side state. Size is not cached and random access is not required; stable noncontiguous
iterators remain supported after insertion. `src/rust/alternating_ffi.h` maps size_t to usize
and explicit uint8 selectors (0 before, 1 after). Nonnegative distances keep the original size_t
conversion with no added range cap. No pointer, allocation, element or ownership crosses the
ABI. Logical end skips movement/completion and keeps the last selected Base iterator, while a
separately constructed end keeps middle; they compare equal by position. Checks: the unchanged
fixed sequences plus public-interface tests. PR #13.

### StringConsumer byte algorithms

`consumer.rs` owns exact unsigned little-endian assembly; bounded read/skip lengths and
shortfall/cursor decisions; byte-prefix matching and conditional consumption;
substring/character-set search and membership; and separator result/consumption decisions. C++
keeps typed optional/default conversions, string_view construction, diagnostic formatting,
cursor commit and trivial accessors, and preserves empty view pointers through the original
substring at the current offset. `src/rust/consumer_ffi.h` returns scalar `repr(C)` metadata by
value. Nonempty search/set/separator assertions remain preconditions; release behavior outside
them (including the original empty-separator loops) is not claimed. The in-place pair, owning
string/container code, allocators, escape parsing and encoded-string transformation remain C++.
SQFile resizes its own buffer and rebuilds the consumer; no Rust borrow survives that boundary.
Checks: the unchanged consumer tests and `python3 tools/run-comparisons.py`. PR #18.

### StringBuilder numeric byte encoders

Rust extracts the bytes for `PutUint8` and `PutUint16LE`/`32LE`/`64LE`, and formats integral
`PutIntegerBase` values in bases 2 through 36: lowercase digits, a leading minus for signed
negatives, no base prefix, one digit for zero. Signed binary wrappers keep their modulo-width
unsigned casts; the signed magnitude uses unsigned negation, including `INT64_MIN`.
`src/rust/builder_ffi.h` takes scalars and returns `repr(C)` byte arrays by value; C++ passes a
span of the returned local array synchronously to its virtual `PutBuffer`, and sinks must not
retain it. Invalid bases are outside the original 2..36 `std::to_chars` precondition. Checks:
the unchanged StringBuilder, InPlaceReplacement and encoded-string tests, plus the integer and
builder comparisons in `python3 tools/run-comparisons.py`. PR #17.

### Encoded-string compatibility and parameter rewriting

Rust owns `FixSCCEncoded`, `FixSCCEncodedNegative`, `EncodedString::ReplaceParam` and the shared
`GetEncodedStringWithArgs` serialization. C++ keeps the save-version dispatch and its order
(legacy encoding before version 350, old markers before 169, negative repair before 353, then
sanitation), general decoding, rendering, ScriptText encoding and sanitation. Retained quirks:
legacy conversion is permissive (old E028/E02A normalize only with fix_code; markers are
recognized inside quotes; quotes toggle and disappear; quoted colons stay bytes; numerics are
not validated). `src/rust/encoded_ffi.h` passes explicit tags (0 monostate, 1 uint64, 2 byte
span), never C++ string/vector/variant layouts; StringID width and the RS/E000..E003 token
contract are asserted. Checks: the unchanged FixSCCEncoded/Negative and ReplaceParam tests, plus
`python3 tools/encoded-comparison.py` (verbatim pinned functions, four NDEBUG/WITH_ASSERT
combinations, C++ sanitizers; the Rust archive is uninstrumented). PR #24.

### Byte-string utilities

Rust (`byte_strings.rs`) owns case-insensitive compare/equal/prefix/suffix/contains, lowercase
conversion, uppercase hex encoding, sequential hex decoding and byte-set trim scanning. C++
keeps std::string owners, views, erases and the installed standard library's equal-prefix
length-comparison policy; Rust returns that C++-supplied scalar only when the shared prefix is
equal. Rust calls native C `toupper` with the original char promotion (signedness supplied by
C++) and `tolower` with unsigned-byte promotion, keeping process-locale behavior. Negative-char
uppercase inputs other than EOF are outside portable C's domain; only observed native behavior
is matched (#26). Lowercase takes an exclusive mutable span with offset at most size. Only the
portable natural-contains fallback uses these helpers; ICU, Windows/macOS collation, validation,
in-place replacement and StringIterator backends remain C++. Checks: the unchanged utility tests
and `python3 tools/byte-strings-comparison.py` (full pinned string.cpp, length saturation above
INT_MAX, -funsigned-char builds). PR #27.

### UTF-8 codec and byte positions

The full UTF-8 codec, `IsUtf8Part`, forward/backward iterator stepping and `GetIterAtByte`
normalization run in Rust. Inline DecodeUtf8/consumer PeekUtf8 handle leading ASCII (including
NUL) without an FFI call. The C++ view keeps its borrowed string_view and iterator facade, pair
adapters, comparison assertions, postfix copying and invalid-data `?` dereference. Retained
behavior: surrogates are accepted; overlong and out-of-range first sequences are rejected;
malformed trailing data after a valid first sequence is ignored; unused encoding bytes are
zeroed. `src/rust/utf8_ffi.h` uses 32-bit codepoints and size_t lengths/offsets and returns
`repr(C)` data by value, with no allocations or output aliases. Checks: the unchanged UTF-8
view, consumer and builder tests and `python3 tools/utf8-comparison.py` (assertion and NDEBUG
builds, including the `offset >= size` end branch). Game logging and Unicode rendering are
outside it. PR #10.

### Rounded square root and runtime integer saturation

`IntSqrt(uint32_t)` and runtime `SoftClamp` call `math.rs` through `src/rust/math_ffi.h`.
`IntSqrt` keeps nearest-integer rounding, including 65536 for UINT32_MAX. `ClampTo` uses the
original inline C++ template (#276); its hot callers avoid an ABI crossing. `DivideApprox`
remains C++: its potentially overflowing signed intermediates need separate work. The ABI is
scalar-only: modulo-2^64 value bits plus explicit width and signedness; C++20 integral
conversion rebuilds the result. Reversed signed 8/16-bit SoftClamp intervals convert min to
unsigned and then promote to int, so `SoftClamp<int8_t>(0, -1, -3)` returns 126. Checks: the
unchanged IntSqrt, ClampTo and SoftClamp cases and `python3 tools/math-comparison.py` (bounded
corpus with GNU link wrapping on Linux, wide templates on the native library; macOS CI runs the
wide templates with `--build build`). PR #36.

### Generic history structural engine

Rust (`history.rs`) owns descriptor-driven validity, rotation scheduling and query traversal.
HistoryRange constexpr construction/layout, typed HistoryData storage, every SumHistory
specialization, graph fillers and GetAndResetAccumulatedAverage remain C++; production averaging
keeps its literal-0 int accumulators and nested reduction grouping. Rust keeps an
arbitrary-depth scalar frame stack and streams staged operations; C++ describes immutable ranges
by value and executes every typed operation after Rust returns. Unsigned index arithmetic wraps
at uint32, and GB's uint32 truncation remains for uint64 validity masks. Retained quirks: update
and rotation use explicit cur_month while queries read the live TimerGameEconomy::month;
children update/rotate first even for saturated or skipped parents; no-prerequisite higher
rotations still shift; query validity ORs all children while IsValidHistory checks only the
first; invalid children still contribute data; invalid query ages keep the original fatal
dispatch. Checks: the unchanged 288-month test and `python3 tools/history-comparison.py`, which
includes the three verbatim production reducers. PR #32.

### Authentication and streaming owners

Rust owns X25519 session and encryption-context ownership behind a versioned, primitive-only
host function table. Bundled Monocypher algorithms are unchanged. Rust owns stable key/session
allocations and vendor-context storage; C++ supplies each vendor context's size/alignment and
starts its trivial lifetime, so Rust mirrors no vendor struct and other archive consumers gain
no vendor symbols. Secrets are initialized in their final allocation; copies go heap to heap and
assignment overwrites fixed storage, as in C++; rvalue copies preserve the source. Streaming
contexts keep the bundled successful-rekey behavior (the counter does not advance); failed
authentication leaves context and output unchanged. Borrowed fixed-width views keep their
address until owner destruction; callers serialize access. Checks: the unchanged network cases
and `python3 tools/auth-comparison.py` (pinned and candidate endpoints in both mixed directions,
with a C++ sanitizer run that does not instrument Rust memory accesses). PR #43.

### Paired Script Admin conversion

`ScriptAdminMakeJSON` and `ScriptEventAdminPort::GetObject` keep their C++ interfaces. Rust
selects types, walks both conversion directions, propagates results and schedules the original
VM and JSON operations; an opaque per-invocation handle returns scalar actions that C++ executes
after each call returns. C++ keeps the bundled VM, JSON objects, network framing and exception
cleanup. No VM/JSON layout crosses the boundary, and no Rust borrow survives a reentrant host
operation. Outgoing: `depth == 25` is checked before reading the VM type or changing JSON (also
for an explicit initial depth); live iteration, key stringification, duplicate-key overwrite
order, `index - 1` and `depth + 1` are kept; a failed child leaves completed root children, with
the original VM and iterator pops before cleanup. Incoming: object root only, floats rejected,
no depth limit; failure restores the stack top before logging and pushing null; malformed input
keeps the original diagnostic. Checks: the unchanged `test_script_admin.cpp` cases and `python3
tools/admin-conversion-comparison.py` (bundled VM, unchanged pinned bodies at O0/O2). Game-log
storage, arbitrary depths, network simulation and complete script equivalence are outside it. PR
#45.

### ScriptList storage and iteration

Rust owns both item/value indexes, all four sort modes, live pending-cursor and end state,
mutation accounting, filters and list algebra. C++ keeps script identity/bindings, pool
enumeration, VM/error/operation charging and save/load adapters. The scalar/pointer ABI avoids
aggregate returns on 32-bit hosts; items/values are signed 64-bit and modification tokens signed
32-bit. Rank arithmetic and modification tokens wrap like the compiled original (#260). Owner
borrows end before VM reentry; callbacks that invalidate iterators while evading the
modification check remain outside the checks. Checks: unchanged `regression_regression` and
`regression_stationlist`, `python3 tools/script-list-comparison.py` (original, Rust and portable
binaries at O0/O2 with the bundled VM) and a Rust unit test of the token-wrap boundary. PR #46.

### ScriptList VM control

Rust owns `Valuate`, VM-filtered `FillList`, `SaveObject`, `LoadObject`, `_get`, `_set` and
`_nexti` control in a scalar controller. C++ executes VM actions, live typed pool reads,
diagnostics and native command/operation-limiter RAII scopes. Valuation touches before
validation, checks type before modification, and commits before pop/charge. Filtering keeps
function/scope timing, validity-before-callback, live index rereads after callbacks, bool-only
results and its original throw type. Original failed-getter undefined values (#67),
unrepresentable conversions, signed overflow and invalidated iterators remain outside defined
behavior. Checks: both unchanged scripted suites and `python3 tools/script-list-comparison.py`
against pinned and portable C++, plus identical generated AI/GameScript/template bindings. The
fixture uses four typed items; production pool/CPU-limit coverage comes from the game
regressions. PR #70.

### Nested widget descriptor parser

`MakeNWidgets` and `MakeWindowNWidgetTree` use a widget-specific pull parser. Rust owns the
descriptor cursor, attribute traversal, container decisions, recursive tree control, end
markers, complete-consumption policy and first/root/body/shade composition. C++ keeps the
single-part factory, attribute operations, widget classes, RTTI checks, `unique_ptr` owners,
Add/GetWidgetOfType, generator callbacks (which may run a nested parser), constexpr builders and
public signatures. The scalar ABI carries uint64 descriptor offsets, owner slots, uint8 widget
tags and capability observations; no union, virtual object, RTTI layout or function pointer
enters Rust. Retained quirks: push-button bits are not masked for container classification;
function-produced subtrees never adopt following nodes; EOF inside a container is accepted; null
generator results keep the unconsumed cursor and end-marker assertion; the `WITH_ASSERT`-only
trailing-parts exception stays separate. Checks: the unchanged `test_window_desc.cpp` bodies
(every registered WindowDesc through the production parser) and `python3
tools/widget-parser-comparison.py` against unchanged pinned parser bodies. Rendering, layout,
events and arbitrary malformed descriptors are outside it. PR #49.

### ChaCha20, Poly1305 and AEAD primitives

Rust owns the bundled Monocypher 4.0.2 ChaCha20/Poly1305/AEAD family. Its C interfaces and
caller-owned context types stay. C++ starts context lifetimes and passes
size/alignment/field-offset descriptors; raw field access reads only initialized fields,
preserves unwritten chunk/padding bytes at init and wipes the complete caller context at
finalization. The nonthrowing wipe and constant-time verify16 come from a two-function explicit
cdecl table; Rust has no vendor imports or global callbacks. Other Monocypher algorithms and the
portable family remain C++. Generated layout probes cover the two-leaf table and two
field-layout descriptors. Checks: `python3 tools/auth-comparison.py` against pinned vendor
functions and the existing network tests. PR #52.

### BLAKE2b family

Bundled Monocypher BLAKE2b keeps all six public C functions. Rust owns the twelve compression
rounds, keyed/unkeyed initialization, word/block buffering, pending-final-block decisions,
digest serialization and final context wiping. C++ owns the context lifetime and supplies
size/alignment/field offsets; the archive borrows the existing nonthrowing wipe leaf, imports no
vendor symbols, reads no context padding and adds no heap allocation. The supplied hash size
stays in the initial parameter; final output writes `min(hash_size, 64)`, keeping defined source
behavior at sizes zero and over 64. Keys of 65-128 bytes fit the original padded block and keep
source behavior (the documented interface limit is 64); keys over 128 overrun original storage
and have no reproduction guarantee. One-shot output may overlap message/key, including
partially: all inputs are consumed before output writes. Checks: `python3
tools/auth-comparison.py` and the unchanged network tests. PR #57.

### Packet framing, binary serialization and transfer state

Rust owns the native-width limit, the persistently narrowed uint16 cursor, binary
encoding/decoding, length-prefix sequencing, framing offsets and transfer planning/commit
decisions. C++ keeps vector/string storage and each original allocation operation, direct spans,
socket policy, assertions, external encryption/transfer callbacks, Send_string/Recv_string
collection and sanitation, and the exact Recv_bytes callback. The facade keeps the original
per-byte append order and advances each received result byte before its potentially throwing C++
push; parsing commits position two only after resize succeeds. Normal false decryption still
skips its MAC; throwing decryption does not. Transfers commit positive results to the live
post-callback cursor with uint16 narrowing; native unsigned arithmetic wraps as in the original.
Checks: the network/authentication cases, `python3 tools/auth-comparison.py`, both scripted
regressions and `python3 tools/packet-comparison.py` at O0/O2. PR #60.

### Normal X25519 and Montgomery ladder

`crypto_x25519`, `crypto_x25519_public_key` and `crypto_eddsa_trim_scalar` keep their public
interfaces; one coarse `scalarmult` adapter serves the unchanged dirty-small/inverse callers.
Rust owns the complete 255/256-bit ladder and its private ten-limb field decode, encode,
add/subtract, masked swap/copy, multiplication, squaring, carry, inverse-square-root and
inversion. C++ shared-field helpers remain for untouched Edwards, conversions, Elligator and
other families; that internal duplicate is unfinished migration, not shared-field closure. A
separate synchronous immutable cdecl table holds only the original nonthrowing wipe and
constant-time verify32 leaves; no field object, callback registration, application operation,
vendor import or heap crosses the ABI. Original explicit wipe points and order remain; compiler
spills/copies are not erased. The fixed 32-byte output may overlap secret/scalar/point,
including partially: inputs are consumed before serialization. The public-key top bit is
ignored, noncanonical field encodings are accepted and raw zero outputs remain primitive
outputs; authentication rejection policy and RNG/protocol behavior are unchanged. Checks: the
network tests and `python3 tools/auth-comparison.py`, whose direct fixture uses the vendor's
private ladder as the oracle. PR #61.

### Coupled string validation and borrowed in-place replacement

Rust owns the shared StrMakeValid scan/policy decisions, the StrValid fixed-span scan and
InPlaceBuilder write progression/copy. Each sanitation decision carries a consumed count and at
most one existing UTF-8 encoded character; C++ advances the actual StringConsumer before its
original output operation. The historical codec stays authoritative, including accepted
surrogate values. C++ keeps allocation, string owners, cursor commit and fatal dispatch. Rust
retains no buffer pointer; raw forward reads/writes preserve defined left overlap without
overlapping references. Checks: the unchanged InPlaceReplacement case and the UTF-8, consumer
and builder tests, plus `python3 tools/utf8-comparison.py` in its O2 assertions/NDEBUG modes
against unchanged pinned string.cpp bodies. Game logging and Unicode rendering remain outside
it. PR #63.

<!-- Subsystem: Landscape, water and map structures. Add a new entry at the end of its group, not of the file. -->

### TGP terrain generation

Rust owns the temporary height map and every TerraGenesis stage, including height estimation,
noise, histogram, smoothing and regional curves, and the advance/apply loops, writing through
typed map services without a native height-output copy. C++ extracts named settings and retains
shared RNG, canonical map storage and native progress/abort-hook operations. The process-wide
abort owner is borrowed only between progress calls. Both progress operations catch
AbortGenerateWorldSignal in the native handoff; cleanup runs with no live generator borrow, then
the landscape facade rethrows after Rust returns. Default generation cases and the native
cancellation probe compare both TGP progress events and single cleanup. Original bodies remain
portable-only. Maximum-map performance and Windows/macOS runtime parity remain untested. PR #94;
loops moved in #364.

### Landscape scheduler, ground and flooding

Rust controls `RunTileLoop`, clear/rail/road/tunnel ground evolution, flooding behaviour,
half-tile rail removal, dry-up, neighbour wakeups and live flood-victim traversal. The
process-global saved cursor commits after the complete source batch; DATE, legacy loading, reset
and zero repair retain their widths/order. Original selected bodies compile only without
`WITH_RUST`; packed map/pool storage remains native. The in-entry LFSR keeps native unsigned
map-log widths and reads each live type byte from audited allocation metadata. All seven Rust
tile owners dispatch directly, void tiles straight to the water owner, which reuses the
dispatched type and reader and derives `IsValidTile` from the map size; only
house/station/object have native scalar handlers with local strong-index construction; the five
unused facade slots are null. Flooding calls the Rust signal flush through the canonical signal
context. Leaves shared by several owners (clear/tree ground, snow, shore, sound, foundation)
live in shared tables. Tree/industry water loops, tree wakeups, rail flooding queries and road
town-type selection call Rust directly. Ground boundaries use actual native enums, unrestricted
transparent Rust carriers, exact counters and typed station handles; legacy protocols adapt
explicitly and canonical rail leaves have one binding. Each native entry owns independent lazy
stack command-result storage. No hot call allocates a protocol object or copies a complete
native tile/vehicle. Vehicle iterators advance after crash/events/news/effects; airport
membership, bridge endpoint order and company restoration follow source. No ordinary tick
callback unwinds; generation-progress cancellation stays outside tick. The scheduler's
clear/water/tree reads use the #274 shared map descriptor (`docs/design/world-state.md`); C++
retains arrays, allocation and all writes, and standalone water/tree, generation,
planting/command and unrelated owner reads keep canonical typed callbacks, with no descriptor.
Assertions-off raw reads add no bounds, type or field rejection. Checks: `verify`, the default
scheduler comparison and `python3 tools/migration.py simulate landscape roads-flooding`. Matrix,
editor, NewGRF and threaded gaps are in #156. PR #255; descriptor #351.

### Landscape clear, terraform and level (#348)

Rust owns `CmdLandscapeClear` and `CmdClearArea` and all of `terraform_cmd.cpp`: the command-local
ordered height and dirty state (matching `std::map` and `std::set` iteration), recursive height
propagation, both terraform validation passes, the height commit, `CmdTerraformLand` and
`CmdLevelLand`. Integrated tile owners and clear callers dispatch to each other directly in
Rust. C++ keeps the packed map arrays and setters, the unported tile procedures and the
presentation leaves behind the original command facades; the original bodies compile only
without `WITH_RUST`. A shared opaque native stack-cost utility supplies a distinct command
result for each recursive frame without heap allocation. No owner borrow spans a nested command.
Checks: `python3 tools/migration.py verify --jobs 2` and the default `landscape` simulation,
which requires the two-pass, recursive-height, indirect-clear and partial-success witnesses.
Editor, NewGRF and threaded gaps are #156. PR #353.

### Landscape partial-pixel height

`GetPartialPixelZ`, the scalar landscape height kernel, runs in Rust behind its original C++
interface. The ABI takes two `int32_t` coordinates and one `uint8_t` slope; a `uint64_t` result
carries the complete `uint32_t` height domain plus `UINT64_MAX` for the original
unsupported-base-slope `NOT_REACHED` dispatch. No pointers, allocations, shared state or
ownership cross it. C++ asserts widths, tile dimensions and slope/corner encodings. Rust
preserves half-tile returns before base validation, upper-bit clearing, signed arithmetic
shifts, unsigned TILE_SIZE/TILE_HEIGHT promotions and wrapping, and signed-to-unsigned returns.
Coordinates outside the tile remain accepted, including legitimate `UINT32_MAX` heights.
Original signed-overflow inputs have no equivalence guarantee; Rust adds no overflow panic.
Unsafe code is denied except for the export attribute. `python3 tools/math-comparison.py` covers
the out-of-tile arithmetic the harness's callers cannot reach, excluding executed
signed-overflow expressions; its corpus is bounded, not exhaustive over all coordinate pairs.
PRs #6 and #91.

### Spiral tile traversal

Rust owns square/hole initialization, position initialization, per-direction movement, shell
jumps, outside-map skipping, end detection and coordinate-only equality. C++ keeps the typed
iterator/sequence facade, TileXY dereference, copies/postfix wrappers and map storage. Map
dimensions are passed at each constructor and prefix increment; Rust caches none. Orthogonal and
diagonal tile-area algorithms remain C++. `src/rust/spiral_ffi.h` has five pointer-free by-value
operations and a copyable 40-byte state (nine uint32_t fields counting the four extents, then a
uint8_t direction at byte 36; alignment 4), asserted on both sides. Directions are NE(-1,0),
SE(0,1), SW(1,0), NW(0,-1), with west shell jumps (+1,-1). Coordinate, extent, position and
radius arithmetic wraps explicitly at 32 bits, including temporary outside-map coordinates;
there is no new clamp or diameter cap. Positive diameter/radius and increment-before-end remain
original preconditions. End is radius equality with a non-invalid direction; equality compares
x,y only. The huge inherited perimeter after a wrapped extent is not traversed. Native
generators do not use spiral traversal. PR #23.

### Tree generation, simulation and planting

Rust owns generation/scattering/grove geometry, placement policy, tile growth, climate loops,
tick seeding and the persisted byte counter, plus command traversal, limits, costs and errors.
Original bodies compile only in portable builds. Map/pools, rendering and the editor
forest-brush loop remain C++; the brush calls Rust placement. Each entry (tile loop, tick,
generate, scatter, place, plant, command, clear) is one plain call with a by-value settings
record. It reads typed plant/tree records and calls typed `noexcept` services from a designated
`OpenTTDTreeServices` table and the shared map predicates (`GetTileZ` only where the original
calls it) at the original points. Water flooding, the NewGRF ambient callback and nested
landscape clears may reenter; no Rust borrow survives them and the tile is read again
afterwards. World-generation abort is the only handoff: the progress wrapper keeps the
exception, Rust returns with no further draw or write, and the facade rethrows. Save/load
reaches only counter-address/reset exports; DATE/TTD/TTO keep the original byte and DATE
LoadCheck omission. Checks: `python3 tools/migration.py simulate trees`, Cargo tests and the
native ABI probe. Editor interaction, diagonal traversal, legacy saves, NewGRF ambient callbacks
and the abort rethrow are unreached. PRs #102 and #208.

### Water-region cache and graph service

Rust owns the map-lifetime cache, validity, optional tile labels, edge masks, patch counts and
aqueduct flags, flood scratch, lazy rebuild, invalidation and ordered neighbour traversal. C++
keeps canonical map storage, shared water track/follower queries, formatting and the
water_regions.h facade. The original implementation compiles only in portable builds; WRGN
legacy skip is unchanged. No persistent cache mirror or RNG enters this port. Direct `noexcept`
queries return copied scalars. A visitor cursor keeps only progress and per-side labels,
returning before arbitrary C++ visitors with no cache borrow. Subsequent sides observe live
changes; the aqueduct flag is read without a forced rebuild after side visitors, with original
per-tile label rebuild points preserved. C++ RAII destroys the cursor when a visitor throws. Map
replacement during a visitor violates original reference lifetimes. Checks: `python3
tools/migration.py simulate water --jobs 2` and native ABI probes for visitor invalidation and
owner replacement that ordinary YAPF visitors cannot trigger.
`tools/water-scenario-ai/README.md` gives preparation, negative-probe and optional
crossing/timing measurement commands. Arbitrary maps/NewGRFs and exhaustive region/path limits
remain open. PR #114.

### Water construction, clearing and restoration (#294)

Rust (`water_cmd.rs`, shared `water.rs`/`ground.rs`) owns ship depot/lock/canal
commands/removal, water clearing/restoration, river/desert traversal and owner/terraform/
build-above policy. Original command bodies and `MakeLock` remain portable-only. Lock
checks/build/removal order, depot partial clears and post-clear accounting, conditional RNG
draws, narrowing and wrapping retain the original sequence. C++ keeps packed map primitives,
depot identities, drawing, native stack `CommandCost` objects and a lazy stack area iterator.
Generic landscape commands retain the complete native `Command::Do` lifecycle. Rust helpers call
the integrated clear-square, flooding/neighbor, region-cache and economy owners directly. Shared
restoration retains class/accounting/RNG order; docking calls its Rust owner. No native world
borrow spans reentry. Checks: `python3 tools/migration.py verify --jobs 2` and the default
`water-commands` and water traffic/mutation/reload scenarios. Pool exhaustion, construction
under bridges, water objects, arbitrary NewGRF/legacy layouts and GUI remain #156 gaps. PR #317.

### Tunnel and bridge commands and ownership (#293)

Rust (`tunnelbridge_cmd.rs`) owns complete bridge/tunnel construction, removal permission and
clearing, ownership, terraform and build-above policy, including ordered replacement, partial
effects, reservations and infrastructure updates. Header-named modules share bridge
prices/foundations/heights, length and track helpers, and source constants. The hidden tunnel
end tile is one process-wide Rust atomic, shared with GUI readers; no component world borrow
crosses reentry. C++ keeps canonical map/spec/pool storage, rendering, vehicle entry and native
stack command results. Direct typed noexcept services retain shared operations; ordinary play
reaches no throwing script VM or save service. Bridge height calls Rust foundation directly,
including existing station/water/object consumers. Slope/z has one canonical shared observation.
Tunnel exit terraform uses the integrated object scratch owner's stable index across recursive
clears. Checks: the default `roads-commands` scenario and its reload. Live train re-reservation,
rectangular-map tunnel intersection choices, external NewGRFs and legacy layouts remain
#156 gaps. PR #352.

### Object construction, generation and HQ lifecycle (#295)

Rust (`objects.rs`) owns build/area commands, clear and ownership policy, terraform/build-above,
HQ growth/acceptance/production, colour updates, object tile loops and generation including
lighthouse/transmitter searches. It also owns object NewGRF variables, nearby/sister queries and
animation triggers; native interpreter, badge, override and generic animation-frame services
remain. Process-wide wrapping type counts and ordered cleared-area command scratch have one Rust
owner. Tunnel clients retain indices over nested append, and industry transactions
snapshot/restore the same scratch at the original points. C++ retains the native object
pool/specs/map/save/render records, stack command results and typed shared services; original
bodies remain portable-only. Generation progress cancellation returns before native rethrow.
Other leaves are synchronous noexcept calls. Borrows end before callbacks; HQ passenger/mail
production shares one random word and lazily discovers stations. Command calls use the shared
`command::nested` lifecycle. Command, finance, clearing and water restoration reuse the
integrated shared owners; ownership enters from and redispatches through the shared Rust
dispatcher (#333). Checks: the default `objects` scenarios and compact unchanged-body probes.
External NewGRF/legacy, autoslope and generation branches remain #156 gaps. PR #313.

### Object invocation and eligibility (#430)

`objects.rs` owns stack object/spec/tile/view context, root selection, successful lazy parent
caching, scope/feature/debug identity, callback entry and complete spec eligibility. Failed
parent lookups retry; construction town scopes are readonly. Native object/spec vectors,
overrides, map/save adapters, rendering and virtual resolver/town facades stay canonical. Common
Action2 uses direct Rust object scope/variable/random dispatch through the canonical context
pointer; native frames supply stack lifetimes. No owner borrow spans reentry. Leaves are typed
noexcept calls; native cancellation and save/load frames stay outside. Command
colour/slope/autoslope and animation enter the owner directly. Shared root/local-ID/GRF,
qualified base-scope and town/storage services are reused; station and object use one
optional-town facade construction helper. No variable, query, command or animation algorithm
receives a second retirement credit. Checks: the default `objects-commands`, `objects-gaps` and
`objects-reload` scenarios, which install transient groups and reinstall them after reload.
External GRF loading and historical layouts remain #156 gaps; the fixture does not establish
loader equivalence. PR #438, integrated in #444.

### Shared tile ownership dispatch (#331)

Rust owns the general `ChangeTileOwner` tile-kind dispatch and directly calls the integrated
rail, road, station, water and industry owners. Native clear, house, tree, void, tunnel/bridge
and object handlers remain separate typed leaves; the original general dispatcher remains
portable. Company transfer/control/buy and native landscape/station entries own call-local
constructed costs and the full canonical road context. Nested station removal returns before
scratch reuse; AI startup still returns to its native outer frame before it can throw. This is
shared boundary maintenance, with no new subsystem ownership. Checks: the default
`water-commands` and `ownership-dispatch` scenarios (a merger and a bankruptcy through
`ChangeOwnershipOfCompanyItems`). PRs #333 and #335.

<!-- Subsystem: Rail and road infrastructure. Add a new entry at the end of its group, not of the file. -->

### Rail signal blocks and fixed work sets (#219)

Rust owns complete signal-block exploration, immediate signal update/propagation, first-segment
classification and all three fixed 64/256/128 work sets, including counts, sticky overflow
flags, private direction tables and last owner. State has process lifetime under the game-state
mutex; loading adds no new reset. C++ keeps public signal constants, shared map/pools/spatial
queries, commands and rendering. Native assertion configuration remains authoritative. Checks:
the default `signals` simulation. Real tbd/global propagation overflow, threaded play and
arbitrary mod layouts remain #156 gaps. PR #245.

### Rail track, depot and signal commands (#265)

Rust (`rail_cmd.rs`) owns single-track build/remove and drag, depot build/removal, single/drag
signal commands and autofill, rail conversion, and rail tile clear, owner-change,
autoslope/terraform and build-above policy. Test/execute partial effects, error precedence,
costs and wrapping remain source-ordered; none draws RNG. Original bodies remain portable-only.
C++ retains stack `CommandCost` slots, nested commands, crossing/depot map writes, PBS, and the
conversion iterator/ordered affected-train list. Shared command checks serve rail, fleet and
industry; canonical road/rail map services supply reads. Shared services also provide slope
settings, `DoClearSquare` for trees/rail, reservation release/path requests and one safe-waiting
service for train/YAPF/rail. Native enum return types retain their widths through transparent
Rust carriers. No owner borrow spans reentry. Rail tile loops, track status and drawing stay
native. Checks: `python3 tools/migration.py simulate signals-rail-commands --jobs 2`.
Coast/half-tile water and PBS path-end removal remain #156 gaps. PR #270.

### Road track, depot and conversion commands (#266)

Rust (`road_cmd.rs`) owns road removal permissions and slopes, single and long build/removal,
depots, clear/owner/nearest-town updates, terraform, conversion and build-above. Portable
originals remain. Native road-type allocation, drawing, track-status/vehicle-enter and tile
loops stay outside this command port. The boundary reuses shared command checks and native stack
costs, road/rail map leaves, company counters, depots, crossings and the rail conversion cursor.
Rust road/station helpers share the ordinary nesting, test/execute, rating and finance
lifecycle. Outer native town, founding, controller and tile-loop frames provide one stack
scratch and two result slots for direct Rust owner calls. Shared Rust also owns road prices and
tile clearing; native facades retain portable originals and typed storage/error leaves.
Town/tram builds, ground removal, nearest-town, bankrupt rail calls and town authority/rating
policy call the Rust owners directly. Crossing reservation reads/writes have one canonical
definition shared with trains. The canonical `MayHaveRoad` predicate preserves rail-station/head
guards. The nearest-town map scan bundles only its reached original predicates per tile. No
world-state borrow spans nested commands or vehicle/town callbacks. Checks: the default
`roads-commands` and `roads-commands-reload` scenarios. The fixture declares builtin-like
road-type inputs; external NewGRF programs and legacy save variants remain #156 gaps. PR #290.

### Road stop bays and drive-through entries (#354)

Rust (`road_stop.rs`) owns each RoadStop's status byte and the shared drive-through entries: bay
admission and freeing, entrance status, `GetByTile`, continuation, both `Enter`/`Leave` levels,
joining (alone, parent, join, merge) and splitting (last, head, tail, split with rebuild),
`Rebuild`, `CheckIntegrity`, destruction and both `AfterLoadRoadStops` passes. One Rust
allocation per shell holds the status and entries pointer; the BaseEntry stop owns the entries,
allocated and freed where the original does. `status` stays a native reference alias for the
ROAD descriptor; `GetEntry` is a const reader. The road controller, station commands and road
YAPF call the owner directly. C++ keeps the shell, pool, `xy`/`next` links and station stop
lists (`OpenTTDRoadStopServices`), two compound vehicle reads, the tile hash and
`GetNextRoadStop`, whose only caller is the native YAPF area leaf. Native vehicle entry,
PreDestructor, cache check and load call typed entries. Checks: `stations-commands-road-stops`
and `-road-stops-reload`. Articulated bay refusal and the unreachable `include` duplicate remain
#156 gaps. PR #362.

### Waypoint construction and command policy (#366)

Rust owns deleted-waypoint reuse in pool order, rail/road axis selection and tile validation,
complete rail/road/buoy construction, buoy removal and waypoint name uniqueness, rename and
sign-move policy. It calls integrated station joining, expansion, road-cost and removal policy
directly, and water restoration directly. C++ retains canonical Waypoint/BaseStation shells,
spec allocation, map setters, presentation and caller-stack `RailStationTileLayout`/iterator and
command costs. No native world borrow spans callbacks; serialized synchronous leaves are
noexcept. Checks: `python3 tools/migration.py rust-checks` and `python3 tools/migration.py
simulate stations-commands-waypoints stations-commands-waypoints-reload
stations-commands-commands stations-commands-reload water-commands`. External station/road-stop
GRFs, exhausted pools/spec lists and legacy saves remain #156 gaps. PR #369.

### Animated-tile scheduling and shared animation control (#385)

Rust owns the process-wide queue, scheduling/control, six typed bindings, trigger traversals and
dispatch; existing owners call it directly. Native map frames/bits, road records,
pools/specs/PSA and stack resolvers remain. Borrows end before callbacks. Generated visitors
include all six version8 fields. ANIT retains native Sl* frames, nested active pointers,
append/resize and partial publication. Shared station lookup/membership, IsAirport and
orthogonal iteration preserve original callees; unused native animation table/facades are
portable-only. Airport SB retains its unmasked OR, and rejection cases retain mask mutation.
Checks: `python3 tools/migration.py simulate objects-gaps objects-reload effects-controllers-1`.
Area/spec/member/mask reads and initial-RNG-before-area are source-compared, not independently
traced (#156). Historical/pre6/TTO saves, external GRFs, GUI threads and raw>=64 outside pinned
Linux compilation remain #156 gaps. PR #389.

<!-- Subsystem: Vehicles, pathfinding and orders. Add a new entry at the end of its group, not of the file. -->

### Effect vehicles

Rust owns all twelve controllers, movement tables and private animation bytes. Each C++
EffectVehicle shell owns one zero-created opaque state, also on indexed load; RAII destroys it
on ordinary deletion and pool cleanup. Copied call-scoped observations and immediate scalar
writes keep original mutation order. Rust calls common Random/map and effect-specific
sound/viewport noexcept leaves directly, plus the Rust animated-tile owner; the native leaves
cannot reenter Rust. The immutable shared service table is borrowed only until the controller
returns (#299); no full table is copied. Checks: `python3 tools/migration.py simulate effects`.
Viewport pixels, audible output, full legacy fixtures and every caller remain limits. PRs #105,
#115 and #300.

### Disaster scheduling and vehicles

Rust owns all fifteen subtype controllers, eight initializers, eligibility, countdown/reset,
industry construction reset and square-clearing policy, movement, shadow/rotor updates and both
target-release hooks, plus each shell's state, flags, image override and destroyer target, and
the persisted global delay. Original bodies compile only in portable builds. Copied observations
and direct noexcept shared services preserve original ordering and shared RNG draws. Private
scalar addresses remain stable through modern and legacy save staging; flight helpers borrow
only the flags scalar for their call. World clones share an immutable service-table borrow tied
to the Run/future lifetime (#299), ending on destruction, including cancellation. Production
uses the process-wide const table; Rust fixtures own separate tables through destruction, and
native partial tables stay in scope. Callbacks may mutate their separate context, never the
borrowed table. Movement uses a borrowed typed occupancy context (#377): it resolves each shell
once, updates the Rust index and calls the base viewport leaf; clear retains native costs.
Checks: the default `disasters` simulation. Legacy save fixtures, viewport pixels, sound output,
allocator failure timing and exhaustive NewGRF combinations remain limits. PRs #118 and #300.

### Road vehicle control and private state

Rust owns tick/day control, movement, blocking/overtaking, reversal, station/depot transitions,
crash expiry, servicing and cache policy. Each shell owns seven scalars and the canonical path;
road YAPF (#124) writes that same path. Modern, split-vector and TTD/TTO adapters retain field
widths through C++ staging. Shared typed vehicle and GroundVehicle services live in
`vehicle.rs`, `vehicle_ffi.h` and `vehicle_services.{h,cpp}`. No owner, path or world borrow
spans callbacks, reentry or deletion. One lean max-speed calculation is shared by road control
and native queries. Road control supplies that limit between the shared native visual-effect
gate and tail, then observes movement/speed inputs again after effects; train callers keep the
shared policy. The native tick entry captures the tick front: named read-only scalar places are
loaded at the prologue branches, then discarded before controller callbacks; counter writes stay
native and wrap explicitly in Rust. Checks: the default `roads-` simulation. Legacy saves,
custom NewGRFs, articulated/tram turns, sounds and viewport pixels remain #156 gaps. PRs #128,
#215, #319 and #332.

### Ship controller and private state (#146)

Rust owns the complete ship tick/movement, locks, acceleration, rotation/reversal,
track/depot/destination policy, economy-day/service/cache/build initialization, and
nearest-depot region BFS with ordered pool selection. Four canonical scalars belong to each Rust
ship owner; ship YAPF (#119) remains the sole path/search owner. C++ keeps shared
Vehicle/map/pool storage, renderer bounds/sprites, allocation, Money accounting and
modern/legacy save staging. Services are individually named `noexcept` functions; narrow getters
fetch only used fields, typed results carry positions and search choices, and one reusable
game-thread Rust set/deque owns depot scratch. Tile/depot, orders/loading, viewport, breakdown,
effect/sound, cache/YAPF and NewGRF callbacks remain direct: none can unwind a script VM or
save/load error during ordinary play. Checks: `python3 tools/migration.py verify --jobs 2` and
`python3 tools/migration.py simulate water --jobs 2`, which requires the `ship.*` witnesses.
Legacy save versions, viewport pixels and arbitrary custom NewGRF combinations remain gaps. PR
#152.

### Rail vehicles and controller state (#130)

Rust owns train consist/cache/curve/speed policy, both tick passes, movement, reversal, crossing
control, crash/deletion, servicing and day handling. It also owns reservation extension/rollback
and temporary order lookahead; rail YAPF (#122) retains search ownership. C++ train shells hold
one Rust allocation for the controller state, while pools, map storage and shared services
remain native. Typed calls preserve the original callback and RNG order, and no owner borrow
crosses reentry or deletion. Checks: the default rail simulation and native checks. Arbitrary
NewGRFs, legacy saves and unexercised controller branches remain #156 gaps. PRs #143, #195 and
#345.

### Aircraft controllers and airport movement

Rust owns both tick passes, movement/speed/flight, shadow/rotor, FTA traversal, terminal/helipad
reservations, service/range, crash cleanup and airport lifecycle. Fixed allocations own private
counters/cache and airport masks; trivial C++ save aliases preserve their addresses. Rust
borrows shared vehicle/map/settings/RNG services directly. No owner borrow crosses
ProcessOrders/loading, depot/refit, animation, crash or deletion. Runtime dimensions are
captured per entry; wrapping arithmetic/RNG order remain. The controller holds one stack
descriptor of named canonical native x/y/z, speed, Direction and subtype addresses; position
alone acquires only subtype/z. Raw reads end before callbacks, with fresh loads after native
mutations. Geometry, map, speed/dirty, direction/new-position, NewGRF cache/image, sound and
crash-marking services preserve the shell; pool growth moves slots, not vehicles. Descriptors
end before airport state handlers/depot work, deleting crash cleanup, replacement or load/reset.
Standalone flight/disaster entries retain their narrow services. Checks: the default `aircraft`
and `disasters` scenarios. Real NewGRF/refit/airport rotations, threaded GUI, legacy-save
fixtures and presentation remain #156 gaps. PRs #145, #238 and #326.

### Vehicle scheduling and base maintenance (#213)

Rust owns the base vehicle calendar-day and economy-day scheduling, the live pool traversal of
`CallVehicleTicks`, survival checks, cargo aging, motion and sound policy, loading and fleet
sequencing, renewal, service and depot resets, reliability decay, breakdown triggers and
countdowns, and age and value decrease. A growth-checked pool cursor and post-Tick scalar
addresses keep creation, holes and deletion during a tick in source order; Tick completes before
survival checks and no owner borrow spans a callback that can reenter or delete. C++ keeps the
canonical `Vehicle` shells and pools, the type-specific controllers and day procedures, NewGRF
callbacks, commands and presentation behind the original facades; the original bodies compile
only without `WITH_RUST`. Checks: `python3 tools/migration.py verify --jobs 2` and the default
vehicle, fleet and aircraft simulations. Replacement and old-news branches, NewGRF and reentrant
sound, custom and articulated combinations, and legacy saves remain #156 gaps. PR #257.

### Fleet grouping and replacement lifecycle (#147)

Rust owns group metadata/names/ordered children, group/company statistics and engine counts,
renewal rules/links and company renewal heads, plus the pending tick-end map. It runs every
group command, traversal/statistics/livery update, renewal-rule command, replacement
eligibility/refit choice, speculative build, rearrangement, rollback, execution and the
ascending-VehicleID drain. Original bodies compile only in portable builds. C++ keeps pool
identities, group numbers, GUI folding, save adapters and `Vehicle::group_id`. Rust reads/writes
that shell field through services; no owner borrow spans a nested command. Three named-slot
noexcept tables serve Rust; native stack CommandCost objects keep AddCost first-error and Money
saturation, and original assertion sites run in assertion builds. Checks: `python3
tools/migration.py verify --jobs 2` and the default `fleet-` simulation. NewGRF refit/attach
callbacks and articulated parts remain #156 gaps. PR #197.

### Common vehicle transactions and nonrail construction (#314)

Rust (`vehicle_transactions/`) owns the `vehicle_cmd.cpp` commands: build, sell, refit, clone,
start/stop, mass start/stop, depot sell/replace, send-to-depot dispatch, naming and service
intervals, with vehicle-set selection and depot/sort lists. It also owns the road vehicle and
aircraft constructors (`construct.rs`, shadow and helicopter rotor included), articulated
assembly and its consistency check. Speculative refit builds restore both RNG streams; clone
cleanup sells partial consists. Commands nest through `command::nested` and call ship, orders,
cargo, fleet, road and aircraft owners directly. Original bodies compile only portably. C++
keeps pools, shells and their field stores, specs/NewGRF callbacks, unit allocation, native
stack `CommandCost`s and UI leaves. Rail construction, moves and sale are #315 (below). Checks:
`python3 tools/migration.py verify --jobs 2` and the default `fleet-` scenarios, including
`transaction-build-road`/`-aircraft` and `transaction-clone`. Articulated road vehicles (NewGRF
only), wrong-depot road types, airports refusing an engine and clone build failure after the
front part are #156 gaps. PR #329.

### Rail construction and depot consist transactions (#315)

Rust (`vehicle_transactions/rail.rs`) owns the `train_cmd.cpp` bodies from
`GetFreeWagonsInDepot` through `CmdSellRailWagon`: wagon, engine and rear-head construction,
free-chain joining and `NormalizeTrainVehInDepot`, backup/restore lists, arrangement, dual-head
and subtype normalisation, attachment/limit validation (an error leaves the chain split until
the caller restores, as the original), head promotion/demotion with order, configuration and
unit transfer, and rail sale. `CmdMoveRailVehicle` is a facade; build, clone and fleet
autoreplace nest the Rust body directly. Original bodies compile only portably. C++ keeps shell
allocation and source-ordered field stores (including the rear head's `Money` halving), the
attach callback, unit-ID pools and presentation leaves. Chain links stay native and change only
through `SetNext`; rail moves call the Rust cargo capacity owner directly. Shared leaves in
`vehicle_services.h`/`engine_services.h` serve the train controller, cargo capacity and this
owner. `vehicle_base.rs`/`ground_vehicle.rs` hold the GVSF values and subtype helpers. Checks:
`python3 tools/migration.py simulate fleet-transaction-rail --jobs 2`. NewGRF attach-callback
results, the GRF-version branches and order-pool exhaustion are #156 gaps. PR #360.

### Engine lifecycle and availability (#220)

Rust owns each engine's lifecycle scalars and name, the process-wide aging-stop state,
startup/reliability, daily/monthly preview and availability policy, variant synchronization and
control/name/visibility commands. Native Engine shells expose canonical scalar aliases;
temporary owners survive relocation and TTO replacement. Original bodies compile only portably.
Pool/configuration/spec, NewGRF/articulated, company/type availability and presentation remain
native through typed synchronous noexcept services shared with fleet/company consumers. No owner
borrow spans a callback; script events enqueue without running a VM. RNG draws, narrowing and
command/effect order retain the original behavior. Modern ENGN scalar descriptors address
aliases directly; only opaque names use a native string scope, preserving partial decoding on
failure. Historical loaders address canonical prefixes without live mirrors.
Panics/environmental exceptions abort; native save/load errors remain outside Rust. Checks:
`python3 tools/migration.py verify --jobs 2` and `python3 tools/migration.py simulate engines-
--jobs 2`, whose identical adapter declares variant and cutoff spec inputs for unchanged
reference bodies. External NewGRF programs/retire_early, ENGS and TTO/TTD fixtures, legacy
preview conversion and threaded GUI/presentation remain #156 gaps. PR #248.

### Orders lifecycle ownership

Rust owns order storage, links, backups and consist timetable/unbunching state, including
traversal, command edits, sharing, depot execution and backup lifecycle. C++ retains pool
identity, vehicle shells, command execution, VM callbacks and save/load adapters; original
bodies remain in portable builds. Typed synchronous services preserve source ordering. Rust
releases owner borrows before callbacks, commands or deletion, and native exceptions remain in
C++. Checks: the default orders simulation and existing tests. NewGRF callback variants and
legacy saves remain #156 gaps. PR #176.

### Ship YAPF searches and canonical path cache

Rust owns both region and track searches, arena indices/parent chains, open and closed
membership, the original binary-heap tie/removal rules, destination estimates, region corridor,
two attempts, random lost paths and cache extraction. Search arenas/maps/heaps retain
process-wide capacity (#306); active searches own storage, with no cached world pointers. Each
Ship owns one opaque Rust path with copied controller access; modern VEHS stages one byte across
C++ load errors and commits it on unwind, while the pre-struct-list loader retains its reversed
path order. C++ keeps ship/world pools, track following, station/dock queries, engine/settings
observations and canonical map arrays. No ordinary throwing/reentrant world callback occurs in
this search; save errors remain outside Rust frames. Checks: `python3 tools/migration.py
simulate water --jobs 2`. Arbitrary maps/NewGRFs and complete legacy saves are not covered. PRs
#126 and #308.

### Rail YAPF search, caches and reservation

Rust owns all four searches, node/segment arenas, exact-order heap, costs, lookahead, limits and
reconstruction, six specialization-specific cache banks, rail-change invalidation, reservation
traversal and ordered signal rollback. C++ retains the train controller, shared track follower,
PBS and canonical world services. Explicit platform/waypoint station triggers return to C++ with
borrows released; waypoint track reservation also retains the original synchronous PBS station
triggers while search/reservation borrows remain active, before its second trigger. Original
search bodies remain portable-only; diagnostic dumps use temporary views and the original
format, with no canonical C++ search mirror. Checks: the default `padhattan` and
`rail-reservation` simulations under both 90-degree policies, and native unchanged-base/heap
probes for finite limits, live cost invalidation and ordered opposing PBS signal restoration.
They do not prove full rail-cost semantics for arbitrary maps, NewGRFs, legacy saves or every
signal family. Controller reservation extension belongs to #130. PR #133.

### Road YAPF search and path construction

Rust owns track/depot searches, exit-direction keys, node arena, lookup maps, exact-order heap,
segment traversal/cost, heuristic, limits and intermediate fallback. Reconstruction and
station-area trimming write the road controller's sole canonical path. Original
specialization/node types compile only portably. C++ retains map/pools/settings, station/stop
observations and `CFollowTrackRoad` behind copied scalar `noexcept` leaves. None runs scripts,
mutates/reenters the search/road owner or throws in ordinary play. Short owner access ends
before leaves. Neither search draws RNG; wrapping, exit-key replacement, strict ties and
follow-before-limit timing retain source behavior. Panics/OOM abort. Arena/maps/heap retain
process-wide scratch capacity (#278); active searches own it, with no cached native pointers.
Inputs/destinations/counters reset per call. One tile observation is shared across segment
cost/destination/depot checks. Endpoint predicates invalidate on advance; startup and >4096
exits remain unevaluated until needed, without stop-cost lookups. Only accepted continuations
read ordered current/next ground-vehicle center Z, vehicle speed fields and old-tile follower
limits; arithmetic stays Rust and Follow is never repeated. Checks: `python3 tools/migration.py
simulate roads --jobs 2` and native heap/depot transcripts against unchanged C++. Arbitrary
maps, NewGRFs, articulated/tram, waypoints, exhaustive stop/loop/segment branches and legacy
saves remain #156 gaps; shared follower logic remains native. PRs #135, #279 and #289.

<!-- Subsystem: Stations, cargo and economy. Add a new entry at the end of its group, not of the file. -->

### Station cargo-list queries

The Rust ScriptList owner is reused. Rust owns the four selector/filter rules, pending run keys
and unsigned 32-bit totals, positive flush decisions, add-versus-set merges, per-origin
cumulative-share decoding, and query-plan selection (waiting/all versus equal_range, planned/all
versus find). C++ keeps station/cargo validation, pool/GoodsEntry access, HasData checks,
packet/flow iterators and scalar extraction. A caller-owned 16-byte collector (uint32
amount/previous, uint16 station IDs including 0xFFFF, byte selector/finalized) is asserted in
C++ and by a generated layout probe; Windows x86 uses explicit cdecl. Rust borrows the
destination only during feed/finalize, and the destination outlives the collector; no world
pointer, STL/VM layout, iterator, callback or C++ exception enters or survives a Rust call.
Fresh collector totals are bounded by packet-pool/origin limits; modification tokens use the
shared owner's wrapping arithmetic. Checks: the unchanged stationlist script, both scripted
suites and the bounded cargo mode of `python3 tools/script-list-comparison.py`. PR #53.

### Station cargo service

Rust owns the ordered loading queue, station service counters, acceptance mask and cargo
rating/pickup/speed/age/waiting/fraction metadata in one opaque owner per BaseStation. Native
scalar references and the queue facade share it; ordinary/indexed construction, retirement,
reuse, STNN/STNS and legacy fixups share it without persistent native mirrors. Packet, flow,
link-graph, map and geometry storage remain C++. Borrowed typed noexcept tables, narrow
observations and caller-owned native iterator/container frames form the boundary (#214).
Portable bodies remain unchanged. Checks: `python3 tools/migration.py simulate stations
economy`. Legacy, NewGRF and refit gaps remain in #156. PRs #142 and #217.

### Station construction, joining and removal (#267)

Rust owns the station command policy for rail platforms and waypoints, bay and through road
stops, airports, docks and oil rigs, plus sign/name/owner, terrain, bridge clearance, joining,
deletion/reuse and docking-neighbour decisions. `water_cmd.cpp` docking eligibility and
registration belong to this owner. Original command bodies compile only in portable builds. The
existing station cargo service state remains canonical; there is no second station owner or save
layout. C++ retains shell/spec/road-stop allocation, NewGRF resolvers, spatial indices,
rendering/description and the hot tile-loop/animation/vehicle-entry/track/slope services.
Entries use borrowed typed synchronous tables, shared Rust command pre/post checks, prices,
clear-square/water restoration and native result slots. No ordinary-play script VM or save/load
exception crosses the boundary. Rust ends owner access before reentry and calls owned
aircraft/station/reservation policy directly. Reservation lookahead saves the existing
layout-checked order value in each Rust stack guard; native pool/container allocation follows
the original. Retargeting uses virtual `SetDestTile`; shared native traversal serves stations
and flooding. Checks: `python3 tools/migration.py simulate stations-commands
aircraft-controller-removal water-structures-mutate --jobs 2`; the station module includes
unchanged-reference private callbacks in fixture executables. Default #320 cases
(removed-destination path clearing, `TileAddWrap` border refusals, type-63 overbuilding through
`AllocateRailType`), builtin tram, airport noise and reload cases remain required. External
NewGRF programs, legacy and unusual occupied/reservation branches remain #156 gaps. PRs #303 and
#323.

### Cargo storage and movement (#139)

Rust owns packet fields, ordered station/vehicle lists, count/age/feeder/action/reservation
caches, cumulative flow shares and origin maps. It runs packet operations, cargo
movement/staging/rerouting, live-flow reconciliation, capacity redistribution and autoreplace
cargo transfer. Stable arena slots and linked ordering give constant-time list operations;
destination endpoints bound station lookups to the selected queue. Freed slots are reused; arena
capacity is retained. C++ retains packet pool identity/allocation/hooks, payment/delivery
(#117), loading policy (#125), link-graph computation (#74), world access and thread join. CAPA
saves the repr(C) packet in place; STNN/VEHS/flow adapters export call-local containers or
scalar-key readers; pointer fixups and save errors finish outside Rust. Payment getters reenter
after owner borrows end; ConsistChanged runs after Rust returns. Portable bodies stay. Checks:
`python3 tools/migration.py verify --jobs 2`, the default cargo-storage, economy, station,
Padhattan and aircraft scenarios, and `python3 tools/cargo-storage-comparison.py` for the
unreachable pool-limit Split failure. Legacy conversions and complete refit/autoreplace
transactions remain #156 gaps. PRs #151 and #268.

### Cargo payment and delivery

Rust owns CargoPayment state/lifetime, final/transfer payment and settlement, industry
acceptance, statistics/payment ordering, destination deduplication and the complete production
flush. C++ retains pool shells, canonical world and packet storage, loading/reservation, subsidy
checks, NewGRF resolution and UI. CAPY saves the repr(C) payment in place through `CargoPayment`
reference aliases. Cargo/refusal/production/randomisation/animation and vehicle-sound resolution
uses the NewGRF evaluator, without script VM entry or payment/destination reentry; wrappers are
noexcept. Money saturates arithmetic but retains native shifts; callback unsigned products,
BigMulS narrowing, stockpile limits and ordered randomisation/animation are explicit. Checks:
the default `economy` simulation. NewGRF resolver execution, legacy CAPY files and every
industry layout are not exhausted; the narrow fixture supplies callback results. PR #123.

### Company finances and economy lifecycle

Rust owns cash/fraction accounting, loans, histories, rating/value, construction budgets,
maintenance/interest/year rotation, bankruptcy/recovery, ranked offers, creation/deletion,
acquisition and the complete ownership-transfer traversal, plus periodic scheduling,
prices/multipliers, inflation/recession, score parts and the company tick cursor. Selected
bodies compile only portably except StartupCompanies/InitializeCompanies: their native
timer/reset control remains, and InitializeCompanies writes Rust's tick cursor. Identity,
settings, networking, UI, AI and shared world services remain C++. One Rust allocation per
CompanyProperties holds finance/history fields with C++ field lifetimes and stable addresses;
economy/prices/scores have process lifetimes. Every entry is a named synchronous call taking
borrowed designated tables (`OpenTTDCompanyFinanceServices`, `OpenTTDCompanyServices`) of typed
noexcept services; facades resolve owners and settings once. Pool scans filter owner, then type,
before detail reads; tile-owner changes, deletions, allocation, the nested service-interval
command, the bankruptcy Post and `StopAI` run directly. Only `AI::StartNew` (script memory
policy can throw before ScriptInstance's catch) and the `CCA_NEW_AI` Post return through
caller-owned stack frames (startup, control, tick, competitor timeout). Checks: `python3
tools/migration.py simulate companies economy` and native CTest for lifetimes, network deferred
deletion, posting order and abandoned startup frames. Subsidies, exclusive rights, goals/story
pages, hostile purchase, multiplayer runtime and NewGRFs remain gaps. PRs #149 and #209.

### Link graph job computation

Rust owns demands, both MCF passes, cycle elimination, path lifetime and flow mapping, with
copied graph/settings, private demand/edge annotations, a reusable path arena and ordered
cumulative-share maps. C++ keeps worker launch/join and abort flags, LGRP/LGRJ save/load and the
existing station flow join. The sole callback reads the job's atomic abort flag and cannot
throw/reenter. C++ allocations happen outside Rust. Checks: the unchanged
regressions/comparisons and `python3 tools/migration.py simulate` with manual/cargodist road
saves and reload from a reference snapshot with outstanding LGRJ records (GLOG history is reset
only in prepared road inputs so both loaders log their revision). Graphs, allocator failures and
abort timing are not exhausted; resource exhaustion timing differs with the allocation strategy.
PR #95.

### Link graph scheduler ownership (#373)

One process-wide Rust owner holds ordered queued/running pointers and controls rotation,
spawn/join/requeue, pool-wide date shifts and pause policy. Native pools, job snapshots, worker
launch/join, abort and acquire/release completion publication remain unchanged; existing job
computation is not counted as new retirement. A constant-initialized member table supplies the
graph owner and typed native leaves; Time is one narrow entry observation. Visits and cargo
admission call queues directly in Rust. Shared graph owner/ID/validity/get helpers serve the job
reader and existing consumers. No queue borrow crosses native callbacks, including synchronous
fallback and deletion. LGRS keeps two native scoped vectors with the original reference field
names/order. Exported spans are copied without callbacks/yield/reset under game-state
serialization and discarded before SlObject. SlSaveChunks precedes asynchronous compression.
Native RAII publishes partial staging on return/unwind; recursive SlNullPointers uses and
publishes the same active vectors before returning, preventing stale restoration. Checks:
default symmetric/asymmetric cargodist reloads and module-compiled unchanged pinned scheduler
bodies for worker/pause, duplicate rotation, recycled-ID, clear and pool-shift gaps. Actual
concurrent worker, multiplayer/GUI timing and historical LGRS versions remain #156 gaps. PR
#380, integrated in #391.

### Live link graph contents and mutation (#378)

Rust owns cargo, compression date, nodes and sorted edge vectors in one owner per native shell.
Lifecycle/copy/init, node and edge mutation, supply/demand/location, restriction/release,
merge/scaling, compression, dates and queries run in Rust. C++ keeps pool IDs/reuse and station
membership leaves; Rust owns the scheduler. Each job deep-copies the same representation; native
annotations borrow canonical station scalars and immutable edge references until the existing
worker join. The computation and flow-application ports are unchanged ownership, not new
retirement. LGRP/LGRJ retain field names, widths/order and legacy matrix/list conversions.
Native SlObject writes canonical scalar addresses and appends edges before decoding; partial
effects remain visible. Legacy matrix staging retains its original scope.
SlErrorCorrupt/SlNullPointers and all exception unwinding remain C++ frames. LGRP/J have no
pointer-fixup handler; graph borrows end before station writes or deletion. Read-only job access
never creates an exclusive Rust borrow during worker execution. Checks: the station fixture
against unchanged reference bodies and focused station stale/removal/reload and
symmetric/asymmetric LGRJ reload. Legacy/corrupt-load matrices and real-thread timing remain
#156 gaps. PR #383.

<!-- Subsystem: Towns, industries and subsidies. Add a new entry at the end of its group, not of the file. -->

### Built-in town-name generation

Rust owns all 21 built-in generators, seed selection, English replacements, Finnish vowel
handling, Czech grammatical matching and private constants. C++ keeps NewGRF routing, settings
and save/load; town founding (#246) owns retries/uniqueness. One coarse call owns complete
output behind an opaque result. C++ appends the immutable byte view with RAII destruction,
including on append exceptions; no builder/world pointer, callback or shared RNG enters Rust.
The harness samples seeds through read-only AI logs; it does not cover all 21 * 2^32
combinations, every allocator failure or NewGRF town-name behavior. PR #98.

### Town growth and lifecycle state

Rust owns town tick traversal, road/build choices, terrain retries, house selection/placement
and expansion, plus town metadata, histories, ratings and building counts (#148). House tile
loops and deletion commands belong to #256. Native field aliases expose one Rust owner, with
process-wide state under the game-state mutex. Rust ends owner borrows before commands or NewGRF
callbacks that can reenter. Checks: the default `town-growth` simulation and native tests.
Natural tunnel construction, custom NewGRF callbacks and complete historical saves remain gaps;
#148 is not a complete lifecycle port. PRs #127 and #216.

### Town founding, generation and names

Rust extends the sole town owner with canonical custom-name bytes and owns founding,
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
unchanged in non-Rust branches. Checks: `python3 tools/migration.py simulate town-founding-
--jobs 2`, with module-owned adapters comparing cancellation, full-width count clamps/RNG and
otherwise unreachable name bytes against unchanged source. Real NewGRF, historical saves, GUI
threading and the final 10,000-attempt fallback remain #156 gaps; house lifecycle belongs to
#256. PR #258.

### Town commands, authority actions and deletion (#247)

Rust extends the sole town owner with the encoded town text and owns the deity and editor
commands (cargo goal, text, growth rate, rating, expansion, deletion), all eight authority
actions with their costs and masks, `TestTownOwnsBridge`, and the ordered town destructor and
post-destructor cleanup. Command and error order, random draws, the statue inner-circle
fall-through and the queried-company mask quirk follow the original, and the real-oilrig refusal
precedes the later deletion phases. C++ keeps pool and map identities, persistent-storage
shells, native costs, news and events, and save decoding behind the original facades; the
original bodies compile only without `WITH_RUST`. The growth, rating, road, station-rating,
cargo, subsidy and object owners are called directly in Rust. Post-destruction never resolves a
recycled ID, and no owner borrow spans a callback. Checks: `python3 tools/migration.py simulate
town-commands town-commands-reload --jobs 2`. Station allocation and statue search, NewGRF
callbacks, live-vehicle deletion, legacy text decoding and threaded-GUI gaps are #156. PR #347.

### House lifecycle and tile simulation (#256)

`town/house.rs` owns `TileLoop_Town` (construction stages, rebuild/replacement, original and
binomial cargo with recession halving), builtin lift animation, produced/accepted cargo,
north-part normalisation, `ClearTile_Town`/`ClearTownHouse`/`RemoveNearbyStations`,
`TerraformTile_Town`, `CmdPlaceHouse`, cache rebuild/repair traversals, yearly aging and the
NewGRF house tile loop, `CanDeleteHouse`, animation triggers and rerandomisation. It writes the
sole Rust town owner directly and ends each owner access before any callback, command or station
service. House resolution (#409) shares its north-part helper. C++ keeps stack virtual facades,
common Action2, `StationFinder`/`MoveGoodsToStation`; original bodies compile only portably.
Narrow phase observations replace whole tile/spec copies; projections compose canonical
spec-field readers. Each entry reuses its town and map context, including construction triggers
and rebuilds. Town count, recession, autoslope, foundation price, tick counter and animated-tile
removal are shared services; house clearing calls the canonical Rust ground owner directly.
Cargo scaling, translation, label lookup and delivery, plus NewGRF boolean conversion and
inspect-window deletion, share canonical typed native leaves with industry consumers. Checks:
`python3 tools/migration.py simulate town-houses- --jobs 2`, which places every original house
through `CmdPlaceHouse`. Station removal on house clearance remains a #156 gap. PR #269.

### House and shared-town NewGRF resolution (#409)

`town/resolver.rs` owns HouseScope/HouseResolver and TownScope/TownResolver query policy,
root/scope selection, process-wide ordered house classes and count reset. Count/history/radius
and north-part queries call canonical Rust town/house owners. Lifecycle, construction,
foundation and animation enter the resolver directly; displaced table callbacks and native
helper bodies are excluded with WITH_RUST. C++ retains stack virtual facades, Action2,
specs/override managers, map/pool shells and one ordered call-local StationFinder. Var64 reads
canonical Rust cargo status and uses the shared GoodsEntry conversion. No new map view is
exposed. Native leaves are noexcept; no ordinary-play throwing service is used. Save/load
exceptions stay native. Contexts end before load/reset/yield; owner/container borrows end before
evaluator, storage and station reentry. Checks: the default `town-houses-` scenarios, which
install real groups and reinstall transient groups after reload. External-GRF badge/content
combinations, historical save adapters and GUI-thread integration remain #156 gaps. PR #416.

### Industry periodic production and builder ownership (#129)

Rust owns the produced/accepted slots, optional accepted histories, production fields, sound
countdown and industry-builder records. C++ industry shells expose nonowning views of these
allocations; INDY, IBLD, ITBL, ECMY and TTO/TTD load adapters preserve the original names,
widths, version gates and ordering. INDY, IBLD, ECMY and TTO/TTD industry scalars load and save
through the reference aliases; only the Rust-owned industry text is staged. Rust runs
production/transport ticks, farm and lumber updates, NewGRF production application, monthly
statistics/closure, daily changes and builder targets/retries. Borrows end before reentry.
Pool/map/spec storage, construction and landscape-clear transactions, presentation and bounded
NewGRF resolution stay in C++ behind typed `noexcept` slots (#203). CargoPayment delivery
remains #117. Checks: the default `industry-` scenarios and native ABI/lifetime tests. Legacy
saves, NewGRF resolver programs and every industry layout are not exhausted. PRs #144 and #205.

### Industry construction and tile lifecycle (#150)

Rust owns industry metadata/text, ordered type registries, builder scratch, destruction, initial
generation, full validation/funding/prospecting transactions, initialization, construction-stage
completion, tile loops, builtin animation, acceptance, clear/terraform policy and NewGRF
tile/parent trigger traversal. Native aliases and scoped load adapters preserve saved
names/reference fixups, partial initialization and ID reuse. Water destruction retains the
tile-type recheck. Packed map/pool shells, rendering, specs, resolvers and persistent storage
stay C++. Only progress cancellation (`AbortGenerateWorldSignal`) returns to the native
exception frame after source-order restoration. Direct synchronous services use native stack
costs; temporary resolver shells borrow a Rust stack owner through the original
constructor/destructor scope. No state borrow survives reentry. Shared map/RNG leaves use the
canonical typed services; stable dimensions are captured once per consuming entry. Checks:
`python3 tools/migration.py simulate industry-command-fund industry-command-prospect
industry-command-clear industry-command-terraform --jobs 2`, the default industry scenarios and
comparisons, and `industry-water-check-*` (oil-rig funding refusal over real depots/lower lock
parts, #234). Legacy saves, resolver programs and exhaustive industry layouts remain
#156 gaps. PR #218.

### Subsidy lifecycle, route search and awards (#316)

Rust owns each subsidy's canonical scalars, all route searches, creation/deity checks,
source/destination cache rebuild and deletion, monthly generation/expiry, delivery awards and
company transfer/deletion. Native SUBS descriptors and GUI/script readers alias the sole Rust
allocation; original bodies stay portable. C++ retains pool identity/allocation order,
map/catchment and cargo specifications, news formatting and event enqueueing through typed
noexcept leaves. Payment, company and industry owners call Rust directly; no owner borrow spans
a callback. The deity command retains duplicate/self/distant routes, and monthly policy keeps
RNG short circuits, 1001 attempts, u16 wrapping/narrowing and expiry's bypass of both
disabled-generation early returns. Cache flags use the original Set/Test operations; house cargo
scans call the integrated Rust house owner directly. Checks: `python3 tools/migration.py verify
--jobs 2`, the default `subsidies-control` and `subsidies-reload` scenarios, and the console
adapter compiling the unchanged reference monthly lambda for zero-countdown, full-pool retry and
disabled-expiry RNG gaps. External NewGRF programs, legacy conversion fixtures and threaded
GUI/presentation remain #156 gaps. PR #346.

### Complete PBS reservation service (#367)

Rust (`pbs.rs`) owns reservation reads/writes, complete platform scans, traversal, loop
termination, train selection and waiting policy. `follow_track.rs` owns reserved-track masking
between native Follow and GetSpeedLimit calls. Native map bits remain canonical. The named
`CFollowTrackRail::Follow` leaf carries all partial failure observations in a call-local typed
result; spatial enumeration retains native hash order, while Rust selects the lowest front-train
ID. Crossing updates and train state/trackdir call their Rust owners directly. Header-matched
track/PBS helpers are shared. Station randomisation, animation and redraw retain source order
and the same native station identity. No world borrow spans a trigger or shared service. Checks:
the default rail-reservation, reload, controller and signal scenarios, whose adapter compiles
the pinned algorithm bodies; native runtime export/import carries both once-built PBS adapters
into CI shards. External NewGRF trigger reentry, custom railtype turn flags, legacy conversion
and threaded GUI coverage remain #156 gaps. PR #370.

### Tile movement dispatch (#371)

Rust owns tile track status, admission, pixel height (including outside-map), foundation
dispatch and every tile policy, rail/road foundations, depot distances, portal frames and bridge
entry preparation. Commands, drawing and existing tile loops retain their owners. Native map
arrays, pools and vehicle shells remain canonical; typed noexcept leaves carry consumed scalars
or stable identities. Owner loads/stores end before depot, road-stop, sound, viewport and NewGRF
calls. Train stopping, crossing occupancy, order stopping, slope/pixel and bridge helpers call
their integrated Rust owners directly. Road pathfinder track bits and road/ship YAPF plus
water-region movement queries also call Rust directly; header preconditions follow native
assertion policy, without valid-input FFI. Native facades serve remaining native callers;
portable original bodies remain excluded from WITH_RUST. Checks: `python3 tools/migration.py
rust-checks`, `build`, native ABI tests, the existing rail/road/station scenarios and `python3
tools/landscape-comparison.py` (foundation and signed-coordinate cases). NewGRF foundations,
train/ship bridges, road tunnels, partial-depot reversal timing, half-water and extreme
bridge-speed branches remain #156 gaps. PR #374.

### Vehicle station/depot visits and route-link prediction (#349)

`rust/openttd-kernels/src/vehicle/visits.rs` owns `BeginLoading`, `HandleLoading`,
`LeaveStation`, `CancelReservation`, `ResetRefitCaps`, `CalcPercentVehicleFilled`,
`VehicleEnterDepot` with depot-entry recording, all of `LinkRefresher` and both `IncreaseStats`
overloads. It calls the Rust orders, station loading (`prepare_unload`), cargo storage, refit
command, train/ship state, maintenance and pending-autoreplace owners directly, including
aircraft hangar entry. Train, ship, aircraft, road, orders, station-command and loading callers
reach it through `VehicleServices::visits`; the C++ member functions are thin facades for the
remaining native callers (destruction, station removal, saves). C++ keeps link-graph pool
identity, allocation, node/merge/schedule and edge updates, station/vehicle shells, NewGRF
triggers, window/news/sound leaves, and two `CommandCost` stack slots for the nested depot
refit. Prediction frames are entry-local (`Vec`, `BTreeSet`); temporary refit cargo/subtype
writes are restored in source order. No ordinary-play callback unwinds; `AI::NewEvent` queues.
Direct Rust entries preserve original assertions through the shared native assertion policy.
Checks: `orders-visits` (padhattan-ridge-2000, cargodist) and the station scenarios. Link-graph
pool exhaustion is a #156 gap. PR #359.

### Landscape generation and rivers (#350)

Rust owns initialization, complete GenerateLandscape orchestration, original terrain templates,
coverage lines, desert/rainforest, springs, lakes/wetlands, recursive river flow/widening, and
the complete river YAPF state/search. It also owns heightmap greyscale conversion/loading, flat
worlds, slope repair and ground watering. C++ keeps decoding, map storage/writes, sprite lookup
and presentation. Native helpers remain callees; existing tree, water, commands and scheduler
owners are called directly. The scheduler alone acquires its audited descriptor for each loop
call; generation reads use typed map services throughout. Sea BFS/recursion remains Rust-owned.
One stack optional unordered_set preserves native flattening order; each search reconstructs it
and ends before recursive flow. Both generation progress operations use generation_services.hpp,
returning before native exception rethrow; no generator borrow survives a progress call. Local
queues, lookup-only marks and YAPF nodes/heaps retain source traversal, admission, tie, RNG and
narrowing order, including native search diagnostics. Checks: the default
generated.py/landscape.py cases and the native sea-order unit test. External heightmap formats,
maximum dimensions, alternate graphics and late/GUI aborts remain #156 gaps. PR #364.

### Common ground-vehicle dynamics and canonical cache (#372)

`ground_vehicle.rs` owns train/road power and cargo cache rebuilds, acceleration, whole-chain
depot checks, crash slope-flag clearing, inclination/Z updates and speed accumulation, including
the selected train/road physics callees. Each native shell holds one Rust cache/flags allocation
with stable native aliases; normal/indexed construction and destruction share that owner.
`gv_flags` keeps its uint16 save field and old-loader access; the cache remains rebuilt on load.
C++ keeps shell/pool identity, engine/cargo specifications, NewGRF properties, viewport updates,
typed map operations and the shared base crash leaf. Controllers call the Rust owner directly;
original dynamics/helpers compile only portably. No owner borrow spans callbacks, and links are
read at the original loop points. Checks: `python3 tools/migration.py verify`, the default road,
rail and fleet scenarios and the fleet console comparing unchanged reference bodies. External
powered/articulated NewGRFs, maglev traffic, legacy saves and threaded GUI remain #156 gaps. PR
#375.

### Vehicle tile occupancy and live spatial queries (#377)

`vehicle/occupancy.rs` owns the process-wide tile buckets, complete on-tile/near cursors, hash
update/reset, and ground/track/tunnel occupancy command checks. Each stable native shell carries
the sole intrusive link record; Rust owns its mutations. Native pools, IDs, coordinates and
viewport indexing stay in C++. Original bodies compile only portably. Rust road/train collision,
signal, flooding, road-stop rebuild, ship, disaster and fleet/command scans enter the owner
directly; position operations update the index before calling native viewport leaves. No
query/update/advance allocation or mirrored occupancy state is introduced. Reset retains stale
links and rebuild order; iterators read live next after callbacks. No owner borrow spans
crash/flood reentry, and no generation guard changes source admission or revisitation. Shared
native command costs preserve local error messages; occupancy consumes no random draws. Shared
geometry/bit/track modules supply the original header callees. Lifetime rules are in
`docs/design/world-state.md`. Checks: the default road/crash/flood/reload, train collision,
road-stop and fleet scenarios and the fleet console comparing unchanged public iterator bodies.
GUI/external articulated-GRF gaps remain #156. PR #384, integrated in #391.

### Cargo-monitor accounting and ordered maps (#386)

Rust owns both process-wide ordered pickup/delivery maps, shared bit helpers,
activation/query/reset/stop, selective/global cleanup and delivery accounting. Payment and
company owners call Rust directly. Payment resolves stations through its existing subsidy
service at each original observation; independent stack cursors preserve nested industry scans.
Arrivals update registered keys only, without allocating, and unsigned amounts narrow to
signed32 before saturation. Native facades retain script validation, canonical stations/pools
and assertions. CMDL/CMPU retain two-scalar TempStorage, ascending save order, native SlObject
and SlError frames, historical key conversion and incremental first-insertion wins; no Rust map
borrow spans a native callback or decoding operation. Checks: `python3 tools/migration.py
simulate economy-manual economy-cargodist economy-stockpile companies-delete companies-acquire
engines-monitors`, `python3 tools/cargo-income-comparison.py` and native company/payment/ABI
tests. Historical monitor-key conversion and external GameScript callers remain #156 gaps. PR
#388, integrated in #391.

### Vehicle NewGRF resolver runtime (#392)

`newgrf_vehicle.rs` owns the five-value cache/validity allocation and lifetime, cache
fill/invalidation, stack self/parent/relative/root selection, wagon overrides, real-group
selection, complete vehicle/purchase variable control including 0x61,
callback/property/probability entries, subtype/aircraft helpers, randomisation and trigger
recursion. Native virtual facades keep the portable originals excluded from `WITH_RUST`; common
Action2 evaluation, specifications, pools, saved vehicle random/waiting fields and rendering
stay native. Existing train/road/aircraft/ship, cargo, command and loading owners enter Rust
properties/callbacks/randomisation directly. Typed observations read consumed fields only; typed
shared services preserve metadata observation, native indexing and GetSpec fallback behavior.
Neither cache/state borrows nor references into native storage cross common evaluation, storage
or reentry. Invocation paths allocate nothing; each native shell owns one canonical cache
allocation with native aliases, including indexed construction. Checks: the default fleet
resolver/reload, rail and aircraft fixtures with real groups, native ABI and existing aircraft
comparison probes. Native GRF reload destroys the synthetic groups and reinstalls fresh ones.
Arbitrary external GRF programs, historical saves, GUI scheduling and remaining branch
combinations are #156 gaps; the fixtures do not claim persistent synthetic sprite groups. PR
#397, integrated in #415.

### NewGRF transactional storage (#393)

`newgrf_storage.rs` owns process-wide modes, the pointer-ordered changed registry, temporary
registers/generation keys and canonical 16/256-word payloads/backups. Native pool shells retain
IDs and feature associations; save/debug aliases address the Rust arrays directly. Every mode
switch restores temporary changes in shell pointer order. PSAC saving restores payloads without
removing registry membership; destruction removes pending entries, and the registry outlives
native destructors. Rust also owns industry/airport/town StorePSA and town GRFID
selection/traversal. Native typed leaves retain pool allocation/publication and ordered town
associations; town publishes after StoreValue, while industry/airport publish before it.
Original bodies remain portable. Native SlObject, reference fixup, legacy copies and SlError
frames keep their original order, with no live Rust frame or borrow across save errors.
Compression workers receive already serialized bytes. Checks: the default `town-storage-reload`
scenario with real resolvers and PSAC, and native storage tests. External NewGRF programs,
complete pre-161 conversion saves and GUI thread scheduling remain #156 gaps. PR #400.

### Industry and industry-tile NewGRF resolvers (#398)

Rust owns both stack feature contexts, roots/lazy town, variables, count/search and
tile-relative helpers, probability/refusal and feature resolution policy. Existing
industry/registry/cargo owners supply canonical state directly; PSA reads and writes use the
integrated storage owner. Small copied invocation state ends native context borrows before
evaluator/storage reentry. Native adapters retain common Action2 frames, virtual facade
construction, specs/map/pool identity and consumed result copying; no per-resolution allocation
or world-record view. Original bodies remain portable. Checks: the default `industry-resolver`
scenario with real production, parent, self and tile reseed groups, alongside existing
production/lifecycle/animation/acceptance and cargo probes. External and historical gaps remain
#156. PR #407, integrated in #415.

### Rail-station and waypoint NewGRF resolution (#399)

Rust owns stack scope/root/cargo/lazy-town state, complete station/waypoint variables,
real-group selection, platform/end/continuation, slope callbacks and randomisation. All six
optional caches follow the resolver object's lifetime, including repeated DoResolve. Native
specs/association vectors, map/station shells, saved random/waiting bits, common Action2 and
optional TownScope frames remain. Direct cargo/station/movement owners serve command, animation
and PBS callers. Shared goods conversion, scope fallback, container/airport and metadata helpers
retain native types, narrowing and original GetSpec behavior. No owner/cache/container borrow
spans evaluation/storage; setters remain immediate. Original portable bodies and native callback
CommandCost/error frames retain order. Checks: the default resolver/reload fixtures with real
groups; transient groups are reconstructed after native load. Arbitrary external-GRF, historical
and GUI paths remain
#156 gaps. PR #411.

### Common Action2 execution and invocation state (#417)

`newgrf_spritegroup.rs` owns complete group dispatch, deterministic adjustments,
variables/procedures/ranges/errors/defaults, randomized selection and common resolver
reset/callback/rerandomisation state. `newgrf.rs` owns every shared global variable, including
Action7/9/D's public entry. Each native resolver embeds one canonical Rust stack carrier for its
full lifetime; compatibility aliases address those fields. Parsed graphs/specs, pools, terminal
payloads and profiler output remain native. Node observations copy only consumed metadata; no
graph mirror, per-call allocation, execution continuation or recursion guard is introduced. Rust
directly calls integrated register, feature-variable/real and PSA owners. Remaining native
feature scopes retain their original shared behavior. All raw state accesses end before
scope/procedure/storage reentry. Native facade and profiler ordering remain synchronous, with
noexcept leaves and aborting panics. Shared calendar converters serve engine consumers. Checks:
the default `town-storage-reload` scenario, which constructs actual groups before saving and
after reload, plus native layouts and table visitors. Arbitrary external GRFs/loader
conditionals, historical saves, remaining native feature combinations and GUI scheduling remain
#156 gaps. Native signed dword MIN/-1 division overflow has a compiler/platform-dependent
undefined result; the cross-game fixture excludes it and Rust uses explicit wrapping division.
PR #425.

### Road-stop NewGRF runtime and associations (#410)

Rust owns road-stop/road-waypoint stack context, root/cargo selection, lazy parent and variable
policy, callback and randomisation control, spec-slot selection, assignment/removal and
trigger-cache reconstruction. Native station shells retain shared spec/tile vectors, saved bits
and serialization; specs, parsed Action2 and rendering remain native. Thin virtual/evaluator
frames keep portable callers. Station variables, existing-cargo counts, empty masks and storage
use their Rust owners directly; station/spec metadata has one shared native definition. Ordinary
road-stop randomisation constructs CBID_NO_CALLBACK in the original; real groups preserve
waiting publication/retention, gates and zero new draws. Explicit CBID_RANDOM_TRIGGER fixtures
establish resolver masks, without claiming ordinary execution of the otherwise-unreachable
reseed/consumption branches. Transient spec/group reattachment after load does not establish
arbitrary external-GRF loading. Historical, external-GRF and named unreachable branches remain
#156 gaps. PR #422, integrated in #427.

### Airport and airport-tile NewGRF runtime (#418)

Rust owns both resolver stack contexts, roots, self/parent/default scopes, lazy town selection,
variables, random packing, tile callbacks and live airport selection/availability/bounds. Native
parsing, spec/layout/override storage, rendering, preview drawing and text remain. Canonical
station/cargo/town/storage owners and the industry relative-position helper are called directly.
Common Action2 and airport operations dispatch directly in Rust; native stack frames retain
their synchronous lifetimes. Animation and construction admission call the Rust owner. Shared
named support ports CanVehicleUseStation, IsHangar/IsHangarTile, GetVehiclePosOnBuild,
GetAirport and Airport::GetSpec preserve the original gates. Native hangar and rotation
algorithms accept selected specs; existing aircraft/orders/ship controls remain in their owners.
GetAirportCallback has no Rust export. Checks: the default airport/reload fixtures with real
groups, explicitly reinstalled after reload. Temporary admission builds carry no storage-writing
group because native removal leaves a deleted PSA pointer; the retained airport's PSA is saved
and reloaded. External loading, historical saves, GUI scheduling and that native removal setup
remain #156 gaps. PR #428, integrated in #444.

### Generic NewGRF callbacks (#429)

`newgrf_generic.rs` owns the process-wide ordered feature registry, reverse candidate traversal,
stack invocation/root/scope/variables, AI selection and ambient enable/chance/parameter/playback
policy. Registration alone allocates; resolution copies one pointer pair at a time, with no
registry snapshot. Native load phases exclude registration/reset during callbacks.
Context/container borrows end before Action2 or sound reentry; nested entries own their stack
state. Common Action2 evaluation and generic scope/variable dispatch call their Rust owners
directly. The native evaluation leaf constructs only the required stack virtual/profiler frame,
exposing the actual invocation and common carrier through typed bindings; no mirrored state or
virtual forwarding. C++ retains files, parsed graphs, specs, map, loading callers and playback.
Typed noexcept leaves do not throw during play; script commands follow callback return. Portable
originals stay. Clear/tree/water call Rust directly; tree retains its post-call feature
observation before refresh. Cargo bitnum and GRF-aware playback have canonical shared leaves
used by vehicle/animation consumers. Checks: the default `landscape-generic-callbacks` scenario
with two files; transient registrations are cleared before graph destruction and rebuilt after
native reload. External-GRF loading, script station-build integration, other feature
registrations, arbitrary programs, historical saves and GUI scheduling remain #156 gaps. PR
#436, integrated in #444.
