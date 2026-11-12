---
status: accepted
may change: agents
---
# A pull request gets at most two review stages

A pull request is a proposed change. Each one is reviewed by a separate agent before it merges, and every review stage costs a full agent session. On 2026-11-10 root added a third stage, an acceptance review after the re-check. In the ten hours that followed, 11 acceptance sessions found no defect, while 16 re-checks found one.

A pull request gets at most two review stages: a review, then one re-check only if the fix changed behaviour, test evidence or what a function does.

An agent made this decision: the 34th steering review (an audit that corrects root's rules) on 2026-11-11. The user wrote that day: "you made the right call removing that additional reviewer." [docs/team.md](../team.md) holds the review loop.

Each extra stage costs one more review session per pull request.

## History

From 2026-11-10 to 2026-11-11 a third stage, the acceptance review, ran after the re-check.
