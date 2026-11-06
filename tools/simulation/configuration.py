"""Driver-supplied paths and mode for simulation helpers."""

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Configuration:
    common_local: Path
    reference: Path
    reference_build: Path
    candidate_build: Path
    sanitize: bool


_current: Configuration | None = None


def configure(value: Configuration) -> None:
    global _current
    _current = value


def configuration() -> Configuration:
    if _current is None:
        raise RuntimeError("simulation configuration has not been supplied")
    return _current
