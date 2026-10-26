"""Load-independent loop work; reuse canonical runs and semantic comparisons."""

import hashlib
import json
import platform
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ENTRIES = ("StateGameLoop()", "_GenerateWorld()")

# Quiet speed pass at f4923c79eb (#275): ratio is the median of
# four three-pair medians; Ir and compatible identity come from profiles of the
# same build.
SPEED_HEAD = "f4923c79eb"
# fmt: off
SPEED_PASS = (
    ("generate-tgp-256-1", 1.1783150183150184, 2887178684, "f6f82b6bbc4769f652d1451890003fef0f2c7f12c76c6d8d5549614a7d35a1e1"),
    ("play-opus-55-167-002-manual", 1.4018075231030847, 29051671196, "b4ecf7337dcda62268175b3e99de2e43ffb3a3ffe4f608ee365751dbaabc23a2"),
    ("play-grok-159-001-manual", 1.394822381542774, 27562796094, "46dcbfc4ceba5d11801053ac2eb8d35ac640082a6393744d0825fe9dca7229c8"),
    ("play-padhattan-ridge-1996-manual", 1.3354044451788813, 4673474662, "27801b82dceb31797d73fc2f2f9448be781d1551b2594408b562d7f96b79323d"),
    ("play-padhattan-ridge-2000-manual", 1.3934479388770433, 7553601680, "17e34235425021ca2f96c739312d9f15ca167ac5f579617d6c9eb1047e8c2627"),
    ("play-padhattan-ridge-2016-manual", 1.4694601348806955, 13575725183, "602255e34b6a04367a3aea9749e289d8efe8bdd921ba2f2a58e9bc57d11ed7b6"),
)
# Loop Ir at the anchor, in SPEED_PASS order, for projecting instruction-count growth.
SPEED_INTEGRATED = SPEED_HEAD, tuple(ir for _, _, ir, _ in SPEED_PASS)
# fmt: on
SPEED_ANCHORS = {
    name: dict(
        anchor=dict(head=SPEED_HEAD, ratio=ratio, Ir=ir, identity={"compatible": c}),
        integrated=dict(head=SPEED_INTEGRATED[0], Ir=integrated),
    )
    for (name, ratio, ir, c), integrated in zip(
        SPEED_PASS, SPEED_INTEGRATED[1], strict=True
    )
}
SPEED_BUDGETS = {name: ratio for name, ratio, _, _ in SPEED_PASS}


def speed_budget(name, ratio, integrated=None):
    """Report the speed record and ceiling without making timings a CI failure."""
    record = SPEED_BUDGETS.get(name) if integrated is None else integrated
    if record is None or ratio is None:
        return None
    ceiling = 1.5 if name.startswith("play-") else 1.3
    return {
        "record": record,
        "change_percent": (ratio / record - 1) * 100,
        "ceiling": ceiling,
        "within_ceiling": ratio <= ceiling,
        "near_ceiling": ratio >= 0.95 * ceiling,
    }


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
            if "DW_AT_stmt_list" not in info or re.search(
                r"DW_AT_(type|decl_file)\b", info
            ):
                raise ValueError(
                    "compiled Rust requires effective line-tables-only debug"
                )


def record_build(build, source, role, env):
    """Publish only successful builds, bound to the executable being measured."""
    import migration

    settings = (build / "CMakeCache.txt").read_text()
    if role == "candidate":
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
    # Longest first: the clone's shared .local stays absolute even under the main
    # checkout's source path, so every worktree records one identity.
    shared = str(migration.COMMON_LOCAL)
    labels = {str(source): "<source>", str(build): "<build>", shared: "\0"}
    for path in sorted(labels, key=len, reverse=True):
        settings = settings.replace(path, labels[path])
    settings = settings.replace("\0", shared)
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
            # CMake bookkeeping embeds each worktree's build path; games never read it.
            if p.is_file() and (
                base == folder
                or p.parts[len(build.parts)] in RUNTIME_DIRECTORIES
                and p.suffix != ".cmake"
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
    anchor, integrated = record["anchor"], record["integrated"]
    if measurement["identity"]["compatible"] != anchor["identity"]["compatible"]:
        raise ValueError(
            "instruction anchor identity mismatch; quiet speed pass required"
        )
    ratio = projection(anchor, measurement["Ir"])
    rolling = projection(anchor, integrated["Ir"])
    return dict(projected_ratio=ratio, **speed_budget(name, ratio, rolling))
