---
status: accepted
may change: user
---
# Other people's test content is never committed or uploaded

The fork tests the game with content written by other people: AIs from OpenTTD's online content service and sets of NewGRF mods (add-on vehicles, buildings and graphics). Much of it is licensed under the GPL, which requires anyone who redistributes it to offer the matching source. The repo is public, so committing or uploading that content would make the fork a redistributor.

Test inputs made by other people, such as AIs and NewGRF mod sets, are never committed to the repo or uploaded from CI. They are downloaded by pinned version and checksum into an ignored folder. The repo is public, and redistributing nothing avoids licence obligations for that content.

Agents made the rule. Commit `a25223ec22` (2026-11-02) generalised it from one comment on [#507](https://github.com/fetch-laboratories/openttd-rust/issues/507) to all third-party inputs, after the user chose a new set of NewGRF test content (#654). The record holds no words of the user on the rule itself. It is the user's to change because it is a legal matter.

Reversing it would bring back the source-supply work that #526 was opened for and then closed, and the licence exposure that work was meant to cover.
