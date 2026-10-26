#!/usr/bin/env python3
"""Check native macOS arm64 and Windows MSVC Rust builds against cmake/Rust.cmake.

Rust consumers need WITH_RUST, the original assertion flags and (Windows) only
the static release CRT; Windows refuses unsupported configurations; the archive
follows its effective configuration; fresh generators run. A WITH_RUST consumer
cannot link without the archive, so linkage and architecture follow the build.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
from pathlib import Path

import migration

# Consumer: compile source suffix and binary. Tools builds hold the first two;
# macOS game builds the first four.
CONSUMERS = {
    "strgen": ("/strgen/strgen.cpp", "src/strgen/strgen"),
    "settingsgen": ("/settingsgen/settingsgen.cpp", "src/settingsgen/settingsgen"),
    "game": ("/openttd.cpp", "openttd"),
    "tests": ("/tests/test_main.cpp", "openttd_test"),
    "abi": ("/migration/windows-abi.cpp", "openttd_rust_abi"),
}
MODE = "RelWithDebInfo and OPTION_USE_ASSERTS=ON"
CRT = "dynamic and debug CRTs are unsupported"
FLAGS = "conflicting CRT flags"
MISMATCH = "CRT mismatch suppression"
ARCHITECTURE = "Configured C++ architecture/platform/pointer width"
RUNTIME = "-DCMAKE_MSVC_RUNTIME_LIBRARY="
LINKER = "-DCMAKE_EXE_LINKER_FLAGS="
# /NODEFAULTLIB breaks project()'s smoke link before the policy runs; an
# archive-only smoke test lets that refusal reach the unchanged CRT guard.
ARCHIVE_ONLY = "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY"
NO_CRT = "-C\x1ftarget-feature=-crt-static"
UNVALIDATED = "contradictory or unvalidated flags"
# Windows configurations (CMake arguments, environment) that must fail with a diagnostic.
REFUSALS = [
    (["-DCMAKE_BUILD_TYPE=Debug"], {}, MODE),
    (["-DCMAKE_BUILD_TYPE=Release"], {}, MODE),
    (["-DOPTION_USE_ASSERTS=OFF"], {}, MODE),
    ([RUNTIME + "MultiThreadedDLL"], {}, CRT),
    ([RUNTIME + "MultiThreadedDebug"], {}, CRT),
    (["-DCMAKE_CXX_FLAGS=/MD"], {}, FLAGS),
    (['-DCMAKE_CXX_FLAGS="/MD"'], {}, FLAGS),
    ([LINKER + "/force:multiple"], {}, MISMATCH),
    ([LINKER + '"/NODEFAULTLIB"', ARCHIVE_ONLY], {}, MISMATCH),
    (["-DHOST_BINARY_DIR=external"], {}, "external HOST_BINARY_DIR"),
    (["-DCMAKE_SYSTEM_NAME=Windows"], {}, "cross-OS builds are unsupported"),
    ([], {"RUSTFLAGS": "-C target-feature=-crt-static"}, UNVALIDATED),
    ([], {"CARGO_ENCODED_RUSTFLAGS": NO_CRT}, "unvalidated CARGO_ENCODED_RUSTFLAGS"),
    ([], {"CARGO_PROFILE_RELEASE_PANIC": "unwind"}, "release panic=abort"),
    (["-DCMAKE_CXX_FLAGS=/D_M_ARM64"], {}, ARCHITECTURE),
    (["-G", "Ninja Multi-Config"], {}, "single-config Ninja"),
]


def run(argv, log, env=None):
    result = subprocess.run(
        list(map(str, argv)), env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
    )
    Path(log).write_bytes(result.stdout)
    return result


def assertion_policy(command, consumer, build_type, windows):
    """Debug: WITH_ASSERT for game/tests. Release: NDEBUG for all, and on Windows
    (RelWithDebInfo with OPTION_USE_ASSERTS) also WITH_ASSERT for game/tests/abi."""
    definitions = set(re.findall(r'(?:^|[\s"])[-/]D\s*"?([A-Za-z_]\w*)', command))
    custom = consumer in ("game", "tests", "abi")
    expected = {
        "WITH_RUST": True,
        "NDEBUG": build_type != "Debug",
        "WITH_ASSERT": custom and (windows or build_type == "Debug"),
    }
    actual = {name: name in definitions for name in expected}
    if build_type not in ("Debug", "RelWithDebInfo") or actual != expected:
        raise RuntimeError(f"{consumer} lacks the original flags {expected}: {command}")
    runtimes = re.findall(r'(?:^|[\s"])[-/](M[DT]d?)(?=[\s"]|$)', command)
    if windows and (not runtimes or set(runtimes) != {"MT"}):
        raise RuntimeError(f"{consumer} is not only the static release CRT: {command}")


def check_windows_link(out, build, label, consumer, binary, configuration):
    """MSVC may link successfully after CMake drops repeated native arguments."""
    target = {
        "game": "openttd",
        "tests": "openttd_test",
        "abi": "openttd_rust_abi",
    }.get(consumer, consumer)
    result = run(
        ["ninja", "-C", build, "-t", "commands", target],
        out / f"{label}-{consumer}-link.log",
    )
    if result.returncode:
        raise RuntimeError(f"Cannot read {consumer} link command")

    def expand(text, seen):
        for match in re.finditer(r'@(?:"([^"]+)"|([^\s"&]+))', text):
            path = (build / (match[1] or match[2])).resolve()
            if path not in seen:
                seen.add(path)
                text += "\n" + expand(path.read_text(encoding="utf-8-sig"), seen)
        return text

    links = []
    for line in result.stdout.decode().splitlines():
        if not re.search(r"(?:vs_link_exe|\blink(?:\.exe)?\b)", line, re.I):
            continue
        command = expand(line, set())
        normalized = command.replace("\\", "/").replace('"', "")
        if re.search(
            r"(?:^|\s)/out:" + re.escape(binary) + r"\.exe(?=\s|$)", normalized, re.I
        ):
            links.append(command)
    arguments = [
        r'(?:"' + re.escape(arg) + r'"|' + re.escape(arg) + r")"
        for arg in configuration["native_libraries"]
    ]
    if (
        len(links) != 1
        or not arguments
        or not re.search(
            r"(?:^|\s)" + r"\s+".join(arguments) + r"(?=\s|$)", links[0], re.I
        )
    ):
        raise RuntimeError(f"{consumer} lost the ordered native library argument group")


def check_build(out, label, build, target, windows):
    configuration = migration.rust_configuration(build)
    if configuration["target"] != target:
        raise RuntimeError(f"{label} targets {configuration['target']}, not {target}")
    debug = configuration["build_type"] == "Debug"
    if not windows and configuration["assertions"] != ("ON" if debug else "OFF"):
        raise RuntimeError("Unexpected C++ assertion option for this matrix mode")
    built, configured = (
        re.findall(r"native-static-libs: ([^\r\n]+)", (build / log).read_text())[-1:]
        for log in ("rust-build.log", "rust-native-libs.log")
    )
    if not built or built != configured:
        raise RuntimeError("Archive native libraries differ from the configure query")
    count = 2 if label == "tools" else 5 if windows else 4
    commands = json.loads((build / "compile_commands.json").read_text())
    for consumer, (suffix, binary) in list(CONSUMERS.items())[:count]:
        matching = [
            entry["command"]
            for entry in commands
            if entry["file"].replace("\\", "/").endswith(suffix)
        ]
        if not matching:
            raise RuntimeError(f"Missing {label}/{consumer} compile command")
        for command in matching:
            assertion_policy(command, consumer, configuration["build_type"], windows)
        if windows:
            check_windows_link(out, build, label, consumer, binary, configuration)
            log = out / f"{label}-{consumer}-imports.log"
            imports = run(["dumpbin", "/dependents", build / f"{binary}.exe"], log)
            dynamic = rb"\b(?:VCRUNTIME\w*|MSVCP\w*|UCRTBASE)\.dll"
            if imports.returncode or re.search(dynamic, imports.stdout, re.I):
                raise RuntimeError(f"{consumer} imports a dynamic C/C++ runtime")
    return configuration


def refuse(out, name, flags, changes, diagnostic):
    generator = [] if flags[:1] == ["-G"] else ["-G", "Ninja"]
    argv = ["cmake", "-S", migration.ROOT, "-B", out / f"reject-{name}"]
    argv += ["-DOPTION_RUST=ON", "-DOPTION_TOOLS_ONLY=ON", "-DOPTION_USE_ASSERTS=ON"]
    argv += ["-DCMAKE_BUILD_TYPE=RelWithDebInfo", RUNTIME + "MultiThreaded"]
    log = out / f"reject-{name}.log"
    result = run([*argv, *generator, *flags], log, os.environ | changes)
    if result.returncode == 0 or diagnostic.encode() not in result.stdout:
        raise RuntimeError(f"Expected refusal {diagnostic!r}; see {log}")


def refusals(out, configuration):
    for index, (flags, changes, diagnostic) in enumerate(REFUSALS):
        refuse(out, index, flags, changes, diagnostic)
    clang = shutil.which("clang-cl") or r"C:\Program Files\LLVM\bin\clang-cl.exe"
    # clang-cl defaults to the host architecture; use the job's actual target so
    # its smoke test passes before the unsupported-compiler guard.
    flags = [f"-DCMAKE_CXX_COMPILER={clang}"]
    flags += [f"-DCMAKE_CXX_COMPILER_TARGET={configuration['target']}"]
    refuse(out, "clang-cl", flags, {}, "VS 2022 MSVC")
    libdir = subprocess.check_output(
        ["rustc", "--print", "target-libdir", "--target", configuration["target"]],
        text=True,
    ).strip()
    hidden = [(p, Path(f"{p}.hidden")) for p in Path(libdir).glob("libstd-*.rlib")]
    if not hidden:
        raise RuntimeError("No installed target std rlib for the missing-std refusal")
    # Hide only the ephemeral CI toolchain's target std rlibs, restoring them after.
    try:
        for path, backup in hidden:
            path.rename(backup)
        refuse(out, "missing-std", [], {}, "Cannot obtain native static libraries")
    finally:
        for path, backup in hidden:
            if backup.exists():
                backup.rename(path)


def freshness(out, tools, configuration):
    """Unchanged reconfiguration keeps the archive; a configuration change and its
    restoration each change the stamp and rebuild the archive."""
    archive = migration.rust_archive(tools)
    stamp = tools / "rust-build-configuration.txt"
    original = stamp.read_bytes()
    changed = os.environ | {"CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "2"}
    minimum = configuration["deployment_target"]
    darwin = configuration["platform"] == "Darwin"
    for name in ("unchanged", "changed", "restored", "restored-unchanged"):
        env = changed if name == "changed" and not darwin else None
        expected = configuration
        flags = []
        if darwin:
            # Cargo tracks profile flags itself; the deployment minimum also
            # needs CMake's configuration dependency to invalidate the crate.
            target = "12.0" if name == "changed" and minimum != "12.0" else minimum
            if name == "changed" and target == minimum:
                target = "11.0"
            flags = [f"-DCMAKE_OSX_DEPLOYMENT_TARGET={target}"]
            expected = configuration | {"deployment_target": target}
        before = archive.stat().st_mtime_ns
        for step, argv in (
            ("configure", ["cmake", "-S", migration.ROOT, "-B", tools, *flags]),
            ("build", ["cmake", "--build", tools, "--target", "tools", "-j", "4"]),
        ):
            if run(argv, out / f"freshness-{name}-{step}.log", env).returncode:
                raise RuntimeError(f"Freshness {name} {step} failed; see its log")
        current = stamp.read_bytes()
        rebuilt = archive.stat().st_mtime_ns > before
        unchanged = name in ("unchanged", "restored-unchanged")
        successful = (tools / "rust-build-configuration.txt.successful").read_text()
        if (
            (current == original) == (name == "changed")
            or rebuilt == unchanged
            or successful != hashlib.sha256(current).hexdigest()
            or migration.rust_configuration(tools) != expected
        ):
            raise RuntimeError(f"Freshness {name}: stamp, rebuild or metadata is wrong")


def fresh_generators(out, tools, exe):
    generated = out / "fresh-generated"
    shutil.rmtree(generated, ignore_errors=True)
    generated.mkdir()
    lang = migration.ROOT / "src/lang"
    table = migration.ROOT / "src/table"
    ini = re.findall(
        r"\$\{CMAKE_CURRENT_SOURCE_DIR\}/(\w+\.ini)",
        (table / "settings/CMakeLists.txt").read_text(),
    )
    strgen = [tools / f"src/strgen/strgen{exe}", "-s", lang, "-d", generated]
    settingsgen = [tools / f"src/settingsgen/settingsgen{exe}"]
    settingsgen += ["-o", generated / "settings.h"]
    settingsgen += ["-b", table / "settings.h.preamble"]
    settingsgen += ["-a", table / "settings.h.postamble"]
    for name, argv in (
        ("strgen-header", strgen),
        ("strgen-english", [*strgen, lang / "english.txt"]),
        ("strgen-french", [*strgen, lang / "french.txt"]),
        ("settingsgen", [*settingsgen, *(table / "settings" / name for name in ini)]),
    ):
        if run(argv, out / f"fresh-{name}.log").returncode:
            raise RuntimeError(f"Fresh {name} failed; see its log")
    for name in ("strings.h", "english.lng", "french.lng", "settings.h"):
        if not (generated / name).is_file() or not (generated / name).stat().st_size:
            raise RuntimeError(f"Fresh generator output {name} is missing or empty")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    for name in ("--build", "--tools", "--output"):
        parser.add_argument(name, type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    windows = args.target.endswith("-windows-msvc")
    tools = args.tools.resolve()
    check_build(out, "game", args.build.resolve(), args.target, windows)
    configuration = check_build(out, "tools", tools, args.target, windows)
    if windows:
        refusals(out, configuration)
    freshness(out, tools, configuration)
    fresh_generators(out, tools, ".exe" if windows else "")
    print(f"Native Rust platform check passed for {args.target}")


if __name__ == "__main__":
    main()
