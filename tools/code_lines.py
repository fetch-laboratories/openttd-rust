"""Pinned standard lexers for code-line metrics and boundary token scans."""

import os
import sys

try:
    import pygments

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

from pygments import lex
from pygments.lexers import CppLexer, PythonLexer, RustLexer
from pygments.token import Comment, String


def tokens(text: str, path: str) -> list[tuple]:
    lexer = (
        RustLexer
        if path.endswith(".rs")
        else PythonLexer
        if path.endswith(".py")
        else CppLexer
    )
    return list(lex(text, lexer(ensurenl=False, stripnl=False)))


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
    import re

    return "".join(
        re.sub(r"[^\n]", " ", value)
        if kind in Comment and kind not in Comment.Preproc or strings and kind in String
        else value
        for kind, value in tokens(text, path)
    )
