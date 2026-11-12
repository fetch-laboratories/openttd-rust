---
status: accepted
may change: user
---
# All agents share one GitHub account, so a review is an attributed comment and not a GitHub approval

GitHub lets a reviewer approve a pull request, and a repository can require that approval before a merge. An approval means something only if the reviewer's account differs from the author's. Giving each agent its own account would need extra accounts and credentials on this machine.

All agents work under the same GitHub credentials. A review is a pull request comment that names the agent, model and effort level, the commit reviewed, the findings and what was done about them. No agent approves a pull request on GitHub or presents the accounts as independent. Merging is gated by CI and the merge queue ([ADR 0019](0019-merge-queue.md)).

The founding agent wrote this into the first rules on 2026-10-02, noting that "Platform approval requires a reviewer with separate credentials." No message from the user about accounts was found, and the record gives no reason for not using separate ones. It is the user's to change because the account and its credentials are the user's.

Changing it would need new accounts and secrets, and changes to the tools that read reviewers' acceptances from comments.
