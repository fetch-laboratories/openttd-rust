"""Common fixture-only console compilation and frozen-object relinking."""

import json
import shlex
import subprocess
from pathlib import Path


def console_adapter(build, folder, env, source, kind, wraps=(), replace_objects=()):
    """Relink a component console adapter with the frozen game's built objects."""
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
    link_args += [
        str(folder / f"{kind}.o"),
        "-Wl,--wrap=_Z22IConsoleStdLibRegisterv",
        *["-Wl,--wrap=" + symbol for symbol in wraps],
    ]
    subprocess.run(link_args, cwd=build, env=env, check=True)
    return binary


def prepared_console(kind, build_adapter, binaries, builds, folder, env):
    """Reuse a prepared console, or relink under its existing native build lock."""
    import migration

    from .core import ROOT, RUNTIME_DIRECTORIES
    from .runtime import prepared_adapter

    executables = {}
    for role in ("reference", "candidate"):
        target = folder / role
        executable = prepared_adapter(kind, role, builds, target)
        if executable is None:
            reference = role == "reference" or binaries[role] == binaries["reference"]
            native = migration.REFERENCE_BUILD if reference else ROOT / "build-rust"
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
