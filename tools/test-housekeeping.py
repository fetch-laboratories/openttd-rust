#!/usr/bin/env python3
"""Exercise housekeeping on a fixture clone: the dry run lists exactly what --apply removes."""

import contextlib
import io
import os
import stat
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

import housekeeping

# Injected PR states; branches absent here have no PR.
PR_STATES = {
    "open": "OPEN",
    "closed": "CLOSED",
    "closed-local": "CLOSED",
    "squashed": "MERGED",
}
IDENTITY = {
    "GIT_AUTHOR_NAME": "t",
    "GIT_AUTHOR_EMAIL": "t@t",
    "GIT_COMMITTER_NAME": "t",
    "GIT_COMMITTER_EMAIL": "t@t",
}


def git(cwd: Path, *arguments: str) -> str:
    env = dict(os.environ, **IDENTITY)
    return subprocess.check_output(["git", *arguments], cwd=cwd, env=env, text=True)


def write(path: Path, text: str = "x") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def read_only(folder: Path) -> Path:
    write(folder / "file")
    folder.chmod(stat.S_IRUSR | stat.S_IXUSR)
    return folder


def pr_state(repo: Path, branch: str) -> str | None:
    return PR_STATES.get(branch)


class HousekeepingTests(unittest.TestCase):
    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.top = Path(scratch.name)
        # TemporaryDirectory cleanup cannot enter a read-only folder that survives.
        self.addCleanup(housekeeping.writable, self.top)
        origin, self.main = self.top / "origin.git", self.top / "main"
        git(self.top, "init", "--quiet", "--bare", str(origin))
        git(self.top, "init", "--quiet", "-b", "rust-migration", str(self.main))
        write(self.main / ".git/info/exclude", ".local/\n")
        git(self.main, "commit", "--quiet", "--allow-empty", "-m", "Add: base")
        git(self.main, "remote", "add", "origin", str(origin))
        git(self.main, "push", "--quiet", "-u", "origin", "rust-migration")
        self.removable: set[Path] = set()
        self.kept: set[Path] = set()

        merged = self.branch("merged")
        read_only(merged / ".local/runtime")
        self.branch("merged-dirty")
        for name in ("merged", "merged-dirty"):
            git(self.main, "merge", "--quiet", "--no-ff", "-m", "Add: merge", name)
        git(self.main, "push", "--quiet", "origin", "rust-migration")
        write(self.top / "trees/merged-dirty/stray.txt")
        self.branch("open")
        self.branch("closed")
        closed_local = self.branch("closed-local")
        git(closed_local, "commit", "--quiet", "--allow-empty", "-m", "Add: unpushed")
        self.branch("squashed")
        self.branch("fresh", commit=False)
        self.removable |= {self.top / "trees" / n for n in ("merged", "closed")}
        self.removable |= {self.top / "trees" / n for n in ("closed-local", "squashed")}
        self.kept |= {self.top / "trees" / n for n in ("merged-dirty", "open", "fresh")}
        reference = self.main / ".local/reference/openttd"
        git(self.main, "worktree", "add", "--quiet", "--detach", str(reference))
        self.kept.add(reference)
        self.fixture_local()

    def branch(self, name: str, commit: bool = True) -> Path:
        tree = self.top / "trees" / name
        git(self.main, "worktree", "add", "--quiet", "-b", name, str(tree))
        if commit:
            git(tree, "commit", "--quiet", "--allow-empty", "-m", f"Add: {name}")
            git(tree, "push", "--quiet", "-u", "origin", name)
        return tree

    def fixture_local(self) -> None:
        local = self.main / ".local"
        sim = local / "simulation"
        profile = "scenario/plain/candidate/callgrind.out"
        stale = read_only(sim / "20260101T000000Z-1/ro").parent
        write(stale / "report.json")
        write(sim / "20260102T000000Z-2/report.json")
        write(sim / "20260102T000000Z-2" / profile)
        write(sim / "20260103T000000Z-3/report.json")
        write(sim / "20260103T000000Z-3" / profile)
        write(sim / "20260104T000000Z-4/report.json")
        write(sim / "20260105T000000Z-5/scenario/log")
        interrupted = write(sim / "20260106T000000Z-6/scenario/log").parents[1]
        old = time.time() - 2 * housekeeping.RUNNING_SECONDS
        for path in (interrupted, interrupted / "scenario"):
            os.utime(path, (old, old))
        coverage = write(local / "coverage/20260101T000000Z/zero.log").parent
        os.utime(coverage, (old, old))
        os.utime(coverage / "zero.log", (old, old))
        current = f'{{"simulation_report": "{sim}/20260104T000000Z-4/report.json"}}'
        write(local / "deep/20260101T000000Z-aaaa/report.json", "{}")
        write(local / "deep/20260102T000000Z-bbbb/report.json", current)
        self.removable |= {
            stale,
            sim / "20260102T000000Z-2",
            interrupted,
            coverage,
            write(local / "verification/20260101T000000.000000Z/report.json").parent,
            local / "deep/20260101T000000Z-aaaa",
            write(local / "adhoc.log"),
            write(local / "comparison-logs/a.log").parent,
        }
        runtime = local / "tip-profile-anchored-runtime"
        write(runtime / "provenance.json", "{}")
        read_only(runtime / "runtime/usr/lib")
        self.kept |= {
            sim / "20260103T000000Z-3",
            sim / "20260104T000000Z-4",
            sim / "20260105T000000Z-5",
            local / "deep/20260102T000000Z-bbbb",
            runtime,
            write(local / "compiler-cache/entry").parent,
            write(local / "build-reference/openttd").parent,
            write(local / "cargo/env").parent,
            write(local / "rustup/settings.toml").parent,
            write(local / "deps/usr/lib").parents[1],
            write(local / "speed-reference-profiles/p.callgrind.out").parent,
            write(local / "deep.lock", ""),
            write(local / "mystery/output").parent,
        }

    def run_main(self, *arguments: str) -> list[Path]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            code = housekeeping.main([*arguments, "--repo", str(self.main)], pr_state)
        self.assertEqual(code, 0)
        lines = output.getvalue().splitlines()
        removals = [line for line in lines if line.startswith("remove ")]
        return [Path(line.split(maxsplit=2)[2].rsplit(" (", 1)[0]) for line in removals]

    def branches(self) -> set[str]:
        return set(git(self.main, "branch", "--format=%(refname:short)").split())

    def test_dry_run_lists_exactly_what_apply_removes(self) -> None:
        listed = self.run_main()
        self.assertEqual(sorted(listed), sorted(self.removable))
        for path in self.removable | self.kept:
            self.assertTrue(path.exists(), path)
        before = self.branches()

        self.assertEqual(sorted(self.run_main("--apply")), sorted(self.removable))
        for path in self.removable:
            self.assertFalse(path.exists(), path)
        for path in self.kept:
            self.assertTrue(path.exists(), path)
        # Merged and closed-on-origin branches go; unpushed and unmerged heads stay.
        self.assertEqual(before - self.branches(), {"merged", "closed"})
        worktrees = git(self.main, "worktree", "list", "--porcelain")
        for path in (p for p in self.removable if "trees" in p.parts):
            self.assertNotIn(f"worktree {path}\n", worktrees)

        self.assertEqual(self.run_main(), [])


if __name__ == "__main__":
    unittest.main()
