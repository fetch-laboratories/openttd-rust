"""Recapture #736's frozen oracles from the pinned original and check them byte for byte.

Each recipe names the retired fixture at its scaffold commit, the patch in
tools/oracle-recipes/ that turns it into an original-only capture program, the
pinned bodies it includes, and the conversion of its printed stream into the
committed oracle. The program is built at -O0 and at -O2 -DNDEBUG against the
pinned sources only; both streams must match before conversion. No candidate
header, archive or library is compiled or linked.

    python3 tools/oracle-capture.py [name ...]

A recipe records how an oracle was made; it is provenance, not a CI check. A PR
that changes an oracle, a recipe or the pin reruns it for the oracles it touches.
"""

import argparse
import itertools
import subprocess
import sys
from collections.abc import Callable, Iterable, Iterator, Sequence
from dataclasses import dataclass, field
from pathlib import Path

import probe

import migration

ROOT = migration.ROOT
RECIPES = ROOT / "tools/oracle-recipes"
KERNELS = ROOT / "rust/openttd-kernels/src"
WIDTH = 100


def runs(refs: Sequence[int]) -> list[str]:
    """Case references with runs written `i*n`, wrapped at WIDTH characters."""
    tokens = []
    for value, group in itertools.groupby(refs):
        count = len(list(group))
        tokens.append(str(value) if count == 1 else f"{value}*{count}")
    rows: list[str] = []
    for token in tokens:
        if rows and len(rows[-1]) + 1 + len(token) <= WIDTH:
            rows[-1] += " " + token
        else:
            rows.append(token)
    return rows


def section(name: str, outputs: Iterable[str]) -> list[str]:
    """One oracle.rs section: each distinct output once, then the case references."""
    ids: dict[str, int] = {}
    refs = [ids.setdefault(output, len(ids)) for output in outputs]
    return ["@@ " + name, *ids, "%%", *runs(refs)]


def nested(lengths: Sequence[int], order: Sequence[int]) -> Iterator[int]:
    """Original-order indices of a nest of `lengths`, enumerated outermost-first in `order`."""
    strides = [1] * len(lengths)
    for d in reversed(range(len(lengths) - 1)):
        strides[d] = strides[d + 1] * lengths[d + 1]
    for digits in itertools.product(*(range(lengths[d]) for d in order)):
        yield sum(v * strides[d] for d, v in zip(order, digits, strict=True))


def streams(lines: Sequence[str]) -> dict[str, list[str]]:
    """Printed `<stream> <fields>` lines grouped by stream, in print order."""
    out: dict[str, list[str]] = {}
    for line in lines:
        name, _, rest = line.partition(" ")
        out.setdefault(name, []).append(rest)
    return out


def income(lines: Sequence[str]) -> list[str]:
    s = streams(lines)
    lengths = (7, 3, 5, 7, 6, 7, 6)
    cases = s["income"]
    return [
        *section(
            "income",
            (
                " ".join(cases[i].split()[:2])
                for i in nested(lengths, (3, 6, 0, 1, 2, 4, 5))
            ),
        ),
        *section(
            "var18",
            (cases[i].split()[2] for i in nested(lengths, (5, 4, 3, 6, 1, 2, 0))),
        ),
        *section("transfer", (" ".join(v.split()[:2]) for v in s["transfer"])),
        *section("transfer-callback", (" ".join(v.split()[2:]) for v in s["transfer"])),
        *section("invalid", s["invalid"]),
    ]


def pool(lines: Sequence[str]) -> list[str]:
    s = streams(lines)
    return [*section("lifetime", s["lifetime"]), *section("cursor", s["cursor"])]


def tile_loop(lines: Sequence[str]) -> list[str]:
    s = streams(lines)
    return [*section("loop", s["loop"]), *section("mutation", s["mutation"])]


def tunnel(lines: Sequence[str]) -> list[str]:
    return section("tunnel", streams(lines)["tunnel"])


def pair_runs(pairs: Sequence[tuple[int, int]]) -> list[str]:
    """Ordered (random, parameter) pairs as greedy `count:first1:first2:delta1:delta2`
    runs, each delta modulo 2^32 and taken from the run's first two pairs."""
    runs = []
    i = 0
    while i < len(pairs):
        j = i
        delta = (0, 0)
        if i + 1 < len(pairs):
            delta = tuple(
                (b - a) % 2**32 for a, b in zip(pairs[i], pairs[i + 1], strict=True)
            )
            j = i + 1
            while (
                j + 1 < len(pairs)
                and tuple(
                    (b - a) % 2**32 for a, b in zip(pairs[j], pairs[j + 1], strict=True)
                )
                == delta
            ):
                j += 1
        first = pairs[i]
        runs.append(f"{j - i + 1}:{first[0]}:{first[1]}:{delta[0]}:{delta[1]}")
        i = j + 1
    return runs


def production(lines: Sequence[str]) -> list[str]:
    outputs = []
    for case in streams(lines)["I"]:
        fields = [int(v) for v in case.split()]
        pairs = [(fields[i], fields[i + 1]) for i in range(10, len(fields), 2)]
        outputs.append(" ".join([*map(str, fields[:10]), *pair_runs(pairs)]))
    return section("I", outputs)


# The builtin animation nest: seed, tick, gfx, frame, loop, ambient, custom.
BUILTIN = (4, 22, 15, 26, 3, 2, 2)


def builtin_skipped(index: Sequence[int]) -> bool:
    """The capture skips the toffee quarry (gfx 165, index 12) past its 70-entry table
    (frame 70, index 15, onwards)."""
    return index[2] == 12 and index[3] >= 15


def industry_lifecycle(lines: Sequence[str]) -> list[str]:
    s = streams(lines)
    present = [
        i
        for i in itertools.product(*(range(n) for n in BUILTIN))
        if not builtin_skipped(i)
    ]
    cases = dict(zip(present, s["B"], strict=True))
    # Stored custom, gfx, tick, frame, loop, ambient, seed outermost-first, for long runs.
    order = (6, 2, 1, 3, 4, 5, 0)
    stored = []
    for digits in itertools.product(*(range(BUILTIN[d]) for d in order)):
        index = [0] * len(BUILTIN)
        for d, v in zip(order, digits, strict=True):
            index[d] = v
        if not builtin_skipped(index):
            stored.append(cases[tuple(index)])
    return [
        *section("toffee", s["T"]),
        *section("lifecycle", s["L"]),
        *section("builtin", stored),
        *section("acceptance", s["A"]),
    ]


def linkgraph_schedule(lines: Sequence[str]) -> list[str]:
    return section("trace", streams(lines)["trace"])


def train_edge(lines: Sequence[str]) -> list[str]:
    """Curve limits with even (original-model) trials first, then the swaps in trial order."""
    s = streams(lines)
    curve = s["curve"][0::2] + s["curve"][1::2]
    return section("curve", curve) + section("swap", s["swap"])


def policy(lines: Sequence[str]) -> list[str]:
    s = streams(lines)
    return [line for kind in "HRZAP" for line in section(kind, s[kind])]


@dataclass(frozen=True)
class Recipe:
    oracle: str
    # The retired fixture as `commit:path`, or None when the patch creates the program.
    fixture: str | None
    convert: Callable[[Sequence[str]], list[str]]
    # (include name, pinned source, start marker, end marker): bodies in source order.
    # An empty end marker takes the body through its matching brace (probe.body).
    extract: tuple[tuple[str, str, str, str], ...] = ()
    sources: tuple[str, ...] = ()
    flags: tuple[str, ...] = field(default=("-DUNIX",))


RECIPE: dict[str, Recipe] = {
    "session_pause": Recipe(
        oracle="session/pause-oracle.txt",
        fixture=None,
        convert=lambda lines: [
            *section("pause", streams(lines)["pause"]),
            *section("query", streams(lines)["query"]),
        ],
        extract=(
            (
                "session-pause-original.inc",
                "src/misc_cmd.cpp",
                "CommandCost CmdPause(",
                "",
            ),
            (
                "session-pause-flags.inc",
                "src/command_type.h",
                "enum class DoCommandFlag : uint8_t {",
                "/**\n * Command flags for the command table",
            ),
        ),
    ),
    "policy": Recipe(
        oracle="town/policy-oracle.txt",
        fixture="45843e22f6:tools/simulation/town-reference-gaps.cpp",
        convert=policy,
        extract=tuple(
            ("town-tunnel-original.hpp", "src/town_cmd.cpp", signature, "")
            for signature in (
                "static bool CanRoadContinueIntoNextTile(",
                "static bool GrowTownWithTunnel(",
                "template <>\nTown::SuppliedHistory SumHistory(",
                "void SetTownRatingTestMode(",
                "static int GetRating(",
                "void ChangeTownRating(",
                "void UpdateTownRadius(",
                "CommandCost CheckIfAuthorityAllowsNewStation(",
                "CommandCost CheckforTownRating(",
            )
        )
        + (
            (
                "town-percent-original.hpp",
                "src/town.h",
                "uint8_t PctTransported() const",
                "",
            ),
        ),
        flags=("-DUNIX", "-fno-strict-overflow"),
    ),
    "production": Recipe(
        oracle="industry/production-oracle.txt",
        fixture="45843e22f6:tools/migration/industry-production-gap.cpp",
        convert=production,
        extract=(
            (
                "industry-production-reference.inc",
                "src/newgrf_industries.cpp",
                "void IndustryProductionCallback(Industry *ind, int reason)",
                "/**\n * Check whether an industry temporarily",
            ),
        ),
        flags=("-DUNIX", "-fno-strict-overflow"),
    ),
    "industry_lifecycle": Recipe(
        oracle="industry/lifecycle-oracle.txt",
        fixture="a30091d652:tools/migration/industry-lifecycle-gap.cpp",
        convert=industry_lifecycle,
        extract=tuple(
            (
                "industry-lifecycle-reference.inc",
                "src/newgrf_industrytiles.cpp",
                signature,
                "",
            )
            for signature in (
                "static bool DoTriggerIndustryTileAnimation(",
                "bool TriggerIndustryTileAnimation_ConstructionStageChanged(",
                "bool TriggerIndustryTileAnimation(",
                "bool TriggerIndustryAnimation(",
                "static void DoTriggerIndustryTileRandomisation(",
                "static void DoReseedIndustry(",
                "void TriggerIndustryTileRandomisation(",
                "void TriggerIndustryRandomisation(",
            )
        )
        + (
            (
                "industry-acceptance-reference.inc",
                "src/industry_cmd.cpp",
                "static void AddAcceptedCargo_Industry(",
                "",
            ),
            (
                "industry-animation-reference.inc",
                "src/table/industry_land.h",
                "static const uint8_t _industry_anim_offs_toffee[]",
                "\n\n/* this is ONLY used for the Bubble Generator*/",
            ),
        )
        + tuple(
            (
                "industry-animation-reference.inc",
                "src/industry_cmd.cpp",
                f"static void {name}(",
                "",
            )
            for name in (
                "AnimateSugarSieve",
                "AnimateToffeeQuarry",
                "AnimateBubbleCatcher",
                "AnimatePowerPlantSparks",
                "AnimateToyFactory",
                "AnimatePlasticFountain",
                "AnimateOilWell",
                "AnimateMineTower",
                "AnimateTile_Industry",
            )
        )
        + (
            (
                "industry-graphics-reference.inc",
                "src/industry_map.h",
                "enum IndustryGraphics :",
                "\n\n/**\n * Get the industry ID of the given tile",
            ),
        ),
        flags=("-DUNIX", "-fno-strict-overflow"),
    ),
    "linkgraph_schedule": Recipe(
        oracle="linkgraph_schedule/schedule-oracle.txt",
        fixture="f0f67cf819:tools/simulation/linkgraph-schedule-reference.cpp",
        convert=linkgraph_schedule,
        extract=tuple(
            (
                "scheduler-original.inc",
                "src/linkgraph/linkgraphschedule.cpp",
                signature,
                "",
            )
            for signature in (
                "void LinkGraphSchedule::SpawnNext()",
                "bool LinkGraphSchedule::IsJoinWithUnfinishedJobDue() const",
                "void LinkGraphSchedule::JoinNext()",
                "void LinkGraphSchedule::SpawnAll()",
                "void LinkGraphSchedule::Clear()",
                "void LinkGraphSchedule::ShiftDates(",
                "void StateGameLoop_LinkGraphPauseControl()",
                "void AfterLoad_LinkGraphPauseControl()",
                "void OnTick_LinkGraph()",
            )
        )
        + tuple(
            (
                "scheduler-queue-original.inc",
                "src/linkgraph/linkgraphschedule.h",
                signature,
                "",
            )
            for signature in ("void Queue(", "void Dequeue(")
        ),
    ),
    "train_edge": Recipe(
        oracle="train/edge-oracle.txt",
        fixture="a00b377774:tools/migration/train-edge-comparison.cpp",
        convert=train_edge,
        extract=tuple(
            ("train-reference.inc", "src/train_cmd.cpp", signature, "")
            for signature in (
                "uint16_t Train::GetCurveSpeedLimit() const",
                "static void SwapTrainFlags(uint16_t *swap_flag1, uint16_t *swap_flag2)",
                "static void UpdateStatusAfterSwap(Train *v)",
                "void ReverseTrainSwapVeh(Train *v, int l, int r)",
            )
        ),
    ),
    "tunnel": Recipe(
        oracle="town/tunnel-oracle.txt",
        fixture="d9cae2c923:tools/simulation/town-reference-gaps.cpp",
        convert=tunnel,
        extract=(
            (
                "town-tunnel-original.hpp",
                "src/town_cmd.cpp",
                "static bool CanRoadContinueIntoNextTile(",
                "",
            ),
            (
                "town-tunnel-original.hpp",
                "src/town_cmd.cpp",
                "static bool GrowTownWithTunnel(",
                "",
            ),
        ),
    ),
    "tile_loop": Recipe(
        oracle="tile_loop/comparison-oracle.txt",
        fixture="7f8d7a2652^:tools/simulation/landscape-loop-reference.cpp",
        convert=tile_loop,
        extract=(
            (
                "landscape-loop-reference.inc",
                "src/landscape.cpp",
                "void RunTileLoop()",
                "\nvoid InitializeLandscape()",
            ),
        ),
    ),
    "pool": Recipe(
        oracle="vehicle/scheduler/pool-oracle.txt",
        fixture=None,
        convert=pool,
        sources=("src/core/pool_func.cpp",),
        flags=("-DUNIX", "-DFMT_HEADER_ONLY"),
    ),
    "income": Recipe(
        oracle="cargo_payment/income-oracle.txt",
        fixture="1b8b85c524:tools/migration/cargo-income-gap.cpp",
        convert=income,
        extract=(
            (
                "cargo-reference.inc",
                "src/economy.cpp",
                "static inline int32_t BigMulS(",
                "typedef std::vector<Industry *>",
            ),
            (
                "cargo-reference.inc",
                "src/economy.cpp",
                "Money GetTransportedGoodsIncome(",
                "/** The industries",
            ),
            (
                "cargo-reference.inc",
                "src/economy.cpp",
                "Money CargoPayment::PayTransfer(",
                "/**\n * Prepare the vehicle",
            ),
        ),
    ),
}


def tabbed(text: str) -> str:
    """Text without trailing whitespace, each line's leading spaces as tabs (#920).

    The commit checker rejects both in C++ diffs, so a program copied back into a
    recipe passes it.
    """
    lines = []
    for raw in text.splitlines():
        line = raw.rstrip()
        body = line.lstrip(" \t")
        tabs, spaces = divmod(len(line[: len(line) - len(body)].expandtabs(4)), 4)
        if not tabs and spaces > 1:
            # Alignment spaces may follow a tab; two leading ones fail the checker.
            tabs, spaces = 1, 0
        lines.append("\t" * tabs + " " * spaces + body)
    return "\n".join(lines) + "\n"


def program(name: str, recipe: Recipe, out: Path) -> Path:
    """Write the patched capture program and its extracted bodies into `out`."""
    # The patch names a non-source file, so the commit checker does not read the
    # patch's own diff headers as C++ source.
    (out / "capture.in").unlink(missing_ok=True)
    if recipe.fixture is not None:
        fixture = subprocess.check_output(
            ["git", "show", recipe.fixture], cwd=ROOT, text=True
        )
        (out / "capture.in").write_text(fixture)
    with (RECIPES / f"{name}.patch").open() as patch:
        subprocess.run(["patch", "-s", "-p1", "-d", str(out)], stdin=patch, check=True)
    (out / "capture.cpp").write_text(tabbed((out / "capture.in").read_text()))
    (out / "capture.in").unlink()
    includes: dict[str, list[str]] = {}
    for include, path, start, end in recipe.extract:
        text = (probe.REFERENCE / path).read_text()
        if end:
            offset = text.index(start)
            body = text[offset : text.index(end, offset)]
        else:
            body = probe.body(text, start)
        includes.setdefault(include, []).append(body)
    for include, bodies in includes.items():
        (out / include).write_text("\n\n".join(bodies) + "\n")
    return out / "capture.cpp"


def capture(name: str) -> bool:
    recipe = RECIPE[name]
    out = probe.output(f"oracle-capture/{name}")
    source = program(name, recipe, out)
    printed = []
    for level in (["-O0"], ["-O2", "-DNDEBUG"]):
        binary = out / ("capture" + "".join(level).replace("-", "_"))
        command = [
            probe.compiler(),
            "-std=c++20",
            *level,
            *recipe.flags,
            "-I",
            str(probe.REFERENCE / "src"),
            "-I",
            str(out),
            str(source),
        ]
        command += [str(probe.REFERENCE / s) for s in recipe.sources]
        subprocess.run([*command, "-o", str(binary)], check=True)
        printed.append(subprocess.check_output([str(binary)], text=True))
    if printed[0] != printed[1]:
        print(f"{name}: -O0 and -O2 streams differ", file=sys.stderr)
        return False
    oracle = KERNELS / recipe.oracle
    committed = [
        line for line in oracle.read_text().splitlines() if not line.startswith("#")
    ]
    # A committed oracle carries no trailing whitespace (the commit checker's rule).
    captured = [line.rstrip() for line in recipe.convert(printed[0].splitlines())]
    if captured != committed:
        (out / "captured.txt").write_text("\n".join(captured) + "\n")
        print(
            f"{name}: recapture differs from {oracle.relative_to(ROOT)}; see {out}/captured.txt",
            file=sys.stderr,
        )
        return False
    rows = len(printed[0].splitlines())
    print(f"{name}: recapture matches {oracle.relative_to(ROOT)} ({rows} printed rows)")
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("names", nargs="*", metavar="name", help=", ".join(RECIPE))
    names = parser.parse_args().names or list(RECIPE)
    unknown = sorted(set(names) - set(RECIPE))
    if unknown:
        parser.error("unknown recipe: " + ", ".join(unknown))
    return 0 if all([capture(name) for name in names]) else 1


if __name__ == "__main__":
    sys.exit(main())
