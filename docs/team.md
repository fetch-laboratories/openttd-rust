# Team

Readers: Root and user-invoked skills that spawn agents.

## Roles and models

- Root (`gpt-6.1-sol`, xhigh) orchestrates and delegates heavily, selects components, integrates and spot-checks delegated diffs.
- Use the host's available subagent slots; the repository imposes no lower subagent cap (user, 2026-11-06). Codex's per-session limit is 12 subagents plus root and cannot be removed. Existing sessions and worktrees may retain a lower configuration; respect the actual host limit. The main checkout's uncommitted `.codex/config.toml` change belongs to the user and remains untouched. Agent slots and the six unintegrated component branches in [integration](workflow.md#integration) are different limits: reviews, planning and measurements need no new component branch.
- Specify model and effort on every spawn. Each subagent uses less effort than root's xhigh.
- Use `gpt-6.1-sol` for most implementation and analysis and all independent review. Use high effort unless a clearly bounded task justifies lower.
- Use `gpt-6-luna` at low or medium effort for bounded mechanical work.
- Use `gpt-6-astra` at high effort for all delegated planning.

## Spawning

- Spawn a fresh agent named for each task; its name is its attribution. Do not reassign a finished agent to unrelated work. Close it when its task ends.
- Each review round, including re-review, gets a fresh reviewer who reads the PR's earlier attributed review reports.
- Follow the host's orchestration rules: use native agents for compatible shared-checkout work completed in the same turn; use T3 delegation for background work, another provider or an isolated checkout. T3 creates and binds isolated checkouts. Follow its completion notifications and returned next action; do not poll a running delegate.
- At the host slot limit, root works its own queue (integration, spot-checking fix commits, roadmap) until a slot frees. Do not archive threads or edit host databases to free slots.
- A quiet-host hold lasts only through timed games. At an independent wake while holding other agents, inspect the named job of a child already known to be idle; release the hold as soon as the measurement ends, before report preparation. A failed or declined peripheral host action does not cancel unrelated authorized work.
- A long-job brief links the [background rule](../AGENTS.md#every-agent). Root never runs a child's jobs for it. A child names its job ID and uses the host's background-completion mechanism before ending its turn. At each independent wake, root handles completed results and checks jobs of children already known to be idle on them. If a job ended without waking its child, use the host's supported resume action; do not poll a running delegate or duplicate its work. This recovery remains necessary: four missed resumes cost about six agent-hours on 11-02. A child that reports a running job without an ID has not established a resumable handoff; correct it immediately. A subagent waiting only on CI or another task reports and stops.

## Claude Code root

- Claude Code uses its actual host limit under [roles and models](#roles-and-models); the former five-subagent repository cap is lifted.
- A user-directed session outside Codex names its exact model and host-reported effort. When it runs root, retain the same structure with its host's models: subagents at lower effort than root and a fresh reviewer per round.
- Claude Code root uses its host's models for implementation and planning, and `gpt-6.1-sol` high through T3 `delegate_task` for independent review, so reviewer and implementer differ in model family.
- Opus handles implementation, fixes and code-touching follow-ups; Sonnet handles only non-code work such as runs, measurements and tracker edits (user, 2026-10-27). Opus writes code at medium effort; use high for judgment such as planning, speed attribution and hard fidelity analysis (user, 2026-10-30).
- Spawn each code-touching agent through T3 `delegate_task` with `isolation: "worktree"`. T3 creates the worktree from root's commit, binds the shell to it and removes it when its PR closes. The agent names its branch and opens its PR there. Root makes worktrees by hand under `.local/worktrees` only for its own small PRs and reviews.

## Provider handoff

For unattended handoff, establish an independently executable takeover before the source provider exhausts its allowance. A scheduled prompt is not a fallback if it needs that provider to launch its replacement. Verify the host's actual execution provider; a model name in a prompt is insufficient. If the host cannot provide independent takeover, record the limitation and preserve a checkpoint before stopping. This rule creates no schedule.

## Skills

`docs/skills/` holds four user-invoked skills: `/navigate` (macro course review of the goal and phases, before `/steer`), `/steer` (steering review), `/start-development` (root's restart prompt), and `/wind-down` (the user's message pausing root). Two automatically discoverable skills support selected work: `brief-storage-owner` prepares a pool/map ownership brief or checkpoint handoff, and `review-evidence-contract` examines production-path evidence for a port claim or changed fixture/oracle. Their descriptions define when they apply; they add no universal review gate. Steering reviews put corrections in the roadmap and edit `/start-development` only to improve it. Only steering and navigation reviews edit the four orchestration skills unless the user requests an edit; root otherwise follows them and leaves them unedited.
