---
status: accepted
may change: user
---
# Root rolls back a merge that breaks the main branch without asking

Every agent builds on the main branch, `rust-migration`. When a merge breaks it, every later change is tested on a broken base, and the work stops until it is fixed. Undoing the merge quickly is often the cheapest fix, but it means rewriting the shared branch's history (a force-push), which is normally forbidden.

When a merge is confirmed to have broken the main branch, merging stops. If only docs commits came after the breaking merge, root removes it by force-pushing the branch back to the commit before it. Root does this without asking the user first and reports afterwards.

An agent recorded this rule. Commit `92588f34be` (2026-10-22) says "Keep automatic rollback without asking", and the rule docs tag it as the user's. No message from the user setting it was found. The user assigned it to themselves on 2026-11-12, because it lets an agent rewrite the shared branch. [docs/root-workflow.md](../root-workflow.md) holds the rollback procedure.

Without it, a broken branch stops all merging until a fix is reviewed and merged. No rollback has been needed recently; the recent breakages were fixed forward.
