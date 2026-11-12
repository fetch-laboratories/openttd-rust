---
name: process-review
description: Review agent transcripts for a question the user asks about the project's process, with counts, and propose fixes. Proposal-only unless the user asks for implementation.
disable-model-invocation: true
---

# Review the agents' process

User question and window: $ARGUMENTS (in Codex, any text sent with `$process-review`). The window defaults to the last 14 days. Answer a focused question first, in plain words, before anything broader.

Attribution on anything you write to GitHub:
`Agent: /process-review (<host>) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## 1. Build the digest

- `python3 tools/transcripts.py digest --since DATE` reads both hosts' sessions in every worktree. Read `python3 tools/transcripts.py --help` for the queries and record kinds.
- Use the queries for counts. Read a raw transcript, named in each record's `path` and `ln`, only to confirm a pattern.
- The digest is private: keep it and quotes from it out of Git, issues, fixtures and CI artifacts. Quote session IDs and counts instead.

## 2. Find what counts

A finding needs all of these:

- it recurs across sessions, not one agent's habit;
- it has a cost: calls, retries, wall time, a refused push, a correction from the user or root;
- it still occurs in the last three days of the window;
- nothing existing covers it. Check the tools' `--help` and the docs AGENTS.md routes to. When a tool exists and agents do not use it, the finding is discoverability;
- a mechanical fix exists.

Each finding carries its count, the query that produced it, and two to four session IDs.

Choose lenses by the question. When the window is large, give each lens its own reviewer:

- inline programs agents rewrite;
- GitHub and CI calls;
- git, worktree and build environment;
- failed calls and what followed;
- spawning, messaging and waiting;
- corrections from the user and between agents.

Counts from different lenses overlap: do not add them into a promised saving.

## 3. Propose

Prefer, in this order: a clearer message or a flag on a tool agents already run; a new subcommand on such a tool; a git hook; a doc line; a host hook or a new skill. Search open issues for the same proposal before making it, and cite it when found.

Stop at proposals unless the user asks for implementation. Then:

1. Have the proposals critiqued by the independent review model in the [role table](../../team.md#roles-and-models) before building.
2. Build in a worktree under `.local/worktrees/`, without pushing.
3. Report what was cut and why.

## 4. Report

- The answer to the user's question first.
- Findings, each with its numbers, query and session IDs.
- What was proposed, and what already existed.
- What only the user can change: host settings, the user-invoked skills, T3.
