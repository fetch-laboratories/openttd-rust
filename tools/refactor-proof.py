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

from cleanup_contract import context, reviewed
from vcs import git

ROOT = Path(__file__).resolve().parents[1]


def normalize(value: str, renames: dict[str, str]) -> str:
    # Simultaneous exact symbol replacements, including section-name suffixes;
    # substring edits could accidentally rewrite another relocation target.
    return re.sub(
        r"[A-Za-z_.$][A-Za-z_0-9.$]*", lambda m: renames.get(m[0], m[0]), value
    )


def archive_sections(archive: Path, renames: dict[str, str]) -> Counter[str]:
    """Inventory ELF section bytes, relocation targets/addends, and bound symbols.

    Ignore archive member filenames and non-allocated metadata. Keep section
    alignment and symbol offsets: identical instructions alone prove too little.
    .data.rel.ro is writable in the object but read-only after relocation.
    """
    env = os.environ | {"LC_ALL": "C"}
    output = subprocess.check_output(
        ["objdump", "-h", "-s", "-r", "-t", str(archive)], text=True, env=env
    )
    result: Counter[str] = Counter()
    sections: dict[str, dict[str, object]] = {}
    current = ""
    mode = ""
    pending: tuple[str, str, str] | None = None

    def finish() -> None:
        for name, row in sections.items():
            if row["selected"]:
                payload = dict(row, name=normalize(name, renames))
                del payload["selected"]
                result[normalize(json.dumps(payload, sort_keys=True), renames)] += 1

    objects = 0
    for line in output.splitlines():
        if ":     file format " in line:
            finish()
            sections = {}
            objects += 1
            mode = ""
            continue
        header = re.match(
            r"\s*\d+\s+(\S+)\s+([0-9a-f]+)\s+[0-9a-f]+\s+[0-9a-f]+\s+[0-9a-f]+\s+(2\*\*\d+)",
            line,
        )
        if header:
            pending = (header[1], header[2], header[3])
            continue
        if pending:
            name, size, alignment = pending
            selected = "ALLOC" in line and (
                "CODE" in line or "READONLY" in line or name.startswith(".data.rel.ro")
            )
            sections[name] = {
                "size": int(size, 16),
                "alignment": alignment,
                "flags": line.strip(),
                "bytes": "",
                "relocations": [],
                "symbols": [],
                "selected": selected,
            }
            pending = None
            continue
        relocation = re.match(r"RELOCATION RECORDS FOR \[(.*)\]:", line)
        contents = re.match(r"Contents of section (.*):", line)
        if relocation or contents:
            match = relocation or contents
            assert match
            current = match[1]
            mode = "relocations" if relocation else "bytes"
            continue
        if line == "SYMBOL TABLE:":
            mode = "symbols"
            continue
        if mode == "symbols":
            fields = line.split()
            if (
                len(fields) >= 5
                and fields[-3] in sections
                and re.fullmatch(r"[0-9a-f]+", fields[0])
            ):
                symbols = sections[fields[-3]]["symbols"]
                assert isinstance(symbols, list)
                symbols.append(" ".join(fields))
        elif (
            current in sections
            and mode == "relocations"
            and re.match(r"^[0-9a-f]+\s+R_", line)
        ):
            relocations = sections[current]["relocations"]
            assert isinstance(relocations, list)
            relocations.append(" ".join(line.split()))
        elif (
            current in sections and mode == "bytes" and re.match(r"^ [0-9a-f]+ ", line)
        ):
            # Four fixed-width hex groups precede objdump's ASCII rendering.
            data = line.split(None, 1)[1][:35]
            sections[current]["bytes"] = str(sections[current]["bytes"]) + "".join(
                data.split()
            )
    finish()
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
            f"{label}: {sum(rows.values())} code/read-only sections; canonical sha256 {digest}"
        )
    removed, added = before - after, after - before
    print(
        f"refactor proof: {sum(removed.values())} removed/changed; {sum(added.values())} added/changed sections"
    )
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
    if not git("diff", "--name-only", args.base_ref, "HEAD", "--", "rust/").strip():
        print("refactor proof: no Rust source changes")
        return 0
    author, proposals, comments, head = context()
    claims = [p for p in proposals if p.get("kind") == "refactor-proof"]
    if not claims:
        print(
            "refactor proof: no archive-equivalence claim in PR; behavior validation remains required"
        )
        return 0
    if len(claims) != 1 or claims[0].get("method") != "archive":
        raise ValueError("require one archive proof contract")
    claim = claims[0]
    renames = claim.get("renames", {})
    if renames and not reviewed(claim, comments, author, head):
        raise ValueError(
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
    raise SystemExit(main())
