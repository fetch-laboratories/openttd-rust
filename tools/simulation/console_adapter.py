"""Common fixture-only console compilation and frozen-object relinking."""

import json
import re
import shlex
import shutil
import subprocess
from pathlib import Path


def console_adapter(
    build: Path,
    folder: Path,
    env: dict[str, str],
    source: Path,
    kind: str,
    wraps: tuple[str, ...] = (),
    replace_objects: tuple[str, ...] = (),
    native_test_support: bool = False,
    interpose: tuple[str, ...] = (),
) -> Path:
    """Relink a component console adapter with the frozen game's built objects.

    ``wraps`` redirects only undefined references, so it misses Rust callers that
    rustc places in the callee's codegen unit. Each ``interpose`` symbol is instead
    weakened in a copy of the native-test archive, with its body kept as
    ``__real_<symbol>``; the console's strong definition receives every call.
    """
    folder.mkdir(parents=True, exist_ok=True)
    commands = json.loads((build / "compile_commands.json").read_text())
    entry = next(item for item in commands if item["file"].endswith("/console.cpp"))
    compile_args = shlex.split(entry["command"])
    compile_args[compile_args.index("-o") + 1] = str(folder / f"{kind}.o")
    compile_args[compile_args.index("-c") + 1] = str(source)
    compile_args += ["-I" + str(Path(entry["file"]).parent), "-I" + str(folder)]
    subprocess.run(compile_args, cwd=entry["directory"], env=env, check=True)
    target = "openttd-rust" if (build / "openttd-rust").exists() else "openttd"
    command = subprocess.check_output(
        ["ninja", "-t", "commands", target], cwd=build, env=env, text=True
    ).splitlines()[-1]
    link_args = shlex.split(command.removeprefix(": && ").removesuffix(" && :"))
    binary = folder / f"openttd-{kind}"
    link_args[link_args.index("-o") + 1] = str(binary)
    link_args = [
        arg
        for arg in link_args
        if not arg.startswith("-Wl,--dependency-file=")
        and not any(arg.endswith(suffix) for suffix in replace_objects)
    ]
    if native_test_support and target == "openttd-rust":
        import migration

        archive = migration.rust_archive(build)
        support = migration.rust_archive(
            build, target_dir=build / "cargo-native-test-support"
        )
        archive_args = [arg for arg in link_args if (build / arg).resolve() == archive]
        if len(archive_args) != 1:
            raise RuntimeError("expected one production Rust archive in fixture link")
        if interpose:
            support = interposed_archive(support, folder, interpose, env)
        link_args[link_args.index(archive_args[0])] = str(support)
    elif interpose:
        raise RuntimeError("interposition needs the native-test Rust archive")
    link_args += [
        str(folder / f"{kind}.o"),
        "-Wl,--wrap=_Z22IConsoleStdLibRegisterv",
        *["-Wl,--wrap=" + symbol for symbol in wraps],
    ]
    subprocess.run(link_args, cwd=build, env=env, check=True)
    return binary


def interposed_archive(
    archive: Path, folder: Path, symbols: tuple[str, ...], env: dict[str, str]
) -> Path:
    """Copy ``archive`` with each symbol weak and its body aliased as ``__real_``."""
    copy = folder / f"interposed-{archive.name}"
    shutil.copyfile(archive, copy)
    members = folder / "interposed-members"
    shutil.rmtree(members, ignore_errors=True)
    members.mkdir()
    listing = subprocess.check_output(
        ["nm", "-A", "--defined-only", str(copy)], env=env, text=True
    )
    for symbol in symbols:
        owners = [
            line.split(":")[1]
            for line in listing.splitlines()
            if line.endswith(f" T {symbol}")
        ]
        if len(owners) != 1:
            raise RuntimeError(f"expected one Rust definition of {symbol}")
        member = members / owners[0]
        subprocess.run(
            ["ar", "x", str(copy), owners[0]], cwd=members, env=env, check=True
        )
        # Calls inside the member relocate against the global symbol, so the
        # strong console definition also replaces them.
        sections = subprocess.check_output(
            ["readelf", "-SW", str(member)], env=env, text=True
        )
        names = dict(re.findall(r"^\s*\[\s*(\d+)\]\s+(\S+)", sections, re.M))
        table = subprocess.check_output(
            ["readelf", "-sW", str(member)], env=env, text=True
        )
        rows = [
            row.split()
            for row in table.splitlines()
            if row.split()[-1:] == [symbol] and " GLOBAL " in row
        ]
        if len(rows) != 1 or rows[0][6] not in names:
            raise RuntimeError(f"unexpected {symbol} symbol in {owners[0]}")
        offset = int(rows[0][1], 16)
        subprocess.run(
            [
                "objcopy",
                f"--weaken-symbol={symbol}",
                f"--add-symbol=__real_{symbol}={names[rows[0][6]]}:{offset},global,function",
                str(member),
            ],
            env=env,
            check=True,
        )
        subprocess.run(["ar", "r", str(copy), str(member)], env=env, check=True)
    return copy


def prepared_console(kind, build_adapter, binaries, builds, folder, env):
    """Reuse a prepared console, or relink under its existing native build lock."""
    import migration

    from .core import RUNTIME_DIRECTORIES
    from .runtime import prepared_adapter

    executables = {}
    for role in ("reference", "candidate"):
        target = folder / role
        executable = prepared_adapter(kind, role, builds, target)
        if executable is None:
            reference = role == "reference" or binaries[role] == binaries["reference"]
            native = (
                migration.REFERENCE_BUILD if reference else migration.CANDIDATE_BUILD
            )
            lock = migration.candidate_lock(native, shared=True)
            if reference:
                lock = migration.reference_lock(shared=True)
            with lock:
                executable = build_adapter(native, target, env)
        executables[role] = executable
        for name in RUNTIME_DIRECTORIES:
            data = builds[role] / name
            if data.is_dir():
                (target / name).symlink_to(data, target_is_directory=True)
    return executables
