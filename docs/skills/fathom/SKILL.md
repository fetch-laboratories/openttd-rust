---
name: fathom
description: Use when a failure, stall or rework in running the OpenTTD-Rust migration recurs or survives a correction. Finds its cause, proves a fix and lands it.
---

# Fathom one problem

Take one problem in how the migration is run to its cause, fix it there and prove the fix. [Steer](../steer/SKILL.md) and [navigate](../navigate/SKILL.md) find problems and rank them, and root builds; you find out why. One problem a run.

The problem: $ARGUMENTS (in Codex, any text sent with `$fathom`), a case issue or a description.

If you hold other work (root, or a steering or navigation review), hand this off: write the case (step 1), start a fathom agent as [spawning](../../team.md#spawning) says and return to your work. Any other agent reports the problem to root.

Attribution on everything you write:
`Agent: /fathom-<case issue> | Model: <your exact model ID> | Reasoning effort: <as reported by host, else "not reported to the agent by host"> | Host: <host>`. The path is one word: `ci.py ready` and the merge group's growth gate read the author from it, and refuse a PR whose line has none.

Root may be running. Work in your own worktree under `.local/worktrees/`, take no wall-clock timings while it builds, and after pushing a rule that a running agent follows, send root the commit (`t3_thread_send`, mode `queue`).

## 1. Write the case

The case is an issue with a line on the checklist of the fathom tracker, #1067. Its body states:

- **The problem**, as something that can be counted: what happens, and to which agent, tool, rule or PR.
- **Its cost** over a named window, in hours, runs or changed lines, with the command or query that counts it.
- **What has been tried**: each earlier correction aimed at it, by commit or issue.
- **What must hold**: the ADRs marked `may change: user` and the user's instructions that bound the fix.

A case stands when the problem recurs, or cost more once than its fix would; when it still occurs in the latest sessions; and when no open issue or existing tool covers it. Where a tool covers it and agents do not use the tool, the problem is that they do not find it. Close any other case with that verdict.

Done when the issue lets a reader count the problem again.

## 2. Find the cause

- **Every instance.** Collect every occurrence in the window, and the cases beside them where it did not occur: sessions on both hosts (`python3 tools/transcripts.py --help`), PR timelines (`python3 tools/retrospective.py --help`), CI runs, review reports and the Git history. Give a known pattern to a script or to routine-model subagents to count, and an open question to stronger subagents to read; check yourself what you will act on.
- **What produced it.** Trace each instance to its source:
  - something an agent did: the message that started it and each doc that message sent it to, read in its order;
  - a rule's result: the rule's text and each case it was applied to;
  - a tool or CI step: its code and its logs;
  - work badly ordered or sized: the selection rule and the figures it was given.
- **Why the source is so.** Most rules were written to stop something: find what, before changing one (`git log -S`, the ADRs, the commit that wrote it). An earlier correction aimed at this problem did not work; find why first.
- **The claim.** State the cause as a mechanism: what, given what, produces the problem. Name each other explanation you considered and the evidence that rules it out.

Session text is private. Quote session IDs and counts; the digest and text copied from it stay out of Git, issues, fixtures and CI artifacts.

Done when the cause tells the failing instances from the passing ones, with every instance counted.

## 3. Fix it at the cause

- Fix the kind of problem, not the instance that showed it.
- Take the first of these that removes the cause: remove or relax the rule that produces it; have a tool do, refuse or show the thing; give an agent a way to do what it is already told to do; add a rule.
- A task added to a brief takes from the brief's other tasks: reviewers spend the same minutes whatever theirs holds.
- A gate says what happens on a miss: a fix round, not a deferral.
- A cause that is a decision marked `may change: user`, or a fix that raises paid CI cost, goes to the user with the evidence, the cost and your recommendation ([ADRs](../../adr/README.md#who-may-change-one)). Fix what you can inside it meanwhile.

## 4. Prove it

Before landing, show the problem on past cases without the fix, and its absence with it.

- Write first: the cases, what each should show, and the result that counts as a pass. Include cases you did not use to design the fix.
- Use the proof the problem allows. Run the changed agent, tool or check again on the past cases, with the model and settings production uses; or recompute from the record what a changed rule, threshold or order would have done. Measure a threshold against its noise under the conditions it runs in.
- Measure what the fix costs and what it stops: whatever worked on those cases before still works.
- Where a result varies from run to run, repeat it until the difference exceeds the variation, and give the runs behind each figure.
- Size the proof to the problem: it costs less than the problem does in one stocktake interval, or the case says why it is worth more.
- A fix that misses its bar is reported as a miss; return to step 2.
- Where nothing can be run again or recomputed, say so. The fix lands provisional on the number in step 5.

Done when the result meets the bar written before the run, or the case says the fix is provisional and why.

## 5. Land it and name the number

- A change to docs or briefs is a [docs-only](../../workflow.md#change-types) commit on `rust-migration`. A tool or CI fix is your own PR under the [PR lifecycle](../../workflow.md#pr-lifecycle); it takes no implementer slot ([ADR 0053](../../adr/0053-a-fathom-takes-no-slot.md)). Work on game code becomes an issue for root's work order, with its measurements and acceptance.
- A fix inside one of the four user-invoked [skills](../../team.md#skills) goes to the user as the proposed edit with its proof.
- A rule goes once in the doc that owns it ([routing](../../../AGENTS.md#routing)), and a real decision in an ADR.
- Leave the case open with one comment: the cause, the proof with its figures, what changed by commit or PR, and **the number to watch**, with the command that reads it, its value now and the value at which the fix has failed. Steering reads that number at its next review and closes the case or sends it back.
- Report in your thread: the cause first, then the proof, the change and the number.
