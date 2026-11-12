"""Shared driver for pinned-reference comparisons and gap probes.

A comparison builds one fixture two ways: against the unchanged pinned
sources, and against the candidate with WITH_RUST and the build-rust archive.
It then requires identical output.
"""

import difflib
import os
import re
import subprocess
import sys
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path

import migration

ROOT = migration.ROOT
BUILD = ROOT / "build-rust"
REFERENCE = migration.REFERENCE.resolve()
LABELS = (("reference", REFERENCE), ("candidate", ROOT))
ENV = migration.environment()
# A command argument; compile and execute stringify each one.
Argument = str | Path
Labels = Sequence[tuple[str, Path]]
# Preflight's compile check (#923) sets this: compile each fixture for syntax only,
# against freshly generated ABI headers, and link or run nothing.
GENERATED = os.environ.get("PROBE_COMPILE_ONLY", "")
# Every comparison guards each run, and each check of a run's output, with RUN.
RUN = not GENERATED
COMPILE_FAILED = 3


def compile_only(
    command: Sequence[Argument], generated: str = GENERATED
) -> list[str] | None:
    """The compile without its output, link inputs or the build's generated ABI.

    None for a command that compiles no C or C++ source: a link alone, or a
    generator a comparison runs, which compile-only mode does not run.
    """
    argv = list(map(str, command))
    compilers = re.compile(r"(c\+\+|[gc]\+\+|clang\+\+|gcc|cc|clang)(-[0-9.]+)?$")
    if not argv or not compilers.search(Path(argv[0]).name):
        return None
    if not any(a.endswith((".c", ".cc", ".cpp", ".cxx", ".mm")) for a in argv[1:]):
        return None
    built = BUILD / "generated-abi"
    stale = {str(built), str(built.resolve())}
    checked: list[str] = []
    arguments = iter(argv)
    for argument in arguments:
        if argument == "-o":
            next(arguments, None)
        elif not argument.startswith(("-l", "-L", "-Wl,")) and not argument.endswith(
            (".a", ".so", ".o")
        ):
            for path in stale:
                argument = argument.replace(path, generated)
            checked.append(argument)
    return [*checked, "-fsyntax-only"]


def output(name: str, directory: Argument | None = None) -> Path:
    """Check the pinned reference and return the comparison's scratch directory."""
    # Shallow CI checkouts lack the immutable oracle commit the reference needs.
    present = subprocess.run(
        ["git", "cat-file", "-e", migration.BASELINE["commit"] + "^{commit}"],
        cwd=ROOT,
        capture_output=True,
    )
    if present.returncode and not REFERENCE.exists():
        subprocess.run(
            ["git", "fetch", "--no-tags", "origin", migration.BASELINE["commit"]],
            cwd=ROOT,
            check=True,
        )
    migration.ensure_reference()
    out = Path(directory or migration.LOCAL / name).resolve()
    out.mkdir(parents=True, exist_ok=True)
    return out


def body(text: str, signature: str) -> str:
    """Signature through its matching brace; pinned bodies balance braces, even in comments."""
    start = text.index(signature)
    end = text.index("{", start) + 1
    depth = 1
    while depth:
        depth += (text[end] == "{") - (text[end] == "}")
        end += 1
    return text[start:end]


def compiler(build: Path = BUILD) -> str:
    return migration.cmake_cache(build)["CMAKE_CXX_COMPILER"]


def target_flags(build: Path = BUILD) -> list[str]:
    """Pointer-width and Darwin SDK flags CMake gives Rust-consuming C++."""
    configuration = migration.rust_configuration(build)
    flags = ["-DPOINTER_IS_64BIT"] if configuration["pointer_bytes"] == 8 else []
    if configuration["platform"] == "Darwin":
        architecture = migration.cmake_cache(build).get("CMAKE_OSX_ARCHITECTURES")
        if (
            architecture != "arm64"
            or not configuration["sdk"]
            or not configuration["deployment_target"]
        ):
            raise RuntimeError("Probe needs the validated macOS architecture and SDK")
        flags += ["-arch", architecture, "-isysroot", configuration["sdk"]]
        flags.append("-mmacosx-version-min=" + configuration["deployment_target"])
    return flags


def rust(build: Path = BUILD, *, define: bool = True) -> list[str]:
    """WITH_RUST (unless define is false), the validated archive and its libraries."""
    if GENERATED:
        return ["-DWITH_RUST"] * define
    configuration = migration.rust_configuration(build)
    archive = str(migration.rust_archive(build))
    return ["-DWITH_RUST"] * define + [archive, *configuration["native_libraries"]]


def compile(
    command: Sequence[Argument],
    log: Argument,
    env: Mapping[str, str] | None = None,
    *,
    cwd: Path | None = None,
) -> None:
    """Run a compile or link command, retaining its combined output in log."""
    argv = compile_only(command) if GENERATED else list(map(str, command))
    if argv is None:
        Path(log).write_text("compile-only: not a compile; not run\n")
        return
    with Path(log).open("wb") as handle:
        code = subprocess.run(
            argv, cwd=cwd, env=env or ENV, stdout=handle, stderr=subprocess.STDOUT
        ).returncode
    if code and GENERATED:
        print(f"compile-only: failed; see {log}", file=sys.stderr)
        raise SystemExit(COMPILE_FAILED)
    if code:
        raise subprocess.CalledProcessError(code, argv)


def execute(
    argv: Sequence[Argument],
    log: Argument,
    *,
    input: bytes | None = None,
    env: Mapping[str, str] | None = None,
    stderr: str = "empty",
    cwd: Path | None = None,
) -> bytes:
    """Return stdout; stderr is "empty", "merged", "allowed" or "inherited"."""
    if not RUN:
        raise RuntimeError(f"compile-only mode runs nothing, not {argv[0]}")
    result = subprocess.run(
        list(map(str, argv)),
        input=input,
        cwd=cwd,
        env=env or ENV,
        stdout=subprocess.PIPE,
        stderr=(
            None
            if stderr == "inherited"
            else subprocess.STDOUT
            if stderr == "merged"
            else subprocess.PIPE
        ),
    )
    Path(log).with_suffix(".out").write_bytes(result.stdout)
    if result.stderr:
        Path(log).with_suffix(".err").write_bytes(result.stderr)
    if result.returncode or (stderr == "empty" and result.stderr):
        raise RuntimeError(f"{argv[0]} exited {result.returncode}; see {log}.*")
    return result.stdout


def same(expected: bytes, actual: bytes, name: str, out: Path) -> None:
    """Raise with a retained unified diff when two outputs differ."""
    if expected == actual:
        return
    diff = difflib.unified_diff(
        expected.decode(errors="replace").splitlines(True),
        actual.decode(errors="replace").splitlines(True),
        fromfile="reference",
        tofile=name,
    )
    (out / f"{name}.diff").write_text("".join(diff))
    raise RuntimeError(f"{name} differs from the reference; see {out / name}.diff")


def compare(
    out: Path,
    mode: str,
    command: Callable[[str, Path], Sequence[Argument]],
    *,
    labels: Labels = LABELS,
    input: bytes | None = None,
    stderr: str = "empty",
) -> bytes:
    """Build command(label, source) + [-o binary] for each label and compare runs."""
    outputs = {}
    for label, source in labels:
        binary = out / f"{label}-{mode}"
        argv = [*command(label, source), "-o", binary]
        compile(argv + (rust() if label == "candidate" else []), f"{binary}.log")
        if RUN:
            outputs[label] = execute([binary], binary, input=input, stderr=stderr)
            same(outputs[labels[0][0]], outputs[label], binary.name, out)
    return outputs.get(labels[0][0], b"")


def passed(name: str, **counts: int) -> None:
    """Recheck the reference and print the inventory run-comparisons reports."""
    if not RUN:
        print(f"{name}: every fixture compiled; nothing ran")
        return
    if not counts or 0 in counts.values():
        raise RuntimeError(f"{name} compared no cases: {counts}")
    migration.ensure_reference()
    summary = ", ".join(
        f"{value} {key.replace('_', ' ')}" for key, value in counts.items()
    )
    print(f"{name} passed: {summary}")
