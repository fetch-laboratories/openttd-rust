#!/usr/bin/env python3
"""Reject incomplete mixed-network evidence using real save decoding/comparison."""

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from simulation import core, multiplayer

SERVER = """MIXED_READY
client[2] Receive_CLIENT_ACK(): frame=24
client[2] status = ACTIVE
client[2] SendSync(), frame_counter=24, sync_seed_1=10
client[2] SendSync(), frame_counter=124, sync_seed_1=20
client[2] Receive_CLIENT_QUIT()
"""
CLIENT = """MIXED_FIXTURE revision 15.3 tagged 1 newgrf 523791716
using thread for game-loop
Client::status = ACTIVE
Client::Receive_SERVER_SYNC(): sync_frame=24, sync_seed_1=10
Client::Receive_SERVER_SYNC(): sync_frame=124, sync_seed_1=20
Client::SendQuit()
"""


class MultiplayerTests(unittest.TestCase):
    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.source, _ = core.normalize(multiplayer.SOURCE, self.root / "input")
        self.stop = self.root / "stop.sav"
        core.patch(
            self.source,
            self.stop,
            {
                "DATE": {
                    0: {
                        "date": multiplayer.STOP[0],
                        "date_fract": 0,
                        "economy_date": multiplayer.STOP[0] - 1,
                        "economy_date_fract": 73,
                        "tick_counter": multiplayer.STOP[2],
                        "random_state[0]": 100,
                        "random_state[1]": 200,
                    }
                }
            },
        )
        self.session = multiplayer.Session(
            "reference", str(self.root / "session"), exits={"server": 0, "client": 0}
        )
        for side, log in (("server", SERVER), ("client", CLIENT)):
            folder = Path(self.session.directory) / side
            (folder / "save").mkdir(parents=True)
            (folder / "stderr.log").write_text(log)
            (folder / "console.log").write_text("MIXED_STOP\n")
            shutil.copy2(self.stop, folder / "save/stop.sav")

    def test_missing_protocol_exit_identity_and_disconnect_fail(self) -> None:
        multiplayer.check(self.session, self.source)
        for side, before, after in (
            ("server", "MIXED_READY", ""),
            ("server", "status = ACTIVE", "status = AUTHORIZED"),
            ("server", "Receive_CLIENT_QUIT()", ""),
            ("client", "Client::SendQuit()", ""),
            ("client", "using thread", "using no thread"),
            ("client", "MIXED_FIXTURE", "missing identity"),
            ("client", "sync_frame=124", "sync_frame=125"),
            ("client", "sync_seed_1=20", "sync_seed_1=21"),
            ("client", "Client::SendQuit()", "Sync error detected"),
            ("server", "Receive_CLIENT_QUIT()", "SendError(): error=8"),
        ):
            path = Path(self.session.directory) / side / "stderr.log"
            old = path.read_text()
            path.write_text(old.replace(before, after))
            with self.subTest(before=before), self.assertRaises(RuntimeError):
                multiplayer.check(self.session, self.source)
            path.write_text(old)
        for code in (-9, 1):
            self.session.exits["client"] = code
            with self.assertRaises(RuntimeError):
                multiplayer.check(self.session, self.source)
        self.session.exits.clear()
        with self.assertRaises(RuntimeError):
            multiplayer.check(self.session, self.source)
        self.session.exits = {"server": 0, "client": 0}
        for side in ("server", "client"):
            path = Path(self.session.directory) / side / "stderr.log"
            old = path.read_text()
            path.write_text(
                "\n".join(line for line in old.splitlines() if "sync_" not in line)
            )
            with self.assertRaises(RuntimeError):
                multiplayer.check(self.session, self.source)
            path.write_text(old)

    def test_missing_save_phase_and_rng_progress_fail(self) -> None:
        path = Path(self.session.directory) / "client/save/stop.sav"
        path.unlink()
        with self.assertRaises(RuntimeError):
            multiplayer.check(self.session, self.source)
        for fields in (
            {"tick_counter": multiplayer.STOP[2] - 1},
            {"random_state[1]": 999},
            {"date_fract": 1},
        ):
            core.patch(self.stop, path, {"DATE": {0: fields}})
            with self.subTest(fields=fields), self.assertRaises(RuntimeError):
                multiplayer.check(self.session, self.source)
        for side in ("server", "client"):
            path = Path(self.session.directory) / side / "save/stop.sav"
            fields = {
                f"random_state[{i}]": core.rows(self.source, "DATE")[0][
                    f"random_state[{i}]"
                ]
                for i in (0, 1)
            }
            core.patch(self.stop, path, {"DATE": {0: fields}})
        with self.assertRaises(RuntimeError):
            multiplayer.check(self.session, self.source)

    def test_received_sync_that_skips_rng_comparison_fails(self) -> None:
        path = Path(self.session.directory) / "client/stderr.log"
        path.write_text(CLIENT + "Missed frame for sync-test: 124 / 125\n")
        with self.assertRaises(RuntimeError):
            multiplayer.check(self.session, self.source)

    def test_swapped_sessions_compare_every_chunk_including_aipl(self) -> None:
        other = self.root / "other-session"
        shutil.copytree(self.session.directory, other)
        result = multiplayer.Result(
            sessions=[self.session, multiplayer.Session("candidate", str(other))]
        )
        multiplayer.compare(result)
        self.assertTrue(result.passed)
        self.assertEqual(set(result.comparisons), {"server", "client"})
        path = other / "client/save/stop.sav"
        path.write_bytes(
            core.set_ai_slots(path.read_bytes(), {0: core.ai_config(b"Different")})
        )
        multiplayer.compare(result)
        self.assertFalse(result.passed)
        self.assertEqual(result.differences[0]["chunk"], "AIPL")
        with self.assertRaises(RuntimeError):
            multiplayer.compare(
                multiplayer.Result(sessions=[self.session, self.session])
            )

    def test_revision_fixture_changes_only_exact_identity_constants(self) -> None:
        candidate = (
            'const std::string _openttd_revision = "fork";\n'
            "const uint8_t _openttd_revision_tagged = 0;\n"
            "const uint32_t _openttd_newgrf_version = 10;\n"
            'const std::string_view _openttd_revision_hash = "candidate";\n'
        )
        reference = (
            'const std::string _openttd_revision = "original";\n'
            "const uint8_t _openttd_revision_tagged = 1;\n"
            "const uint32_t _openttd_newgrf_version = 20;\n"
        )
        self.assertEqual(
            multiplayer.fixture_revision(candidate, reference),
            reference
            + 'const std::string_view _openttd_revision_hash = "candidate";\n',
        )
        for invalid in (
            reference.replace("_openttd_revision_tagged", "absent"),
            reference + reference,
        ):
            with self.assertRaises(RuntimeError):
                multiplayer.fixture_revision(candidate, invalid)
        build = self.root / "build"
        obj = build / "CMakeFiles/openttd_lib.dir/game.o"
        obj.parent.mkdir(parents=True)
        obj.write_bytes(b"game object")
        (build / "compile_commands.json").write_text(json.dumps([{"output": str(obj)}]))
        before = multiplayer.game_objects(build, False)
        obj.write_bytes(b"changed object")
        self.assertNotEqual(before, multiplayer.game_objects(build, False))


if __name__ == "__main__":
    unittest.main()
