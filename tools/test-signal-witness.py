#!/usr/bin/env python3
"""Exercise sender attribution, launch races and owned job termination on Linux."""

import contextlib
import json
import os
import select
import signal
import subprocess
import sys
import unittest
from pathlib import Path

WRAPPER = Path(__file__).with_name("signal-witness.py")


@unittest.skipUnless(sys.platform == "linux", "Linux signal sender information")
class SignalWitnessTests(unittest.TestCase):
    def launch(self, script: str) -> subprocess.Popen[bytes]:
        process = subprocess.Popen(
            [sys.executable, str(WRAPPER), "--", sys.executable, "-c", script],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.addCleanup(self.cleanup, process)
        return process

    def cleanup(self, process: subprocess.Popen[bytes]) -> None:
        if process.returncode is None:
            process.terminate()
        try:
            process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate(timeout=5)

    def read_ready(self, fd: int) -> bytes:
        self.assertTrue(select.select([fd], [], [], 5)[0], "child readiness timed out")
        data = os.read(fd, 4096)
        self.assertTrue(data, "child exited before readiness")
        return data

    def kill_group(self, pid: int) -> None:
        with contextlib.suppress(ProcessLookupError):
            os.killpg(pid, signal.SIGKILL)

    def test_normal_and_nonzero_exit(self) -> None:
        for code in (0, 7):
            with self.subTest(code=code):
                process = self.launch(f"print('output'); raise SystemExit({code})")
                output, errors = process.communicate(timeout=5)
                self.assertEqual(process.returncode, code)
                self.assertEqual(output, b"output\n")
                self.assertEqual(errors, b"")

    def test_child_signal_outcome(self) -> None:
        for signum in (signal.SIGUSR1, signal.SIGKILL):
            with self.subTest(signal=signum):
                process = self.launch(f"import os; os.kill(os.getpid(),{signum})")
                process.communicate(timeout=5)
                self.assertEqual(process.returncode, -signum)

    def test_sender_and_group_termination_reap_both_generations(self) -> None:
        script = """
import json,os,signal,subprocess,sys
grandchild = subprocess.Popen(
    [sys.executable, '-c', "import signal; print('ready',flush=True); signal.pause()"],
    stdout=subprocess.PIPE)
assert grandchild.stdout.readline() == b'ready\\n'
def terminate(signum, frame):
    assert grandchild.wait(timeout=3) == -signal.SIGTERM
    signal.signal(signal.SIGTERM, signal.SIG_DFL)
    os.kill(os.getpid(), signal.SIGTERM)
signal.signal(signal.SIGTERM, terminate)
print(json.dumps([os.getpid(), grandchild.pid, os.getpgrp(), os.getsid(0),
                  list(signal.pthread_sigmask(signal.SIG_BLOCK, set()))]), flush=True)
signal.pause()
"""
        process = self.launch(script)
        assert process.stdout is not None
        child, grandchild, group, session, blocked = json.loads(
            self.read_ready(process.stdout.fileno())
        )
        self.addCleanup(self.kill_group, child)
        self.assertEqual(group, child)
        self.assertNotEqual(group, os.getpgid(process.pid))
        self.assertEqual(session, os.getsid(process.pid))
        self.assertNotIn(signal.SIGTERM, blocked)
        self.assertNotIn(signal.SIGCHLD, blocked)
        os.kill(process.pid, signal.SIGTERM)
        _, errors = process.communicate(timeout=5)
        self.assertEqual(process.returncode, -signal.SIGTERM)
        self.assertIn(
            f"si_pid={os.getpid()} si_uid={os.getuid()} si_code=0", errors.decode()
        )
        self.assertIn(f"pid={os.getpid()} cmdline=", errors.decode())
        self.assertIn(Path(__file__).name, errors.decode())
        self.assertRegex(errors.decode(), r"\d{4}-\d\d-\d\dT.*\+00:00 SIGTERM:")
        self.assertFalse(Path(f"/proc/{child}").exists(), "child was not reaped")
        self.assertFalse(
            Path(f"/proc/{grandchild}").exists(), "grandchild was not reaped"
        )

    def test_sigterm_during_process_creation_is_forwarded(self) -> None:
        ready_read, ready_write = os.pipe()
        gate_read, gate_write = os.pipe()
        for fd in (ready_read, ready_write, gate_read, gate_write):
            self.addCleanup(os.close, fd)
        # Hold Popen before fork so the test sender can hit the creation window.
        launcher = f"""
import os,runpy,subprocess,sys
original = subprocess.Popen
def held(*args, **kwargs):
    os.write({ready_write}, b'ready')
    assert os.read({gate_read}, 1) == b'!'
    return original(*args, **kwargs)
subprocess.Popen = held
sys.argv = [{str(WRAPPER)!r}, '--', sys.executable, '-c', 'import signal; signal.pause()']
runpy.run_path({str(WRAPPER)!r}, run_name='__main__')
"""
        process = subprocess.Popen(
            [sys.executable, "-c", launcher],
            pass_fds=(ready_write, gate_read),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.addCleanup(self.cleanup, process)
        self.assertEqual(self.read_ready(ready_read), b"ready")
        os.kill(process.pid, signal.SIGTERM)
        os.write(gate_write, b"!")
        _, errors = process.communicate(timeout=5)
        self.assertEqual(process.returncode, -signal.SIGTERM)
        self.assertIn(f"si_pid={os.getpid()} ", errors.decode())


if __name__ == "__main__":
    unittest.main()
