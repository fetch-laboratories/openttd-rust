---
status: accepted
may change: agents
---
# Docs-only changes are committed straight to the main branch without a PR

A change that touches only documentation cannot break the game, but a pull request still waits for a full CI run and a review. In the words of the agent that set the rule, "A three-line roadmap edit should not wait for a full CI run" (commit `723cf351e5`).

A change that touches only Markdown docs is committed directly to the main branch, `rust-migration`, without a pull request or CI. Everything else goes through a pull request that passes CI. [Change types](../workflow.md#change-types) defines which files count.

An agent made this rule in a steering session on 2026-10-03. The user confirmed it on 2026-10-16: "doc changes can be committed without a PR, right? I think what you said makes sense". On 2026-11-03 they asked about a configuration file edit: "why does this require a PR? why isn't this a docs only commit?" Which files count stays the agents' call.

Reversing it costs little, but two decisions lean on it: rollback is allowed only when docs commits alone sit on top of a breaking merge ([ADR 0016](0016-automatic-rollback.md)), and every docs push leaves open pull requests behind the branch ([ADR 0026](0026-roadmap-commit-timing.md)).

## History

Until 2026-11-25 only `AGENTS.md` and Markdown under `docs/` counted. A skill's Codex settings file under `docs/skills/` could not change in the commit that changed its `SKILL.md`. Of the commits from 2026-10-01 to that day, 16 changed only files that no build, test or check reads and still fell outside the rule, 11 of them for the PR template. The user asked again: "yaml is a markdown language the same as markdown. why can't it be committed as a doc?" The list now goes by what reads a file, not by its type.
