---
status: accepted
may change: user
---
# Root takes stock after every 12 merges

A stocktake is root's periodic measurement of progress. It comes with a profile of the game's speed. Each one costs agent-hours and machine time and takes an implementer's slot while it runs. On 2026-11-06 stocktakes ran after every second merge, so a four-line change and a two-line change each triggered a full stocktake and profile.

After every 12 merges, root takes stock: it measures progress and profiles the game's speed.

On 2026-11-06 an agent offered numbers from 2 to 50, and the user wrote: "there's a whole lot of fucking numbers between 2 and 50. Fucking pick one… do 6-12." The agent chose six. The move to twelve is recorded only by an agent: commit `5de4962e1f` says the user "set a twelve-integration cadence (2026-11-08)". The user's own words on twelve were not found. [docs/speed.md](../speed.md) holds the profiling steps.

## History

Every second merge until 2026-11-06, then every sixth, then every twelfth from 2026-11-08. On 2026-11-07 the user also had one stocktake skipped ("skip this stocktake and profile").
