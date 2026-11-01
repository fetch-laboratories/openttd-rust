"""Shared driver for pinned-reference comparisons and gap probes.

A comparison builds one fixture two ways: against the unchanged pinned
sources, and against the candidate with WITH_RUST and the build-rust archive.
It then requires identical output.
"""

import difflib
import subprocess
from pathlib import Path

import migration

ROOT = migration.ROOT
BUILD = ROOT / "build-rust"
REFERENCE = migration.REFERENCE.resolve()
LABELS = (("reference", REFERENCE), ("candidate", ROOT))
ENV = migration.environment()


def output(name, directory=None):
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


def body(text, signature):
    """Signature through its matching brace; pinned bodies balance braces, even in comments."""
    start = text.index(signature)
    end = text.index("{", start) + 1
    depth = 1
    while depth:
        depth += (text[end] == "{") - (text[end] == "}")
        end += 1
    return text[start:end]


def compiler(build=BUILD):
    return migration.cmake_cache(build)["CMAKE_CXX_COMPILER"]


def target_flags(build=BUILD):
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


def rust(build=BUILD, *, define=True):
    """WITH_RUST (unless define is false), the validated archive and its libraries."""
    configuration = migration.rust_configuration(build)
    archive = str(migration.rust_archive(build))
    return ["-DWITH_RUST"] * define + [archive, *configuration["native_libraries"]]


def compile(command, log, env=None):
    """Run a compile or link command, retaining its combined output in log."""
    with Path(log).open("wb") as handle:
        subprocess.run(
            list(map(str, command)),
            env=env or ENV,
            stdout=handle,
            stderr=subprocess.STDOUT,
            check=True,
        )


def execute(argv, log, *, input=None, env=None, stderr="empty"):
    """Run a fixture and return stdout; stderr is "empty", "merged" or "allowed"."""
    result = subprocess.run(
        list(map(str, argv)),
        input=input,
        env=env or ENV,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT if stderr == "merged" else subprocess.PIPE,
    )
    Path(log).with_suffix(".out").write_bytes(result.stdout)
    if result.stderr:
        Path(log).with_suffix(".err").write_bytes(result.stderr)
    if result.returncode or (stderr == "empty" and result.stderr):
        raise RuntimeError(f"{argv[0]} exited {result.returncode}; see {log}.*")
    return result.stdout


def same(expected, actual, name, out):
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


def compare(out, mode, command, *, labels=LABELS, input=None, stderr="empty"):
    """Build command(label, source) + [-o binary] for each label and compare runs."""
    outputs = {}
    for label, source in labels:
        binary = out / f"{label}-{mode}"
        argv = [*command(label, source), "-o", binary]
        compile(argv + (rust() if label == "candidate" else []), f"{binary}.log")
        outputs[label] = execute([binary], binary, input=input, stderr=stderr)
        same(outputs[labels[0][0]], outputs[label], binary.name, out)
    return outputs[labels[0][0]]


def passed(name, **counts):
    """Recheck the reference and print the inventory run-comparisons reports."""
    migration.ensure_reference()
    summary = ", ".join(
        f"{value} {key.replace('_', ' ')}" for key, value in counts.items()
    )
    print(f"{name} passed: {summary}")
