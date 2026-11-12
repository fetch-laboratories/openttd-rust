"""Classify base-update merges by docs/workflow.md's Integration rule.

A reviewed PR's base update needs only CI when `git show --remerge-diff` shows
no conflict or merge-time edit under the guarded paths, or shows only removed
conflict markers there. Anything else, including output this module cannot
parse with confidence, needs a reviewer.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from dataclasses import asdict, dataclass, field
from pathlib import Path

GUARDED = ("src/", "rust/", "tools/", ".github/")
DEFAULT_BASE = "origin/rust-migration"
RULE = "docs/workflow.md#integration"
CLEAN, BOTH_SIDES, NEEDS_REVIEW, NOT_BASE = (
    "clean",
    "both-sides",
    "needs review",
    "not a base update",
)
KEPT = "conflict resolved by keeping both sides"
EXIT_NEEDS_REVIEW = 3
EXIT_NOT_BASE = 4
# Headers git writes between `diff --git` and the first hunk.
HEADERS = (
    "index ",
    "old mode ",
    "new mode ",
    "deleted file mode ",
    "new file mode ",
    "similarity index ",
    "dissimilarity index ",
    "copy from ",
    "copy to ",
)
HUNK = re.compile(r"@@ -\d+(?:,(\d+))? \+\d+(?:,(\d+))? @@")
MARKERS = (
    re.compile(r"<{7}(?: .*)?"),
    re.compile(r"={7}\r?"),
    re.compile(r">{7}(?: .*)?"),
)
ESCAPES = {"a": 7, "b": 8, "t": 9, "n": 10, "v": 11, "f": 12, "r": 13}


class Unparseable(ValueError):
    """Remerge output this module cannot read with confidence."""


@dataclass
class FileDiff:
    paths: list[str] = field(default_factory=list)
    messages: list[str] = field(default_factory=list)
    removed: list[str] = field(default_factory=list)
    added: list[str] = field(default_factory=list)
    hunks: int = 0
    other: list[str] = field(default_factory=list)


@dataclass
class Finding:
    path: str
    guarded: bool
    reason: str


@dataclass
class Merge:
    sha: str
    parents: list[str]
    classification: str
    findings: list[Finding]
    detail: str = ""
    unclassified: bool = False

    @property
    def label(self) -> str:
        unknown = ": could not classify" if self.unclassified else ""
        return self.classification + unknown


def git_bytes(cwd: Path, *arguments: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(["git", *arguments], cwd=cwd, capture_output=True)


def git(cwd: Path, *arguments: str) -> str:
    result = git_bytes(cwd, *arguments)
    if result.returncode:
        message = result.stderr.decode(errors="replace").strip()
        raise RuntimeError(f"git {' '.join(arguments)} failed: {message}")
    return result.stdout.decode(errors="surrogateescape").strip()


def unquote(text: str) -> str:
    """Read a path as git writes it, C-quoted when it has unusual bytes."""
    text = text.rstrip("\t")
    if not text.startswith('"'):
        return text
    if len(text) < 2 or not text.endswith('"'):
        raise Unparseable(f"unterminated quoted path {text!r}")
    body, out, index = text[1:-1], bytearray(), 0
    while index < len(body):
        char = body[index]
        if char != "\\":
            out.extend(char.encode(errors="surrogateescape"))
            index += 1
        elif body[index + 1 : index + 2] in ESCAPES:
            out.append(ESCAPES[body[index + 1]])
            index += 2
        elif re.fullmatch(r"[0-7]{3}", body[index + 1 : index + 4]):
            out.append(int(body[index + 1 : index + 4], 8))
            index += 4
        else:
            out.extend(body[index + 1 : index + 2].encode())
            index += 2
    return out.decode(errors="surrogateescape")


def side_path(text: str) -> str | None:
    path = unquote(text)
    if path == "/dev/null":
        return None
    if path[:2] not in ("a/", "b/"):
        raise Unparseable(f"path without a/ or b/ prefix: {text!r}")
    return path[2:]


def header_paths(line: str) -> list[str]:
    """Paths from `diff --git a/X b/Y`, used when no ---/+++ lines follow."""
    rest = line.removeprefix("diff --git ")
    quoted = re.fullmatch(r'("(?:[^"\\]|\\.)*")\s+("(?:[^"\\]|\\.)*")', rest)
    if quoted:
        return [p for p in dict.fromkeys(map(side_path, quoted.groups())) if p]
    half = (len(rest) - 1) // 2
    if rest[half : half + 1] == " " and rest[:half][2:] == rest[half + 1 :][2:]:
        return [p for p in (side_path(rest[:half]),) if p]
    raise Unparseable(f"ambiguous diff header {line!r}")


def read_hunk(lines: list[str], index: int, diff: FileDiff) -> int:
    match = HUNK.match(lines[index])
    if not match:
        raise Unparseable(f"bad hunk header {lines[index]!r}")
    old, new = (int(n) if n is not None else 1 for n in match.groups())
    diff.hunks += 1
    index += 1
    while old > 0 or new > 0:
        if index >= len(lines):
            raise Unparseable("hunk ends early")
        line = lines[index]
        kind, text = line[:1], line[1:]
        if kind == " ":
            old, new = old - 1, new - 1
        elif kind == "-":
            old -= 1
            diff.removed.append(text)
        elif kind == "+":
            new -= 1
            diff.added.append(text)
        elif kind != "\\":
            raise Unparseable(f"bad hunk line {line!r}")
        index += 1
    if index < len(lines) and lines[index].startswith("\\"):
        index += 1
    if old < 0 or new < 0:
        raise Unparseable("hunk longer than its header says")
    return index


def read_header(line: str, diff: FileDiff) -> None:
    if line.startswith("remerge "):
        diff.messages.append(line.removeprefix("remerge "))
    elif line.startswith(("--- ", "+++ ", "rename from ", "rename to ")):
        path = line.split(" ", 2)[-1] if line.startswith("rename") else line[4:]
        named = unquote(path) if line.startswith("rename") else side_path(path)
        if named and named not in diff.paths:
            diff.paths.append(named)
    elif line.startswith("Binary files ") or line == "GIT binary patch":
        diff.other.append("binary content")
    elif line.startswith(("old mode ", "new mode ", "deleted file ", "new file ")):
        diff.other.append(line)
    elif not line.startswith(HEADERS):
        raise Unparseable(f"unexpected line {line!r}")


def parse(text: str) -> list[FileDiff]:
    """Split remerge-diff output into files; refuse anything unrecognised."""
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    diffs: list[FileDiff] = []
    header = ""
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("diff --git "):
            if diffs and not diffs[-1].paths:
                diffs[-1].paths = header_paths(header)
            diffs.append(FileDiff())
            header = line
            index += 1
        elif not diffs:
            raise Unparseable(f"output before the first diff: {line!r}")
        elif line.startswith("@@ "):
            index = read_hunk(lines, index, diffs[-1])
        else:
            read_header(line, diffs[-1])
            index += 1
    if diffs and not diffs[-1].paths:
        diffs[-1].paths = header_paths(header)
    return diffs


def markers_only(diff: FileDiff) -> bool:
    """True when the resolution removes only whole conflict-marker triples."""
    if diff.added or diff.other or not diff.hunks or len(diff.removed) % 3:
        return False
    return all(
        MARKERS[index % 3].fullmatch(line) for index, line in enumerate(diff.removed)
    )


def reason(diff: FileDiff) -> str | None:
    """Why this file needs a reviewer, or None when markers alone were removed."""
    if not diff.messages:
        return "merge-time edit outside any conflict"
    if any(not m.startswith("CONFLICT (content)") for m in diff.messages):
        kinds = sorted({m.split(":", 1)[0] for m in diff.messages})
        return f"{', '.join(kinds)} resolved by choosing a side or a new version"
    if markers_only(diff):
        return None
    if diff.other:
        return f"conflict resolution changes {', '.join(sorted(set(diff.other)))}"
    sides = sum(not any(m.fullmatch(r) for m in MARKERS) for r in diff.removed)
    return f"conflict resolved with an edit (+{len(diff.added)} lines, -{sides} side lines)"


def guarded(paths: list[str]) -> bool:
    return any(path.startswith(GUARDED) for path in paths)


def classify_diffs(diffs: list[FileDiff]) -> tuple[str, list[Finding]]:
    findings: list[Finding] = []
    resolved = False
    for diff in diffs:
        why = reason(diff)
        inside = guarded(diff.paths)
        if why is None:
            resolved |= inside
            why = KEPT
        findings.append(Finding(" -> ".join(diff.paths), inside, why))
    if any(f.guarded and f.reason != KEPT for f in findings):
        return NEEDS_REVIEW, findings
    return (BOTH_SIDES if resolved else CLEAN), findings


def remerge(cwd: Path, sha: str) -> str:
    result = git_bytes(
        cwd,
        "-c",
        "merge.conflictStyle=merge",
        "-c",
        "diff.noprefix=false",
        "-c",
        "diff.mnemonicPrefix=false",
        "-c",
        "diff.suppressBlankEmpty=false",
        "show",
        "--remerge-diff",
        # Root paths, whatever the working directory or diff.relative.
        "--no-relative",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--format=",
        sha,
    )
    if result.returncode:
        message = result.stderr.decode(errors="replace").strip()
        raise Unparseable(f"git show --remerge-diff failed: {message}")
    return result.stdout.decode(errors="surrogateescape")


def classify(cwd: Path, sha: str, base: str) -> Merge:
    """Classify one commit; the result says why when it is not a base update."""
    full = git(cwd, "rev-parse", "--verify", f"{sha}^{{commit}}")
    parents = git(cwd, "rev-list", "--parents", "-n", "1", full).split()[1:]
    if len(parents) != 2:
        count = f"{len(parents)} parent" + ("" if len(parents) == 1 else "s")
        detail = f"it has {count}; a base update has two"
        return Merge(full, parents, NOT_BASE, [], detail)
    second = git_bytes(cwd, "merge-base", "--is-ancestor", parents[1], base)
    if second.returncode:
        detail = f"its second parent {parents[1][:10]} is not on {base}"
        return Merge(full, parents, NOT_BASE, [], detail)
    try:
        classification, findings = classify_diffs(parse(remerge(cwd, full)))
    except Unparseable as error:
        return Merge(full, parents, NEEDS_REVIEW, [], str(error), unclassified=True)
    return Merge(full, parents, classification, findings)


def merges_since(cwd: Path, reviewed: str) -> list[str]:
    """Merges on HEAD's first-parent line after the reviewed commit, oldest first."""
    if git_bytes(cwd, "merge-base", "--is-ancestor", reviewed, "HEAD").returncode:
        raise RuntimeError(
            f"{reviewed} is not a commit that HEAD contains; pass the reviewed head"
            " of this branch, or check out the PR's head first"
        )
    listing = git(
        cwd, "rev-list", "--reverse", "--first-parent", "--min-parents=2",
        f"{reviewed}..HEAD",
    )  # fmt: skip
    return listing.split()


def exit_code(merges: list[Merge]) -> int:
    kinds = {merge.classification for merge in merges}
    if NOT_BASE in kinds:
        return EXIT_NOT_BASE
    return EXIT_NEEDS_REVIEW if NEEDS_REVIEW in kinds else 0


def review_files(merges: list[Merge]) -> list[str]:
    return sorted(
        {f.path for m in merges for f in m.findings if f.guarded and f.reason != KEPT}
    )


def status_line(merges: list[Merge], reviewed: str | None) -> str:
    if not merges:
        return f"No merges since {reviewed}: there is no base update to classify."
    named = ", ".join(f"{m.sha[:10]} {m.label}" for m in merges)
    subject = f"Base updates since {reviewed[:10]}: {named}" if reviewed else ""
    if not reviewed:
        subject = f"Base update {merges[0].sha[:10]}: {merges[0].label}"
    code = exit_code(merges)
    if code == EXIT_NOT_BASE:
        return f"{subject}; the base-update rule does not apply ({RULE})"
    if code == 0:
        return f"{subject}; needs only CI ({RULE})"
    files = review_files(merges)
    where = f" in {', '.join(files)}" if files else ""
    return f"{subject}; a fresh reviewer checks the conflict resolution{where} ({RULE})"


def merge_lines(merge: Merge, base: str) -> list[str]:
    parents = " ".join(p[:10] for p in merge.parents) or "none"
    lines = [f"{merge.sha[:10]} parents {parents}: {merge.label}"]
    if merge.detail:
        lines.append(f"  {merge.detail}")
    if merge.classification == NOT_BASE:
        lines.append(
            f"  A base update merges {base} as its second parent. Pass the merge"
            " commit, or --since REVIEWED_SHA to classify every merge after it."
        )
    for finding in merge.findings:
        scope = "" if finding.guarded else " (outside src/ rust/ tools/ .github/)"
        lines.append(f"  {finding.path}: {finding.reason}{scope}")
    if merge.classification == NEEDS_REVIEW:
        lines.append(f"  Show it with: git show --remerge-diff {merge.sha[:10]}")
    return lines


def add_parser(commands: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    command = commands.add_parser(
        "base-update",
        help="whether a base update needs a reviewer",
        description=(
            "Classify base-update merges as clean, both-sides or needs review"
            f" ({RULE}). Exit 0 when only CI is needed, {EXIT_NEEDS_REVIEW} when a"
            f" reviewer is, {EXIT_NOT_BASE} when a commit is not a base update."
        ),
    )
    command.add_argument(
        "merge",
        nargs="?",
        metavar="MERGE_SHA",
        help="the merge commit to classify (default HEAD)",
    )
    command.add_argument(
        "--since",
        metavar="REVIEWED_SHA",
        help="classify every merge on HEAD's first-parent line after this commit",
    )
    command.add_argument(
        "--base",
        default=DEFAULT_BASE,
        metavar="REF",
        help=f"the base branch's ref (default {DEFAULT_BASE})",
    )
    command.add_argument("--json", action="store_true")


def run(args: argparse.Namespace, cwd: Path) -> int:
    if args.since and args.merge:
        print("base-update: pass MERGE_SHA or --since REVIEWED_SHA, not both.")
        print("--since classifies every merge from the reviewed commit to HEAD.")
        return 2
    try:
        git(cwd, "rev-parse", "--verify", f"{args.base}^{{commit}}")
        if args.since:
            args.since = git(cwd, "rev-parse", "--verify", f"{args.since}^{{commit}}")
        shas = merges_since(cwd, args.since) if args.since else [args.merge or "HEAD"]
        merges = [classify(cwd, sha, args.base) for sha in shas]
    except RuntimeError as error:
        print(f"base-update: {error}")
        print(f"Fetch the base with `git fetch origin rust-migration` if {args.base}"
              " is missing, then run this again.")  # fmt: skip
        return 2
    status = status_line(merges, args.since)
    code = exit_code(merges)
    if args.json:
        result = {
            "schema_version": 1,
            "base": args.base,
            "merges": [{**asdict(m), "label": m.label} for m in merges],
            "status_line": status,
            "review_files": review_files(merges),
            "exit_code": code,
        }
        print(json.dumps(result, indent=2))
        return code
    for merge in merges:
        print("\n".join(merge_lines(merge, args.base)))
    print(status)
    return code
