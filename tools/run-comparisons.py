#!/usr/bin/env python3
"""Run every reference comparison tool in parallel after a candidate build.

Tools are discovered as tools/*-comparison.py plus tools/compare-integers.py, so
adding a comparison never requires a workflow edit. Each tool keeps its own
scratch directory under .local/; their logs go to .local/comparison-logs/.
Each passing tool's case inventory (its "passed" and "cases" lines) is printed.
"""

import argparse
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from validation_execution import run_logged

ROOT = Path(__file__).resolve().parents[1]
LOGS = ROOT / ".local/comparison-logs"
TIMEOUT_SECONDS = 1200
# Summary lines; compiler diagnostics start with a path, file:line or space.
INVENTORY = re.compile(r"^(?![^\s:]+:\d+:)[A-Za-z].*(?:\bpassed\b|cases: \d+$)")


def discover():
    tools = sorted(ROOT.glob("tools/*-comparison.py"))
    tools.append(ROOT / "tools/compare-integers.py")
    return [tool for tool in tools if tool.is_file()]


def run(tool):
    log = LOGS / f"{tool.stem}.log"
    start = time.monotonic()
    try:
        code = run_logged(
            [sys.executable, str(tool)],
            cwd=ROOT,
            env=None,
            log=log,
            phase=tool.name,
            timeout=TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        with log.open("a") as output:
            output.write(f"\nTimed out after {TIMEOUT_SECONDS} seconds\n")
        code = -1
    return tool, code, time.monotonic() - start, log


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jobs", type=int, default=2)
    parser.add_argument(
        "names", nargs="*", help="Only run tools whose file name contains one of these"
    )
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    tools = [
        tool
        for tool in discover()
        if not args.names or any(name in tool.name for name in args.names)
    ]
    if not tools:
        sys.exit("No comparison tools selected")
    LOGS.mkdir(parents=True, exist_ok=True)
    failed = []
    with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        futures = [pool.submit(run, tool) for tool in tools]
        for future in as_completed(futures):
            tool, code, seconds, log = future.result()
            print(
                f"{'ok  ' if code == 0 else 'FAIL'} {seconds:6.1f}s  {tool.name}",
                flush=True,
            )
            if code:
                failed.append(log)
            else:
                for line in log.read_text(errors="replace").splitlines():
                    if INVENTORY.match(line):
                        print(f"      {line}", flush=True)
    for log in failed:
        print(f"\n===== {log.relative_to(ROOT)} (last 80 lines) =====")
        print("\n".join(log.read_text(errors="replace").splitlines()[-80:]))
    if failed:
        sys.exit(f"{len(failed)} of {len(tools)} comparisons failed")
    print(f"All {len(tools)} comparisons passed")


if __name__ == "__main__":
    main()
