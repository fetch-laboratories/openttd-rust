#!/usr/bin/env python3
"""Build the pinned original and migration fork, retaining validation evidence."""

import argparse
import hashlib
import json
import os
import platform
import shlex
import shutil
import subprocess
import sys
import time
from contextlib import contextmanager
from contextvars import ContextVar
from datetime import datetime, timezone
from pathlib import Path

from validation_execution import file_lock, run_logged

ROOT = Path(__file__).resolve().parents[1]
LOCAL = ROOT / ".local"


def _common_root():
    """The main checkout; linked worktrees share its reference and caches."""
    try:
        common = subprocess.check_output(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            cwd=ROOT,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return ROOT
    return Path(common).parent


COMMON_ROOT = _common_root()
COMMON_LOCAL = COMMON_ROOT / ".local"
# One pinned reference checkout and build per clone, shared by every worktree.
REFERENCE = COMMON_LOCAL / "reference/openttd"
# The ASan and UBSan flavours (#505) build in their own trees. --sanitize exports
# SANITIZE_VARIABLE, so simulate, its adapters and its scenario modules see them too.
SANITIZE_VARIABLE = "OPENTTD_SANITIZE"
SANITIZE = os.environ.get(SANITIZE_VARIABLE) == "1"
TSAN_VARIABLE = "OPENTTD_TSAN"
TSAN = os.environ.get(TSAN_VARIABLE) == "1"
SANITIZED = "-sanitized" if SANITIZE else "-tsan" if TSAN else ""
REFERENCE_BUILD = COMMON_LOCAL / f"build-reference{SANITIZED}"
CANDIDATE_BUILD = ROOT / f"build-rust{SANITIZED}"
# Stable rustc accepts -Zsanitizer only under RUSTC_BOOTSTRAP; the precompiled std
# stays uninstrumented.
SANITIZER_FLAGS = "-fsanitize=address,undefined -fno-omit-frame-pointer"
TSAN_FLAGS = "-fsanitize=thread -fno-omit-frame-pointer"
RUST_SANITIZER_FLAGS = "-Zsanitizer=address -Cforce-frame-pointers=yes"
BASELINE = json.loads((ROOT / "migration/baseline.json").read_text())
SCENARIO_COVERAGE_PREFIX: ContextVar[str | None] = ContextVar(
    "scenario_coverage_prefix", default=None
)


def bootstrapped(name: str) -> Path:
    """Bootstrapped toolchains/dependencies live in the main checkout; worktrees
    use them directly instead of needing per-worktree symlinks."""
    return LOCAL / name if (LOCAL / name).exists() else COMMON_LOCAL / name


def deps_libraries() -> Path:
    return bootstrapped("deps") / "usr/lib/x86_64-linux-gnu"


def environment():
    env = os.environ.copy()

    def prepend(name, value):
        env[name] = str(value) + (os.pathsep + env[name] if env.get(name) else "")

    if (bootstrapped("cargo") / "bin").exists():
        env["CARGO_HOME"] = str(bootstrapped("cargo"))
        env["RUSTUP_HOME"] = str(bootstrapped("rustup"))
        prepend("PATH", bootstrapped("cargo") / "bin")
    deps = bootstrapped("deps")
    if deps.exists():
        prepend("PATH", deps / "usr/bin")
        prepend("LD_LIBRARY_PATH", deps_libraries())
        prepend("CMAKE_PREFIX_PATH", deps / "usr")
        prepend("CMAKE_INCLUDE_PATH", deps / "usr/include")
        prepend("CPATH", deps / "usr/include")
        prepend("PKG_CONFIG_PATH", deps / "usr/lib/x86_64-linux-gnu/pkgconfig")
        env["PKG_CONFIG_SYSROOT_DIR"] = str(deps)
    if (prefix := SCENARIO_COVERAGE_PREFIX.get()) is not None:
        env["OPENTTD_COVERAGE_PREFIX"] = prefix
    return env


def shell_exports() -> str:
    """Shell exports for each variable environment() sets or changes."""
    env = environment()
    return "".join(
        f"export {name}={shlex.quote(value)}\n"
        for name, value in sorted(env.items())
        if os.environ.get(name) != value
    )


def deps_build_options() -> list[str]:
    """Let bare host binaries and CTest load the bootstrapped libraries.

    DT_RPATH rather than DT_RUNPATH, because only DT_RPATH also serves the
    libraries' own dependencies (libicuuc.so needs libicudata.so)."""
    if not bootstrapped("deps").exists():
        return []
    return [
        f"-DCMAKE_BUILD_RPATH={deps_libraries()}",
        "-DCMAKE_EXE_LINKER_FLAGS=-Wl,--disable-new-dtags",
    ]


def rust_configuration(build: Path):
    """Read the target validated by CMake, refusing portable or stale archive layouts."""
    build = build.resolve()
    cache = cmake_cache(build)
    target = cache.get("RUST_TARGET")
    supported = {
        "x86_64-unknown-linux-gnu": ("Linux", 8),
        "aarch64-unknown-linux-gnu": ("Linux", 8),
        "aarch64-apple-darwin": ("Darwin", 8),
        "i686-pc-windows-msvc": ("Windows", 4),
        "x86_64-pc-windows-msvc": ("Windows", 8),
    }
    if cache.get("OPTION_RUST") != "ON" or target not in supported:
        raise RuntimeError(
            f"{build}: Rust must be enabled with a supported validated target"
        )
    platform, width = supported[target]
    if cache.get("RUST_PLATFORM") != platform or cache.get("RUST_POINTER_WIDTH") != str(
        width
    ):
        raise RuntimeError(f"{build}: Rust/C++ target metadata is inconsistent")
    archive_name = (
        "openttd_kernels.lib" if platform == "Windows" else "libopenttd_kernels.a"
    )
    if platform == "Windows" and (
        cache.get("RUST_CRT") != "static-release"
        or cache.get("CMAKE_MSVC_RUNTIME_LIBRARY") != "MultiThreaded"
        or cache.get("CMAKE_BUILD_TYPE") != "RelWithDebInfo"
        or cache.get("OPTION_USE_ASSERTS") != "ON"
        or cache.get("RUST_EFFECTIVE_FLAGS") != "-C;target-feature=+crt-static"
    ):
        raise RuntimeError(f"{build}: Windows target/CRT policy is inconsistent")
    directory = build / "cargo"
    archive = directory / target / "release" / archive_name
    if (
        Path(cache.get("RUST_TARGET_DIR", "")).resolve() != directory
        or Path(cache.get("RUST_ARCHIVE", "")).resolve() != archive
    ):
        raise RuntimeError(
            f"{build}: unexpected Rust archive layout; reconfigure this build"
        )
    return {
        "target": target,
        "host": cache.get("RUST_HOST", ""),
        "platform": platform,
        "pointer_bytes": width,
        "target_dir": str(directory),
        "archive": str(archive),
        "archive_name": archive_name,
        "crt": cache.get("RUST_CRT", ""),
        "effective_flags": cache.get("RUST_EFFECTIVE_FLAGS", "").split(";")
        if cache.get("RUST_EFFECTIVE_FLAGS")
        else [],
        "deployment_target": cache.get("CMAKE_OSX_DEPLOYMENT_TARGET", ""),
        "sdk": cache.get("CMAKE_OSX_SYSROOT", ""),
        "native_libraries": cache.get("RUST_NATIVE_LIBS", "").split(";"),
        "build_type": cache.get("CMAKE_BUILD_TYPE", ""),
        "assertions": cache.get("OPTION_USE_ASSERTS", ""),
    }


def rust_archive(build: Path, *, target_dir: Path | None = None) -> Path:
    """Locate an existing release archive using CMake's validated native target."""
    configuration = rust_configuration(build)
    directory = (
        Path(configuration["target_dir"])
        if target_dir is None
        else target_dir.resolve()
    )
    archive = (
        directory / configuration["target"] / "release" / configuration["archive_name"]
    )
    if not archive.is_file():
        raise RuntimeError(
            f"Rust archive missing: {archive}; build this validated target first"
        )
    return archive


CCACHE_POLICY = {
    "compiler_check": "content",
    "direct_mode": "true",
    "depend_mode": "false",
    "sloppiness": "",
    "hash_dir": "false",
    "remote_storage": "",
}


def compiler_cache_base(role):
    """Root containing the role's source and build trees. ccache rewrites paths
    below it to relative ones, so separate worktrees share cache entries."""
    return COMMON_ROOT.resolve() if role == "reference" else ROOT.resolve()


def compiler_cache_environment(env, role, *, bypass=False):
    """Isolate role caches and ignore ambient cache-policy overrides."""
    if role not in ("reference", "candidate"):
        raise ValueError("Compiler cache role must be reference or candidate")
    result = {key: value for key, value in env.items() if not key.startswith("CCACHE_")}
    result["CCACHE_CONFIGPATH"] = str(ROOT / "migration/ccache.conf")
    result["CCACHE_DIR"] = str(COMMON_LOCAL / "compiler-cache" / role)
    result["CCACHE_BASEDIR"] = str(compiler_cache_base(role))
    if bypass:
        result["CCACHE_DISABLE"] = "1"
    return result


def compiler_cache_settings(executable, env):
    """Retain and verify effective settings rather than trusting a config file."""
    output = subprocess.check_output([executable, "--show-config"], env=env, text=True)
    settings = {}
    for line in output.splitlines():
        if ") " in line and " =" in line:
            key, _, value = line.split(") ", 1)[1].partition(" =")
            settings[key] = value.strip()
    expected_policy = dict(CCACHE_POLICY, base_dir=env.get("CCACHE_BASEDIR", ""))
    for key, expected in expected_policy.items():
        if settings.get(key) != expected:
            raise RuntimeError(
                f"Unsafe ccache setting {key}: {settings.get(key)!r}, expected {expected!r}"
            )
    expected_disable = "true" if env.get("CCACHE_DISABLE") else "false"
    if settings.get("disable") != expected_disable:
        raise RuntimeError("Unexpected ccache bypass setting")
    return settings, output


def compiler_cache_statistics(executable, env):
    output = subprocess.check_output([executable, "--print-stats"], env=env, text=True)
    return {
        key: int(value) for key, value in (line.split() for line in output.splitlines())
    }


def compiler_cache_options(executable):
    """Clear stale launchers/PCH overrides when returning to ordinary builds."""
    return [
        f"-DCMAKE_C_COMPILER_LAUNCHER={executable or ''}",
        f"-DCMAKE_CXX_COMPILER_LAUNCHER={executable or ''}",
        f"-DCMAKE_DISABLE_PRECOMPILE_HEADERS={'ON' if executable else 'OFF'}",
    ]


def cmake_cache(build):
    cache = {}
    for line in (build / "CMakeCache.txt").read_text().splitlines():
        if line and not line.startswith(("#", "//")) and "=" in line:
            key, value = line.split("=", 1)
            cache[key.split(":", 1)[0]] = value
    return cache


def configured_as(build, source, command, env=None):
    """Whether an existing build tree already holds every requested -D value."""
    if not (build / "build.ninja").exists() or not (build / "CMakeCache.txt").exists():
        return False
    cache = cmake_cache(build)
    if Path(cache.get("CMAKE_HOME_DIRECTORY", "")).resolve() != Path(source).resolve():
        return False
    if cache.get("OPTION_RUST") == "ON" and env is not None:
        captured = f"CARGO_PROFILE_RELEASE_DEBUG={env.get('CARGO_PROFILE_RELEASE_DEBUG', '<unset>')}\n"
        if captured not in (build / "rust-build-configuration.txt").read_text():
            return False
    requested = dict(
        argument[2:].split("=", 1) for argument in command if argument.startswith("-D")
    )
    return all(
        cache.get(key.split(":", 1)[0]) == value for key, value in requested.items()
    )


def verify_compiler_cache_options(build, executable):
    cache = cmake_cache(build)
    expected = {
        "CMAKE_C_COMPILER_LAUNCHER": executable or "",
        "CMAKE_CXX_COMPILER_LAUNCHER": executable or "",
        "CMAKE_DISABLE_PRECOMPILE_HEADERS": "ON" if executable else "OFF",
    }
    if any(cache.get(key) != value for key, value in expected.items()):
        raise RuntimeError(
            f"{build}: compiler-cache/PCH mode does not match the requested mode"
        )
    expected.update(
        {key: cache.get(key) for key in ("CMAKE_C_COMPILER", "CMAKE_CXX_COMPILER")}
    )
    return expected


def compiler_cache_compatibility(env=None):
    """Key for restoring cache directories. ccache itself hashes compiler content,
    arguments and every included file, so this only groups compatible entries;
    it deliberately excludes workflow and driver files that component PRs edit."""
    env = environment() if env is None else env
    tools = {}
    for name in (env.get("CC", "cc"), env.get("CXX", "c++"), "ccache"):
        tools[name] = subprocess.check_output([name, "--version"], env=env, text=True)
    files = ("migration/ccache.conf",)
    policy = {
        name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in files
    }
    identity = {
        "os": platform.system(),
        "architecture": platform.machine(),
        "baseline": BASELINE["commit"],
        "policy": policy,
        "tools": tools,
    }
    digest = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
    identity["prefix"] = (
        f"compiler-v1-{identity['os']}-{identity['architecture']}-{BASELINE['commit']}-{digest}"
    )
    return identity


def git(*args, cwd=ROOT):
    return subprocess.check_output(["git", *args], cwd=cwd, text=True).strip()


@contextmanager
def reference_lock(shared=False):
    """Serialize reference checkout/build/test across concurrent worktree runs.

    Shared holders (tools/simulate.py copying the built reference's runtime) may
    overlap each other but never a build."""
    with file_lock(COMMON_LOCAL / "reference.lock", shared=shared, label="reference"):
        yield


@contextmanager
def candidate_lock(build, shared=False):
    """Protect candidate compilation and runtime capture in this build directory."""
    with file_lock(
        Path(build).resolve() / ".migration.lock", shared=shared, label="candidate"
    ):
        yield


def ensure_reference():
    if not REFERENCE.exists():
        REFERENCE.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            [
                "git",
                "worktree",
                "add",
                "--detach",
                str(REFERENCE),
                BASELINE["commit"],
            ],
            cwd=ROOT,
            check=True,
        )
    if git("rev-parse", "HEAD", cwd=REFERENCE) != BASELINE["commit"]:
        raise RuntimeError("Reference checkout differs from migration/baseline.json")
    if git("status", "--porcelain", "--untracked-files=all", cwd=REFERENCE):
        raise RuntimeError(
            "Reference checkout has changes; restore the original before comparison"
        )


def supply_graphics(build):
    candidates = [
        LOCAL / "deps/usr/share/games/openttd/baseset/opengfx",
        COMMON_LOCAL / "deps/usr/share/games/openttd/baseset/opengfx",
        Path("/usr/share/games/openttd/baseset/opengfx"),
        Path("/usr/share/openttd/baseset/opengfx"),
    ]
    source = next((path for path in candidates if path.is_dir()), None)
    if source is None:
        raise RuntimeError(
            "OpenGFX is needed for regression games; install it or run tools/bootstrap-local.py"
        )
    destination = build / "baseset/opengfx"
    destination.parent.mkdir(parents=True, exist_ok=True)
    if not destination.exists():
        destination.symlink_to(source, target_is_directory=True)


def free_cores():
    """Cores the 1-minute load leaves idle, read once per run."""
    return max(2, (os.cpu_count() or 2) - round(os.getloadavg()[0]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "action",
        choices=(
            "build",
            "verify",
            "tools",
            "simulate",
            "coverage",
            "rust-checks",
            "deep",
            "env",
        ),
        nargs="?",
        default="verify",
    )
    parser.add_argument(
        "--jobs", type=int, default=free_cores(), help="default: free cores"
    )
    parser.add_argument(
        "--ccache",
        action="store_true",
        help="Require the shared compiler cache (default: use it when ccache is available)",
    )
    parser.add_argument(
        "--no-ccache",
        action="store_true",
        help="Ordinary builds without the compiler cache (PCH enabled)",
    )
    parser.add_argument(
        "--ccache-bypass",
        action="store_true",
        help="Measure the same no-PCH configuration without cache reuse",
    )
    parser.add_argument(
        "--sanitize",
        action="store_true",
        help="build or simulate the ASan and UBSan flavours of both games",
    )
    parser.add_argument(
        "--tsan", action="store_true", help="C++ ThreadSanitizer builds"
    )
    parser.add_argument(
        "--threaded", action="store_true", help="SDL offscreen scenarios"
    )
    # simulate forwards any further arguments to tools/simulate.py; deep, scenario selectors.
    args, simulate_args = parser.parse_known_args()
    if simulate_args and args.action not in ("simulate", "coverage", "deep"):
        parser.error(f"unrecognized arguments: {' '.join(simulate_args)}")
    if args.no_ccache and (args.ccache or args.ccache_bypass):
        parser.error("--no-ccache conflicts with --ccache/--ccache-bypass")
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    if args.action == "env":
        print(shell_exports(), end="")
        return
    args.sanitize = args.sanitize or SANITIZE
    args.tsan = args.tsan or TSAN
    if args.sanitize and (args.tsan or args.threaded):
        parser.error("--sanitize conflicts with --tsan/--threaded")
    if (args.sanitize or args.tsan or args.threaded) and args.action not in (
        "build",
        "simulate",
    ):
        parser.error("sanitizer/threaded options apply to build and simulate")
    if args.action == "simulate" and args.tsan and not args.threaded:
        parser.error("--tsan simulation requires --threaded")
    if (args.sanitize and not SANITIZE) or (args.tsan and not TSAN):
        exported = os.environ | {
            SANITIZE_VARIABLE if args.sanitize else TSAN_VARIABLE: "1"
        }
        os.execve(sys.executable, [sys.executable, *sys.argv], exported)
    if args.action == "deep":
        from deep import run_deep

        sys.exit(run_deep(ROOT, LOCAL, COMMON_LOCAL, args.jobs, simulate_args))
    started = time.monotonic()
    if args.action != "rust-checks":
        with reference_lock():
            ensure_reference()
    env = environment()
    instruction_mode = args.action == "simulate" and "--instructions" in simulate_args
    if (args.tsan or args.threaded) and instruction_mode:
        parser.error("--tsan/--threaded conflicts with --instructions")
    if args.sanitize:
        if instruction_mode:
            parser.error("--sanitize conflicts with --instructions")
        env |= {
            "RUSTFLAGS": RUST_SANITIZER_FLAGS,
            "RUSTC_BOOTSTRAP": "1",
        }
    if instruction_mode:
        env["CARGO_PROFILE_RELEASE_DEBUG"] = "line-tables-only"
    env["CARGO_BUILD_JOBS"] = str(args.jobs)
    cache_executable = (
        None
        if args.no_ccache or args.action == "rust-checks"
        else shutil.which("ccache", path=env["PATH"])
    )
    if (
        args.action != "rust-checks"
        and (args.ccache or args.ccache_bypass)
        and cache_executable is None
    ):
        parser.error("--ccache requires ccache on PATH")
    args.ccache = cache_executable is not None
    env["CARGO_TARGET_DIR"] = str(ROOT / "build-rust/cargo")
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    evidence = LOCAL / "verification" / stamp
    evidence.mkdir(parents=True)
    report = {
        "schema_version": 1,
        "jobs": args.jobs,
        "candidate_root": str(ROOT.resolve()),
        "baseline": BASELINE,
        "candidate_commit": git("rev-parse", "HEAD"),
        "candidate_status": git("status", "--short"),
        "action": args.action,
        "candidate_rust_enabled": args.action not in ("coverage", "rust-checks"),
        "started_at": stamp,
        "commands": [],
        "compiler_cache": {
            "enabled": args.ccache,
            "bypassed": args.ccache_bypass,
            "executable": cache_executable,
            "version": subprocess.check_output(
                [cache_executable, "--version"], env=env, text=True
            ).splitlines()[0]
            if cache_executable
            else None,
            "roles": {},
        },
        "passed": False,
    }

    def run(name, command, cwd=ROOT):
        log = evidence / f"{name}.log"
        start = time.monotonic()
        code = run_logged(
            command, cwd=cwd, env=env, log=log, phase=name, stream=name == "simulate"
        )
        report["commands"].append(
            {
                "name": name,
                "argv": list(map(str, command)),
                "cwd": str(cwd),
                "exit_code": code,
                "seconds": round(time.monotonic() - start, 3),
                "log": str(log.relative_to(ROOT)),
            }
        )
        if code:
            print(log.read_text(errors="replace")[-16000:], file=sys.stderr)
            raise RuntimeError(f"{name} failed; see {log}")
        return log

    candidate_build = ROOT / CANDIDATE_BUILD.name
    builds = {"reference": REFERENCE_BUILD.resolve(), "candidate": candidate_build}
    if args.action == "coverage":
        builds = {"reference": LOCAL / "build-reference-coverage"}
    original_environment = env.copy()
    cache_before = {}

    def select_role(role):
        nonlocal env
        env = (
            compiler_cache_environment(
                original_environment, role, bypass=args.ccache_bypass
            )
            if args.ccache
            else original_environment.copy()
        )
        if cache_executable:
            settings, text = compiler_cache_settings(cache_executable, env)
            (evidence / f"{role}-ccache-config.log").write_text(text)
            cache_before[role] = compiler_cache_statistics(cache_executable, env)
            report["compiler_cache"]["roles"][role] = {
                "settings": settings,
                "before": cache_before[role],
            }

    try:
        if args.action in ("verify", "rust-checks"):
            for name, command in (
                ("rust-fmt", ["cargo", "fmt", "--all", "--", "--check"]),
                (
                    "rust-check",
                    ["cargo", "check", "--workspace", "--all-targets", "--locked"],
                ),
                (
                    "rust-clippy",
                    [
                        "cargo",
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--locked",
                        "--",
                        "-D",
                        "warnings",
                    ],
                ),
                ("rust-tests", ["cargo", "test", "--workspace", "--locked"]),
            ):
                run(name, command)
        if args.action == "rust-checks":
            report["passed"] = True
            print("rust-checks: passed", flush=True)
            return
        common = [
            "-G",
            "Ninja",
            "-DCMAKE_BUILD_TYPE=RelWithDebInfo",
            "-DOPTION_USE_ASSERTS=ON",
            "-DOPTION_DEDICATED=OFF",
            "-DCMAKE_DISABLE_FIND_PACKAGE_Grfcodec=ON",
            "-DBUILD_TESTING=ON",
        ]
        if args.sanitize or args.tsan:
            common += [
                f"-DCMAKE_{kind}_FLAGS={TSAN_FLAGS if args.tsan else SANITIZER_FLAGS}"
                for kind in ("C", "CXX", "EXE_LINKER")
            ]
        if args.action == "tools":
            builds = {"candidate": LOCAL / "build-tools-rust"}
            common = [
                "-G",
                "Ninja",
                "-DCMAKE_BUILD_TYPE=RelWithDebInfo",
                "-DOPTION_USE_ASSERTS=OFF",
            ]
        if args.action == "coverage":
            common += [
                "-DCMAKE_C_COMPILER=gcc",
                "-DCMAKE_CXX_COMPILER=g++",
                "-DCMAKE_C_FLAGS=--coverage",
                "-DCMAKE_CXX_FLAGS=--coverage",
                "-DCMAKE_C_FLAGS_RELWITHDEBINFO=-O0 -g -DNDEBUG",
                "-DCMAKE_CXX_FLAGS_RELWITHDEBINFO=-O0 -g -DNDEBUG",
                "-DCMAKE_EXE_LINKER_FLAGS=--coverage",
                "-DBUILD_TESTING=OFF",
            ]
        common.extend(compiler_cache_options(cache_executable))
        for name, build in builds.items():
            with (
                reference_lock()
                if name == "reference" and args.action != "coverage"
                else candidate_lock(build)
            ):
                # Other worktrees rebuild the shared pinned reference concurrently;
                # its record stays bound to the executable digest instead.
                if name != "reference":
                    (build / "instruction-build.json").unlink(missing_ok=True)
                select_role(name)
                source = REFERENCE.resolve() if name == "reference" else ROOT
                extra = (
                    []
                    if name == "reference"
                    else [
                        "-DBINARY_NAME=openttd-rust",
                        "-DOPTION_RUST=ON",
                        *deps_build_options(),
                    ]
                )
                if args.action == "tools":
                    extra.append("-DOPTION_TOOLS_ONLY=ON")
                configure = [
                    "cmake",
                    "-S",
                    str(source),
                    "-B",
                    str(build),
                    *common,
                    *extra,
                ]
                # Ninja reruns CMake itself when any CMake input changes, so an
                # explicit configure is only needed when the cache lacks a requested value.
                if configured_as(build, source, configure, env):
                    print(
                        f"{name}-configure: CMakeCache already matches; reusing {build}",
                        flush=True,
                    )
                else:
                    run(f"{name}-configure", configure)
                report[f"{name}_compiler_settings"] = verify_compiler_cache_options(
                    build, cache_executable
                )
                if name == "candidate":
                    report["rust_configuration"] = rust_configuration(build)
                if args.action != "tools":
                    supply_graphics(build)
                # Sanitized trees build only the game and its runtime data: the test
                # executables would also run under ASan during test discovery.
                targets = ["--target", "openttd", "regression_files"]
                run(
                    f"{name}-build",
                    ["cmake", "--build", str(build), "--parallel", str(args.jobs)]
                    + (
                        targets
                        if args.sanitize or args.tsan or args.action == "coverage"
                        else []
                    ),
                )
                if instruction_mode:
                    from simulation.instructions import record_build

                    record_build(build, source, name, env)

        if args.action == "verify":
            inventories = {}
            for name, build in builds.items():
                with reference_lock() if name == "reference" else candidate_lock(build):
                    log = run(
                        f"{name}-test-inventory",
                        ["ctest", "--show-only=json-v1"],
                        cwd=build,
                    )
                    inventories[name] = {
                        test["name"] for test in json.loads(log.read_text())["tests"]
                    }
                    report[f"{name}_test_count"] = len(inventories[name])
                    if not inventories[name]:
                        raise RuntimeError(
                            f"{name.capitalize()} CTest inventory is empty; refusing untested verification"
                        )
                    run(
                        f"{name}-tests",
                        [
                            "ctest",
                            "--output-on-failure",
                            "--parallel",
                            str(args.jobs),
                            "--output-junit",
                            str(evidence / f"{name}-tests.xml"),
                        ],
                        cwd=build,
                    )
            missing = inventories["reference"] - inventories["candidate"]
            if missing:
                raise RuntimeError(
                    f"Candidate removed reference tests: {sorted(missing)}"
                )
        if args.action == "simulate":
            # --jobs is the driver's; each simulation job runs two game processes.
            log = run(
                "simulate",
                [
                    sys.executable,
                    str(
                        ROOT
                        / (
                            "tools/simulation/threaded.py"
                            if args.threaded
                            else "tools/simulate.py"
                        )
                    ),
                    "--jobs",
                    str(max(1, args.jobs // 2)),
                    *simulate_args,
                ],
            )
        if args.action == "coverage":
            run(
                "coverage",
                [
                    sys.executable,
                    str(ROOT / "tools/simulation/coverage.py"),
                    "--build",
                    str(builds["reference"]),
                    "--jobs",
                    str(max(1, args.jobs // 2)),
                    *simulate_args,
                ],
            )
        ensure_reference()
        report["passed"] = True
    finally:
        if cache_executable:
            for role, before in cache_before.items():
                after = compiler_cache_statistics(
                    cache_executable,
                    compiler_cache_environment(
                        original_environment, role, bypass=args.ccache_bypass
                    ),
                )
                report["compiler_cache"]["roles"][role].update(
                    {
                        "after": after,
                        "delta": {
                            key: after.get(key, 0) - value
                            for key, value in before.items()
                        },
                    }
                )
        report["total_seconds"] = round(time.monotonic() - started, 3)
        (evidence / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"Evidence: {evidence / 'report.json'}", flush=True)
    print(f"{args.action}: passed", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
