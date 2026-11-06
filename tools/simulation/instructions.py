"""Load-independent loop work; reuse canonical runs and semantic comparisons."""

import hashlib
import json
import platform
import re
import shutil
import subprocess
import tempfile
from collections.abc import Mapping
from pathlib import Path
from typing import TYPE_CHECKING, Any, TypedDict, cast

if TYPE_CHECKING:
    from .core import Result, Role, Run, Scenario

ENTRIES = ("StateGameLoop()", "_GenerateWorld()")
CANDIDATE_ENTRIES = (
    "openttd_rust_session_state_loop_abi",
    "openttd_kernels::genworld::generate",
)
# Drawing workloads count C++ drawing, which both builds still share (#566).
DRAW_ENTRY = "ViewportDoDraw(Viewport const&, int, int, int, int)"

# Quiet speed pass at 27de6f8c18, the first on the Ubuntu 24.04 host (2026-11-05):
# ratio is the median of four three-pair medians; Ir and compatible identity come
# from profiles of the same build. The three-pass record restarted on this host,
# so each scenario has one pass so far and decisions use the mean of the passes
# it has (docs/speed.md). The drawing workload is anchored for projection only;
# speed_budget applies no ceiling to it.
SPEED_HEAD = "27de6f8c18"
# fmt: off
SPEED_PASS = (
    ("generate-tgp-256-1", (1.1601069121937235,), 2773615856, "4cbae59849fc44afcb65b35e6352c72b40b7caee046e7eb98dbba84ca7941a98"),
    ("play-opus-55-167-002-manual", (1.3451151318876846,), 28611669801, "fd6c4136ee3265c95d39e5492b0bc2bf2e003e5176eccc54196e33bb23d5df1f"),
    ("play-grok-159-001-manual", (1.346652325266398,), 27178405243, "c21abdd5925c474ceb5281286464f9058ac738b858533d9ba07b89cfaefe5111"),
    ("play-padhattan-ridge-1996-manual", (1.3521934654919234,), 4744876185, "7de5a803c02832c70c7b22c739820e2d61bd8eabdae6f968a24947f893e33820"),
    ("play-padhattan-ridge-2000-manual", (1.444530637399886,), 7834343043, "b66513819d453d3b54e0a5945f843a1c86d00ca09fc7856989d700f89e0a2865"),
    ("play-padhattan-ridge-2016-manual", (1.3825932133062313,), 14200776421, "ed5da7ab10aedea72c0c9724dc6046c472c16ee66f9509b4e1888df22bf54ef7"),
    ("draw-padhattan-ridge-1996-giant", (1.0046350128333592,), 2154533064, "ba0981bf678450f011e33e2a90d881e5562ba8b6274aec9992837f69ed0698ff"),
)
# Loop Ir at the anchor, in SPEED_PASS order, for projecting instruction-count growth.
SPEED_INTEGRATED = SPEED_HEAD, tuple(ir for _, _, ir, _ in SPEED_PASS)
# fmt: on


class Anchor(TypedDict):
    head: str
    ratio: float
    Ir: int
    identity: dict[str, str]


class Integrated(TypedDict):
    head: str
    Ir: int


class SpeedAnchor(TypedDict):
    anchor: Anchor
    integrated: Integrated


SPEED_ANCHORS: dict[str, SpeedAnchor] = {
    name: {
        "anchor": {
            "head": SPEED_HEAD,
            "ratio": ratios[-1],
            "Ir": ir,
            "identity": {"compatible": c},
        },
        "integrated": {"head": SPEED_INTEGRATED[0], "Ir": integrated},
    }
    for (name, ratios, ir, c), integrated in zip(
        SPEED_PASS, SPEED_INTEGRATED[1], strict=True
    )
}
SPEED_BUDGETS = {name: sum(ratios) / len(ratios) for name, ratios, _, _ in SPEED_PASS}
# Profiled and speed-pass scenarios: the anchored ones, including the drawing workload.
PROFILED = tuple(dict.fromkeys((*SPEED_BUDGETS, "draw-padhattan-ridge-1996-giant")))


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
    # docs/speed.md defines ceilings only for play and generation.
    if name.startswith("draw-"):
        return None
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


def verify_debug(archive: Path) -> None:
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


# #607 removed the always-on OPTION_RUST toggle and added Git discovery for
# native-test oracles. Neither changes the measured game, so restore the
# anchored cache spelling; a quiet pass would otherwise be needed to re-anchor.
RUST_OPTION = (
    "//Use migrated Rust game/tool algorithms (validated native platforms)\n"
    "OPTION_RUST:BOOL=ON\n\n"
)
TEST_GIT_ENTRIES = (
    r"//Git command line client\nGIT_EXECUTABLE:FILEPATH=[^\n]*\n\n",
    r"//Details about finding Git\nFIND_PACKAGE_MESSAGE_DETAILS_Git:INTERNAL=[^\n]*\n",
    r"//ADVANCED property for variable: GIT_EXECUTABLE\nGIT_EXECUTABLE-ADVANCED:INTERNAL=1\n",
)
# FindPkgConfig's pkg_get_variable overwrites this scratch cache entry on every
# query (features, prefix, etc.). Actual package paths/flags stay in their own
# entries and linked library bytes are hashed separately. Retain the quiet
# anchor's spelling rather than changing existing anchor or reference hashes.
ANCHORED_PREFIX_RESULT = (
    "AsynchDNS;GSS-API;HSTS;HTTP2;HTTPS-proxy;IDN;IPv6;Kerberos;Largefile;"
    "NTLM;PSL;SPNEGO;SSL;TLS-SRP;UnixSockets;alt-svc;brotli;libz;threadsafe;zstd"
)


def anchored_cache(text: str) -> str:
    """Restore only proven non-game discovery bookkeeping to anchor spelling."""
    text = re.sub(
        r"^prefix_result:INTERNAL=[^\n]*$",
        "prefix_result:INTERNAL=" + ANCHORED_PREFIX_RESULT,
        text,
        flags=re.M,
    )
    for entry in TEST_GIT_ENTRIES:
        text = re.sub(entry, "", text)
    if "\nOPTION_RUST:" not in text:
        text = re.sub(
            r"(\nOPTION_PACKAGE_DEPENDENCIES:BOOL=[^\n]*\n\n)",
            rf"\1{RUST_OPTION}",
            text,
        )
    return text


def record_build(build: Path, source: Path, role: str, env: Mapping[str, str]) -> None:
    """Publish only successful builds, bound to the executable being measured."""
    import migration

    settings = (build / "CMakeCache.txt").read_text()
    if role == "candidate":
        settings = anchored_cache(settings)
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


def flags(scenario: "Scenario", reference: bool = True) -> list[str]:
    entries = ENTRIES if reference else CANDIDATE_ENTRIES
    entries = entries if scenario["kind"] == "generate" else entries[:1]
    entries = (DRAW_ENTRY,) if scenario.get("draw") else entries
    return [
        "--command-line-only=yes",
        "--tool=callgrind",
        "--collect-atstart=no",
        *[f"--toggle-collect={e}" for e in entries],
    ]


def count(path: Path) -> int:
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


def digest(value: object) -> str:
    if not isinstance(value, bytes):
        value = json.dumps(value, sort_keys=True).encode()
    return hashlib.sha256(value).hexdigest()


def identity(
    scenario: "Scenario",
    binary: Path,
    build: Path,
    folder: Path,
    env: Mapping[str, str],
    valgrind: Path,
    reference: bool = True,
) -> dict[str, Any]:
    """Compatibility excludes candidate code; full measurement identity retains it."""
    from .core import RUNTIME_DIRECTORIES

    inputs: dict[str, Any] = {}
    for base in (build, folder):
        for p in sorted(base.rglob("*")):
            # CMake bookkeeping embeds each worktree's build path; games never read it.
            if p.is_file() and (
                base == folder
                or p.parts[len(build.parts)] in RUNTIME_DIRECTORIES
                and p.suffix != ".cmake"
            ):
                inputs[str(p.relative_to(base))] = digest(p.read_bytes())
    resolved: dict[str, Any] = dict(scenario)
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
    if not reference:
        # Retained, executable-bound captures predate this normalization too.
        settings = anchored_cache(settings)
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
    result: dict[str, Any] = {"compatible": compatible, "executable": executable}
    if not reference:
        result["entries"] = flags(scenario, reference=False)
    return result


def prepare(
    scenario: "Scenario",
    command: list[str],
    binary: Path,
    build: Path,
    folder: Path,
    env: dict[str, str],
    reference: bool,
) -> tuple[list[str], dict[str, Any]]:
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


def finish(runs: Mapping["Role", "Run"], result: "Result") -> dict[str, Any]:
    if result["differences"] or result["known_failures"] or result["problems"]:
        raise ValueError(
            "semantically unequal/unsuccessful profiles are not speed evidence"
        )
    measurements: dict[str, dict[str, Any]] = {}
    for role, run in runs.items():
        info = cast(dict[str, Any], run["instructions"])
        value = info["cached"]
        if value is None:
            value = count(run["dir"] / "callgrind.out")
        measurements[role] = dict(identity=dict(info["identity"]), Ir=value)
    candidate = measurements["candidate"]["identity"]
    candidate["compatible"] = digest(
        [candidate["compatible"], measurements["reference"]["identity"]]
    )
    info = cast(dict[str, Any], runs["reference"]["instructions"])
    info["cache"].parent.mkdir(parents=True, exist_ok=True)
    # Run folders and the cache may sit on different disks; rename within one.
    with tempfile.NamedTemporaryFile(
        "w", dir=info["cache"].parent, suffix=".tmp", delete=False
    ) as temporary:
        temporary.write(json.dumps(measurements["reference"]["Ir"]))
    Path(temporary.name).replace(info["cache"])
    ratio = measurements["candidate"]["Ir"] / measurements["reference"]["Ir"]
    return dict(measurements, Ir_ratio=ratio)


def projection(anchor: Anchor, value: int) -> float:
    return anchor["ratio"] * (1 + 2 * max(0, value / anchor["Ir"] - 1))


def budget(
    name: str, measurement: Mapping[str, Any], record: SpeedAnchor
) -> dict[str, Any]:
    anchor, integrated = record["anchor"], record["integrated"]
    if measurement["identity"]["compatible"] != anchor["identity"]["compatible"]:
        raise ValueError(
            "instruction anchor identity mismatch; quiet speed pass required"
        )
    ratio = projection(anchor, measurement["Ir"])
    rolling = projection(anchor, integrated["Ir"])
    return dict(
        projected_ratio=ratio,
        **(speed_budget(name, ratio, rolling, anchor["ratio"]) or {}),
    )
