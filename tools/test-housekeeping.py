#!/usr/bin/env python3
"""Exercise housekeeping on a fixture clone: the dry run lists exactly what --apply removes."""

import contextlib
import io
import json
import os
import stat
import subprocess
import sys
import tempfile
import time
import unittest
from collections.abc import Generator
from pathlib import Path
from unittest.mock import patch

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


@contextlib.contextmanager
def held_output(
    path: Path, unreadable: bool = False
) -> Generator[subprocess.Popen[str]]:
    """The pipe confirms open before planning and keeps the descriptor live until close."""
    child = subprocess.Popen(
        [
            sys.executable,
            "-c",
            """
import ctypes, sys
with open(sys.argv[1], "a") as output:
    if sys.argv[2] == "unreadable":
        assert ctypes.CDLL(None).prctl(4, 0, 0, 0, 0) == 0
    print("ready", flush=True)
    for command in sys.stdin:
        if command.strip() == "close":
            break
        output.write("visible\\n")
        output.flush()
        print("written", flush=True)
""",
            str(path),
            "unreadable" if unreadable else "readable",
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert child.stdout is not None
        assert child.stdout.readline() == "ready\n"
        yield child
    finally:
        if child.stdin is not None:
            child.stdin.close()
        child.wait(timeout=10)
        if child.stdout is not None:
            child.stdout.close()


def child_write(child: subprocess.Popen[str]) -> None:
    assert child.stdin is not None and child.stdout is not None
    child.stdin.write("write\n")
    child.stdin.flush()
    assert child.stdout.readline() == "written\n"


class HousekeepingTests(unittest.TestCase):
    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.top = Path(scratch.name)
        self.proc = self.top / "proc"
        self.proc.mkdir()
        (self.proc / str(os.getpid())).symlink_to(Path("/proc") / str(os.getpid()))
        proc_patch = patch.object(housekeeping, "PROC_ROOT", self.proc)
        proc_patch.start()
        self.addCleanup(proc_patch.stop)
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

    def t3_tree(self, name: str, detached: bool = False) -> Path:
        tree = self.top / ".t3/worktrees/openttd-rust" / name
        git(
            self.main,
            "worktree",
            "add",
            "--quiet",
            *(["--detach"] if detached else ["-b", name]),
            str(tree),
        )
        return tree

    def test_finished_t3_and_protected_tasks(self) -> None:
        finished = self.t3_tree("t3-finished")
        detached = self.t3_tree("t3-detached", detached=True)
        protected = [
            self.t3_tree("t3-" + name)
            for name in (
                "dirty",
                "locked",
                "issue",
                "checkpoint",
                "deep",
                "anchor",
                "pinned",
                "open",
            )
        ]
        dirty, locked, issue, checkpoint, deep, anchor, pinned, open_tree = protected
        write(dirty / "untracked")
        git(self.main, "worktree", "lock", str(locked))
        write(deep / ".local/output")
        write(anchor / ".local/output")
        write(pinned / ".local/runtime/provenance.json", "{}")
        write(
            self.main / ".local/deep/99999999/report.json",
            json.dumps({"output": str(deep / ".local/output")}),
        )
        write(
            self.main / "tools/simulation/instructions.py",
            f'RUNTIME = "{anchor}/.local/output"',
        )
        references = f"Open issue body: {issue}\nResume checkpoint: `{checkpoint.name}`"
        with patch.dict(PR_STATES, {open_tree.name: "OPEN"}):
            items = housekeeping.plan_worktrees(
                self.main,
                housekeeping.list_worktrees(self.main)[1:],
                pr_state,
                references,
            )
        candidates = {item.path: item for item in items}
        self.assertTrue(candidates[finished].remove)
        self.assertTrue(candidates[detached].remove)
        for tree in protected:
            self.assertFalse(candidates[tree].remove, (tree, candidates[tree].reason))
        for tree in (finished, detached):
            self.assertIsNone(
                housekeeping.remove(self.main, candidates[tree], references)
            )
            self.assertFalse(tree.exists())
        for tree in protected:
            self.assertTrue(tree.exists())

    def test_cwd_and_references_are_rechecked_at_removal(self) -> None:
        tree = self.t3_tree("t3-cwd")
        item = next(
            i
            for i in housekeeping.plan_worktrees(
                self.main, housekeeping.list_worktrees(self.main)[1:], pr_state
            )
            if i.path == tree
        )
        self.assertTrue(item.remove)
        cwd = self.proc / "123/cwd"
        cwd.parent.mkdir()
        folder = tree / "src"
        folder.mkdir()
        cwd.symlink_to(folder)
        self.assertIn("cwd", housekeeping.remove(self.main, item) or "")
        self.assertTrue(tree.exists())
        cwd.unlink()
        self.assertIn(
            "referenced", housekeeping.remove(self.main, item, str(tree)) or ""
        )
        self.assertIsNone(housekeeping.remove(self.main, item))

    def test_uninspectable_cwd_keeps_worktree(self) -> None:
        tree = self.t3_tree("t3-hidden-cwd")
        original = Path.resolve

        def resolve(path: Path, strict: bool = False) -> Path:
            if path.name == "cwd":
                raise PermissionError("hidden cwd")
            return original(path, strict=strict)

        with patch.object(Path, "resolve", resolve):
            items = housekeeping.plan_worktrees(
                self.main, housekeeping.list_worktrees(self.main)[1:], pr_state
            )
        self.assertFalse(next(item.remove for item in items if item.path == tree))

    def test_dangling_runtime_link_keeps_live_target_protection(self) -> None:
        tree = self.t3_tree("t3-runtime-links")
        runtime = tree / ".local/runtime"
        runtime.mkdir(parents=True)
        (runtime / "game").symlink_to(self.top / "missing-game")
        target = write(self.top / "target.log")
        (runtime / "output").symlink_to(target)
        with held_output(target) as child:
            (self.proc / str(child.pid)).symlink_to(Path("/proc") / str(child.pid))
            handles, _ = housekeeping.open_outputs()
            for error in (None, "hidden process descriptors"):
                with self.subTest(error=error):
                    self.assertEqual(
                        housekeeping.output_refusal(
                            tree, handles if error is None else [], error
                        ),
                        "live open output handle",
                    )
            child_write(child)
            self.assertEqual(target.read_text(), "xvisible\n")
        for error in (None, "hidden process descriptors"):
            with self.subTest(error=error):
                self.assertIsNone(housekeeping.output_refusal(tree, [], error))
        item = housekeeping.Item(tree, True, "finished T3 task", True)
        self.assertIsNone(housekeeping.remove(self.main, item))
        self.assertFalse(tree.exists())
        self.assertEqual(target.read_text(), "xvisible\n")

    def fixture_local(self) -> None:
        local = self.main / ".local"
        sim = local / "simulation"
        integrated = git(self.main, "rev-parse", "HEAD").strip()
        candidate = json.dumps({"candidate_commit": integrated})
        # An open PR's head is not on the base, so its runs may be its evidence.
        open_head = git(self.top / "trees/open", "rev-parse", "HEAD").strip()
        unintegrated = json.dumps({"candidate_commit": open_head})
        profile = "scenario/plain/candidate/callgrind.out"
        stale = read_only(sim / "20260101T000000Z-1/ro").parent
        write(stale / "report.json", candidate)
        write(sim / "20260102T000000Z-2/report.json", candidate)
        write(sim / "20260102T000000Z-2" / profile)
        write(sim / "20260103T000000Z-3/report.json", candidate)
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
        deep_old = json.dumps({"commit": integrated})
        write(local / "deep/20260101T000000Z-aaaa/report.json", deep_old)
        write(local / "deep/20260102T000000Z-bbbb/report.json", current)
        self.removable |= {
            stale,
            sim / "20260102T000000Z-2",
            interrupted,
            coverage,
            write(
                local / "verification/20260101T000000.000000Z/report.json", candidate
            ).parent,
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
            write(sim / "20260109T000000Z-9/report.json", unintegrated).parent,
            write(sim / "20260110T000000Z-10/report.json", "{}").parent,
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

    def test_partial_profile_keeps_other_scenarios_until_replaced(self) -> None:
        simulation = self.main / ".local/simulation"
        earlier = simulation / "20260103T000000Z-3"
        later = simulation / "20260107T000000Z-7"
        write(earlier / "other/plain/candidate/callgrind.out")
        write(later / "scenario/plain/candidate/callgrind.out")
        write(later / "report.json", "{}")
        self.assertNotIn(earlier, self.run_main())
        self.run_main("--apply")
        self.assertTrue((earlier / "other/plain/candidate/callgrind.out").is_file())
        self.assertTrue((later / "scenario/plain/candidate/callgrind.out").is_file())

        replacement = simulation / "20260108T000000Z-8"
        write(replacement / "other/plain/candidate/callgrind.out")
        write(replacement / "report.json", "{}")
        self.assertIn(earlier, self.run_main())
        self.run_main("--apply")
        self.assertFalse(earlier.exists())
        self.assertTrue((later / "scenario/plain/candidate/callgrind.out").is_file())
        self.assertTrue((replacement / "other/plain/candidate/callgrind.out").is_file())

    def test_live_outputs_and_parents_survive_until_close(self) -> None:
        local = self.main / ".local"
        for path in (local / "live.log", local / "coverage/old/output.log"):
            with self.subTest(path=path):
                write(path)
                with held_output(path) as child:
                    (self.proc / str(child.pid)).symlink_to(
                        Path("/proc") / str(child.pid)
                    )
                    old = time.time() - 2 * housekeeping.RUNNING_SECONDS
                    for entry in (path, path.parent):
                        os.utime(entry, (old, old))
                    target = path if path.parent == local else path.parent
                    self.assertNotIn(target, self.run_main())
                    self.run_main("--apply")
                    child_write(child)
                    self.assertEqual(path.read_text(), "xvisible\n")
                os.utime(path, (old, old))
                os.utime(path.parent, (old, old))
                self.assertIn(target, self.run_main())
                self.run_main("--apply")
                self.assertFalse(target.exists())

    def test_unreadable_same_user_output_prevents_deletion(self) -> None:
        path = write(self.main / ".local/private.log")
        item = next(
            i
            for i in housekeeping.plan_local(path.parent, time.time())
            if i.path == path
        )
        with held_output(path, unreadable=True) as child:
            (self.proc / str(child.pid)).symlink_to(Path("/proc") / str(child.pid))
            self.assertNotIn(path, self.run_main())
            self.assertEqual(
                housekeeping.remove(self.main, item), "live open output handle"
            )
            child_write(child)
            self.assertEqual(path.read_text(), "xvisible\n")
        self.assertIn(path, self.run_main())
        self.assertIsNone(housekeeping.remove(self.main, item))
        self.assertFalse(path.exists())

    def test_unreadable_process_does_not_keep_unrelated_closed_outputs(self) -> None:
        path = write(self.main / ".local/private.log")
        closed = write(self.main / ".local/closed.log")
        with held_output(path, unreadable=True) as child:
            (self.proc / str(child.pid)).symlink_to(Path("/proc") / str(child.pid))
            listed = self.run_main()
            self.assertNotIn(path, listed)
            self.assertIn(closed, listed)
            self.run_main("--apply")
            self.assertFalse(closed.exists())
            child_write(child)
            self.assertEqual(path.read_text(), "xvisible\n")

    def test_opened_after_planning_is_protected_at_removal(self) -> None:
        path = write(self.main / ".local/late.log")
        item = next(
            i
            for i in housekeeping.plan_local(path.parent, time.time())
            if i.path == path
        )
        self.assertTrue(item.remove)
        with held_output(path) as child:
            (self.proc / str(child.pid)).symlink_to(Path("/proc") / str(child.pid))
            self.assertIn("live", housekeeping.remove(self.main, item) or "")
            child_write(child)
            self.assertEqual(path.read_text(), "xvisible\n")
        self.assertIsNone(housekeeping.remove(self.main, item))
        self.assertFalse(path.exists())

    def run_main(self, *arguments: str) -> list[Path]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            code = housekeeping.main(
                [*arguments, "--repo", str(self.main)], pr_state, lambda _: ""
            )
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
