---
status: accepted
may change: user
---
# Every change goes through an issue, a branch, a PR, a separate reviewer and a merge, with each agent's model named

An agent can commit straight to the main branch. That is quick, but it leaves no record of why and no second look. On the first day the user asked for the habits of a production team instead.

Substantive work starts from a GitHub issue, is done on its own branch, and arrives as a pull request (PR) that a separate agent reviews before it merges. Every issue, PR, comment and review an agent writes names the agent, its model and its effort level. The fork may depart from upstream's repository conventions where that helps agents, for example with `AGENTS.md`, skills and stricter lint checks. Docs-only changes are the exception ([ADR 0043](0043-docs-only-changes-need-no-pr.md)).

The user wrote on 2026-10-02: "I've never worked on a large team in a production environment, but I'd like this project treated as such. use of GH issues, PR's, and whatever else is best practice. for example, a subagent files a PR, you assign a review agent". The same evening they added that "agents should include their model and effort level in GH comments, issues, PR's, etc.", and, because upstream does not accept AI work, "you can deviate from the original in some aspects". ADRs [0012](0012-root-decides-implementers-review-and-merge.md), [0019](0019-merge-queue.md) and [0025](0025-two-review-stages.md) refine parts of this.

Reversing it would mean rebuilding every tool, template and role around a different flow.
