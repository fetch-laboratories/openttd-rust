"""Recapture #736's frozen oracles from the pinned original and check them byte for byte.

Each recipe names the retired fixture at its scaffold commit, the patch in
tools/oracle-recipes/ that turns it into an original-only capture program, the
pinned bodies it includes, and the conversion of its printed stream into the
committed oracle. The program is built at -O0 and at -O2 -DNDEBUG against the
pinned sources only; both streams must match before conversion. No candidate
header, archive or library is compiled or linked.

    python3 tools/oracle-capture.py [name ...]
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
    (out / "capture.in").replace(out / "capture.cpp")
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
            "c++",
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
    captured = recipe.convert(printed[0].splitlines())
    if captured != committed:
        (out / "captured.txt").write_text("\n".join(captured) + "\n")
        print(
            f"{name}: recapture differs from {oracle.relative_to(ROOT)}; see {out}/captured.txt",
            file=sys.stderr,
        )
        return False
    print(
        f"{name}: recapture matches {oracle.relative_to(ROOT)} ({len(printed[0].splitlines())} printed rows)"
    )
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
