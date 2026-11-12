#!/usr/bin/env python3
"""Exercise the transcript digest and its queries on small synthetic stores."""

import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import Any

import transcripts

MATCH = "sample-repo"
CWD = f"/work/{MATCH}/worktrees/one"
DAY1 = "2026-11-03"
DAY2 = "2026-11-05"
CLAUDE_MAIN = "11111111-aaaa-4bbb-8ccc-000000000001"
CODEX_MAIN = "22222222-aaaa-7bbb-8ccc-000000000002"
CODEX_OTHER = "33333333-aaaa-7bbb-8ccc-000000000003"


def ts(day: str, clock: str) -> str:
    return f"{day}T{clock}Z" if "." in clock else f"{day}T{clock}.000Z"


def ms(day: str, clock: str) -> int:
    return int(transcripts.epoch_ms(ts(day, clock)))


def write_lines(path: Path, records: list[Any], extra: str = "") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = "".join(json.dumps(r) + "\n" for r in records)
    path.write_text(text + extra)


def claude_user(
    day: str, clock: str, content: object, **more: object
) -> dict[str, Any]:
    record = {"type": "user", "timestamp": ts(day, clock), "cwd": CWD}
    record.update(message={"role": "user", "content": content}, **more)
    return record


def claude_assistant(day: str, clock: str, *blocks: object) -> dict[str, Any]:
    message = {"model": "model-a", "content": list(blocks)}
    return {"type": "assistant", "timestamp": ts(day, clock), "message": message}


def tool_use(call_id: str, command: str, background: bool = False) -> dict[str, Any]:
    args = {"command": command, "run_in_background": background}
    return {"type": "tool_use", "id": call_id, "name": "Bash", "input": args}


def tool_result(call_id: str, text: str, error: bool = False) -> dict[str, Any]:
    return {
        "type": "tool_result",
        "tool_use_id": call_id,
        "content": text,
        "is_error": error,
    }


def notification(call_id: str, status: str, summary: str) -> str:
    return (
        "<task-notification>\n<task-id>b1</task-id>\n"
        f"<tool-use-id>{call_id}</tool-use-id>\n<status>{status}</status>\n"
        f"<summary>{summary}</summary>\n</task-notification>"
    )


def claude_store(root: Path) -> None:
    folder = root / "projects" / f"-work-{MATCH}-worktrees-one"
    main = [
        {"type": "custom-title", "customTitle": "Sample title"},
        claude_user(DAY1, "10:00:00", "Please check the build"),
        claude_assistant(DAY1, "10:00:01", tool_use("tu1", "git status")),
        claude_user(DAY1, "10:00:02", [tool_result("tu1", "clean")]),
        claude_assistant(DAY1, "10:00:03", tool_use("tu2", "cargo test")),
        claude_user(
            DAY1, "10:00:04", [tool_result("tu2", "Exit code 101\nfailed", True)]
        ),
        claude_assistant(DAY1, "10:00:05", {"type": "text", "text": "Build fails."}),
        claude_user(DAY2, "09:00:00", "Delegated task node:x finished"),
        claude_assistant(DAY2, "09:00:01", tool_use("tu3", "make all", True)),
        claude_user(
            DAY2, "09:00:02", [tool_result("tu3", "Command running in background")]
        ),
        claude_user(
            DAY2,
            "09:05:00",
            notification(
                "tu3", "failed", 'Background command "make" failed with exit code 2'
            ),
        ),
        claude_assistant(DAY2, "09:05:01", {"type": "text", "text": "Make failed."}),
        claude_user(DAY2, "09:06:00", "Agent: /root | Model: x\nPlease rebase"),
    ]
    write_lines(folder / f"{CLAUDE_MAIN}.jsonl", main, extra="{not json\n")
    sub = [
        claude_user(DAY2, "09:01:00", "Look up the docs", sessionId=CLAUDE_MAIN),
        claude_assistant(DAY2, "09:01:01", tool_use("tu9", "rg docs")),
        claude_user(DAY2, "09:01:02", [tool_result("tu9", "docs/a.md")]),
    ]
    write_lines(folder / CLAUDE_MAIN / "subagents" / "agent-a1.jsonl", sub)
    other = root / "projects" / "-work-unrelated"
    write_lines(other / "x.jsonl", [claude_user(DAY1, "10:00:00", "Not ours")])


def codex_meta(sid: str, cwd: str, source: object = "vscode") -> dict[str, Any]:
    payload = {"id": sid, "cwd": cwd, "source": source}
    return {
        "type": "session_meta",
        "timestamp": ts(DAY1, "08:00:00"),
        "payload": payload,
    }


def codex_item(
    day: str, clock: str, item: dict[str, Any], start: int
) -> dict[str, Any]:
    payload = {"type": "item_completed", "item": item, "started_at_ms": start}
    return {"type": "event_msg", "timestamp": ts(day, clock), "payload": payload}


def command(cid: str, cmd: str, exit_code: int, output: str) -> dict[str, Any]:
    return {
        "type": "CommandExecution",
        "id": cid,
        "command": ["/bin/bash", "-lc", cmd],
        "status": "completed" if exit_code == 0 else "failed",
        "exit_code": exit_code,
        "aggregated_output": output,
    }


def codex_call(day: str, clock: str, call_id: str, name: str, text: str) -> dict:
    kind = "custom_tool_call" if name == "exec" else "function_call"
    field = "input" if name == "exec" else "arguments"
    payload = {"type": kind, "name": name, "call_id": call_id, field: text}
    return {"type": "response_item", "timestamp": ts(day, clock), "payload": payload}


def codex_output(day: str, clock: str, call_id: str, head: str) -> dict:
    output = [
        {"type": "input_text", "text": f"{head}\nWall time 0.1 seconds\nOutput:\n"}
    ]
    payload = {"type": "custom_tool_call_output", "call_id": call_id, "output": output}
    return {"type": "response_item", "timestamp": ts(day, clock), "payload": payload}


def codex_user(day: str, clock: str, text: str) -> dict[str, Any]:
    item = {"type": "UserMessage", "content": [{"type": "text", "text": text}]}
    payload = {"type": "item_completed", "item": item}
    return {"type": "event_msg", "timestamp": ts(day, clock), "payload": payload}


def codex_records() -> list[Any]:
    two = (
        'text(await tools.exec_command({cmd:"git status"}));\n'
        'text(await tools.exec_command({cmd:"ls missing"}));'
    )
    background = (
        'text(await tools.exec_command({cmd:"cargo build",yield_time_ms:1000}));'
    )
    settings = {"settings": {"reasoning_effort": "high"}}
    context = {"model": "model-b", "collaboration_mode": settings}
    return [
        codex_meta(CODEX_MAIN, CWD),
        {"type": "turn_context", "timestamp": ts(DAY1, "08:00:01"), "payload": context},
        codex_user(DAY1, "08:00:02", "Review the open pull request"),
        codex_call(DAY1, "08:00:03", "c1", "exec", two),
        codex_item(
            DAY1,
            "08:00:04",
            command("x1", "git status", 0, "clean"),
            ms(DAY1, "08:00:03.5"),
        ),
        codex_item(
            DAY1,
            "08:00:05",
            command("x2", "ls missing", 2, "No such file"),
            ms(DAY1, "08:00:04.5"),
        ),
        codex_output(DAY1, "08:00:06", "c1", "Script completed"),
        codex_user(DAY2, "08:00:00", "Background command completed: x"),
        codex_call(DAY2, "08:00:01", "c2", "exec", background),
        codex_output(DAY2, "08:00:02", "c2", "Script completed"),
        codex_call(DAY2, "08:00:10", "c3", "wait", '{"cell_id":"g1"}'),
        codex_item(
            DAY2,
            "08:00:20",
            command("x3", "cargo build", 1, "error[E0425]"),
            ms(DAY2, "08:00:01.5"),
        ),
        codex_output(DAY2, "08:00:21", "c3", "Script completed"),
        {
            "type": "event_msg",
            "timestamp": ts(DAY2, "08:00:22"),
            "payload": {
                "type": "task_complete",
                "last_agent_message": "Review posted.",
            },
        },
    ]


def codex_store(root: Path) -> None:
    records = codex_records()
    live = root / "sessions" / "2026" / "11" / "03" / f"rollout-a-{CODEX_MAIN}.jsonl"
    write_lines(live, records)
    archived = root / "archived_sessions" / f"rollout-a-{CODEX_MAIN}.jsonl"
    write_lines(archived, records[:5])
    child = codex_meta(
        CODEX_OTHER,
        CWD,
        {
            "subagent": {
                "thread_spawn": {
                    "parent_thread_id": CODEX_MAIN,
                    "agent_path": "/root/r",
                }
            }
        },
    )
    path = root / "sessions" / "2026" / "11" / "05" / f"rollout-b-{CODEX_OTHER}.jsonl"
    write_lines(path, [child, codex_user(DAY2, "07:00:00", "Subagent brief")])
    stranger = root / "sessions" / "2026" / "11" / "05" / "rollout-c-4444.jsonl"
    write_lines(
        stranger, [codex_meta("4444", "/work/other"), codex_user(DAY2, "07:00:00", "x")]
    )


class TranscriptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        base = Path(self.temp.name)
        self.claude_root, self.codex_root = base / "claude", base / "codex"
        self.out = base / "digest"
        claude_store(self.claude_root)
        codex_store(self.codex_root)
        self.built = self.run_tool(
            "digest",
            "--since",
            "2026-11-01",
            "--out",
            str(self.out),
            "--claude-root",
            str(self.claude_root),
            "--codex-root",
            str(self.codex_root),
            "--match",
            MATCH,
            "--jobs",
            "1",
        )

    def run_tool(self, *argv: str) -> str:
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            self.status = transcripts.main(list(argv))
        return stdout.getvalue() + stderr.getvalue()

    def query(self, *argv: str) -> str:
        return self.run_tool(*argv, "--digest", str(self.out), "--limit", "0")

    def table(self, name: str) -> list[dict[str, Any]]:
        text = (self.out / f"{name}.jsonl").read_text()
        return [json.loads(line) for line in text.splitlines()]

    def calls(self, session: str) -> list[dict[str, Any]]:
        return [r for r in self.table("calls") if r["s"] == session]

    def test_claude_calls_pair_results_and_error_flags(self) -> None:
        calls = {r["id"]: r for r in self.calls(CLAUDE_MAIN)}
        self.assertEqual(calls["tu1"]["r"], "clean")
        self.assertEqual((calls["tu1"]["e"], calls["tu1"]["x"]), (False, 0))
        self.assertEqual((calls["tu2"]["e"], calls["tu2"]["x"]), (True, 101))
        self.assertEqual(calls["tu2"]["model"], "model-a")
        self.assertTrue(calls["tu2"]["path"].endswith(f"{CLAUDE_MAIN}.jsonl"))
        background = calls["tu3"]
        self.assertTrue(background["bg"])
        self.assertEqual((background["e"], background["x"]), (True, 2))
        self.assertEqual(background["done"], ts(DAY2, "09:05:00"))

    def test_claude_subagent_file_names_its_parent(self) -> None:
        sessions = {s["s"]: s for s in self.table("sessions")}
        self.assertTrue(sessions["agent-a1"]["sub"])
        self.assertEqual(sessions["agent-a1"]["parent"], CLAUDE_MAIN)
        self.assertEqual(self.calls("agent-a1")[0]["c"], "rg docs")
        self.assertEqual(sessions[CLAUDE_MAIN]["title"], "Sample title")
        self.assertNotIn("x", sessions)

    def test_codex_exec_records_each_shell_command(self) -> None:
        records = {r["id"]: r for r in self.calls(CODEX_MAIN)}
        outer = records["c1"]
        self.assertEqual(
            (outer["kind"], outer["nested"], outer["e"]), ("call", 2, False)
        )
        first, second = records["x1"], records["x2"]
        self.assertEqual((first["kind"], first["x"], first["e"]), ("shell", 0, False))
        self.assertEqual((second["kind"], second["x"], second["e"]), ("shell", 2, True))
        self.assertEqual(second["owner"], "c1")
        self.assertEqual((second["model"], second["effort"]), ("model-b", "high"))

    def test_codex_background_completion_and_wait_are_distinct(self) -> None:
        records = {r["id"]: r for r in self.calls(CODEX_MAIN)}
        late = records["x3"]
        self.assertEqual(
            (late["kind"], late["owner"], late["x"]), ("background", "c2", 1)
        )
        self.assertEqual(records["c3"]["kind"], "wait")
        out = self.query("count", "cargo build", "--kind", "background")
        self.assertIn("total 1 failed 1", out)

    def test_archived_copy_is_read_once(self) -> None:
        sessions = [s for s in self.table("sessions") if s["s"] == CODEX_MAIN]
        self.assertEqual(len(sessions), 1)
        self.assertIn("/sessions/", sessions[0]["path"])
        self.assertIn("1 duplicate Codex copies skipped", self.built)
        self.assertNotIn("4444", {s["s"] for s in self.table("sessions")})

    def test_codex_subagent_names_parent(self) -> None:
        child = next(s for s in self.table("sessions") if s["s"] == CODEX_OTHER)
        self.assertEqual((child["sub"], child["parent"]), (True, CODEX_MAIN))
        self.assertEqual(child["agent"], "/root/r")

    def test_leaf_queries_do_not_count_codex_scripts_twice(self) -> None:
        self.assertIn("total 1 ", self.query("count", "ls missing"))
        self.assertIn("total 1 ", self.query("count", "ls missing", "--kind", "call"))
        out = self.query("calls", "ls missing", "--failed")
        self.assertIn("shell exec_command failed=True exit=2 in c1", out)

    def test_invalid_json_is_counted_and_printed(self) -> None:
        self.assertIn("invalid JSON lines skipped: 1 in 1 files", self.built)
        meta = json.loads((self.out / "meta.json").read_text())
        self.assertEqual(sum(meta["invalid_lines"].values()), 1)

    def test_since_applies_to_every_query(self) -> None:
        late = ("--since", DAY2)
        self.assertIn("[0 hits", self.query("calls", "git status", *late))
        self.assertIn("[2 hits", self.query("calls", "git status"))
        self.assertIn("total 0 ", self.query("count", "git status", *late))
        self.assertIn("[0 hits]", self.query("users", "check the build", *late))
        self.assertIn("[0 hits]", self.query("finals", "Build fails", *late))
        self.assertIn("[1 hits]", self.query("finals", "Build fails"))
        self.assertIn("[0 hits]", self.query("humans", "check the build", *late))
        session = self.query("session", CODEX_MAIN[:8], *late)
        self.assertNotIn("ls missing", session)
        self.assertIn("cargo build", session)
        listed = self.query("sessions", "--until", DAY1)
        self.assertIn(CODEX_MAIN, listed)
        self.assertNotIn(CODEX_OTHER, listed)

    def test_humans_drops_machine_and_subagent_messages(self) -> None:
        out = self.query("humans")
        self.assertIn("Heuristic candidates", out)
        self.assertIn("Please check the build", out)
        self.assertIn("Review the open pull request", out)
        for machine in ("Delegated task", "Background command", "Please rebase"):
            self.assertNotIn(machine, out)
        self.assertNotIn("Look up the docs", out)
        self.assertNotIn("Subagent brief", out)
        self.assertIn("[2 hits]", out)

    def test_schema_mismatch_asks_for_rebuild(self) -> None:
        meta_path = self.out / "meta.json"
        meta = json.loads(meta_path.read_text())
        meta["schema"] = transcripts.SCHEMA + 1
        meta_path.write_text(json.dumps(meta))
        out = self.query("sessions")
        self.assertEqual(self.status, 2)
        self.assertIn("Rebuild it: python3 tools/transcripts.py digest", out)

    def test_missing_digest_names_the_build_command(self) -> None:
        out = self.run_tool("sessions", "--digest", str(self.out / "none"))
        self.assertEqual(self.status, 2)
        self.assertIn("Build one: python3 tools/transcripts.py digest", out)

    def test_ambiguous_session_prefix_is_refused(self) -> None:
        out = self.query("session", "")
        self.assertEqual(self.status, 2)
        self.assertIn("Give a longer prefix", out)

    def test_old_files_are_skipped_by_modification_time(self) -> None:
        old = 1_700_000_000
        for path in self.claude_root.rglob("*.jsonl"):
            os.utime(path, (old, old))
        files = transcripts.claude_files(self.claude_root, MATCH, old + 1)
        self.assertEqual(files, [])


if __name__ == "__main__":
    unittest.main()
