"""Pinned standard lexers for code-line metrics and boundary token scans."""

import atexit
import hashlib
import importlib
import json
import os
import re
import sys
from functools import lru_cache
from pathlib import Path

try:
    pygments = importlib.import_module("pygments")

    if pygments.__version__ != "2.19.2":
        raise ImportError("wrong Pygments version")
except ImportError:
    # Also covers tools/test-*.py imports: the documented python3 commands work
    # without changing the user's Python installation. uv caches the pinned env.
    os.execvp(
        "uv",
        [
            "uv",
            "run",
            "--no-project",
            "--with",
            "pygments==2.19.2",
            "python3",
            *sys.argv,
        ],
    )

# The pinned dependency may live only in the uv environment after re-exec.
lex = pygments.lex
lexers = importlib.import_module("pygments.lexers")
Comment = importlib.import_module("pygments.token").Comment
String = importlib.import_module("pygments.token").String

RUST_EXPORT = re.compile(
    r"#\[(?:unsafe\()?no_mangle\)?\]\s*(?:#\[[^\n]*\]\s*)*"
    r'(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?extern\s+"C"\s+fn\s+'
    r"(openttd_rust_\w+)\s*\("
)


LEXERS = {
    "rust": lexers.RustLexer,
    "python": lexers.PythonLexer,
    "cpp": lexers.CppLexer,
}


def language(path: str) -> str:
    """The lexer a path selects. A result depends on the text and this alone."""
    return (
        "rust" if path.endswith(".rs") else "python" if path.endswith(".py") else "cpp"
    )


def lexed(text: str, lexer: str) -> list[tuple[object, str]]:
    instance = LEXERS[lexer](ensurenl=False, stripnl=False)
    instance.add_filter("tokenmerge")
    return list(lex(text, instance))


def tokens(text: str, path: str) -> list[tuple[object, str]]:
    return lexed(text, language(path))


# Lexing dominated the metric tools: the growth ratchet spent three quarters of
# 131 seconds lexing about 3,000 files, each twice, because it measures two
# revisions that share nearly every file. Each text is now lexed once in a run,
# and `persist` keeps code-line results across runs: the pinned original's files
# never change, and a commit changes few of the fork's.
class Store:
    """Code-line results on disk, keyed by lexer and text.

    The file name carries this module's own hash and the Pygments version, so a
    change to either starts an empty store and no entry outlives its meaning.
    """

    def __init__(self, directory: Path) -> None:
        version = hashlib.sha256(
            Path(__file__).read_bytes() + pygments.__version__.encode()
        ).hexdigest()[:20]
        self.path = directory / f"code-lines-{version}.json"
        self.entries = self.read()
        self.added: dict[str, list[int]] = {}

    def read(self) -> dict[str, list[int]]:
        try:
            return json.loads(self.path.read_text())
        except (OSError, ValueError):
            return {}

    def save(self) -> None:
        """Add this run's results to whatever another run saved meanwhile."""
        if not self.added:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_name(f"{self.path.name}.{os.getpid()}")
        temporary.write_text(json.dumps(self.read() | self.added))
        os.replace(temporary, self.path)
        self.added = {}


STORE: Store | None = None


def persist(directory: Path) -> None:
    """Keep code-line results in `directory` from this call until exit."""
    global STORE
    if STORE is None:
        STORE = Store(directory)
        atexit.register(STORE.save)


@lru_cache(maxsize=None)
def _code_lines(text: str, lexer: str) -> frozenset[int]:
    key = hashlib.sha256(f"{lexer}\0{text}".encode()).hexdigest()
    if STORE is not None and key in STORE.entries:
        return frozenset(STORE.entries[key])
    result, line = set(), 1
    for kind, value in lexed(text, lexer):
        for offset, part in enumerate(value.split("\n")):
            if kind not in Comment and kind not in String.Doc and part.strip():
                result.add(line + offset)
        line += value.count("\n")
    if STORE is not None:
        STORE.added[key] = sorted(result)
    return frozenset(result)


def code_lines(text: str, path: str) -> set[int]:
    """1-based lines containing code; preprocessor and docstrings are comments."""
    return set(_code_lines(text, language(path)))


@lru_cache(maxsize=None)
def _without_comments(text: str, lexer: str, strings: bool) -> str:
    return "".join(
        re.sub(r"[^\n]", " ", value)
        if kind in Comment
        and kind not in Comment.Preproc
        or strings
        and kind in String
        and value != '"C"'
        else value
        for kind, value in lexed(text, lexer)
    )


def without_comments(text: str, path: str, strings: bool = False) -> str:
    """Mask lexical comments (and optionally strings), retaining positions."""
    return _without_comments(text, language(path), strings)
