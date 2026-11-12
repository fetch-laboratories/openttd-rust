---
status: accepted
may change: user
---
# Agents do not contact OpenTTD's upstream maintainers or submit changes upstream

Agents porting the game find bugs in the original and ideas for improving it. The usual move would be to report them to OpenTTD's maintainers (upstream) or send a fix. But as the user noted on the first day, "the original openttd repo forbids the use of AI", so an agent's report or patch would break upstream's rules.

Agents work on the fork only: ordinary issues, pull requests and review reports there. No agent contacts upstream maintainers, files upstream issues or opens upstream pull requests, even for a bug found in the original. Such bugs and improvements become fork issues ([ADR 0002](0002-original-game-is-the-specification.md)).

The reason is the user's words of 2026-10-02. The founding agent wrote the rule the same evening: "This fork welcomes agent-generated work; upstream contribution policies apply to upstream submissions." No message from the user states the rule itself. It is the user's to change because it faces people outside the project.

A message or pull request sent upstream is public and read by people outside the project, so it cannot be taken back.
