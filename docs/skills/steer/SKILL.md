---
name: steer
description: Steering review of root's autonomous migration work since the last steering commit; audits, corrects docs/issues, refreshes /start-development, and can restart root.
disable-model-invocation: true
---

# Steer the OpenTTD-Rust migration

The user runs root for long autonomous sessions, usually `gpt-6.1-sol` xhigh in
Codex (Astra before 2026-10-08) and sometimes Claude Opus in Claude Code, and
runs this review between them in either host. Your
job is to check that root is still on the path to a game whose simulation runs in Rust, find what it
cannot see in its own work, and correct the *system*: rules, roadmap order,
issues and PR comments. Root's team does the implementation. You edit docs,
issues and PR comments, and you leave code changes to root's PR process.

Root carries out the latest steering section's actions within minutes and
literally, and it parks work rather than question a gate. The rules you write
are the strongest force on its next session, for better or worse.

User focus for this run, if any: $ARGUMENTS (in Codex, any text sent with `$steer`)

A focus that names a question (disk, CI, a crashed root, housekeeping) is
answered first, down to its root cause. Read it broadly: housekeeping covers
code, tests and refactoring as well as docs. Tell the user at the start which
of steps 2 and 3 still run, and land the answer through step 4 with what root
does next. A focus that asks you to restart root is handled in step 6.

Attribution on every issue, comment and commit body you write:
`Agent: /root (<host> steering review) | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host">`.

## 1. Load the last steering

1. Find the last steering commit: `git log --format='%h %ad %s' --date=short | grep -i 'Doc: .*Steer'`.
   Its message and the roadmap's "steering review" section state the standing
   corrections.
2. Read the last steering session's transcript, from whichever host ran it.
   `python3 docs/skills/steer/transcript.py` lists Claude Code sessions and
   `transcript.py --codex` lists Codex sessions, newest first. The steering ones
   open with the steer skill or a request to review root, near the steering
   commit's time. Skip unrelated sessions, but read the conclusion
   of any session in between that changed a rule. `transcript.py <id>` prints one session; read
   its final messages first.
3. Read `docs/skills/start-development/SKILL.md` and `AGENTS.md`. Then read
   root's sessions since the steering, in whichever host and model each ran.
   They open with the start-development skill (`$start-development` in Codex,
   `/start-development` in Claude Code). `transcript.py <id> --tools` prints one
   with a line per tool call. Read the user's messages to root, and how it
   ended.
4. The user's messages to root, including the wind-down message, bind that
   session only. They also restate premises (a gate, a speed figure, a PR's
   state); check each against the code before carrying it forward.

Done when you can list each standing correction, the issue that tracks it, and
the review or message that introduced it.

## 2. Survey root's work since then

Run these together:

- `git fetch`, then `git log` from the steering commit to `origin/rust-migration`, and
  `git diff <steer>..origin/rust-migration -- AGENTS.md docs/ .github/`.
  A loosened rule is a red flag unless the user directed it.
- `docs/roadmap.md` in full, including its resume checkpoint.
- CI on `rust-migration` (`gh run list --branch rust-migration`). Note PR checks and
  how many runs were cancelled.
- Open PRs and issues, and the latest review reports on the integrated and pending PRs.
- `python3 tools/port-metrics.py`, plus the PR-reported metrics and speed ratios.
  Before quoting a progress figure, confirm it counts original C++ retired,
  not glue removed.
- `git worktree list` and disk use.
- `python3 docs/skills/steer/issues.py`: every open issue's place in the roadmap,
  unplaced and excluded first, and the full text of each roadmap exclusion line.

Then post a short status line to the user and continue.

Judge each standing correction from the last two steering reviews, your own
rules included, and each rule changed outside a steering review since the last
one (from that diff, its commit messages and any note under the latest steering
review): applied, slipped, reinterpreted, or harmful as written. For each, note
what it cost root (agent hours, CI runs, PRs parked) against what it caught.
A user-directed change found harmful goes back to the user with its evidence;
do not revert it yourself. A change no root session has run under yet has no
effects to judge: copy it, with what to watch, into your new steering review
section so the following review judges it. Trace each problem you find to the rule, roadmap action or user message
that produced it. Relax or remove a harmful rule before adding a new one.

Watch for the patterns root has repeated before:

- **Wheel-spinning**: evidence, tooling or process growing faster than retired game logic.
- **Sediment**: roadmap and docs turning into logs (run IDs, hashes, probe values).
- **Slippage**: a correction quietly reordered behind other work.
- **Template drift**: one port's boundary pattern copied into every later port.
- **WIP creep**: stacked or conflicting branches, and dependency-merge churn.
- **Finite prompts**: the last prompt ended because its listed items ran out.
- **Artificial hurdles**: a gate, budget or ordering that parks work without
  catching defects, including deferrals that wait on each other.
- **Shell workarounds**: subagent commands prefixed with `cd <worktree> &&`,
  hand-made worktrees for implementation agents, blocked `sleep`, polling
  loops, `timeout N bash -c 'sleep'`, or hand-written environment prefixes.
  Each repeat means a spawn, brief or tool is missing a permanent fix.
- **Passed over**: an issue with no place in the roadmap, a component excluded
  without an issue, or an exclusion with no condition that brings it back,
  copied forward review after review. Finished work left open counts too.

Done when every standing correction has a verdict and a cost, and every issue
`issues.py` reports as unplaced or excluded, and every exclusion line it prints,
has a verdict: selected, queued behind a named event, excluded with the
condition that brings it back, or closed. Judge each issue on its merits
against AGENTS.md's priorities and the defects it would catch; ordering a list
is not a verdict.

## 3. Audit independently

The value of this session is an audit that does not depend on root's reviews.
Use the current host's native subagents: Codex subagents when running in Codex,
and Claude Code subagents when running in Claude Code. This skill sets no
provider default and does not require switching model families. Independence
comes from fresh audit agents comparing the source themselves, rather than
relying on root's review conclusions. Launch these in one message as background
subagents. They read and measure, and leave root's worktrees, branches, profiles
and `.local/` output as they found them. A long measurement runs in the
background and the agent ends its turn rather than sleeping (AGENTS.md); an
agent that changes code is spawned with T3 worktree isolation, not briefed to
`cd` into a hand-made worktree.

- **Fidelity**: one per large port integrated since the last steering, or
  about to integrate. Pick the largest by retired lines first, up to four.
- **Targeted**: one or two aimed where the survey shows the largest cost or
  risk this round, for example:
  - *Blockers*: each deferred or parked item, what would lift it, and any
    re-selection condition that waits on other deferred work.
  - *Process*: root's time on CI waits, base updates, reviews and gates against
    its time on ports, attributed to the rules that require it.
  - *Speed*: profile the candidate against the reference on the play saves and
    generate-tgp-256-1, attribute the gap by port, and judge the current speed
    plan against the profile. Re-measure each PR parked on speed at its exact
    head.

Speed results rest on callgrind instruction counts (valgrind is under
`.local/valgrind`), which do not change with other processes on the machine.
Steer cannot stop the user's own work, so a wall-clock median is quoted with
what ran machine-wide during it: the compiler, linker and `openttd` processes
in `ps -eo pid,pcpu,comm,args`, and the 1-minute load average.

Give each agent `AGENTS.md` and the roadmap. A fidelity agent also gets the
worktree, commit and merge base, and the pinned original at
`.local/reference/openttd`, and compares against the original line by line:

- **Ownership**: Rust owns the state and the control loop, and the original body
  sits under `#else` or `#ifndef WITH_RUST`.
- **Fidelity**: the exact count and order of Random draws, integer widths,
  wrapping and saturation, early returns, and the order of side effects.
- **Added fatal paths**: asserts, unwraps, indexing or shifts that can panic where
  the C++ does not.
- **Boundary cost**: the `AGENTS.md` direct-call and typed-boundary rules, and
  per-call allocation on hot paths.
- **Glue**: its share against retired lines.
- **Coverage**: which Random, crash and contention branches the harness reaches.

Each agent reports findings, not narration. Tag findings REACHABLE-DIVERGENCE,
UNREACHABLE, RULE-VIOLATION, COVERAGE-GAP, PERF or HURDLE (a rule or gate that
costs more than it catches), with `file:line` in both Rust and the original
where they apply, and name what it compared. Relay a short summary to the
user as each audit returns. Before acting on a load-bearing claim, such as red CI,
a reachable divergence or a perf attribution, check it yourself.

Done when every audit has reported and every claim you will act on is verified.

## 4. Correct course

Choose a few high-leverage corrections. A systemic pattern counts for more than
a one-off finding. Each correction lands in exactly one authoritative place:

- **Rule change** goes in `AGENTS.md`, phrased as the target behavior.
- **New work** gets an issue: problem with measurements, scope, evidence and
  acceptance. Label it `roadmap` when the roadmap orders it.
- **Finding on a pending PR** goes in a PR comment: what must change before
  integration, and what can follow.
- **New coverage gaps** go in a comment on the coverage tracker (#156).
- **Roadmap**: rewrite "Where the fork stands" and the work order. Add an "Nth
  steering review" section with findings and corrections. `/start-development`
  tells root to apply that section first, so state each action plainly. Fold
  older steering sections into the compact "still in force" paragraph; an
  exclusion keeps its return condition there, and one without a condition gets
  a verdict instead of being copied. This is
  the roadmap's only pruning, so also collapse completed rows that no longer
  guide work, drop superseded text, and move per-PR plans to their PRs or
  issues.

Write each correction as the outcome you want, with its bounds attached:

- A "do X next" names its constraint, such as a glue budget or the shared
  layer to reuse.
- A gate says what happens on a miss: a profile-guided fix round, not deferral.
- A number you introduce (a threshold, a factor, a budget) rests on a
  measurement of its noise under the conditions root will run it in. Record
  that basis and mark the number provisional in the steering section; the next
  review checks it.
- Read each new rule beside the rules it interacts with.

When a correction reverses a direction the user gave root, the steering section
says so in one line: what it reverses, and why.

Commit `AGENTS.md`, the roadmap and any restart-prompt edit (step 5) as one
docs-only commit pushed straight to `rust-migration`, with a title like
`Doc: Steer migration toward <corrections> [skip ci]`. Earlier steering commits
landed the same way. Confirm `origin/rust-migration` has it.

## 5. Check /start-development

`docs/skills/start-development/SKILL.md` is root's restart prompt. It stays
unchanged by default: it sends root to the roadmap's latest steering review,
and that section carries this review's corrections. Edit the prompt only when
this review shows it needs improving, for example because root stopped early,
misread it, or needs a new standing step in its work loop. Keep it open-ended:
finishing the listed items triggers planning the next work, and only the user
ends the assignment. Include any edit in the step 4 commit.

## 6. Report and hand off

Tell the user, briefly:

- the verdict on root's track;
- what the audits found, with numbers;
- what you changed, with issue and commit links, and any user direction it
  reverses;
- anything only the user can supply, such as a new play save or a policy decision.

If the focus asked you to restart root, launch it once `origin/rust-migration`
has your commit, using the T3 Code tools `t3_thread_list`,
`t3_thread_configuration`, `t3_thread_launch` and `t3_thread_read` (in Claude
Code, load them first with ToolSearch). Root's last thread is the newest
top-level thread in `t3_thread_list` whose first message, read with
`t3_thread_read`, invokes start-development. Read its `modelSelection`,
`runtimeMode` and `interactionMode` with `t3_thread_configuration`. Call
`t3_thread_launch` with those values, or with the model the focus names, plus
its title, `workspaceStrategy: {type: "root"}` and the start-development
message for that host (`$start-development` in Codex, `/start-development` in
Claude Code). The launch has no retry key: after an error, check
`t3_thread_list` before launching again. Confirm with `t3_thread_read` that it
is running, and link it as `[title](t3-thread://v1/<threadId>)`.

Otherwise finish by telling them to start root with the start-development
skill. Either way, say whether this review changed the prompt; the prompt lives
in the skill, not in your reply.
