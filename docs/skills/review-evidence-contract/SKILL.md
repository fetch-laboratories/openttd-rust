---
name: review-evidence-contract
description: Assess OpenTTD-Rust port evidence when a fidelity or completion claim depends on production-path coverage, or a port changes a fixture or oracle; distinguish blockers from existing coverage gaps. Excludes unrelated reviews and routine checks with no such claim or evidence change.
---

# Review a port's evidence contract

Apply the [review](../../workflow.md#review), [fidelity](../../porting.md#fidelity), [evidence](../../porting.md#evidence) and [test](../../porting.md#tests) rules to the claim under review.

- Identify the claimed behavior and changed production entries, then compare replaced bodies with the unchanged pinned original, callee by callee. Trace how each cited scenario or fixture reaches the candidate production path and obtains an independent expectation. A helper compared with itself, a synthetic substitute for production dispatch, or a successful build does not establish the claim.
- For changed fixtures or expectations, explain why the original observation still applies and what the revised check distinguishes. Carry observations from a removed candidate C++ fixture into Rust tests or the harness ([ADR 0023](../../adr/0023-cpp-tests-stay-until-code-is-ported.md)).
- Check that cited witnesses and probes run in default CI and fail when missing. Name what saves or synthetic tests cannot observe.

Report only findings, in the [review report](../../review-brief.md#report) form. Separate a demonstrated regression, invalid oracle or missing evidence this claim needs, which needs a fix or a narrower claim, from a pre-existing coverage gap, which goes on #156's list and is not proof of a defect. Rule on the scope question yourself under [policy questions](../../review-brief.md#policy-questions).
