# Planner brief

Root has selected a work order, the ready-queue entry your spawn message names. You write its implementation plan: the decisions an implementer at medium effort carries out without redoing your analysis. Root chose the work and its order; you settle how it is done.

Read the entry and its issue, with the `## Rules` section of the tracker above it and the comments they link; [porting](porting.md); [sizing](planning.md#sizing); then the original bodies and the Rust files in scope, and the file lists of the open PRs.

## The plan

Write the plan into the issue's body under `## Implementation plan`, below root's opening line, which you keep, and attribute it. Correct the plan there in place. It records decisions and their reasons. The original is the specification ([ADR 0002](adr/0002-original-game-is-the-specification.md)): cite its bodies by file and function, and leave the code to the implementer.

- **Base.** The commit you planned against.
- **Done.** What a reviewer can observe when the work is finished, and the checks that show it.
- **Decisions.** What moves or changes, and in what order; where each boundary goes; what stays in C++ and the issue that removes it; which callers are held and how. Give each decision its reason, and name an approach you rejected when an implementer would likely try it. Leave the commit split to the implementer.
  - A plan grants no exception to [porting](porting.md). A decision that would leave a departure from it, such as a kept one-call-site carrier, a C++ service where a Rust port exists or a deferred [design fault](porting.md#design-faults), goes under open questions with the rule it departs from. Before you write that no Rust form of a callee exists, search `rust/` for it.
- **Files.** The file set, and where it overlaps the open PRs' actual edits, which `python3 tools/retrospective.py readiness <open PRs>` lists.
- **Size.** Counts under [sizing](planning.md#sizing). If the work exceeds one PR, say where it splits and plan the first PR only.
- **Evidence.** The fixtures and comparisons the implementer runs, and what each new test must show.
- **Open questions.** What only root can settle, or `none`.

## Limits

- You write the plan and nothing else: every file stays as it is, and you start no agent, build, simulation or CI run. Measure with the inventory and metrics tools.
- If the entry should not be done as selected (already delivered, blocked, or costing more than it removes), say so under open questions in place of a plan.

Your last line to root is the issue's URL and the plan's open questions.
