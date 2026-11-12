---
status: accepted
may change: user
---
# Root chooses work and settles questions; implementers run their own reviews and merges

Root is the top agent. It runs at the highest effort level, so each of its turns costs the most. On 2026-11-09, passing messages between other agents took 88 of root's 164 outgoing actions ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860)). On 2026-11-10 and 11-11, watching pull requests, scanning for other agents' jobs and passing messages took about 47% of the two roots' tool calls.

Root spends its turns choosing work and settling questions. It does not watch pull requests, check other agents' jobs or pass messages between agents. Each implementer starts its own reviewer, talks to that reviewer directly, puts its own reviewed change into the merge queue ([ADR 0019](0019-merge-queue.md)) and waits for it to merge. A cheaper agent does the routine chores, such as disk cleanup and the periodic platform CI run.

The user wrote on 2026-11-11: "root should absolutely not be watching PR's. that's such a waste of time and tokens. why can't the implementer watch its own PR?"; and "implementers should start their own reviewers." Direct messages between reviewer and implementer were in the list of changes the user approved on 2026-11-10.

Reversing this would cost root's tokens and slow everything root decides. [docs/team.md](../team.md) holds the working rules.

## History

On 2026-11-08 an integrator agent was placed between implementers and merging, and every message went through root. The merge queue removed the integrator on 2026-11-10. Implementers began starting their own reviewers on 2026-11-11, after the user's correction above.
