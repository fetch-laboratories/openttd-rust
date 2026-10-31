"""Giant-screenshot drawing workload: rendered image and Ir under ViewportDoDraw (#566).

The null video driver forces the null blitter and never drains the main-thread
queue `screenshot` posts to, so the paused save is drawn through SDL's headless
dummy video driver instead, without game threads and with an explicit blitter.
"""

import hashlib
from pathlib import Path
from typing import Any

from .core import ROOT

Scenario = dict[str, Any]

NAME = "draw-padhattan-ridge-1996-giant"
BLITTER = "32bpp-optimized"


def scenarios(soak: bool) -> list[Scenario]:
    # The save stays paused: `exit` writes the exit save, then the same draw
    # tick drains the queued screenshot before the main loop ends.
    return [
        {
            "name": NAME,
            "kind": "save",
            "draw": True,
            "save": str(ROOT / "migration/saves/padhattan-ridge-1996.sav"),
            "settings": {"misc": {"screenshot_format": "bmp"}},
            "console": ["screenshot giant drawing", "exit"],
            "video": "-vsdl:no_threads",
            "ticks": 1,
            "short_checkpoint": True,
        }
    ]


def game_args(scenario: Scenario) -> list[str]:
    return ["-b", BLITTER] if scenario.get("draw") else []


def game_environment(scenario: Scenario) -> dict[str, str]:
    return {"SDL_VIDEODRIVER": "dummy"} if scenario.get("draw") else {}


def tree_digest(folder: Path) -> list[tuple[str, str]]:
    files = sorted(p for p in folder.rglob("*") if p.is_file())
    return [(str(p.relative_to(folder)), file_digest(p)) for p in files]


def file_digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: dict[str, Any],
) -> Scenario:
    """Both roles must draw with identical base graphics."""
    if scenario.get("draw"):
        sets = [tree_digest(builds[role] / "baseset") for role in builds]
        if not sets[0] or sets[0] != sets[-1]:
            raise RuntimeError("reference and candidate base graphics differ")
    return scenario


def image(run: dict[str, Any], mode: str, role: str) -> Path:
    images = sorted((run["dir"] / "screenshot").glob("*.bmp"))
    if [p.name for p in images] != ["drawing.bmp"]:
        raise RuntimeError(f"{mode}: {role} wrote screenshots {images}")
    return images[0]


def check(
    scenario: Scenario,
    run: dict[str, Any],
    mode: str,
    role: str,
    result: dict[str, Any],
) -> None:
    """Compare the candidate's image with the sibling reference run's, byte for byte."""
    if not scenario.get("draw"):
        return
    path = image(run, mode, role)
    if role == "reference":
        return
    reference = image(dict(dir=run["dir"].parent / "reference"), mode, "reference")
    a, b = reference.read_bytes(), path.read_bytes()
    width, height = (int.from_bytes(a[i : i + 4], "little") for i in (18, 22))
    result[f"{mode}_image"] = dict(
        width=width, height=height, bytes=len(a), sha256=hashlib.sha256(a).hexdigest()
    )
    if a != b:
        index = next(
            (i for i, (x, y) in enumerate(zip(a, b, strict=False)) if x != y),
            min(len(a), len(b)),
        )
        result["differences"].append(
            {
                "snapshot": f"{mode}/drawing.bmp",
                "chunk": "",
                "element": str(index),
                "field": "byte",
                "reference": f"{len(a)} bytes",
                "candidate": f"{len(b)} bytes",
            }
        )
        return  # Failing runs keep both images.
    reference.unlink()
    path.unlink()
