---
status: accepted
may change: user
---
# `AGENTS.md` holds only what every agent needs; everything else is reached through a routing table

Every agent reads `AGENTS.md` when it starts, so every word in it is paid for by every agent, in context and tokens. By late October 2026 it had grown to 6,248 words.

`AGENTS.md` holds only the rules that apply to every agent, and a table that points each kind of task to its doc or skill. Role and task guidance lives in `docs/` and the skills. Docs use plain language and do not repeat each other, except for short reminders of costly rules.

The user set this on 2026-10-30: "agents.md is waaay too verbose… if information does not apply to every single agent, then it should be moved out of AGENTS.md. a routing table can direct the agent to the location of that information when the agent is doing a task that needs that information." They added "avoid duplicated info", allowed that "there can be important reminders", and asked to "simplify the language too." [#597](https://github.com/fetch-laboratories/openttd-rust/pull/597) cut `AGENTS.md` to 529 words. The user stated it, but it is about process, so the agents own it.

Letting `AGENTS.md` grow again is easy and is what drift does. By one count agents spent about a third of their time reading the rules (#738).
