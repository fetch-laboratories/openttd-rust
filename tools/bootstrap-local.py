#!/usr/bin/env python3
"""Install isolated Ubuntu build prerequisites without sudo or profile changes.

For an Ubuntu 26.04 host without administrator access: it downloads packages
into .local/deps and the pinned Rust toolchain into .local/cargo. A host that
has the packages installed system-wide, as docs/rust-migration.md describes,
must not run it: the driver puts .local/deps ahead of the system on every
compiler, library and pkg-config path. Nothing is installed without --install.
"""

import argparse
import os
import subprocess
import sys
import tomllib
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCAL = ROOT / ".local"
PACKAGES = (
    "libsdl2-dev",
    "liblzma-dev",
    "libpng-dev",
    "libcurl4-openssl-dev",
    "libfreetype-dev",
    "libfontconfig-dev",
    "libharfbuzz-dev",
    "libicu-dev",
    "libicu78",
    "liblzo2-dev",
    "pkgconf",
    "pkgconf-bin",
    "libpkgconf7",
    "openttd-opengfx",
    "ccache",
    "libfmt10",
    "libhiredis1.1.0",
)


def install() -> None:
    downloads = LOCAL / "downloads"
    deps = LOCAL / "deps"
    downloads.mkdir(parents=True, exist_ok=True)
    deps.mkdir(parents=True, exist_ok=True)
    subprocess.run(["apt-get", "download", *PACKAGES], cwd=downloads, check=True)
    for package in sorted(downloads.glob("*.deb")):
        subprocess.run(["dpkg-deb", "-x", str(package), str(deps)], check=True)

    # Development packages link to shared libraries already installed by Ubuntu.
    # Resolve these links inside our isolated prefix without modifying the host.
    library_dir = deps / "usr/lib/x86_64-linux-gnu"
    for path in library_dir.iterdir():
        if path.is_symlink() and not path.exists():
            system = Path("/usr/lib/x86_64-linux-gnu") / path.readlink().name
            if system.exists():
                path.unlink()
                path.symlink_to(system)

    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]
    rustup = LOCAL / "cargo/bin/rustup"
    env = os.environ | {
        "CARGO_HOME": str(LOCAL / "cargo"),
        "RUSTUP_HOME": str(LOCAL / "rustup"),
    }
    if not rustup.exists():
        installer = downloads / "rustup-init.sh"
        with urllib.request.urlopen("https://sh.rustup.rs", timeout=60) as response:
            installer.write_bytes(response.read())
        subprocess.run(
            [
                "sh",
                str(installer),
                "-y",
                "--no-modify-path",
                "--profile",
                "minimal",
                "--default-toolchain",
                toolchain["channel"],
                "--component",
                "rustfmt",
                "--component",
                "clippy",
            ],
            env=env,
            check=True,
        )
    else:
        subprocess.run(
            [
                str(rustup),
                "toolchain",
                "install",
                toolchain["channel"],
                "--profile",
                "minimal",
                "--component",
                "rustfmt",
                "--component",
                "clippy",
            ],
            env=env,
            check=True,
        )
    print("Local prerequisites ready. Run: python3 tools/migration.py verify")


def main(argv: list[str]) -> int:
    # No abbreviations: a mistyped flag must not install.
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
        allow_abbrev=False,
    )
    parser.add_argument(
        "--install",
        action="store_true",
        help="download and install into .local/deps, .local/cargo and .local/rustup",
    )
    args = parser.parse_args(argv)
    if not args.install:
        parser.print_help()
        print("\nNothing installed: pass --install to install.")
        return 2
    install()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
