#!/usr/bin/env python3
"""Run one Linux job, recording and forwarding a received SIGTERM."""

from __future__ import annotations

import contextlib
import os
import signal
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path


def report_sender(info: signal.struct_siginfo) -> None:
    print(
        f"{datetime.now(UTC).isoformat()} SIGTERM: si_pid={info.si_pid} "
        f"si_uid={info.si_uid} si_code={info.si_code}",
        file=sys.stderr,
        flush=True,
    )
    pid = info.si_pid
    for _ in range(8):
        if pid <= 0:
            break
        try:
            with Path(f"/proc/{pid}/cmdline").open("rb") as command:
                cmdline = (
                    command.read(4096).replace(b"\0", b" ").decode(errors="replace")
                )
            print(f"  pid={pid} cmdline={cmdline!r}", file=sys.stderr, flush=True)
            pid = int(
                Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()[1]
            )
        except (OSError, ValueError, IndexError) as error:
            print(
                f"  pid={pid} /proc unavailable: {error}", file=sys.stderr, flush=True
            )
            break


def main() -> int:
    if sys.platform != "linux" or sys.version_info < (3, 11):
        sys.exit("signal-witness requires Linux and Python 3.11 or newer")
    if len(sys.argv) < 3 or sys.argv[1] != "--":
        sys.exit("usage: signal-witness.py -- command [arguments ...]")
    watched = {signal.SIGTERM, signal.SIGCHLD}
    original = signal.pthread_sigmask(signal.SIG_BLOCK, watched)

    def restore_child_mask() -> None:
        signal.pthread_sigmask(signal.SIG_SETMASK, original - watched)

    # Block before Popen so SIGTERM during creation waits until the group exists.
    child = subprocess.Popen(
        sys.argv[2:], process_group=0, preexec_fn=restore_child_mask
    )
    while True:
        info = signal.sigwaitinfo(watched)
        if info is None:
            continue
        if info.si_signo == signal.SIGTERM:
            with contextlib.suppress(OSError):
                report_sender(info)
            with contextlib.suppress(ProcessLookupError):
                os.killpg(child.pid, signal.SIGTERM)
            break
        # SIGCHLD can also announce a stop or continue; only an exit ends the job.
        if os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT):
            if signal.SIGTERM not in signal.sigpending():
                break
    code = child.wait()
    if code < 0:
        if code != -signal.SIGKILL:
            signal.signal(-code, signal.SIG_DFL)
        signal.pthread_sigmask(signal.SIG_UNBLOCK, {-code})
        os.kill(os.getpid(), -code)
    signal.pthread_sigmask(signal.SIG_SETMASK, original)
    return code


if __name__ == "__main__":
    sys.exit(main())
