# Review checklist

Readers: reviewers. Reviewers enforce the standards implementers no longer read in full, and report as [report](#report) says.

## Fidelity first

- Compare every replaced original body callee by callee: guards, fatal paths, assertions, widths, wrapping and truncation, return values.
- Random draws: the same calls in the same order. State reads and writes at the original observation points.
- Judge the whole reachable input domain, beyond the paths scenarios exercise. C++ globals stay process-wide, never thread-local.
- Lifetimes: allocation and ID reuse, publication and destruction order. Borrows end before callbacks, mutation, growth, throws and deletion.

## Code standards

- The original's types and named constants come from their header module. Code reaches the raw integer at ABI, save and bitfield boundaries and wherever the original uses the value as an integer (table indexing, arithmetic, bit scans); a literal standing for a named value is still a finding.
- The original's explanatory comments sit at the matching branch. Comments carry no review defence, history or dates.
- Commands take their argument struct; helpers take context structs.
- Each helper has one port. Grep the callee of every new binding across `src/`, including macros and compound callbacks such as `CheckOwnership` and `GetRoadOwner`; each existing wrapper or binding is a finding.
- Boundary design faults, fixed before integration with no exceptions: a carrier for one call site, opcode dispatch, whole-record copies, per-call allocation the original lacks, Rust-to-C++-to-Rust bounces, a moved field behind a service, a service routed through `Record()`.
- Every replaced body, export, declaration and service field is deleted. Each orphan is a finding.

## Evidence

- Each claim traces to production code and an expectation taken from the original.
- Each new scenario, fixture, test or checker names behaviour existing checks cannot distinguish ([cost decisions](planning.md#cost-decisions)); one that names none, or compares an operation with itself, is a finding, and the fix deletes it. Scenarios that exercise the component stay ([evidence](porting.md#evidence)).
- Changed fixtures and expectations are explained. Each growth or lint exception names the original behaviour that requires it and what retires it; each declared carry pairs the same item before and after a rename or move; each fidelity suppression's reason names the original definition or cast it preserves. The [cleanup checks](rust-migration.md#quick-cleanup-checks) enforce these records' form, not their truth.

## Cleanup PRs

- Type, name and comment changes prove they change no behaviour: identical compiled Rust code, or, where code differs, a source review showing each changed function performs the same operations in the same order, and the merge queue's Full.

## Report

Use the review skeleton in the [PR template](../.github/PULL_REQUEST_TEMPLATE.md), starting with `Reviewed commit: <sha>`. Name the functions or files compared with original bodies and duplicate callbacks, and cite Quick's [orphan-export check](rust-migration.md#shared-code-checks-and-port-metrics) for exports and declarations. Report only under either provider, as the [review brief](review-brief.md#report) says; the implementer commits fixes. End with the disposition, with no narration when there are no findings. Rule on policy questions a recorded decision answers; send unresolved scope or policy questions to root through the implementer ([policy questions](review-brief.md#policy-questions)). Edit #156's current list when the PR introduces or uncovers a branch no scenario reaches; do not post a "no new gaps" comment.
