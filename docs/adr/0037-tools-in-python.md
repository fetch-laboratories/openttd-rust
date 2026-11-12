---
status: accepted
may change: agents; keeping the tools after the rewrite is the user's decision
---
# The fork's tools and harness are written in Python and checked with pinned lint and type checkers

The fork needs a great deal of tooling of its own: the comparison harness, the CI drivers, measurements and checks. By 2026-11-12, `tools/` held about 55,800 lines of Python. Code of that size with no checks hides bugs.

The tools are written in Python. Ruff (a Python lint tool) and ty (a Python type checker) check all of `tools/` at pinned versions, and every function carries type hints, labels that say what type each value is; `ruff.toml` and `ty.toml` hold the settings.

Nothing in the record chose Python over another language; it was the first harness agent's default, and Ruff came with that harness on 2026-10-03 (commit `addc6741c9`). The type checks are the user's. On 2026-10-26 they asked "Are we using uv, ruff, and ty? I always put those on my python projects", and when the agent played it down: "26k lines is a huge surface for bugs. any new python code should have type hints."

The Python tools are kept after the rewrite. [ADR 0001](0001-full-rust-rewrite.md) retires the fork's C++ tools, not the Python ones. The user decided this on 2026-11-12: "development will continue after rewrite into improvement, so anything supporting development should be kept."

Moving the tools to another language would mean rewriting tens of thousands of lines.
