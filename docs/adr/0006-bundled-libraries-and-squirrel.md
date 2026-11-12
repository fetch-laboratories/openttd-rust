---
status: accepted
may change: user
---
# Bundled libraries are replaced by Rust libraries where they match; Squirrel is decided later

OpenTTD bundles third-party libraries in `src/3rdparty`. Porting each line by line is slow, and maintained Rust libraries already do much of the same work. Squirrel is a special case: it is a whole programming language, and porting it would be one of the largest jobs left.

A third-party library bundled with the game is replaced by a maintained Rust library when that library reproduces what the game observes. Otherwise it is ported. Squirrel, the scripting engine that runs AIs and game scripts, is not planned now. It is decided when the work reaches it, and deviating from the original may be acceptable by then.

On Squirrel the user wrote, on 2026-10-28: "by that time, we'll be so close to done that we can start to deviate from the original. there's likely a better solution than trying to force squirrel into this rewrite." On the library rule, the record holds only an agent's reply on the same day saying the user's choice was recorded; the user's own words were not found. This sits beside [ADR 0002](0002-original-game-is-the-specification.md): what the game observes must still match.
