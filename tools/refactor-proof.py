#!/usr/bin/env python3
"""Prove claimed Rust refactors with release code, relocations and read-only data.

Local archives can be compared directly. CI builds both captured revisions only
when the PR requests an archive proof in its cleanup-acceptance block. Renames
require an independent attributed acceptance at the tested head.
"""

import argparse
import hashlib
import io
import json
import os
import re
import subprocess
import tarfile
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

from cleanup_contract import ContractError, context, reviewed, run
from vcs import git

ROOT = Path(__file__).resolve().parents[1]


def normalize(value: str, renames: dict[str, str]) -> str:
    """Simultaneous raw-symbol rewrites, including section-name suffixes."""
    if not renames:
        return value

    def replace(match: re.Match[str]) -> str:
        symbol = match[0]
        if symbol in renames:
            return renames[symbol]
        for prefix in (".text.", ".rodata.", ".data.rel.ro."):
            if symbol.startswith(prefix):
                return prefix + renames.get(
                    symbol[len(prefix) :], symbol[len(prefix) :]
                )
        return symbol

    return re.sub(r"[A-Za-z_.$][A-Za-z_0-9.$]*", replace, value)


def section_header(header: re.Match[str], flags: str) -> dict[str, Any]:
    name, size, alignment = header.groups()
    selected = "ALLOC" in flags and (
        "CODE" in flags or "READONLY" in flags or name.startswith(".data.rel.ro")
    )
    return {
        "size": int(size, 16),
        "alignment": alignment,
        "flags": flags.strip(),
        "bytes": [],
        "relocations": [],
        "symbols": [],
        "selected": selected,
    }


def dump_payload(
    line: str,
    mode: str,
    current: str,
    sections: dict[str, dict[str, Any]],
    renames: dict[str, str],
) -> None:
    fields = line.split()
    if mode == "symbols":
        if (
            len(fields) >= 5
            and fields[-3] in sections
            and sections[fields[-3]]["selected"]
            and re.fullmatch(r"[0-9a-f]+", fields[0])
        ):
            fields[-1] = normalize(fields[-1], renames)
            fields[-3] = normalize(fields[-3], renames)
            # Look up the unnormalized section name before recording its row.
            section = line.split()[-3]
            sections[section]["symbols"].append(" ".join(fields))
    elif (
        current in sections
        and sections[current]["selected"]
        and mode == "relocations"
        and re.match(r"^[0-9a-f]+\s+R_", line)
    ):
        fields[2] = normalize(fields[2], renames)
        sections[current]["relocations"].append(" ".join(fields))
    elif (
        current in sections
        and sections[current]["selected"]
        and mode == "bytes"
        and re.match(r"^ [0-9a-f]+ ", line)
    ):
        # Four fixed-width hex groups precede objdump's ASCII rendering.
        sections[current]["bytes"].append("".join(line.split(None, 1)[1][:35].split()))


def object_sections(output: str, renames: dict[str, str]) -> Counter[str]:
    sections: dict[str, dict[str, Any]] = {}
    current, mode = "", ""
    lines = iter(output.splitlines())
    for line in lines:
        header = re.match(
            r"\s*\d+\s+(\S+)\s+([0-9a-f]+)\s+[0-9a-f]+\s+[0-9a-f]+\s+[0-9a-f]+\s+(2\*\*\d+)",
            line,
        )
        if header:
            sections[header[1]] = section_header(header, next(lines))
            continue
        relocation = re.match(r"RELOCATION RECORDS FOR \[(.*)\]:", line)
        contents = re.match(r"Contents of section (.*):", line)
        if relocation or contents:
            match = relocation or contents
            assert match
            current, mode = match[1], "relocations" if relocation else "bytes"
        elif line == "SYMBOL TABLE:":
            mode = "symbols"
        else:
            dump_payload(line, mode, current, sections, renames)
    result: Counter[str] = Counter()
    for name, row in sections.items():
        if row.pop("selected"):
            row["bytes"] = "".join(row["bytes"])
            result[
                json.dumps(dict(row, name=normalize(name, renames)), sort_keys=True)
            ] += 1
    return result


def archive_sections(archive: Path, renames: dict[str, str]) -> Counter[str]:
    """Inventory ELF bytes, relocation targets/addends and bound symbol offsets.

    Ignore archive member filenames and non-allocated metadata. Keep each
    object's sections together: local relocation targets resolve within that
    object. .data.rel.ro is read-only after relocation.
    """
    output = subprocess.check_output(
        ["objdump", "-h", "-s", "-r", "-t", str(archive)],
        text=True,
        env=os.environ | {"LC_ALL": "C"},
    )
    objects = re.split(r"^.*:     file format .*?$", output, flags=re.M)[1:]
    result: Counter[str] = Counter()
    for obj in objects:
        sections = object_sections(obj, renames)
        if sections:
            result[json.dumps(sorted(sections.items()))] += 1
    if not objects or not result:
        raise ValueError(
            f"archive contains no classified allocated code/data: {archive}"
        )
    return result


def compare(base: Path, head: Path, renames: dict[str, str]) -> bool:
    if len(set(renames.values())) != len(renames):
        raise ValueError("rename map must be injective")
    before, after = archive_sections(base, renames), archive_sections(head, {})
    for label, rows in (("base", before), ("head", after)):
        digest = hashlib.sha256(json.dumps(sorted(rows.items())).encode()).hexdigest()
        print(
            f"{label}: {sum(rows.values())} code/read-only object groups; canonical sha256 {digest}"
        )
    removed, added = before - after, after - before
    print(
        f"refactor proof: {sum(removed.values())} removed/changed; {sum(added.values())} added/changed object groups"
    )
    if not renames:
        return not removed and not added
    # A rename that names nothing is an invalid audit, even if archives match.
    used = "\n".join(archive_sections(base, {}))
    unused = [
        old
        for old in renames
        if not re.search(
            r"(?<![A-Za-z_0-9.$])" + re.escape(old) + r"(?![A-Za-z_0-9.$])", used
        )
    ]
    if unused:
        raise ValueError(f"unused audited renames: {unused}")
    return not removed and not added


def build(revision: str, destination: Path) -> Path:
    import migration

    source = destination / "source"
    source.mkdir(parents=True)
    archive = subprocess.check_output(["git", "archive", revision], cwd=ROOT)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(source, filter="data")
    env = migration.environment()
    env["CARGO_TARGET_DIR"] = str(destination / "target")
    env["RUSTFLAGS"] = "--remap-path-prefix=" + str(source) + "=/openttd-refactor"
    subprocess.run(
        ["cargo", "build", "--release", "--workspace", "--locked"],
        cwd=source,
        env=env,
        check=True,
    )
    return destination / "target/release/libopenttd_kernels.a"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    parser.add_argument(
        "--build",
        action="store_true",
        help="locally build and compare both revisions without a PR claim",
    )
    parser.add_argument("--base-archive", type=Path)
    parser.add_argument("--head-archive", type=Path)
    parser.add_argument(
        "--renames", type=Path, help="local audited old-to-new raw symbol JSON"
    )
    args = parser.parse_args()
    renames = json.loads(args.renames.read_text()) if args.renames else {}
    if args.base_archive or args.head_archive:
        if not (args.base_archive and args.head_archive):
            parser.error("both archives are required")
        return int(not compare(args.base_archive, args.head_archive, renames))
    if args.renames:
        parser.error("CI renames must come from an exact-head reviewed PR contract")
    if (
        not args.build
        and not git("diff", "--name-only", args.base_ref, "HEAD", "--", "rust/").strip()
    ):
        print("refactor proof: no Rust source changes")
        return 0
    claim: dict[str, Any] = {
        "kind": "refactor-proof",
        "method": "archive",
        "renames": {},
    }
    author, proposals, comments, head = (
        ("", [claim], [], "") if args.build else context(args.base_ref)
    )
    claims = [p for p in proposals if p["kind"] == "refactor-proof"]
    if not claims:
        print(
            "refactor proof: no archive-equivalence claim recorded; behavior validation remains required"
        )
        return 0
    if len(claims) != 1 or claims[0]["method"] != "archive":
        raise ContractError("require one archive proof contract")
    claim = claims[0]
    renames = claim["renames"]
    if renames and not reviewed(claim, comments, author, head):
        raise ContractError(
            "symbol renames need independent attributed acceptance at this head"
        )
    output = ROOT / ".local/refactor-proof"
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output) as directory:
        destination = Path(directory)
        base_archive = build(args.base_ref, destination / "base")
        head_archive = build("HEAD", destination / "head")
        return int(not compare(base_archive, head_archive, renames))


if __name__ == "__main__":
    run(main)
