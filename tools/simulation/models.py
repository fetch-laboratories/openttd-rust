"""Shared simulation scenario, game and result records."""

from collections.abc import Callable, Mapping
from pathlib import Path
from typing import Any, Literal, NotRequired, TypedDict, cast

ROOT = Path(__file__).resolve().parents[2]

RUNTIME_DIRECTORIES = ("lang", "baseset", "ai", "game", "regression")

TICKS_PER_DAY = 74
SNAPSHOT_TICKS = 32 * TICKS_PER_DAY

Kind = Literal["generate", "save", "regression"]
Role = Literal["reference", "candidate"]
Mode = Literal["snapshots", "plain"]
ROLES: tuple[Role, Role] = ("reference", "candidate")


class Scenario(TypedDict):
    """Keys core reads; each family's record extends it with its own keys."""

    name: str
    kind: Kind
    ticks: int
    seed: NotRequired[int]
    map_log2: NotRequired[int]
    map_log2_y: NotRequired[int]
    land_generator: NotRequired[int]
    settings: NotRequired[Mapping[str, Mapping[str, int | bool | str]]]
    console: NotRequired[list[str]]
    save: NotRequired[str]
    test: NotRequired[str]
    video: NotRequired[str]
    short_checkpoint: NotRequired[bool]
    snapshot_minimum: NotRequired[int]
    witnesses: NotRequired[bool]
    executables: NotRequired[dict[str, Path]]
    role_inputs: NotRequired[dict[str, dict[str, Any]]]
    scenario_ai: NotRequired[str]


class Plan(TypedDict):
    """A prepared launch (prepare_game) that execute_game runs."""

    dir: Path
    command: list[str]
    env: dict[str, str]
    build: Path
    timeout: int
    isolate: bool
    instructions: dict[str, Any] | None
    native_threads: bool


class Executed(TypedDict):
    """One finished game (execute_game)."""

    dir: Path
    exit: int | Literal["timeout"]
    seconds: float
    command: list[str]
    instructions: dict[str, Any] | None


class Run(Executed):
    """A finished game with its decoded output (read_game)."""

    snapshots: list[Path]
    log: list[str]
    stdout: bytes
    sanitizer: Any


class Difference(TypedDict):
    snapshot: str
    chunk: str
    element: str
    field: str
    reference: str
    candidate: str
    issue: NotRequired[str]


class Stats(TypedDict):
    chunks: int
    elements: int
    masked: dict[str, int]


def empty_stats() -> Stats:
    return {"chunks": 0, "elements": 0, "masked": {}}


class SpeedSample(TypedDict):
    reference_seconds: float
    candidate_seconds: float
    candidate_reference_ratio: float | None
    same_end: bool


class PlainSpeed(TypedDict):
    samples: list[SpeedSample]
    median_candidate_reference_ratio: float | None
    budget: NotRequired[Any]


class Result(TypedDict):
    """One scenario's report entry. Mode-, role- and family-specific keys go
    through findings(); a TypedDict names only literal keys."""

    scenario: str
    plain_speed: PlainSpeed
    differences: list[Difference]
    known_failures: list[Difference]
    problems: list[str]
    notes: list[str]
    snapshots: int
    stats: Stats
    requested_ticks: NotRequired[int]
    instruction_speed: NotRequired[Any]
    plain_commands: NotRequired[dict[Role, list[str]]]
    sanitizer: NotRequired[dict[str, Any]]
    passed: NotRequired[bool]
    seconds: NotRequired[float]


def findings(result: Result) -> dict[str, Any]:
    """The result as an open record, for keys built from a mode, role or family."""
    return cast(dict[str, Any], result)


Install = Callable[[Scenario, Path], None]
Prepare = Callable[
    [Scenario, dict[str, Path], dict[str, Path], Path, int, dict[str, str], Result],
    Scenario,
]
GameArgs = Callable[[Scenario], list[str]]
GameEnvironment = Callable[[Scenario], Mapping[str, str]]
Check = Callable[[Scenario, Run, Mode, Role, Result], None]
