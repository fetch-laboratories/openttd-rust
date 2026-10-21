"""Load-independent loop work; reuse canonical runs and semantic comparisons (#321)."""

import hashlib
import json
import platform
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ENTRIES = ("StateGameLoop()", "_GenerateWorld()")


def verify_debug(archive):
    """Inspect our compiled objects, excluding bundled standard-library DWARF."""
    members = subprocess.check_output(["ar", "t", str(archive)], text=True).splitlines()
    members = [m for m in members if ".openttd_kernels." in m and m.endswith(".o")]
    if not members:
        raise ValueError("missing compiled Rust objects")
    with tempfile.TemporaryDirectory() as directory:
        obj = Path(directory) / "kernel.o"
        for member in members:
            obj.write_bytes(subprocess.check_output(["ar", "p", str(archive), member]))
            info = subprocess.check_output(
                ["readelf", "--debug-dump=info", str(obj)], text=True
            )
            if "DW_AT_stmt_list" not in info or "DW_TAG_subprogram" in info:
                raise ValueError(
                    "compiled Rust requires effective line-tables-only debug"
                )


def record_build(build, source, role, env):
    """Publish only successful builds, bound to the executable being measured."""
    settings = (build / "CMakeCache.txt").read_text()
    if role == "candidate":
        import migration

        files = ("rust-build-configuration.txt", "rust-toolchain.txt")
        settings += "".join((build / f).read_text() for f in files)
        verify_debug(migration.rust_archive(build))
    inputs = [source / "Cargo.toml", source / "rust/openttd-kernels/Cargo.toml"]
    configs = [p / ".cargo" for p in (source, *source.parents)]
    configs.append(Path(env.get("CARGO_HOME", Path.home() / ".cargo")))
    inputs += [p / name for p in configs for name in ("config", "config.toml")]
    settings += json.dumps(
        [(p.name, digest(p.read_bytes())) for p in inputs if p.is_file()]
    )
    for path, label in ((build, "<build>"), (source, "<source>")):
        settings = settings.replace(str(path), label)
    binary = build / ("openttd" if role == "reference" else "openttd-rust")
    (build / "instruction-build.json").write_text(
        json.dumps(dict(settings=settings, executable=digest(binary.read_bytes())))
    )


def flags(scenario):
    entries = ENTRIES if scenario["kind"] == "generate" else ENTRIES[:1]
    return [
        "--command-line-only=yes",
        "--tool=callgrind",
        "--collect-atstart=no",
        *[f"--toggle-collect={e}" for e in entries],
    ]


def count(path):
    text = path.read_text()
    events = re.findall(r"^events: (.+)$", text, re.M)
    totals = re.findall(r"^totals: ([0-9 ]+)$", text, re.M)
    complete = "desc: Trigger: Program termination" in text
    if len(events) != 1 or len(totals) != 1 or not complete:
        raise ValueError("incomplete/malformed callgrind profile")
    values, names = totals[0].split(), events[0].split()
    if len(values) != len(names) or names.count("Ir") != 1:
        raise ValueError("missing/malformed Ir event")
    value = int(values[names.index("Ir")])
    if value <= 0:
        raise ValueError("nonpositive loop Ir")
    return value


def digest(value):
    if not isinstance(value, bytes):
        value = json.dumps(value, sort_keys=True).encode()
    return hashlib.sha256(value).hexdigest()


def identity(scenario, binary, build, folder, env, valgrind):
    """Compatibility excludes candidate code; full measurement identity retains it."""
    from .core import RUNTIME_DIRECTORIES

    inputs = {}
    for base in (build, folder):
        for p in sorted(base.rglob("*")):
            if p.is_file() and (
                base == folder or p.parts[len(build.parts)] in RUNTIME_DIRECTORIES
            ):
                inputs[str(p.relative_to(base))] = digest(p.read_bytes())
    resolved = dict(scenario)
    if "save" in resolved:
        resolved["save"] = digest(Path(resolved["save"]).read_bytes())
    libraries = subprocess.check_output(["ldd", str(binary)], env=env, text=True)
    inputs["libraries"] = [
        digest(Path(p).read_bytes()) for p in re.findall(r"(/\S+) \(", libraries)
    ]
    metadata = json.loads((build / "instruction-build.json").read_text())
    executable = digest(binary.read_bytes())
    if metadata["executable"] != executable:
        raise ValueError(
            "stale instruction build; rebuild with simulate --instructions"
        )
    settings = metadata["settings"]
    version = subprocess.check_output(
        [str(valgrind), "--command-line-only=yes", "--version"], env=env, text=True
    ).strip()
    libraries = Path(env["VALGRIND_LIB"])
    tools = [sorted(libraries.glob(p)) for p in ("callgrind-*", "vgpreload_core-*")]
    if not all(tools):
        raise ValueError("incomplete selected Valgrind tool/core-preload identity")
    tool = (
        str(valgrind),
        version,
        digest(valgrind.read_bytes()),
        [digest(p.read_bytes()) for group in tools for p in group],
    )
    policy = "64x64/serial/witness-off/v1"
    settings = settings, platform.machine(), tool, flags(scenario), policy
    compatible = digest([resolved, inputs, settings])
    return {"compatible": compatible, "executable": executable}


def prepare(scenario, command, binary, build, folder, env, reference):
    import migration

    root = migration.COMMON_LOCAL / "valgrind/usr"
    valgrind = root / "bin/valgrind.bin"
    if valgrind.exists():
        env.update(VALGRIND_LIB=str(root / "libexec/valgrind"))
    else:
        valgrind = Path(shutil.which("valgrind") or "/missing-valgrind")
    if not env.get("VALGRIND_LIB"):
        prefix = valgrind.resolve().parent.parent
        env["VALGRIND_LIB"] = str(
            next(
                p
                for p in (prefix / "libexec/valgrind", prefix / "lib/valgrind")
                if list(p.glob("callgrind-*"))
            )
        )
    env.pop("OPENTTD_WITNESS", None)
    measurement = identity(scenario, binary, build, folder, env, valgrind)
    key = digest(measurement)
    cache = migration.COMMON_LOCAL / "instruction-cache" / f"{key}.json"
    cached = json.loads(cache.read_text()) if reference and cache.exists() else None
    if cached is not None and (type(cached) is not int or cached <= 0):
        raise ValueError("invalid cached reference Ir")
    if cached is None:
        outputs = [
            f"--callgrind-out-file={folder / 'callgrind.out'}",
            f"--log-file={folder / 'callgrind.log'}",
        ]
        command = [str(valgrind), *flags(scenario), *outputs, *command]
    return command, dict(identity=measurement, cached=cached, cache=cache)


def finish(runs, result):
    if result["differences"] or result["known_failures"] or result["problems"]:
        raise ValueError(
            "semantically unequal/unsuccessful profiles are not speed evidence"
        )
    measurements = {}
    for role, run in runs.items():
        info = run["instructions"]
        value = info["cached"]
        if value is None:
            value = count(run["dir"] / "callgrind.out")
        measurements[role] = dict(identity=dict(info["identity"]), Ir=value)
    candidate = measurements["candidate"]["identity"]
    candidate["compatible"] = digest(
        [candidate["compatible"], measurements["reference"]["identity"]]
    )
    info = runs["reference"]["instructions"]
    info["cache"].parent.mkdir(parents=True, exist_ok=True)
    temporary = runs["reference"]["dir"] / "reference-Ir.json"
    temporary.write_text(json.dumps(measurements["reference"]["Ir"]))
    temporary.replace(info["cache"])
    return measurements


def projection(anchor, value):
    return anchor["ratio"] * (1 + 2 * max(0, value / anchor["Ir"] - 1))


def budget(name, measurement, record):
    from .roads import speed_budget

    anchor, integrated = record["anchor"], record["integrated"]
    if measurement["identity"]["compatible"] != anchor["identity"]["compatible"]:
        raise ValueError(
            "instruction anchor identity mismatch; quiet speed pass required"
        )
    ratio = projection(anchor, measurement["Ir"])
    rolling = projection(anchor, integrated["Ir"])
    return dict(projected_ratio=ratio, **speed_budget(name, ratio, rolling))
