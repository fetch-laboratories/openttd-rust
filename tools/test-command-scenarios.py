#!/usr/bin/env python3
"""Reject absent command activity and preserve every outcome in reports."""

import copy
import sys
import unittest
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
from simulation import commands  # noqa: E402


def log() -> list[str]:
    names = [
        "rail.build",
        "road.build",
        "station.bus",
        "order.move",
        "vehicle.start-stop",
        "terraform.raise",
        "rail.build",
        "road.build",
        "station.bus",
        "order.skip",
        "vehicle.reverse",
        "terraform.lower",
        "rail.remove",
        "road.remove",
        "station.rail",
        "order.flags",
        "vehicle.depot",
        "terraform.clear",
        "rail.convert",
        "road.convert",
        "station.truck",
        "order.remove",
        "vehicle.refit",
        "terraform.level",
    ]
    state = 1
    lines = ["COMMAND-BEGIN 1 24 10 100"]
    for i, command in enumerate(names):
        for _ in range(9 if i % 6 == 0 else 8):
            state = state * 48271 % 2147483647
        ok = i < 6 or command in ("order.skip", "order.flags")
        lines.append(
            f"COMMAND {i} {command} 100,101 {str(ok).lower()} {'OK' if ok else 'ERR_PRECONDITION_FAILED'}"
        )
    return [*lines, f"COMMAND-END 24 {state}"]


class Commands(unittest.TestCase):
    def test_required_activity(self) -> None:
        valid = log()
        self.assertEqual(len(commands.outcomes(valid, 24, 1)), 24)
        for broken in (
            [],
            valid[:-1],
            valid[:5] + valid[6:],
            valid + [valid[1]],
            [line.replace("ERR_PRECONDITION_FAILED", "OK") for line in valid],
            [line.replace("order.flags", "order.skip") for line in valid],
            [line.replace("station.bus", "road.build") for line in valid],
            [line.replace("COMMAND-BEGIN 1", "COMMAND-BEGIN 2") for line in valid],
            [line.replace("COMMAND-END 24", "COMMAND-END 23") for line in valid],
        ):
            with self.subTest(log=broken), self.assertRaises(ValueError):
                commands.outcomes(broken, 24, 1)

    def test_histograms_and_full_comparison(self) -> None:
        case = {"command_seed": 1, "command_count": 24}
        result: dict[str, Any] = {"problems": []}
        run = {"log": log()}
        commands.check(case, run, "plain", "reference", result)
        commands.check(case, run, "plain", "candidate", result)
        histogram = result["plain_candidate_commands"]["histogram"]
        self.assertEqual(
            histogram["rail.build"], {"OK": 1, "ERR_PRECONDITION_FAILED": 1}
        )
        self.assertEqual(result["problems"], [])
        for old, new in (
            ("100,101", "100,102"),
            ("ERR_PRECONDITION_FAILED", "ERR_UNKNOWN"),
            (" true ", " false "),
        ):
            changed = copy.deepcopy(run)
            changed["log"] = [line.replace(old, new) for line in changed["log"]]
            commands.check(case, changed, "plain", "candidate", result)
        self.assertEqual(len(result["problems"]), 3)


if __name__ == "__main__":
    unittest.main()
