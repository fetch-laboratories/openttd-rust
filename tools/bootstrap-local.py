#!/usr/bin/env python3
"""Install isolated Ubuntu build prerequisites without sudo or profile changes."""

import os
import subprocess
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


def main() -> None:
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


if __name__ == "__main__":
    main()
