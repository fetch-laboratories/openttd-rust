"""Load-independent loop work; reuse canonical runs and semantic comparisons."""

import hashlib
import json
import platform
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import TypedDict

ENTRIES = ("StateGameLoop()", "_GenerateWorld()")
CANDIDATE_ENTRIES = (
    "openttd_rust_session_state_loop",
    "openttd_kernels::genworld::generate",
)

# Quiet speed pass at 755609b8ea (#275): ratio is the median of
# four three-pair medians; Ir and compatible identity come from profiles of the
# same build. Ratios are the last three quiet passes, oldest first,
# retained from b959f89059, f4923c79eb and 755609b8ea respectively. The latest
# pass alone supplies the compatible instruction anchor; decisions use the mean.
SPEED_HEAD = "755609b8ea"
# fmt: off
SPEED_PASS = (
    ("generate-tgp-256-1", (1.172280764574488, 1.1783150183150184, 1.1117224479856325), 2734468220, "b5985e2afec2a7f3e2479df4e89f1bcf8560e9204398040de25f425cfb3b24a8"),
    ("play-opus-55-167-002-manual", (1.3879109594186272, 1.4018075231030847, 1.3939491755220992), 28553652121, "7822ce2275a9fb521a67d47cdcdac6b1e9c433322de683cdfb6df5a3f2507ca3"),
    ("play-grok-159-001-manual", (1.374991116425806, 1.394822381542774, 1.3871469688959217), 27140212414, "dc7e6fee2c033dc0c1506d3f18c9b8edfc4ffaf2d8962570063d1526dc4d0204"),
    ("play-padhattan-ridge-1996-manual", (1.3413361169102298, 1.3354044451788813, 1.3365508365508365), 4559471842, "f95e791a1617f2e457fdd4f9b92b51bd9a663632f5390f3f66f0e2d92db2be24"),
    ("play-padhattan-ridge-2000-manual", (1.410312146769846, 1.3934479388770433, 1.4005769924269744), 7441795115, "62575c1e01fe179a4b0c2f7f75bba73b2256c23a7b3ba08c2b22e59efe6e1d47"),
    ("play-padhattan-ridge-2016-manual", (1.453359475929639, 1.4694601348806955, 1.522910560065014), 13501045042, "206fefda1a3c05ae702d001fb1b6aff2a36e59ee09e1213a6deba000d8288430"),
)
# Loop Ir at the anchor, in SPEED_PASS order, for projecting instruction-count growth.
SPEED_INTEGRATED = SPEED_HEAD, tuple(ir for _, _, ir, _ in SPEED_PASS)
# fmt: on
SPEED_ANCHORS = {
    name: dict(
        anchor=dict(
            head=SPEED_HEAD, ratio=ratios[-1], Ir=ir, identity={"compatible": c}
        ),
        integrated=dict(head=SPEED_INTEGRATED[0], Ir=integrated),
    )
    for (name, ratios, ir, c), integrated in zip(
        SPEED_PASS, SPEED_INTEGRATED[1], strict=True
    )
}
SPEED_BUDGETS = {name: sum(ratios) / len(ratios) for name, ratios, _, _ in SPEED_PASS}


class SpeedBudget(TypedDict):
    record: float
    quiet_pass_mean: float
    previous_record: float
    change_percent: float
    ceiling: float
    band: float
    within_ceiling: bool
    near_ceiling: bool


def speed_budget(
    name: str,
    ratio: float | None,
    integrated: float | None = None,
    anchor_ratio: float | None = None,
) -> SpeedBudget | None:
    """Report the speed record and ceiling without making timings a CI failure."""
    mean = SPEED_BUDGETS.get(name, anchor_ratio)
    if mean is None or ratio is None:
        return None
    record = mean if anchor_ratio is None else mean * (ratio / anchor_ratio)
    previous = (
        mean
        if integrated is None or anchor_ratio is None
        else mean * (integrated / anchor_ratio)
    )
    ceiling = 1.6 if name.startswith("play-") else 1.3
    return {
        "record": record,
        "quiet_pass_mean": mean,
        "previous_record": previous,
        "change_percent": (ratio / (mean if integrated is None else integrated) - 1)
        * 100,
        "ceiling": ceiling,
        "band": 0.95 * ceiling,
        "within_ceiling": record <= ceiling,
        "near_ceiling": record >= 0.95 * ceiling,
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


def flags(scenario, reference=True):
    entries = ENTRIES if reference else CANDIDATE_ENTRIES
    entries = entries if scenario["kind"] == "generate" else entries[:1]
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


def identity(scenario, binary, build, folder, env, valgrind, reference=True):
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
    # Entry names differ because Rust owns the candidate bodies. Normalize that
    # spelling in compatibility identity so existing quiet-pass anchors and the
    # unchanged reference cache remain valid; full candidate identity records it.
    settings = settings, platform.machine(), tool, flags(scenario), policy
    compatible = digest([resolved, inputs, settings])
    result = {"compatible": compatible, "executable": executable}
    if not reference:
        result["entries"] = flags(scenario, reference=False)
    return result


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
    measurement = identity(scenario, binary, build, folder, env, valgrind, reference)
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
        command = [str(valgrind), *flags(scenario, reference), *outputs, *command]
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
    return dict(
        projected_ratio=ratio, **speed_budget(name, ratio, rolling, anchor["ratio"])
    )
