"""Pinned standard lexers for code-line metrics and boundary token scans."""

import importlib
import os
import re
import sys

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


def tokens(text: str, path: str) -> list[tuple[object, str]]:
    lexer = (
        lexers.RustLexer
        if path.endswith(".rs")
        else lexers.PythonLexer
        if path.endswith(".py")
        else lexers.CppLexer
    )
    instance = lexer(ensurenl=False, stripnl=False)
    instance.add_filter("tokenmerge")
    return list(lex(text, instance))


def code_lines(text: str, path: str) -> set[int]:
    """1-based lines containing code; preprocessor and docstrings are comments."""
    result, line = set(), 1
    for kind, value in tokens(text, path):
        for offset, part in enumerate(value.split("\n")):
            if kind not in Comment and kind not in String.Doc and part.strip():
                result.add(line + offset)
        line += value.count("\n")
    return result


def without_comments(text: str, path: str, strings: bool = False) -> str:
    """Mask lexical comments (and optionally strings), retaining positions."""
    return "".join(
        re.sub(r"[^\n]", " ", value)
        if kind in Comment
        and kind not in Comment.Preproc
        or strings
        and kind in String
        and value != '"C"'
        else value
        for kind, value in tokens(text, path)
    )
