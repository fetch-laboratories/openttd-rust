---
status: accepted
may change: agents; a change that raises paid CI cost needs the user's approval (ADR 0008)
---
# Every merge goes through the merge queue

Before 2026-11-10, merging was done by hand. Each change had to be brought up to date with the main branch and tested again, and any other merge in between made it start over. An agent measured the cost: from the last review to the merge took 19 to 60 minutes, of 53 Full CI runs that did not pass, 14 were cancelled or refused only because the change or the branch had moved, and hand-built batches of changes took 45 to 60 minutes ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860)).

Every merge goes through GitHub's merge queue, which tests each change on top of the current branch and merges it if it passes. Nobody merges by hand or with an admin override.

The merge queue was the first of the nine changes an agent proposed on 2026-11-10, which the user approved ("i'm willing to give these a try"). The queue removed the integrator agent and the hand-built batches ([ADR 0012](0012-root-decides-implementers-review-and-merge.md)). An admin override would merge a change that the queue has not tested on the current branch. [docs/root-workflow.md](../root-workflow.md) holds the procedure.

The queue's own test runs cost paid CI minutes: on 2026-11-10 they used 1,256 paid Blacksmith minutes. How to turn the queue off is recorded on #860.
