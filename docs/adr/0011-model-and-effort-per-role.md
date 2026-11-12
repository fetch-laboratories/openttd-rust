---
status: accepted
may change: user
---
# Each agent role has a fixed model and effort level

The fork runs agents from two providers, Anthropic (Claude Code) and OpenAI (Codex), each with several models and an effort setting that trades cost for care. Which model a task gets decides both its quality and how fast the user's usage allowance runs out: on 2026-11-07 a Codex root took the weekly Codex allowance from 1% to 81% in 15 hours ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860)).

Each agent role has a fixed model and effort level, listed in a table in [docs/team.md](../team.md). Effort follows the role, not root's own setting. A restarted root reuses the previous root's provider, model and settings unless the user says otherwise, and asks the user if it cannot recover them. Reviewers get no priority tier. Planning runs on Fable 5.2 at medium effort.

These are the user's choices. On 2026-11-10 the user answered an agent's questions about the table: "no priority tier for reviewers", and "replace luna medium with terra high. otherwise, looks good". On root's settings they wrote: "if I don't provide a model then it's almost certainly because I forgot. there are a lot of factors that go into why i choose a certain model to run root at a certain time." On 2026-11-11 they wrote: "use fable medium for planning from now on rather than fable high".

A change spends the user's money and usage allowance. No figure is recorded beyond the 81% above.

## History

On 2026-11-10 the user wrote that Codex "should only do reviews which are then fixed by opus implementers". Later that day they approved a table that lets either provider run root and fill every role. Planning ran at high effort until 2026-11-11.
