---
name: review-evidence-contract
description: Assess OpenTTD-Rust port evidence when a fidelity or completion claim depends on production-path coverage, or a port changes a fixture or oracle; distinguish blockers from existing coverage gaps. Excludes unrelated reviews and routine checks with no such claim or evidence change.
---

# Review a port's evidence contract

Apply the existing [review](../../workflow.md#review), [fidelity](../../porting.md#fidelity), [boundary](../../porting.md#boundary), [evidence](../../porting.md#evidence), [test](../../porting.md#tests) and [check-scope](../../workflow.md#checks) rules to the claim under review. Follow the skill's [scope](../../team.md#skills).

- Identify the claimed behavior and changed production entries, then compare replaced bodies with the unchanged pinned original, callee by callee. Trace how each cited scenario or fixture reaches the candidate production path and obtains an independent expectation. A helper compared with itself, a synthetic substitute for production dispatch, or a successful build alone does not establish that claim.
- For changed fixtures or expectations, explain why the original observation still applies and which behavior the revised check distinguishes. Carry observations from retiring candidate C++ fixtures into Rust tests or the harness, keeping reference-only support separate.
- Check that cited witnesses and probes execute in default CI and fail when missing. Name what saves or synthetic tests cannot observe; use the existing harness and focused checks for concrete gaps.

Return a short evidence matrix: **claim or branch | production entry | unchanged oracle and observation | check/default-CI coverage | limit or finding**. Cite files, functions and the reviewed commit in the normal review report.

Separate a demonstrated regression, invalid oracle or missing evidence required for this port's claim from a pre-existing coverage gap. The former needs a fix, or a narrower justified claim, which is root's scope decision; record unreached harness branches in the existing #156 tracker under the review rules. State the gap without treating it as proof of a defect or claiming full equivalence from passing covered cases.
