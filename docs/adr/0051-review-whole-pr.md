---
status: accepted
may change: agents
---
# A pull request is reviewed whole, by two reviewers and one re-check

A pull request is reviewed by separate agents before it merges ([ADR 0032](0032-issue-branch-pr-review-merge.md)), in at most two stages ([ADR 0025](0025-two-review-stages.md)). From 2026-10-20 a large one was also split into review groups of about 1,000 lines, each with its own reviewer, and from 2026-11-13 each group had two reviewers, one comparing behaviour with the original and one checking the porting rules. Nobody measured the group size. It followed one port, #303, whose single reviewer read 6,985 lines and missed three behaviour differences, and was then carried to cleanup passes, which change types, names and structure. The two rules multiplied: #1000 had eight first reviewers, four re-checks and five later reports. The user wrote on 2026-11-24: "I don't think there's really any reason to have 8 separate reviewers on a single PR."

The 37th steering review measured it that day. It checked against the code the 339 findings in the review reports of 14 of the 16 pull requests merged from 2026-11-14 to 2026-11-19, and of one still open. 329 were true, and none changed the state of a game. Replayed on #1000 as first reviewed (5,806 changed lines), two reviewers reading the whole pull request found 17 and 23, in two runs, of the 28 serious and medium defects the eight had found, and both runs found a design fault the eight had missed. The reports after a re-check found nothing serious. Fixing review findings had come to take implementers 334 active minutes against 363 to build the same ten pull requests.

A pull request has two first reviewers whatever its size, one for behaviour and one for porting, and is not split into groups. One that changes no game code has the behaviour reviewer alone. One re-check of the fixing commits is the last review: what the implementer fixes after it is not reviewed.

An agent made this decision: the 37th steering review, on the user's question. [docs/workflow.md](../workflow.md#review) holds the working rule.

The cost is what six more reviewers would have found: on #1000, between 5 and 11 of 28 serious and medium defects, left for the cleanup's closing sweep. Independent audits found 8 defects in the 3,091 lines of `road.rs` after #1000's 17 reports and 4 in 2,149 changed lines after three reports each on #1005 and #1019, so more reports had not left less behind. A fix made after the re-check is caught only by CI or a later pass. No port has been reviewed this way; the first one is measured before the rule is relied on there.

## History

Review groups stood from 2026-10-20 to 2026-11-24, counted in original lines for ports and, from 2026-11-07, in changed lines for cleanup passes. Two reviewers for each group stood from 2026-11-13 to 2026-11-24. From 2026-11-18 to 2026-11-24 a fix made after the re-check got a fresh reviewer.
