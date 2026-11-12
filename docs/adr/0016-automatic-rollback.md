---
status: accepted
may change: user
---
# Root rolls back a merge that breaks the main branch without asking

Every agent builds on the main branch, `rust-migration`. When a merge breaks it, every later change is tested on a broken base, and the work stops until it is fixed. Undoing the merge quickly is often the cheapest fix, but it means rewriting the shared branch's history (a force-push), which is normally forbidden.

When a merge is confirmed to have broken the main branch, merging stops. If only docs commits ([ADR 0043](0043-docs-only-changes-need-no-pr.md)) came after the breaking merge, root removes it by force-pushing the branch back to the commit before it. Root does this without asking the user first and reports afterwards.

On 2026-10-22 a merge (#347) broke the branch and wind-down spent about three hours on it. The user asked: "why didn't you fix it by removing 347 and allowing the next agent to fix it?" The same day an agent wrote the force-push procedure (commit `2e3e42691f`). [docs/root-workflow.md](../root-workflow.md) holds the rollback procedure.

Without it, a broken branch stops all merging until a fix is reviewed and merged.
