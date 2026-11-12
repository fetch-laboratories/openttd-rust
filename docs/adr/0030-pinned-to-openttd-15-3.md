---
status: accepted
may change: user
---
# The fork stays on OpenTTD 15.3 and does not follow upstream

Upstream, the OpenTTD project the fork was copied from, keeps releasing new versions. If the fork followed them, the behaviour it must match ([ADR 0002](0002-original-game-is-the-specification.md)) would change under work already in progress.

The original is pinned at OpenTTD 15.3, the latest stable release on 2026-10-02 (`migration/baseline.json`). Upstream changes are not merged. Moving the pin would be a separate, deliberate task.

The founding agent made this choice on 2026-10-02: "keeping that original commit fixed would give every comparison a stable meaning. Later upstream changes can be incorporated deliberately." The record holds no comment from the user. It is the user's to change because it decides which game the fork reproduces.

Reversing it costs more each week. A later upstream release would have to be merged into C++ that has since been deleted and rewritten in Rust, and every expected result captured again.
