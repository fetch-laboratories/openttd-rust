# OpenTTD-Rust migration tools

Building, local checks and the comparison harness. CI is in [CI](ci.md).

## Repository and reference

| Item | Location |
| --- | --- |
| Fork and `origin` | https://github.com/fetch-laboratories/openttd-rust |
| `upstream` | https://github.com/OpenTTD/OpenTTD.git |
| Integration branch | `rust-migration` |
| Original | OpenTTD 15.3, `14ec60f248547d4d062a1160f0fc26d742319888`, pinned in `migration/baseline.json` |
| Shared reference worktree and build | `.local/reference/openttd` and `.local/build-reference` in the main checkout |
| Candidate build | `build-rust` in each worktree; the game is `build-rust/openttd-rust` |
| Rust crate | `rust/openttd-kernels` |

- Read the original from any worktree with `python3 tools/migration.py original show FILE[:A-B]` or `original grep PATTERN [PATH ...]`, which run `git show <baseline commit>:src/FILE` and `git grep PATTERN <baseline commit> -- src/`; `original path` prints the reference worktree after checking it is clean and at the pin.
- Direct CMake builds need the pinned commit object for the native-test oracle headers: `git fetch --no-tags --depth=1 origin 14ec60f248547d4d062a1160f0fc26d742319888`.

## Prerequisites and host

Native Linux builds need a C++20 compiler, CMake, Ninja, Python 3.11 or newer, the libraries in `COMPILING.md`, OpenGFX and the Rust toolchain pinned in `rust-toolchain.toml`. Node.js runs `tools/test-ci.py`; an authenticated `gh` sends CI requests.

```sh
sudo apt-get install build-essential cmake ninja-build python3 pkg-config \
  libsdl2-dev liblzma-dev libpng-dev libcurl4-openssl-dev libfreetype-dev \
  libfontconfig-dev libharfbuzz-dev libicu-dev liblzo2-dev openttd-opengfx
```

- This host runs Ubuntu 24.04 x86-64 with these packages plus `valgrind`, `ccache`, `unifdef` and `gdb`. Keep `.local/deps` absent here: the driver puts it ahead of the system on every compiler, library and pkg-config path.
- `tools/bootstrap-local.py --install` is for an Ubuntu 26.04 host without administrator access. Do not run it here.

## Build and verification

| Command | Use |
| --- | --- |
| `python3 tools/migration.py rust-checks` | The four Cargo checks (fmt, check, clippy, test), without building the reference. Run Clippy only this way, so the ratchet's replay of the same command stays cached. |
| `python3 tools/migration.py build` | Build both games without tests. |
| `python3 tools/migration.py verify` | Cargo checks, both games and both CTest suites. Logs go to `.local/verification/<timestamp>/`. |
| `python3 tools/migration.py tools` | Build the native Rust generators. |
| `python3 tools/migration.py simulate`, `coverage`, `deep` | See [simulation comparison](#simulation-comparison), [coverage](#original-simulation-coverage) and [deep checks](deep-checks.md). |
| `python3 tools/run-comparisons.py` | Run every `tools/*-comparison.py`: the component comparisons, not the simulation scenarios. |
| `python3 tools/migration.py exec -- CMD ...` | Run one command, such as `cargo test -p openttd-kernels`, in the driver environment. |
| `python3 tools/migration.py fmt` | Format the Rust workspace and the Python Quick checks, at the pinned versions; `--check` only reports, `--fix` also applies Ruff's lint fixes. It stages nothing. |
| `eval "$(python3 tools/migration.py env)"` | Set the driver environment in a shell that runs many commands. |
| `python3 tools/git_hooks.py install` | Install the pre-push and pre-commit hooks. |

- Run the candidate with `./build-rust/openttd-rust -X`, which avoids the global game folders.
- Supported targets: native GNU/Linux x86-64 (aarch64 builds but no CI job tests it), macOS arm64, Emscripten and the scoped [Windows MSVC mode](#windows-msvc). Unsupported: 32-bit Windows ([ADR 0007](adr/0007-no-32-bit-windows.md)), other platforms, cross compilation, macOS universal or Intel.

### Quick cleanup checks

[Quick](ci.md#jobs-and-profiles) runs the Cargo checks and these gates; `python3 tools/preflight.py --base origin/rust-migration` runs them all locally. Run one gate with `python3 tools/preflight.py --base origin/rust-migration --step NAME`, which runs the Quick steps whose name contains NAME (`--list` names them) with Quick's pinned versions and base. `python3 tools/preflight.py --fast` is the seconds-long check before a commit: the pre-push hook's fast tier and Quick's Ruff and ty lines, with no stamp. Preserve the original's casts and control flow: a faithful truncation must not become a checked conversion to reduce debt.

| Command | Fails on |
| --- | --- |
| `python3 tools/lint-ratchet.py clippy --base-ref origin/rust-migration` | More or larger force-warned Clippy diagnostics than the merge base's `migration/clippy-debt.json`. |
| `python3 tools/lint-ratchet.py ruff --base-ref origin/rust-migration` | The same for C901 and PLR0911/12/13/15 over `tools/`, against `migration/ruff-debt.json`. |
| `python3 tools/lint-policy.py --base-ref origin/rust-migration` | Weaker lint configuration, new Rust `allow`/`expect` or Python `noqa` sites. |
| `python3 tools/growth-ratchet.py --base-ref origin/rust-migration` | Growth in tools C++, fork-added `src/` test C++, Rust exports and callback fields. |
| `python3 tools/refactor-proof.py --base-ref origin/rust-migration` | A claimed release-archive equivalence that does not hold. |
| `python3 tools/code-standards-inventory.py --gate origin/rust-migration` | A file in `migration/finished-files.txt` gaining a raw adapter or export call, an internal `execute: bool` parameter, a `GameState<Vec<...>>` scratch pool or a command over seven parameters. Each file pass adds its files to the list. |

- Function length, nesting and cognitive complexity are not measured ([ADR 0024](adr/0024-lint-ratchet-scope.md)).
- When a ratchet reports `rename not recorded`, or lint policy prints a ready carry, run the same command with `--write`.

#### Cleanup records

A change's carries and exception proposals go in `migration/cleanup-records/<branch with / as ->.json`, a JSON list of records, not in the PR body. The checks name any malformed field.

- **Carry**: a rename or move that keeps a lint row or suppression site. `{"kind":"carry","tool":"clippy"|"ruff"|"rust"|"python","from":"<base key>","to":"<head key>"}`; it needs no review. The tools print ready carries.
- **Lint exception** (`kind: lint`) or **suppression** (`kind: suppression`): add only the exact item or site row, and give `tool`, `site`, `operation`, `source_contract`, `reason`, `retiring_owner`, `retirement_condition`, and `before` and `after` (lint: the item's magnitude lists; suppression: occurrence counts). Module-wide attributes, lint groups, new `noqa` and weaker configuration are not accepted.
- **Cast reason**: an item-scoped `allow` or `expect` of `non_upper_case_globals` or the Clippy `cast_possible_truncation`, `cast_possible_wrap` or `cast_sign_loss` lints needs no record when its `reason = "..."` names the original definition or cast it preserves.
- **Growth exception** (`kind: growth`): the `counter` and `added` mapping the check prints, plus `reason`, `existing_alternatives`, `retiring_owner` and `retirement_condition`.
- **Archive proof**: `{"kind":"refactor-proof","method":"archive","renames":{}}`. A proof is not behavioural coverage of unexercised branches.
- **Acceptance**: the independent reviewer's attributed report repeats each proposal exactly as `{"head":"<tested PR SHA>","accept":[<proposal>]}` in a `cleanup-acceptance` fence. Growth awaiting acceptance reports `pending review`; it passes on PR runs and fails in the merge group. An acceptance survives base updates while the accepted item text or added source is unchanged, except export and callback-slot growth, which needs re-acceptance at each new head.

### Generated ABI probes and fixture defaults

- Add ABI declarations to `layouts!` in the owning Rust module, with `rust_field => cpp_field` for aliases and `width field: RustType` for a native-width check. Add no second C++ field list or numeric type ID.
- Fixtures specify the callbacks they use; other callback defaults abort.

## Original simulation coverage

`python3 tools/migration.py coverage --jobs 4` measures the default scenarios' coverage of the pinned original into `.local/coverage`. Re-measure after scenario inputs change. It is neither equivalence nor integration evidence. `--timeout SECONDS` sets the per-game limit (default 600).

## Compiler cache

Cache build outputs only when the key covers everything that determines them; do not cache test results. Worktrees share the main checkout's reference, reference build and `.local/compiler-cache/`. After a configure-time environment change, such as a dependency prefix, use a fresh build directory: the driver skips CMake configure when the cached `-D` values match.

## Output disk

Write-once output lives on the HDD at `/mnt/storage` behind symlinks; caches, toolchains, the pinned original and its builds, and each worktree's `build-rust` stay on the SSD.

| Path | Symlink target |
| --- | --- |
| Main checkout `.local/` `simulation`, `verification`, `coverage`, `deep`, `speed-pass`, `worktrees` | `/mnt/storage/openttd-rust/main-local/<name>` |
| Each other worktree's `.local` | `/mnt/storage/openttd-rust/worktree-local/<worktree>-<id>/local` |

- `.git/hooks/post-checkout` links each new worktree's `.local` through `.git/hooks/offload-local`; both are untracked. Worktrees inside the main `.local` keep a real `.local`.
- `offload-local migrate` moves a worktree's real `.local` onto the HDD, for example after `git worktree add --no-checkout` skipped the hook, and recreates a missing main-checkout link. `offload-local sweep` lists output of removed worktrees; `--apply` deletes it. The hook also deletes output of worktrees removed more than 24 hours ago.
- If the HDD is unmounted, the links dangle and runs fail. Mount it and rerun.
- In tool code, `.resolve()` can take a `.local` path off the checkout's tree, so compute `relative_to(ROOT)` from the unresolved path. `Path.rglob` follows a symlinked starting folder but not symlinked subfolders.

## Simulation comparison

`python3 tools/migration.py simulate` builds both games and runs `tools/simulate.py`. Scenario comparisons run on CI, not on this host; run one locally only by name, to diagnose a failure ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)).

- Add scenarios and their prepare and check hooks to the component's module in `tools/simulation/`; `tools/simulation/registry.py` assembles them, and a hook for other modules' scenarios goes in its `SHARED_HOOKS`.
- `MASKS` (`tools/simulation/savegame.py`) masks the random save ID, build revision, NewGRF version and the original's uninitialized `round_trip_time`; a port touching the last needs its own check. Record a known port divergence in `KNOWN_FAILURES` by first divergence and an open issue, not as a new mask.
- After changing scenarios, comparison cases, `MASKS` or `KNOWN_FAILURES`, run `python3 tools/harness_inventory.py --write`; after changing comparison cases, `python3 tools/run-comparisons.py --record`. Quick fails on an unrecorded change.
- Community AI, arctic and NewGRF scenarios use pinned downloads ([input setup](../migration/community-ai/README.md), [evidence](porting.md#evidence)). Saves are described in `migration/saves/README.md`.

See [speed](speed.md) for benchmarks and [deep checks](deep-checks.md) for soak, sanitizer, threaded and multiplayer runs.

## Platform linkage

`rust_archive(build)` in `tools/migration.py` locates the Rust archive. `tools/check-rust-platform.py` checks linkage and refusals of unsupported configurations.

### macOS

- Requires `CMAKE_OSX_ARCHITECTURES=arm64`, a deployment minimum of 11.0 or newer, and matching `SDKROOT` and `MACOSX_DEPLOYMENT_TARGET` for Rust. Linux results do not establish Darwin support.

### Emscripten

- Requires the `wasm32-unknown-emscripten` std, emsdk 4.0.23 and `-fwasm-exceptions`.

### Windows MSVC

- Supported: VS 2022 MSVC, x64, single-config Ninja, `RelWithDebInfo`, `OPTION_USE_ASSERTS=ON` and the static release CRT, with the `x86_64-pc-windows-msvc` std.
- CMake refuses every other configuration, naming the requirement.

## Shared-code checks and port metrics

Quick runs all but `port-metrics.py`, against `--base-ref origin/rust-migration` where they take one. Each baseline file only shrinks.

| Command | Checks |
| --- | --- |
| `python3 tools/shared-accessors.py` | The [shared-accessor rule](porting.md#callees-and-shared-accessors); macros, derived fields and compound callbacks need review. |
| `python3 tools/rebound-callees.py` | Duplicate bindings and forwarded service fields; macros, lambdas, wrappers, some templates and namespace resolution need review. |
| `python3 tools/rust-only.py` | `WITH_RUST` conditionals in `src/`. |
| `python3 tools/ffi-bool-returns.py` | Rust-defined C `bool` returns. |
| `python3 tools/orphan-exports.py` | Rust exports and C++ declarations without production use; tests and harnesses do not count. An export used only by uncalled C++ members needs a row in `tools/uncalled-export-baseline.txt` naming the owner issue that retires it. |
| `python3 tools/port-metrics.py` | Code-line counts and original retirement; `--whole-fork` adds source areas, C++ evidence and exports; `--functions [AREA ...]` adds remaining original lines per file and function. |

- No checker detects Rust copies of original header helpers and constants ([porting](porting.md#callees-and-shared-accessors)).
- Metric scope: version 2 does not count extracted-original comparison fragments or C++ embedded in Python strings, so `fork_cpp` and `candidate_cpp_upper_bound` cover only the listed inventory. Report a later coverage correction separately from deletion. Raw `rust` totals include oracle text under `rust/`; report implementation and oracle data separately. No line metric claims behavioural completion.
