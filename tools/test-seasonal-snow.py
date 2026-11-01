#!/usr/bin/env python3
"""Seasonal matrices reject incomplete dates, duplicate keys and truncated payloads."""

import sys
import unittest
from datetime import date, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from simulation.newgrf_loading import seasonal_observations  # noqa: E402


def observations(version: int) -> list[str]:
    result: list[str] = []
    for year in (1999, 2000, 2001, 2100):
        current = date(year, 1, 1)
        while current.year == year:
            raw = ((current.month - 1) * 37 + current.day * 11) % 256
            value = 255 if version == 7 and raw >= 128 else raw
            limit = 127 if version == 7 else 255
            result.append(
                f"LOADING snow first {year} {current.month - 1} {current.day} true {value} 255 0 {version} {limit} 19 1 2 3 4"
            )
            current += timedelta(days=1)
    return result


class SeasonalSnow(unittest.TestCase):
    def test_complete_matrices(self) -> None:
        for version in (7, 8):
            valid = observations(version)
            self.assertEqual(seasonal_observations(valid, ("first",), version), valid)
            for broken in (
                [],
                valid[:-1],
                valid + [valid[0]],
                [valid[0], *valid[:-1]],
                [valid[0].rsplit(" ", 1)[0], *valid[1:]],
                [valid[0].replace(" 255 0 ", " 254 0 "), *valid[1:]],
                [valid[0].replace(" 1 2 3 4", " 1 2 3 5"), *valid[1:]],
                [valid[0].replace(" true ", " false "), *valid[1:]],
            ):
                with self.subTest(version=version), self.assertRaises(RuntimeError):
                    seasonal_observations(broken, ("first",), version)

    def test_reset_uses_live_static_height_without_a_loaded_file(self) -> None:
        valid = observations(8)
        reset = []
        for line in valid:
            fields = line.split()
            fields[2] = "reset"
            fields[6:11] = ["false", "19", "19", "19", "0"]
            reset.append(" ".join(fields))
        self.assertEqual(seasonal_observations(reset, ("reset",), 8), reset)
        with self.assertRaises(RuntimeError):
            seasonal_observations(valid, ("reset",), 8)


if __name__ == "__main__":
    unittest.main()
