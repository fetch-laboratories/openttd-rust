#!/usr/bin/env python3
"""The package archives a paid CI job keeps in the Actions cache (#896).

apt uses an archive already in its directory when the name and size are the
ones it expects, without reading its hash. So the cache is restored into a
directory apt does not read. `admit`, which apt runs as a hook after every
`apt-get update` that reaches its scripts, moves into apt's directory each
kept archive whose SHA-256 the authenticated index lists for that package,
version and architecture, and deletes the rest: a stale, damaged or foreign
file is downloaded again, never installed, and where the hook does not run
apt finds no kept archive at all. `record` and `collect` tell the job whether
it holds anything the restored set lacked, which is when it saves.
"""

import argparse
import hashlib
import json
import shutil
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


def admit(restored: Path, directory: Path) -> list[str]:
    """Move each restored archive the index vouches for into apt's directory and
    delete the others; return the names deleted."""
    kept = {path: identity(path) for path in archives(restored)}
    index = listed([name for name in kept.values() if name])
    discarded = []
    for path, name in kept.items():
        if name is not None and digest(path) in index.get(name, set()):
            shutil.move(path, directory / path.name)
        else:
            path.unlink()
            discarded.append(path.name)
    return discarded


def record(restored: Path, listing: Path) -> None:
    listing.write_text(
        json.dumps({path.name: digest(path) for path in archives(restored)})
    )


def collect(directory: Path, restored: Path, listing: Path) -> int:
    """Leave in `restored`, the directory the cache saves, exactly the archives
    apt's directory ends with. Return how many of them the restored set lacked:
    a new name, or a restored name whose content was replaced."""
    for path in archives(restored):
        path.unlink()
    for path in archives(directory):
        shutil.move(path, restored / path.name)
    before = json.loads(listing.read_text())
    return sum(before.get(path.name) != digest(path) for path in archives(restored))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name, paths in (
        ("admit", ("restored", "directory")),
        ("record", ("restored", "listing")),
        ("collect", ("directory", "restored", "listing")),
    ):
        command = commands.add_parser(name)
        for path in paths:
            command.add_argument(path, type=Path)
    args = parser.parse_args()
    if args.command == "admit":
        for name in admit(args.restored, args.directory):
            print(f"apt archive cache: discarded {name}, which the index does not list")
    elif args.command == "record":
        record(args.restored, args.listing)
    else:
        print(f"added={collect(args.directory, args.restored, args.listing)}")


if __name__ == "__main__":
    main()
