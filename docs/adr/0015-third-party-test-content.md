---
status: accepted
may change: user
---
# Other people's test content is not committed or uploaded

The fork tests the game with content written by other people: AIs from OpenTTD's online content service and sets of NewGRF mods (add-on vehicles, buildings and graphics). Much of it is licensed under the GPL, which requires anyone who redistributes it to offer the matching source. The repo is public, so committing or uploading that content would make the fork a redistributor.

Test inputs made by other people, such as AIs and NewGRF mod sets, are not committed to the repo or uploaded from CI. They are downloaded by pinned version and checksum into an ignored folder.

The rule began with the user. On 2026-10-28, three minutes after picking an agent's recommended NewGRF sets for #525 to commit, the user asked: "why are you bringing them into the repo?" and "why not just keep a local copy uncommitted?" Commit `a25223ec22` (2026-11-02) extended it to all third-party inputs; the licence reasoning is the agents'.

Reversing it would bring back the source-supply work that #526 was opened for and then closed, and the licence exposure that work was meant to cover.
