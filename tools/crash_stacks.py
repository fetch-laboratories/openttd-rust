#!/usr/bin/env python3
"""Run a test binary as Cargo's runner; on a fatal signal, print every thread's stack.

`tools/migration.py rust-checks` passes this script to Cargo as the runner of
Linux test binaries (#1012). The binary runs unchanged with the core-size
limit raised, and its output and exit status pass through. When it dies by a
signal that writes a core, the script reads that core with
`gdb -batch -ex 'thread apply all bt'`, prints the stacks and writes them in
full to CRASH_STACKS_DIR, so a rare crash is diagnosable from its own log. The
binary is never run under a debugger: that would change the timing it watches.

On CI only (GITHUB_ACTIONS), the script points kernel.core_pattern at a folder
it creates and installs gdb after a crash when it is absent, both with
`sudo -n`. Elsewhere it changes no system setting: it reads a core only where
the host's pattern already writes a file, and otherwise prints the command that
reruns the binary under gdb. It never turns a crash into a pass or a pass into
a failure.
"""

import os
import resource
import shutil
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

# Signals whose default action writes a core.
CORE_SIGNALS = frozenset(
    {
        signal.SIGSEGV,
        signal.SIGABRT,
        signal.SIGBUS,
        signal.SIGILL,
        signal.SIGFPE,
        signal.SIGQUIT,
        signal.SIGTRAP,
        signal.SIGSYS,
        signal.SIGXCPU,
        signal.SIGXFSZ,
    }
)
PATTERN = Path("/proc/sys/kernel/core_pattern")
TRACE_DIR = "CRASH_STACKS_DIR"
DEBUGGER = ("gdb", "-batch", "-nx", "-ex", "thread apply all bt")
INSTALL_TIMEOUT = 600
DEBUG_TIMEOUT = 600


def say(text: str) -> None:
    print(f"crash_stacks: {text}", file=sys.stderr, flush=True)


def sudo(*command: str, timeout: int = 60) -> bool:
    """Run a command with non-interactive sudo; whether it succeeded."""
    try:
        done = subprocess.run(
            ["sudo", "-n", *command], capture_output=True, text=True, timeout=timeout
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        say(f"sudo {command[0]} failed: {error}")
        return False
    if done.returncode:
        say(f"sudo {command[0]} failed: {(done.stderr or done.stdout).strip()}")
    return done.returncode == 0


def on_ci() -> bool:
    return bool(os.environ.get("GITHUB_ACTIONS"))


def prepare_cores() -> Path | None:
    """On CI, a fresh folder the kernel writes cores to; otherwise None."""
    if not on_ci():
        return None
    folder = Path(tempfile.mkdtemp(prefix="crash-stacks-"))
    folder.chmod(0o1777)
    if not sudo("sysctl", "-w", f"kernel.core_pattern={folder}/core.%p"):
        say("kernel.core_pattern unchanged; a crash will have no core")
    return folder


def raise_core_limit() -> None:
    _, hard = resource.getrlimit(resource.RLIMIT_CORE)
    resource.setrlimit(resource.RLIMIT_CORE, (hard, hard))


def core_file(folder: Path | None, pid: int, cwd: Path) -> Path | None:
    """The crashed process's core: in our folder on CI, else where the host's
    pattern writes a plain file. A piped pattern (apport, systemd) has none."""
    if folder is not None:
        found = folder / f"core.{pid}"
        return found if found.is_file() else None
    try:
        pattern = PATTERN.read_text().strip()
    except OSError:
        return None
    if not pattern or pattern.startswith("|"):
        return None
    name = pattern.replace("%p", str(pid)).replace("%%", "%")
    if "%" in name:
        return None  # Other specifiers name a file this script cannot predict.
    path = Path(name) if name.startswith("/") else cwd / name
    for candidate in (path, path.with_name(f"{path.name}.{pid}")):
        if candidate.is_file():
            return candidate
    return None


def debugger() -> str | None:
    """gdb, installed on CI after a crash when it is absent."""
    if found := shutil.which("gdb"):
        return found
    if not on_ci():
        return None
    say("gdb is absent; installing it")
    install = ("apt-get", "install", "-y", "--no-install-recommends", "gdb")
    if not sudo(*install, timeout=INSTALL_TIMEOUT):
        sudo("apt-get", "update", timeout=INSTALL_TIMEOUT)
        sudo(*install, timeout=INSTALL_TIMEOUT)
    return shutil.which("gdb")


def trace_path(binary: str, pid: int) -> Path:
    folder = Path(os.environ.get(TRACE_DIR) or tempfile.gettempdir())
    folder.mkdir(parents=True, exist_ok=True)
    return folder / f"{Path(binary).name}-{pid}.txt"


def stacks(binary: str, core: Path, gdb: str) -> str:
    try:
        done = subprocess.run(
            [gdb, *DEBUGGER[1:], binary, str(core)],
            capture_output=True,
            text=True,
            errors="replace",
            timeout=DEBUG_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        return f"gdb did not finish within {DEBUG_TIMEOUT} seconds\n"
    return done.stdout + done.stderr


def report(command: list[str], pid: int, number: int, folder: Path | None) -> None:
    """Print the signal and, from the core when there is one, every stack."""
    name = signal.Signals(number).name
    binary = command[0]
    say(f"{binary} (pid {pid}) died by {name}")
    rerun = " ".join(
        [*DEBUGGER[:3], "-ex", "run", "-ex", "'thread apply all bt'", "--args"]
        + command
    )
    if number not in CORE_SIGNALS:
        say(f"{name} writes no core, so there are no stacks")
        return
    core = core_file(folder, pid, Path.cwd())
    if core is None:
        say(f"no core file was written; to rerun under gdb: {rerun}")
        return
    gdb = debugger()
    if gdb is None:
        say(f"gdb is not available to read {core}; to rerun under gdb: {rerun}")
        return
    text = stacks(binary, core, gdb)
    path = trace_path(binary, pid)
    path.write_text(f"{binary} (pid {pid}) died by {name}\n{text}")
    print(text, file=sys.stderr, flush=True)
    say(f"every thread's stack is in {path}")


def run(command: list[str]) -> int:
    folder = prepare_cores()
    try:
        process = subprocess.Popen(command, preexec_fn=raise_core_limit)
        code = process.wait()
        if code < 0:
            try:
                report(command, process.pid, -code, folder)
            except Exception as error:  # The crash's status stands regardless.
                say(f"reading the stacks failed: {error}")
            return 128 - code
        return code
    finally:
        if folder is not None:
            shutil.rmtree(folder, ignore_errors=True)


def main() -> int:
    if len(sys.argv) < 2:
        say("usage: crash_stacks.py BINARY [ARGUMENTS...]")
        return 2
    return run(sys.argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
