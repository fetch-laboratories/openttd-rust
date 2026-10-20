"""Package game runtimes and once-built scenario adapters for CI workers."""

import shutil
import subprocess

from . import core


def strip_debug(source, destination=None):
    command = ["objcopy", "--strip-debug", str(source)]
    if destination is not None:
        command.append(str(destination))
    subprocess.run(command, check=True)


def export(destination, migration):
    from . import engines, fleet, objects, orders, signals, towns

    destination.mkdir(parents=True)
    builds = {
        "reference": migration.REFERENCE_BUILD.resolve(),
        "candidate": core.ROOT / "build-rust",
    }
    for role, build in builds.items():
        lock = (
            migration.reference_lock(shared=True)
            if role == "reference"
            else migration.candidate_lock(build, shared=True)
        )
        with lock:
            source = build / ("openttd" if role == "reference" else "openttd-rust")
            runtime = destination / f"{role}-runtime"
            binary = core._copy_runtime(build, source, runtime)
            for module in (fleet, orders, engines, objects, signals, towns):
                folder = runtime / module.__name__.rsplit(".", 1)[1]
                executable = module.adapter(build, folder, migration.environment())
                for object_file in folder.glob("*.o"):
                    object_file.unlink()
                strip_debug(executable)
            strip_debug(binary)


def prepared_adapter(kind, role, builds, folder):
    source = builds[role] / kind / f"openttd-{kind}"
    if not source.is_file():
        return None
    folder.mkdir(parents=True, exist_ok=True)
    binary = folder / source.name
    shutil.copy2(source, binary)
    return binary
