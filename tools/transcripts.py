#!/usr/bin/env python3
"""Digest and query this project's Claude Code and Codex transcripts.

Build the digest once, then query it:

  python3 tools/transcripts.py digest --since 2026-11-01
  python3 tools/transcripts.py count 'cargo: not found' --failed
  python3 tools/transcripts.py calls 'gh pr merge' --failed --full
  python3 tools/transcripts.py session 01a1cd27

The digest is private text from agent sessions. It stays under the checkout's
ignored .local/transcripts/ by default: keep it, and anything copied from it,
out of Git, test fixtures and CI artifacts.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from bisect import bisect_right
from collections import Counter
from collections.abc import Callable, Iterable, Iterator
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from typing import Any

SCHEMA = 2
ROOT = Path(__file__).absolute().parents[1]
DEFAULT_DIGEST = ROOT / ".local" / "transcripts"
TABLES = ("sessions", "calls", "users", "finals")
CLIP_RESULT = 500
CODEX_WAITS = ("wait", "wait_agent")
CODEX_CALLS = ("custom_tool_call", "function_call", "local_shell_call")
CODEX_OUTPUTS = ("custom_tool_call_output", "function_call_output")
KINDS = ("leaf", "call", "shell", "mcp", "background", "wait", "all")
EXIT_CODE = re.compile(r"^Exit code (-?\d+)")
NOTICE_ID = re.compile(r"<tool-use-id>([^<]+)</tool-use-id>")
NOTICE_STATUS = re.compile(r"<status>([^<]+)</status>")
NOTICE_SUMMARY = re.compile(r"<summary>(.*?)</summary>", re.S)
NOTICE_EXIT = re.compile(r"exit code (-?\d+)")
# User-role messages a host or another agent writes. The transcripts do not
# record who typed a message, so `humans` drops these, and sessions an agent
# started, and calls what is left heuristic candidates.
MACHINE_PREFIXES = (
    "Delegated task",
    "Background command completed",
    "<task-notification>",
    "<system-reminder>",
    "[SYSTEM NOTIFICATION",
    "[Request interrupted",
    "Act as the ",
    "Reviewer thread",
    "Agent: /",
    "Root (",
    "Root to /",
    "From root",
    "From /root",
    "/root/",
    "You are routine agent",
    "Base directory for this skill",
    "This session is being continued",
    "Update on pull request",
    "Caveat:",
    "# AGENTS.md",
    "<environment_context>",
    "<user_instructions>",
    "<skill>",
    "<local-command",
)
ATTRIBUTION = re.compile(r"^Agent: /", re.M)

HELP_EPILOG = """\
sources:
  Claude Code: CLAUDE_ROOT/projects/<dir>/<session>.jsonl and
    <dir>/<session>/subagents/*.jsonl, for each <dir> whose name contains MATCH.
  Codex: CODEX_ROOT/sessions/**/rollout-*.jsonl and
    CODEX_ROOT/archived_sessions/**/rollout-*.jsonl whose session_meta cwd
    contains MATCH. A session in both stores is read once, from its larger copy.
  MATCH defaults to the main checkout's directory name, so sessions in every
  worktree, including removed ones, are included; --match overrides it.

records (calls.jsonl, field "kind"):
  call        a tool call: every Claude call, and a Codex outer call (for exec,
              the JavaScript). A Codex call that ran commands has "nested".
  shell       a shell command run inside a Codex exec, with its own exit code.
  mcp         an MCP tool called inside a Codex exec.
  background  a Codex shell command that finished after its exec returned.
              A Claude background Bash call stays a call with "bg" set; its
              completion notice fills "done", "x" and "e".
  wait        a Codex wait or wait_agent call.
  Queries read "leaf" records by default: one per command, so Codex scripts
  are not counted twice. --kind call reads the outer calls.
  The digest reads each session whole, so a result, a completion or the
  session's opening outside the window still counts, then keeps the records
  whose t falls in it: a call's t is when it started. A session's start, end
  and first message span the whole session; its calls, cmds and failed count
  the records kept.
  Every record keeps h (host), s (session), parent, path and ln (raw file and
  line), t, id (call ID), model, effort (Codex), e (failed), x (exit code),
  c (input or command, in full) and r (result: its first and last 500
  characters, with rl its full length). Queries match c, and users and
  finals match m (the message, in full); no query reads r.

humans (heuristic):
  User-role messages that start with none of the machine-written prefixes in
  MACHINE_PREFIXES and carry no "Agent: /" attribution line, in sessions that
  are not subagents and whose first message is not machine-written either
  (delegated threads). The transcripts do not record who typed a message.

privacy:
  The digest holds private session text. It stays under .local/transcripts/
  by default, which Git ignores. Keep it and anything copied from it out of
  Git, test fixtures and CI artifacts.
"""


@dataclass(frozen=True)
class Window:
    since: str = ""
    until: str = ""  # exclusive: the day after --until

    def has(self, ts: str) -> bool:
        if not ts:
            return True
        return (not self.since or ts >= self.since) and (
            not self.until or ts < self.until
        )


@dataclass
class Parsed:
    session: dict[str, Any]
    calls: list[dict[str, Any]] = field(default_factory=list)
    users: list[dict[str, Any]] = field(default_factory=list)
    finals: list[dict[str, Any]] = field(default_factory=list)
    invalid: int = 0


class JsonLines:
    """Iterate a JSON-lines file, counting lines that are not JSON objects."""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.invalid = 0

    def __iter__(self) -> Iterator[tuple[int, dict[str, Any]]]:
        with self.path.open(errors="replace") as lines:
            for number, line in enumerate(lines, 1):
                if not line.strip():
                    continue
                try:
                    value = json.loads(line)
                except ValueError:
                    self.invalid += 1
                    continue
                if isinstance(value, dict):
                    yield number, value
                else:
                    self.invalid += 1


def clip(text: str, head: int, tail: int = 0) -> str:
    if len(text) <= head + tail + 20:
        return text
    middle = f" ...[{len(text) - head - tail} chars]... "
    return text[:head] + middle + (text[-tail:] if tail else "")


def as_text(value: object) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        return as_text(value.get("text", ""))
    if not isinstance(value, list):
        return "" if value is None else json.dumps(value, ensure_ascii=False)
    parts = []
    for block in value:
        if isinstance(block, str):
            parts.append(block)
        elif isinstance(block, dict) and "text" in block:
            parts.append(str(block["text"]))
        elif isinstance(block, dict) and block.get("type") == "tool_reference":
            parts.append(f"[tool_reference {block.get('tool_name')}]")
    return "\n".join(parts)


def epoch_ms(ts: str) -> float:
    try:
        return datetime.fromisoformat(ts.replace("Z", "+00:00")).timestamp() * 1000
    except ValueError:
        return 0.0


def iso_ms(ms: float) -> str:
    moment = datetime.fromtimestamp(ms / 1000, timezone.utc)
    return moment.isoformat(timespec="milliseconds").replace("+00:00", "Z")


def new_session(host: str, sid: str, path: Path, sub: bool) -> dict[str, Any]:
    return {
        "h": host,
        "s": sid,
        "parent": "",
        "sub": sub,
        "agent": "",
        "path": str(path),
        "proj": "",
        "cwd": "",
        "start": "",
        "end": "",
        "model": "",
        "effort": "",
        "title": "",
        "first": "",
        "calls": 0,
        "cmds": 0,
        "failed": 0,
        "cost": None,
        "invalid": 0,
    }


class Reader:
    """State shared by both hosts' session readers."""

    def __init__(self, session: dict[str, Any], window: Window) -> None:
        self.out = Parsed(session)
        self.window = window
        self.model = ""
        self.effort = ""
        self.last_text: tuple[str, int, str] | None = None

    @property
    def meta(self) -> dict[str, Any]:
        return self.out.session

    def base(self, ts: str, ln: int) -> dict[str, Any]:
        meta = self.meta
        record = {"h": meta["h"], "s": meta["s"], "parent": meta["parent"]}
        record.update(path=meta["path"], ln=ln, t=ts, model=self.model)
        if self.effort:
            record["effort"] = self.effort
        return record

    def user(self, ts: str, ln: int, text: str) -> None:
        self.flush_final()
        record = self.base(ts, ln)
        record.update(sub=self.meta["sub"], m=text)
        self.out.users.append(record)
        if not self.meta["first"]:
            self.meta["first"] = text

    def final(self, ts: str, ln: int, text: str) -> None:
        record = self.base(ts, ln)
        record.update(m=text)
        self.out.finals.append(record)

    def flush_final(self) -> None:
        if self.last_text:
            self.final(*self.last_text)
            self.last_text = None

    def call(self, ts: str, ln: int, **fields: object) -> dict[str, Any]:
        record = self.base(ts, ln)
        record.update(fields)
        record["e"] = False
        self.out.calls.append(record)
        return record

    def finish(self, invalid: int) -> Parsed | None:
        """Keep the records the window selects, once the whole session is read."""
        self.flush_final()
        out, meta, has = self.out, self.out.session, self.window.has
        stamps = sorted(r["t"] for t in (out.calls, out.users, out.finals) for r in t)
        stamps = [ts for ts in stamps if ts]
        out.invalid = meta["invalid"] = invalid
        if not any(has(ts) for ts in stamps):
            return None
        out.calls = [r for r in out.calls if has(r["t"])]
        out.users = [r for r in out.users if has(r["t"])]
        out.finals = [r for r in out.finals if has(r["t"])]
        meta.update(start=stamps[0], end=stamps[-1], model=meta["model"] or self.model)
        meta["calls"] = sum(r["kind"] in ("call", "wait") for r in out.calls)
        meta["cmds"] = sum(r["kind"] in ("shell", "background") for r in out.calls)
        meta["failed"] = sum(bool(r.get("e")) for r in out.calls)
        return out


def set_result(record: dict[str, Any], text: str) -> None:
    record.update(r=clip(text, CLIP_RESULT, CLIP_RESULT), rl=len(text))


class ClaudeReader(Reader):
    def __init__(self, path: Path, window: Window) -> None:
        sub = path.parent.name == "subagents"
        session = new_session("claude", path.stem, path, sub)
        session["proj"] = (path.parents[2] if sub else path.parent).name
        super().__init__(session, window)
        self.path = path
        self.pending: dict[str, dict[str, Any]] = {}

    def read(self) -> Parsed | None:
        lines = JsonLines(self.path)
        for ln, record in lines:
            self.record(ln, record)
        return self.finish(lines.invalid)

    def record(self, ln: int, record: dict[str, Any]) -> None:
        kind = record.get("type")
        if kind == "custom-title":
            self.meta["title"] = record.get("customTitle", "")
        elif kind == "cost-state":
            self.meta["cost"] = round(record.get("totalCostUSD") or 0, 2)
        if kind not in ("user", "assistant"):
            return
        ts = record.get("timestamp", "")
        if self.meta["sub"] and not self.meta["parent"]:
            self.meta["parent"] = record.get("sessionId", "")
        self.meta["cwd"] = self.meta["cwd"] or record.get("cwd", "")
        message = record.get("message") or {}
        if kind == "assistant":
            self.assistant(ts, ln, message)
        else:
            self.human_or_result(ts, ln, message.get("content"))

    def assistant(self, ts: str, ln: int, message: dict[str, Any]) -> None:
        model = message.get("model") or ""
        if model and not model.startswith("<"):
            self.model = model
        for block in message.get("content") or []:
            if not isinstance(block, dict):
                continue
            if block.get("type") == "tool_use":
                self.tool_use(ts, ln, block)
            elif block.get("type") == "text" and block.get("text", "").strip():
                self.last_text = (ts, ln, block["text"])

    def tool_use(self, ts: str, ln: int, block: dict[str, Any]) -> None:
        name = block.get("name") or ""
        args = block.get("input") or {}
        command = args.get("command") if name == "Bash" else None
        text = command if isinstance(command, str) else json.dumps(args)
        record = self.call(ts, ln, id=block.get("id", ""), kind="call", n=name, c=text)
        record["bg"] = bool(args.get("run_in_background"))
        self.pending[record["id"]] = record

    def human_or_result(self, ts: str, ln: int, content: object) -> None:
        if isinstance(content, str):
            self.notice(ts, content)
            self.user(ts, ln, content)
            return
        texts = []
        for block in content if isinstance(content, list) else []:
            if not isinstance(block, dict):
                continue
            if block.get("type") == "tool_result":
                self.tool_result(block)
            elif block.get("type") == "text":
                texts.append(block.get("text", ""))
        text = "\n".join(texts)
        self.notice(ts, text)
        if texts and not text.startswith("<system-reminder>"):
            self.user(ts, ln, text)

    def tool_result(self, block: dict[str, Any]) -> None:
        record = self.pending.get(block.get("tool_use_id", ""))
        if record is None:
            return
        text = as_text(block.get("content"))
        set_result(record, text)
        record["e"] = bool(block.get("is_error"))
        found = EXIT_CODE.match(text)
        if found:
            record["x"] = int(found.group(1))
        elif record["n"] == "Bash" and not record["e"] and not record["bg"]:
            record["x"] = 0

    def notice(self, ts: str, text: str) -> None:
        """Fill a background Bash call from its completion notice."""
        if "<task-notification>" not in text:
            return
        call_id = NOTICE_ID.search(text)
        record = self.pending.get(call_id.group(1)) if call_id else None
        if record is None or record.get("done"):
            return
        status = NOTICE_STATUS.search(text)
        summary = NOTICE_SUMMARY.search(text)
        record["done"] = ts
        record["status"] = status.group(1) if status else ""
        record["e"] = record["status"] == "failed"
        exit_code = NOTICE_EXIT.search(summary.group(1)) if summary else None
        if exit_code:
            record["x"] = int(exit_code.group(1))
        elif record["status"] == "completed":
            record["x"] = 0


def codex_meta(path: Path) -> dict[str, Any] | None:
    """The session_meta payload on a rollout's first line."""
    try:
        with path.open(errors="replace") as lines:
            first = json.loads(lines.readline())
    except (OSError, ValueError):
        return None
    if not isinstance(first, dict) or first.get("type") != "session_meta":
        return None
    payload = first.get("payload")
    return payload if isinstance(payload, dict) else None


def shell_text(command: object) -> str:
    if isinstance(command, list):
        words = [str(word) for word in command]
        if len(words) >= 3 and words[1] in ("-lc", "-c"):
            return words[-1]
        return " ".join(words)
    return str(command or "")


class CodexReader(Reader):
    def __init__(self, path: Path, meta: dict[str, Any], window: Window) -> None:
        source = meta.get("source")
        sub = isinstance(source, dict)
        sid = meta.get("id") or path.stem[-36:]
        session = new_session("codex", sid, path, sub)
        session.update(cwd=meta.get("cwd", ""), proj=meta.get("cwd", ""))
        spawn = (source or {}).get("subagent", {}) if sub else {}
        spawn = spawn.get("thread_spawn", {}) if isinstance(spawn, dict) else {}
        if isinstance(spawn, dict):
            session["parent"] = spawn.get("parent_thread_id") or ""
            session["agent"] = spawn.get("agent_path") or ""
        super().__init__(session, window)
        self.path = path
        self.pending: dict[str, dict[str, Any]] = {}
        self.nested: list[dict[str, Any]] = []

    def read(self) -> Parsed | None:
        lines = JsonLines(self.path)
        for ln, record in lines:
            self.record(ln, record)
        self.assign_owners()
        return self.finish(lines.invalid)

    def record(self, ln: int, record: dict[str, Any]) -> None:
        kind = record.get("type")
        payload = record.get("payload") or {}
        if kind == "turn_context":
            self.context(payload)
            return
        ts = record.get("timestamp", "")
        if kind == "response_item":
            self.response(ts, ln, payload)
        elif kind == "event_msg" and payload.get("type") == "item_completed":
            self.item(ts, ln, payload)
        elif kind == "event_msg" and payload.get("type") == "task_complete":
            message = payload.get("last_agent_message")
            if message:
                self.final(ts, ln, message)

    def context(self, payload: dict[str, Any]) -> None:
        self.model = payload.get("model") or self.model
        mode = payload.get("collaboration_mode") or {}
        settings = mode.get("settings") or {}
        self.effort = settings.get("reasoning_effort") or self.effort
        self.meta["model"] = self.meta["model"] or self.model
        self.meta["effort"] = self.meta["effort"] or self.effort

    def response(self, ts: str, ln: int, payload: dict[str, Any]) -> None:
        kind = payload.get("type")
        if kind in CODEX_CALLS:
            name = payload.get("name") or kind
            text = payload.get("input") or payload.get("arguments") or ""
            if not isinstance(text, str):
                text = json.dumps(text, ensure_ascii=False)
            role = "wait" if name in CODEX_WAITS else "call"
            call_id = payload.get("call_id") or ""
            record = self.call(ts, ln, id=call_id, kind=role, n=name, c=text)
            record["_start"] = epoch_ms(ts)
            self.pending[call_id] = record
        elif kind in CODEX_OUTPUTS:
            record = self.pending.get(payload.get("call_id") or "")
            if record is not None:
                text = as_text(payload.get("output"))
                set_result(record, text)
                record["done"] = ts
                record["_end"] = epoch_ms(ts)
                head = text[:200]
                record["e"] = head.startswith("Script failed") or "Script error" in head

    def item(self, ts: str, ln: int, payload: dict[str, Any]) -> None:
        item = payload.get("item") or {}
        kind = item.get("type")
        if kind == "UserMessage":
            self.user(ts, ln, as_text(item.get("content")))
        elif kind == "CommandExecution":
            self.shell(ts, ln, payload, item)
        elif kind == "McpToolCall":
            self.mcp(ts, ln, payload, item)

    def nested_record(
        self, ts: str, ln: int, payload: dict[str, Any], **fields: object
    ) -> dict[str, Any]:
        start = payload.get("started_at_ms") or epoch_ms(ts)
        record = self.call(iso_ms(start), ln, **fields)
        record.update(done=ts, _start=start, _end=epoch_ms(ts))
        self.nested.append(record)
        return record

    def shell(
        self, ts: str, ln: int, payload: dict[str, Any], item: dict[str, Any]
    ) -> None:
        text = shell_text(item.get("command"))
        name = "exec_command"
        record = self.nested_record(
            ts, ln, payload, id=item.get("id", ""), kind="shell", n=name, c=text
        )
        exit_code = item.get("exit_code")
        record["x"] = exit_code
        record["e"] = item.get("status") == "failed" or exit_code not in (None, 0)
        set_result(record, as_text(item.get("aggregated_output")))

    def mcp(
        self, ts: str, ln: int, payload: dict[str, Any], item: dict[str, Any]
    ) -> None:
        name = f"mcp__{item.get('server', '')}__{item.get('tool', '')}"
        text = json.dumps(item.get("arguments"), ensure_ascii=False)
        record = self.nested_record(
            ts, ln, payload, id=item.get("id", ""), kind="mcp", n=name, c=text
        )
        result = item.get("result") or {}
        failed = "error" in item or bool(result.get("isError"))
        record["e"] = failed or item.get("status") == "failed"
        set_result(record, as_text(result.get("content") or item.get("error")))

    def assign_owners(self) -> None:
        """Attach each nested record to the outer call that started it."""
        outer = sorted(self.pending.values(), key=lambda r: r["_start"])
        starts = [call["_start"] for call in outer]
        for record in self.nested:
            index = bisect_right(starts, record["_start"]) - 1
            if index < 0:
                continue
            owner = outer[index]
            record["owner"] = owner["id"]
            owner["nested"] = owner.get("nested", 0) + 1
            if "_end" in owner and record["_end"] > owner["_end"]:
                record["kind"] = "background"


def parse(job: tuple[str, str, Window]) -> Parsed | None:
    host, name, window = job
    path = Path(name)
    if host == "claude":
        return ClaudeReader(path, window).read()
    meta = codex_meta(path)
    return CodexReader(path, meta, window).read() if meta else None


def claude_files(root: Path, match: str, oldest: float) -> list[Path]:
    """Matching session files written since OLDEST.

    A file holding a record from the window was last written at or after that
    record's time, so the modification-time test keeps every such session.
    """
    projects = root / "projects"
    if not projects.is_dir():
        return []
    found: list[Path] = []
    for folder in sorted(projects.iterdir()):
        if match in folder.name and folder.is_dir():
            found += sorted(folder.glob("*.jsonl"))
            found += sorted(folder.glob("*/subagents/*.jsonl"))
    return [path for path in found if path.stat().st_mtime >= oldest]


def codex_files(root: Path, match: str, oldest: float) -> tuple[list[Path], int]:
    """Matching rollouts, one per session id, and how many copies were dropped."""
    copies: dict[str, list[Path]] = {}
    for store in ("sessions", "archived_sessions"):
        for path in sorted((root / store).rglob("rollout-*.jsonl")):
            if path.stat().st_mtime < oldest:
                continue
            meta = codex_meta(path)
            if meta is None or match not in str(meta.get("cwd", "")):
                continue
            copies.setdefault(meta.get("id") or path.stem, []).append(path)
    chosen = [max(paths, key=lambda p: p.stat().st_size) for paths in copies.values()]
    dropped = sum(len(paths) - 1 for paths in copies.values())
    return sorted(chosen), dropped


def default_match() -> str:
    try:
        common = subprocess.run(
            ["git", "-C", str(ROOT), "rev-parse", "--git-common-dir"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return ROOT.name
    common_dir = Path(common) if Path(common).is_absolute() else ROOT / common
    return common_dir.absolute().parent.name


def day(text: str) -> date:
    """A YYYY-MM-DD date, or Nd for N days before today."""
    if re.fullmatch(r"\d+d", text):
        return datetime.now(timezone.utc).date() - timedelta(days=int(text[:-1]))
    try:
        return date.fromisoformat(text)
    except ValueError:
        raise argparse.ArgumentTypeError(
            f"{text!r} is not a date: give YYYY-MM-DD or Nd, for example 14d"
        ) from None


def window_of(since: date | None, until: date | None) -> Window:
    after = until + timedelta(days=1) if until else None
    return Window(
        since.isoformat() if since else "", after.isoformat() if after else ""
    )


def run_jobs(
    jobs: list[tuple[str, str, Window]], workers: int
) -> Iterator[Parsed | None]:
    if workers <= 1:
        yield from map(parse, jobs)
        return
    with ProcessPoolExecutor(workers) as pool:
        yield from pool.map(parse, jobs, chunksize=4)


def strip_private(record: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in record.items() if not key.startswith("_")}


def write_digest(out: Path, results: Iterable[Parsed | None]) -> dict[str, Any]:
    totals: Counter[str] = Counter()
    invalid: dict[str, int] = {}
    files = {name: (out / f"{name}.jsonl").open("w") for name in TABLES}
    try:
        for parsed in results:
            if parsed is None:
                continue
            if parsed.invalid:
                invalid[parsed.session["path"]] = parsed.invalid
            write_parsed(files, parsed, totals)
    finally:
        for handle in files.values():
            handle.close()
    return {"totals": dict(totals), "invalid_lines": invalid}


def write_parsed(files: dict[str, Any], parsed: Parsed, totals: Counter[str]) -> None:
    host = parsed.session["h"]
    rows = {"sessions": [parsed.session], "calls": parsed.calls}
    rows.update(users=parsed.users, finals=parsed.finals)
    for name, records in rows.items():
        for record in records:
            files[name].write(json.dumps(strip_private(record), ensure_ascii=False))
            files[name].write("\n")
    totals[f"{host} sessions"] += 1
    totals[f"{host} calls"] += parsed.session["calls"]
    totals[f"{host} commands"] += parsed.session["cmds"]
    totals[f"{host} failed"] += parsed.session["failed"]


def build(args: argparse.Namespace) -> int:
    since = args.since or day("14d")
    window = window_of(since, args.until)
    oldest = datetime.combine(since, datetime.min.time(), timezone.utc).timestamp()
    match = args.match or default_match()
    claude = claude_files(args.claude_root, match, oldest)
    codex, dropped = codex_files(args.codex_root, match, oldest)
    jobs = [("claude", str(p), window) for p in claude]
    jobs += [("codex", str(p), window) for p in codex]
    out: Path = args.out
    out.mkdir(parents=True, exist_ok=True)
    (out / "meta.json").unlink(missing_ok=True)
    summary = write_digest(out, run_jobs(jobs, args.jobs))
    meta = {"schema": SCHEMA, "since": since.isoformat(), "match": match}
    meta.update(until=args.until.isoformat() if args.until else "", built=now_iso())
    meta.update(files=len(jobs), codex_copies_dropped=dropped, **summary)
    (out / "meta.json").write_text(json.dumps(meta, indent=1) + "\n")
    report_build(out, meta)
    return 0


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def report_build(out: Path, meta: dict[str, Any]) -> None:
    window = f"{meta['since']} .. {meta['until'] or 'now'}"
    print(f"digest {out} (schema {SCHEMA}, {window}, match {meta['match']!r})")
    print(
        f"{meta['files']} files read; {meta['codex_copies_dropped']} duplicate Codex copies skipped"
    )
    for host in ("claude", "codex"):
        t = meta["totals"]
        print(
            f"{host}: {t.get(f'{host} sessions', 0)} sessions, "
            f"{t.get(f'{host} calls', 0)} calls, "
            f"{t.get(f'{host} commands', 0)} nested commands, "
            f"{t.get(f'{host} failed', 0)} failed"
        )
    invalid = meta["invalid_lines"]
    print(
        f"invalid JSON lines skipped: {sum(invalid.values())} in {len(invalid)} files"
    )
    for path, count in sorted(invalid.items(), key=lambda item: -item[1])[:5]:
        print(f"  {count} {path}")
    print("Private text: keep this digest out of Git, test fixtures and CI artifacts.")
    print("Next: python3 tools/transcripts.py --help lists the queries.")


class DigestError(Exception):
    pass


def load_meta(folder: Path) -> dict[str, Any]:
    rebuild = f"python3 tools/transcripts.py digest --since YYYY-MM-DD --out {folder}"
    meta_path = folder / "meta.json"
    if not meta_path.is_file():
        raise DigestError(f"no complete digest at {folder}. Build one: {rebuild}")
    meta = json.loads(meta_path.read_text())
    if meta.get("schema") != SCHEMA:
        since = meta.get("since") or "YYYY-MM-DD"
        raise DigestError(
            f"the digest at {folder} has schema {meta.get('schema')}, and this tool "
            f"reads schema {SCHEMA}. Rebuild it: python3 tools/transcripts.py "
            f"digest --since {since} --out {folder}"
        )
    return meta


def rows(folder: Path, table: str) -> Iterator[dict[str, Any]]:
    with (folder / f"{table}.jsonl").open() as lines:
        for line in lines:
            yield json.loads(line)


def in_scope(record: dict[str, Any], args: argparse.Namespace, window: Window) -> bool:
    return window.has(record.get("t", "")) and (
        not args.host or record["h"] == args.host
    )


def is_leaf(record: dict[str, Any]) -> bool:
    return record["kind"] != "call" or not record.get("nested")


def kind_matches(record: dict[str, Any], kind: str) -> bool:
    if kind == "all":
        return True
    if kind == "leaf":
        return is_leaf(record)
    if kind == "background":
        return record["kind"] == "background" or bool(record.get("bg"))
    return record["kind"] == kind


def select_calls(
    folder: Path, args: argparse.Namespace, window: Window
) -> list[dict[str, Any]]:
    pattern = re.compile(args.regex, re.S)
    return [
        record
        for record in rows(folder, "calls")
        if in_scope(record, args, window)
        and kind_matches(record, args.kind)
        and (not args.tool or record["n"] == args.tool)
        and (not args.failed or record.get("e"))
        and pattern.search(record["c"])
    ]


def shown(count: int, limit: int) -> int:
    return count if limit <= 0 else min(count, limit)


def call_heading(record: dict[str, Any]) -> str:
    flags = f"failed={bool(record.get('e'))} exit={record.get('x')}"
    owner = f" in {record['owner']}" if record.get("owner") else ""
    return (
        f"--- {record['h']} {record['s']} {record['t'][:19]} {record['kind']} "
        f"{record['n']} {flags}{owner} [{record['path']}:{record['ln']}]"
    )


def query_calls(folder: Path, args: argparse.Namespace, window: Window) -> None:
    hits = select_calls(folder, args, window)
    for record in hits[: shown(len(hits), args.limit)]:
        print(call_heading(record))
        print(record["c"] if args.full else record["c"][:400])
        if args.full and "r" in record:
            print(f"=> ({record['rl']} chars) {record['r']}")
    print(f"[{len(hits)} hits, showed {shown(len(hits), args.limit)}]")


def query_count(folder: Path, args: argparse.Namespace, window: Window) -> None:
    hits = select_calls(folder, args, window)
    failed = sum(bool(r.get("e")) for r in hits)
    print(f"total {len(hits)} failed {failed} sessions {len({r['s'] for r in hits})}")
    print("by host", dict(Counter(r["h"] for r in hits)))
    print("by day", dict(sorted(Counter(r["t"][:10] for r in hits).items())))


def is_machine(text: str) -> bool:
    text = text.lstrip()
    return text.startswith(MACHINE_PREFIXES) or bool(ATTRIBUTION.search(text))


def delegated_sessions(folder: Path) -> set[str]:
    """Sessions an agent started: subagents, and those a machine message opened."""
    return {
        s["s"] for s in rows(folder, "sessions") if s["sub"] or is_machine(s["first"])
    }


def print_messages(
    folder: Path,
    table: str,
    args: argparse.Namespace,
    window: Window,
    keep: Callable[[dict[str, Any]], bool],
) -> None:
    pattern = re.compile(args.regex or "", re.S)
    count = 0
    for record in rows(folder, table):
        if (
            in_scope(record, args, window)
            and keep(record)
            and pattern.search(record["m"])
        ):
            count += 1
            if args.limit <= 0 or count <= args.limit:
                text = record["m"] if args.full else record["m"][:800]
                print(
                    f"--- {record['h']} {record['s']} {record['t'][:19]} [{record['path']}:{record['ln']}]"
                )
                print(text)
    print(f"[{count} hits]")


def query_users(folder: Path, args: argparse.Namespace, window: Window) -> None:
    print_messages(folder, "users", args, window, lambda r: True)


def query_finals(folder: Path, args: argparse.Namespace, window: Window) -> None:
    print_messages(folder, "finals", args, window, lambda r: True)


def query_humans(folder: Path, args: argparse.Namespace, window: Window) -> None:
    print(
        "Heuristic candidates for human-typed messages: not in a subagent session "
        "or one a machine-written message opened, no machine-written prefix, no "
        "attribution line. The transcripts do not record who typed a message."
    )
    delegated = delegated_sessions(folder)
    print_messages(
        folder,
        "users",
        args,
        window,
        lambda r: r["s"] not in delegated and not is_machine(r["m"]),
    )


def matching_sessions(folder: Path, prefix: str) -> list[dict[str, Any]]:
    return [s for s in rows(folder, "sessions") if s["s"].startswith(prefix)]


def query_session(folder: Path, args: argparse.Namespace, window: Window) -> None:
    found = matching_sessions(folder, args.regex)
    if len(found) != 1:
        ids = ", ".join(s["s"] for s in found[:10])
        raise DigestError(
            f"{len(found)} sessions start with {args.regex!r}{': ' + ids if ids else ''}. "
            "Give a longer prefix, or list them: python3 tools/transcripts.py sessions"
        )
    session = found[0]
    print(session_line(session) + f" [{session['path']}]")
    items = []
    for table, tag in (("calls", "CALL"), ("users", "USER"), ("finals", "FINAL")):
        for record in rows(folder, table):
            if record["s"] == session["s"] and window.has(record["t"]):
                items.append((record["t"], tag, record))
    items.sort(key=lambda item: item[0])
    for _, tag, record in items[: shown(len(items), args.limit)]:
        print(session_item(tag, record, args.full))
    print(f"[{len(items)} rows, showed {shown(len(items), args.limit)}]")


def session_item(tag: str, record: dict[str, Any], full: bool) -> str:
    if tag != "CALL":
        body = record["m"] if full else record["m"][:500]
        return f"--- {record['t'][:19]} {tag}\n{body}"
    body = record["c"] if full else record["c"][:500]
    if full and "r" in record:
        body += f"\n=> {record['r']}"
    flags = f" failed exit={record.get('x')}" if record.get("e") else ""
    return f"--- {record['t'][:19]} {record['kind']} {record['n']}{flags}\n{body}"


def session_line(session: dict[str, Any]) -> str:
    model = session["model"] + (
        f"/{session['effort']}" if session.get("effort") else ""
    )
    sub = f" sub of {session['parent'] or '?'}" if session["sub"] else ""
    return (
        f"{session['start'][:16]} {session['h']} {session['s']} {model} "
        f"calls={session['calls']} cmds={session['cmds']} "
        f"failed={session['failed']}{sub} | {' '.join(session['first'].split())[:100]}"
    )


def query_sessions(folder: Path, args: argparse.Namespace, window: Window) -> None:
    found = [
        s
        for s in rows(folder, "sessions")
        if (not args.host or s["h"] == args.host)
        and (not window.since or s["end"] >= window.since)
        and (not window.until or s["start"] < window.until)
    ]
    found.sort(key=lambda s: s["start"])
    for session in found[: shown(len(found), args.limit)]:
        print(session_line(session))
    print(f"[{len(found)} sessions, showed {shown(len(found), args.limit)}]")


QUERIES: dict[str, Callable[[Path, argparse.Namespace, Window], None]] = {
    "calls": query_calls,
    "count": query_count,
    "users": query_users,
    "finals": query_finals,
    "humans": query_humans,
    "session": query_session,
    "sessions": query_sessions,
}


def run_query(args: argparse.Namespace) -> int:
    folder: Path = args.digest
    try:
        meta = load_meta(folder)
        window = window_of(args.since, args.until)
        if window.since and window.since < meta["since"]:
            print(
                f"note: the digest starts at {meta['since']}; rebuild with an earlier "
                "--since to see older records",
                file=sys.stderr,
            )
        QUERIES[args.command](folder, args, window)
    except DigestError as error:
        print(f"transcripts.py: {error}", file=sys.stderr)
        return 2
    return 0


def query_options() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--since", type=day, help="first day, YYYY-MM-DD or Nd")
    common.add_argument("--until", type=day, help="last day, inclusive")
    common.add_argument("--host", choices=("claude", "codex"))
    common.add_argument(
        "--limit", type=int, default=40, help="rows to print; 0 prints all"
    )
    common.add_argument(
        "--full", action="store_true", help="print full text; results stay clipped"
    )
    common.add_argument(
        "--digest", type=Path, default=DEFAULT_DIGEST, help="digest folder to read"
    )
    return common


def call_filters(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("regex", help="Python regex over the call input or command")
    parser.add_argument("--tool", help="exact tool name, such as Bash or exec_command")
    parser.add_argument("--failed", action="store_true", help="failed records only")
    parser.add_argument(
        "--kind", choices=KINDS, default="leaf", help="record kind (default leaf)"
    )


def add_queries(
    commands: argparse._SubParsersAction, common: argparse.ArgumentParser
) -> None:
    calls = commands.add_parser(
        "calls", parents=[common], help="tool calls and commands matching REGEX"
    )
    call_filters(calls)
    count = commands.add_parser(
        "count", parents=[common], help="totals, sessions, by day and by host"
    )
    call_filters(count)
    for name, text in (
        ("users", "user-role messages matching REGEX"),
        ("finals", "final assistant messages matching REGEX"),
    ):
        commands.add_parser(name, parents=[common], help=text).add_argument("regex")
    humans = commands.add_parser(
        "humans", parents=[common], help="heuristic candidates for human-typed messages"
    )
    humans.add_argument("regex", nargs="?", default="")
    session = commands.add_parser(
        "session", parents=[common], help="one session's records in time order"
    )
    session.add_argument("regex", metavar="ID_PREFIX")
    commands.add_parser(
        "sessions", parents=[common], help="sessions with model, start, calls, failures"
    )


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(
        description=__doc__,
        epilog=HELP_EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    commands = top.add_subparsers(dest="command", required=True)
    digest = commands.add_parser(
        "digest",
        help="build the digest the queries read",
        epilog=HELP_EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    digest.add_argument("--since", type=day, help="first day (default 14d)")
    digest.add_argument("--until", type=day, help="last day, inclusive")
    digest.add_argument("--out", type=Path, default=DEFAULT_DIGEST)
    digest.add_argument("--claude-root", type=Path, default=Path.home() / ".claude")
    digest.add_argument("--codex-root", type=Path, default=Path.home() / ".codex")
    digest.add_argument(
        "--match",
        help="substring of a Claude project folder name or Codex working "
        "directory that selects this project's sessions (default: the main "
        "checkout's directory name)",
    )
    digest.add_argument("--jobs", type=int, default=min(8, os.cpu_count() or 1))
    add_queries(commands, query_options())
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.command == "digest":
        return build(args)
    return run_query(args)


if __name__ == "__main__":
    sys.exit(main())
