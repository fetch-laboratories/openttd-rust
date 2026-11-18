#!/usr/bin/env python3
"""Bound every wait of a network-protocol session and retry only the #693 stall."""

import contextlib
import io
import os
import subprocess
import sys
import tempfile
import time
import unittest
from collections.abc import Iterator
from pathlib import Path
from unittest.mock import patch

from simulation import network_protocol
from simulation.network_protocol import Peer, StepExpired

# A stub peer: prints its lines, then sleeps and ignores every trigger.
STUB = "import sys, time\nprint(sys.argv[1], flush=True)\ntime.sleep(60)\n"
RESOLVED = (
    "dbg: [net:6] 127.0.0.2:3979 resolved in:\ndbg: [net:6] - 127.0.0.2:3979 (IPv4)"
)
CONNECTED = RESOLVED + "\ndbg: [net:5] Attempting to connect to 127.0.0.2:3979"


class StubPeers:
    """Stub processes as peers, each in its own folder, with a short step."""

    def __init__(self, root: Path, step: int = 1) -> None:
        self.root = root
        self.timeout = step
        self.peers: list[Peer] = []

    def start(self, name: str, text: str, command: list[str] | None = None) -> Peer:
        folder = self.root / name
        folder.mkdir()
        process = subprocess.Popen(
            command or [sys.executable, "-c", STUB, text],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        peer = Peer(process, folder, self.timeout)
        self.peers.append(peer)
        return peer

    def close(self) -> None:
        for peer in self.peers:
            if peer.process.poll() is None:
                peer.process.kill()
            peer.process.wait()
            if peer.process.stdin is not None:
                peer.process.stdin.close()
            peer.reader.join(5)


@contextlib.contextmanager
def stubs(step: int = 1) -> Iterator[StubPeers]:
    with tempfile.TemporaryDirectory() as directory:
        found = StubPeers(Path(directory), step)
        try:
            yield found
        finally:
            found.close()


class StepLimitTests(unittest.TestCase):
    def test_a_missing_marker_fails_within_the_step_limit(self) -> None:
        # The per-game timeout is 1,200 seconds; the wait ends at the step limit.
        with stubs() as peers, patch.object(network_protocol, "STEP_LIMIT", 1):
            peers.timeout = 1200
            issuer = peers.start("issuer", "status = CONNECTING")
            started = time.monotonic()
            with self.assertRaises(StepExpired) as raised:
                issuer.wait("NET574 ready issuer", 2)
            self.assertLess(time.monotonic() - started, 10)
        message = str(raised.exception)
        for part in ("issuer", "NET574 ready issuer of 2", "status = CONNECTING"):
            self.assertIn(part, message)

    def test_the_signature_is_a_resolve_with_no_later_attempt(self) -> None:
        stuck = network_protocol.stuck_resolving
        self.assertTrue(stuck(RESOLVED.splitlines()))
        self.assertFalse(stuck(CONNECTED.splitlines()))
        self.assertTrue(stuck([*CONNECTED.splitlines(), *RESOLVED.splitlines()]))
        self.assertFalse(stuck(["status = CONNECTING"]))

    def test_a_joiner_that_has_exited_does_not_block_its_release(self) -> None:
        with stubs() as peers:
            joiner = peers.start("joiner", "gone", [sys.executable, "-c", "pass"])
            os.mkfifo(joiner.folder / "map-release")
            joiner.process.wait()
            started = time.monotonic()
            with self.assertRaisesRegex(StepExpired, "joiner: map release not read"):
                joiner.release()
            self.assertLess(time.monotonic() - started, 10)

    def test_a_peer_that_ignores_its_exit_trigger_fails_within_the_step(self) -> None:
        with stubs() as peers:
            issuer = peers.start("issuer", "NET574 saved x")
            with self.assertRaisesRegex(StepExpired, "issuer: no exit within 1 s"):
                issuer.finish()

    def test_a_log_held_open_after_kill_is_reported_not_awaited(self) -> None:
        # A child keeps the pipe open after its parent is killed.
        held = [
            sys.executable,
            "-c",
            "import subprocess, sys, time\n"
            "subprocess.Popen(['sleep', '5'])\nprint('up', flush=True)\ntime.sleep(60)",
        ]
        with stubs() as peers:
            peer = peers.start("joiner", "", held)
            peer.wait("up")
            started = time.monotonic()
            with self.assertRaisesRegex(StepExpired, "joiner: log still open"):
                network_protocol.clean_up([peer])
            self.assertLess(time.monotonic() - started, 4)


class SessionTests(unittest.TestCase):
    def session(self, issuer_text: str) -> tuple[Exception, float, list[Peer]]:
        """A session whose server starts and whose issuer never connects."""
        texts = {"server": "NET574 ready server\nNET574 allowed", "issuer": issuer_text}
        with stubs() as peers:

            def start(*args: object) -> Peer:
                folder = Path(str(args[1]))
                return peers.start(folder.name, texts[folder.name])

            started = time.monotonic()
            with (
                patch.object(network_protocol, "start", side_effect=start),
                patch.object(
                    network_protocol.core, "game_slot", contextlib.nullcontext
                ),
                self.assertRaises(RuntimeError) as raised,
            ):
                network_protocol.session(
                    {"reference": Path("r"), "candidate": Path("c")},
                    Path("source.sav"),
                    peers.root,
                    "reference",
                    1,
                    {},
                )
            elapsed = time.monotonic() - started
            return raised.exception, elapsed, peers.peers

    def test_a_peer_that_never_connects_ends_the_session_cleanup_included(
        self,
    ) -> None:
        error, elapsed, peers = self.session(RESOLVED)
        self.assertIsInstance(error, network_protocol.Stalled)
        self.assertIn("issuer: missing NET574 ready issuer of 1", str(error))
        self.assertIn("TCPConnecter stuck in Resolving (#693)", str(error))
        self.assertLess(elapsed, 10)
        self.assertTrue(all(peer.process.poll() is not None for peer in peers))
        # Without the signature the error is the plain missing marker.
        error, _, _ = self.session(CONNECTED)
        self.assertNotIsInstance(error, network_protocol.Stalled)
        self.assertIn("missing NET574 ready issuer", str(error))


class RetryTests(unittest.TestCase):
    def sessions(
        self, outcomes: list[str], differs: str | None = None
    ) -> tuple[network_protocol.Result, list[str], str, Exception | None]:
        """Run both roles' sessions; each outcome is pass, stall or fail."""
        steps = iter(outcomes)
        folders: list[str] = []

        def session(*args: object) -> tuple[Path, dict[str, int]]:
            folders.append(Path(str(args[2])).name)
            outcome = next(steps)
            if outcome == "stall":
                raise network_protocol.Stalled("issuer: stuck (#693)", "issuer")
            if outcome == "fail":
                raise RuntimeError("issuer: missing NET574 ready issuer")
            return Path("stop.sav"), {"callbacks": 24}

        result: network_protocol.Result = {}
        output = io.StringIO()
        error = None
        with (
            patch.object(network_protocol, "session", side_effect=session),
            patch.object(network_protocol, "connecter_differs", return_value=differs),
            contextlib.redirect_stdout(output),
        ):
            try:
                network_protocol.sessions({}, Path("s"), Path("f"), 1200, {}, result)
            except RuntimeError as raised:
                error = raised
        return result, folders, output.getvalue(), error

    def test_a_stall_is_retried_once_only_while_the_source_is_the_pins(self) -> None:
        result, folders, output, error = self.sessions(["stall", "pass", "pass"])
        self.assertIsNone(error)
        self.assertEqual(folders, ["reference", "reference-retry", "candidate"])
        self.assertEqual(result["network_protocol_retries"], 1)
        (warning,) = output.splitlines()
        self.assertTrue(warning.startswith("::warning::network-protocol-loopback: "))
        self.assertIn("issuer", warning)
        self.assertIn("#693", warning)
        reason = "src/thread.h differs from the pinned original's"
        _, folders, output, error = self.sessions(["stall"], differs=reason)
        self.assertIn(f"no retry: {reason}", str(error))
        self.assertEqual((folders, output), (["reference"], ""))

    def test_a_second_stall_or_a_failed_retry_fails(self) -> None:
        for outcomes, folders in (
            (["stall", "pass", "stall"], ["reference", "reference-retry", "candidate"]),
            (["pass", "stall", "stall"], ["reference", "candidate", "candidate-retry"]),
            (["stall", "fail"], ["reference", "reference-retry"]),
        ):
            with self.subTest(outcomes=outcomes):
                result, seen, _, error = self.sessions(outcomes)
                self.assertIsNotNone(error)
                self.assertEqual(seen, folders)
                self.assertEqual(result["network_protocol_retries"], 1)

    def test_no_retry_without_the_signature_and_zero_reported(self) -> None:
        result, _, output, error = self.sessions(["pass", "pass"])
        self.assertIsNone(error)
        self.assertEqual((result["network_protocol_retries"], output), (0, ""))
        _, folders, _, error = self.sessions(["fail"])
        self.assertIsNotNone(error)
        self.assertEqual(folders, ["reference"])

    def test_the_race_files_are_compared_with_the_pinned_original(self) -> None:
        self.assertIsNone(network_protocol.connecter_differs())
        for files, reason in (
            (("README.md",), "README.md differs"),
            (("tools/ci.py",), "tools/ci.py could not be read"),
        ):
            with patch.object(network_protocol, "RACE_FILES", files):
                self.assertIn(reason, network_protocol.connecter_differs() or "")


if __name__ == "__main__":
    unittest.main()
