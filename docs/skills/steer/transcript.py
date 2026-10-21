#!/usr/bin/env python3
"""Print the conversation text of this project's Claude Code and Codex sessions.

Usage: transcript.py                     list Claude Code sessions, newest first
       transcript.py --codex             list top-level Codex sessions, newest first
       transcript.py <id|path> [--tools] print one session's user and assistant
                                         text; --tools adds a line per tool call

An id may be a unique prefix. A Codex id is the session id that ends a rollout
file name. Codex list times are local; printed timestamps are UTC.
"""

import itertools
import json
import sys
from collections import Counter
from pathlib import Path

CLAUDE = Path.home() / ".claude/projects/-home-fetch-projects-openttd-rust"
CODEX = Path.home() / ".codex/sessions"
CODEX_PREAMBLE = ("# AGENTS.md", "<environment_context>", "<skill>")


def records(path):
    for line in path.open():
        try:
            yield json.loads(line)
        except ValueError:
            continue


def tool_line(name, args):
    return " ".join(f"{name}: {args}".split())[:200]


def claude_items(path, tools):
    for d in records(path):
        if d.get("isSidechain") or d.get("type") not in ("user", "assistant"):
            continue
        ts = d.get("timestamp", "")
        c = d["message"].get("content")
        blocks = [{"type": "text", "text": c}] if isinstance(c, str) else c or []
        for b in blocks:
            if b.get("type") == "text":
                yield d["type"], ts, b["text"]
            elif tools and b.get("type") == "tool_use":
                yield "tool", ts, tool_line(b.get("name"), json.dumps(b.get("input")))


def codex_items(path, tools):
    for d in records(path):
        p = d.get("payload") or {}
        if d.get("type") != "response_item":
            continue
        ts = d.get("timestamp", "")
        if p.get("type") == "message" and p.get("role") in ("user", "assistant"):
            parts = p.get("content") or []
            t = "\n".join(c.get("text", "") for c in parts if isinstance(c, dict))
            if p["role"] == "user" and t.startswith(CODEX_PREAMBLE):
                continue
            yield p["role"], ts, t
        elif tools and p.get("type") in ("function_call", "custom_tool_call"):
            args = p.get("arguments") or p.get("input") or ""
            yield "tool", ts, tool_line(p.get("name"), args)


def first_prompt(items):
    for role, _, t in itertools.islice(items, 200):
        if role == "user" and not t.startswith("<") and "task-notification" not in t:
            return t.replace("\n", " ")[:160]
    return ""


def codex_sessions():
    top, agents = [], Counter()
    for p in CODEX.glob("*/*/*/rollout-*.jsonl"):
        with p.open() as f:
            try:
                meta = json.loads(f.readline()).get("payload") or {}
            except ValueError:
                continue
        if "openttd-rust" not in meta.get("cwd", ""):
            continue
        source = meta.get("source")
        if isinstance(source, dict) and "subagent" in source:
            spawn = source["subagent"].get("thread_spawn") or {}
            agents[spawn.get("parent_thread_id")] += 1
        else:
            top.append((p, meta.get("id", "")))
    for p, sid in sorted(top, key=lambda s: s[0].name, reverse=True):
        size = p.stat().st_size // 1024
        prompt = first_prompt(codex_items(p, False))
        print(sid, p.name[8:27], size, "KB", f"+{agents[sid]} agents |", prompt)


def find(arg):
    a = Path(arg)
    if a.exists():
        return a
    matches = list(CLAUDE.glob(f"{arg}*.jsonl"))
    matches += CODEX.glob(f"*/*/*/rollout-*-{arg}*.jsonl")
    if len(matches) != 1:
        sys.exit(f"{len(matches)} sessions match {arg}")
    return matches[0]


tools = "--tools" in sys.argv
args = [a for a in sys.argv[1:] if a != "--tools"]
if not args:
    for p in sorted(
        CLAUDE.glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True
    ):
        print(
            p.stem,
            p.stat().st_size // 1024,
            "KB |",
            first_prompt(claude_items(p, False)),
        )
elif args[0] == "--codex":
    codex_sessions()
else:
    p = find(args[0])
    items = codex_items if p.name.startswith("rollout-") else claude_items
    for role, ts, t in items(p, tools):
        print(
            f"  [tool {ts[:16]}] {t}" if role == "tool" else f"[{role} {ts[:16]}] {t}\n"
        )
