---
status: accepted
may change: agents
---
# Root commits the roadmap only at a stocktake, a change of course or wind-down

The roadmap is the plan of work, in `docs/roadmap.md`. Docs changes are pushed straight to the main branch ([ADR 0043](0043-docs-only-changes-need-no-pr.md)). Each push leaves every open pull request behind the main branch, and with the merge queue ([ADR 0019](0019-merge-queue.md)) each push makes the queue rebuild every change waiting in it. During root's session of 2026-11-08 to 11-09, 18 of the 28 direct commits to the main branch changed only the roadmap ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860)).

Root commits the roadmap only at a stocktake, at a change of course and at wind-down, which is when the user pauses the project. The resume checkpoint is written only at wind-down.

This was the sixth of the nine changes an agent proposed on 2026-11-10, which the user approved.

More roadmap pushes would cost CI minutes and time waiting in the queue.
