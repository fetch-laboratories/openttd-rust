#!/usr/bin/env python3
"""The package archives a paid CI job keeps in the Actions cache (#896).

apt uses an archive already in its directory when the name and size are the
ones it expects, without reading its hash. So `verify`, which apt runs as a
hook after each successful `apt-get update`, deletes every kept archive whose
SHA-256 the freshly authenticated index does not list for that package,
version and architecture: a stale, damaged or foreign file is downloaded
again, never installed. `record` and `added` tell the job whether it holds
anything the restored set lacked, which is when it saves.
"""

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from urllib.parse import unquote


def digest(path: Path) -> str:
    with path.open("rb") as archive:
        return hashlib.file_digest(archive, "sha256").hexdigest()


def archives(directory: Path) -> list[Path]:
    return sorted(directory.glob("*.deb"))


def identity(path: Path) -> tuple[str, str, str] | None:
    """The package, version and architecture apt's archive name states:
    `PACKAGE_VERSION_ARCHITECTURE.deb`, the version percent-encoded."""
    parts = path.name.removesuffix(".deb").split("_")
    if len(parts) != 3 or not all(parts):
        return None
    package, version, architecture = parts
    return package, unquote(version), architecture


def listed(names: list[tuple[str, str, str]]) -> dict[tuple[str, str, str], set[str]]:
    """Each identity's SHA-256 values in apt's index; one apt-cache call, which
    prints nothing for a version the index no longer has."""
    if not names:
        return {}
    shown = subprocess.run(
        ["apt-cache", "show", *(f"{name}={version}" for name, version, _ in names)],
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    found: dict[tuple[str, str, str], set[str]] = {}
    for record in shown.split("\n\n"):
        fields = dict(
            line.split(": ", 1)
            for line in record.splitlines()
            if ": " in line and not line.startswith(" ")
        )
        key = (
            fields.get("Package", ""),
            fields.get("Version", ""),
            fields.get("Architecture", ""),
        )
        if sha := fields.get("SHA256"):
            found.setdefault(key, set()).add(sha.strip().lower())
    return found


def verify(directory: Path) -> list[str]:
    """Delete each archive the index does not vouch for; return their names."""
    kept = {path: identity(path) for path in archives(directory)}
    index = listed([name for name in kept.values() if name])
    discarded = []
    for path, name in kept.items():
        if name is None or digest(path) not in index.get(name, set()):
            path.unlink()
            discarded.append(path.name)
    return discarded


def record(directory: Path, restored: Path) -> None:
    restored.write_text(
        json.dumps({path.name: digest(path) for path in archives(directory)})
    )


def added(directory: Path, restored: Path) -> int:
    """How many archives the directory holds that the restored set lacked: a new
    name, or a restored name whose content was replaced."""
    before = json.loads(restored.read_text())
    return sum(before.get(path.name) != digest(path) for path in archives(directory))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("verify", "record", "added"):
        command = commands.add_parser(name)
        command.add_argument("directory", type=Path)
        if name != "verify":
            command.add_argument("restored", type=Path)
    args = parser.parse_args()
    if args.command == "verify":
        for name in verify(args.directory):
            print(f"apt archive cache: discarded {name}, which the index does not list")
    elif args.command == "record":
        record(args.directory, args.restored)
    else:
        print(f"added={added(args.directory, args.restored)}")


if __name__ == "__main__":
    main()
