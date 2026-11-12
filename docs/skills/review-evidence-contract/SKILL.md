---
name: review-evidence-contract
description: Assess OpenTTD-Rust port evidence when a fidelity or completion claim depends on production-path coverage, or a port changes a fixture or oracle; distinguish blockers from existing coverage gaps. Excludes unrelated reviews and routine checks with no such claim or evidence change.
---

# Review a port's evidence contract

Apply the existing [review](../../workflow.md#review), [fidelity](../../porting.md#fidelity), [boundary](../../porting.md#boundary), [evidence](../../porting.md#evidence), [test](../../porting.md#tests), [tests in Rust and C++](../../porting.md#tests-in-rust-and-c) and [check-scope](../../workflow.md#checks) rules to the claim under review. Follow the skill's [scope](../../team.md#skills).

- Identify the claimed behavior and changed production entries, then compare replaced bodies with the unchanged pinned original, callee by callee. Trace how each cited scenario or fixture reaches the candidate production path and obtains an independent expectation. A helper compared with itself, a synthetic substitute for production dispatch, or a successful build alone does not establish that claim.
- For changed fixtures or expectations, explain why the original observation still applies and which behavior the revised check distinguishes. Carry observations from a removed candidate C++ fixture into Rust tests or the harness, keeping reference-only support separate; a C++ test of code still in C++ stays ([ADR 0023](../../adr/0023-cpp-tests-stay-until-code-is-ported.md)).
- Check that cited witnesses and probes execute in default CI and fail when missing. Name what saves or synthetic tests cannot observe; use the existing harness and focused checks for concrete gaps.

Report only findings, in the [review report](../../review-brief.md#report) form; work through each claim or branch, its production entry, its unchanged oracle and observation, and its default-CI coverage, but do not list them.

Separate a demonstrated regression, invalid oracle or missing evidence required for this port's claim from a pre-existing coverage gap. The former needs a fix or a narrower justified claim; rule on the scope question yourself under [policy questions](../../review-brief.md#policy-questions). Record unreached harness branches in #156's list under [evidence](../../porting.md#evidence). State the gap without treating it as proof of a defect or claiming full equivalence from passing covered cases.
