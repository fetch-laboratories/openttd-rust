---
status: accepted
may change: user
---
# At most two implementers work at once

Implementers are the agents that write code. They all compile and test on this one machine, which has four cores (eight hardware threads). On 2026-11-08 and 11-09 an agent measured how much work got done: with a cap of two implementers, 1.6 were working at any moment, and with a cap of four, 2.0 were. The extra agents spent their time waiting for builds ([#860](https://github.com/fetch-laboratories/openttd-rust/issues/860)).

At most two implementers work at once. An implementer whose changes are all waiting on CI, review or the merge queue does not count toward the two. Root starts a fresh implementer for the free slot and gives the waiting one no new work.

The user wrote on 2026-11-06, "I can't add more cores. so, you can limit implementers", and on 2026-11-10, "we can limit to two implementers to suit the machine's capacity." On 2026-11-11 they rejected handing waiting implementers new work: "terrible decision to give the existing implementers new work." New work fills a waiting agent's context before it gets back to its own change.

Changing the cap either way costs agent-hours: more implementers crowd the machine, and fewer leave it idle. [docs/team.md](../team.md) holds the working rules.

## History

The cap was six, then four in the plan of 2026-11-06. On 2026-11-08 the user limited one session to two, then allowed four again the same evening; two became standing on 2026-11-10. Until 2026-11-11 a waiting implementer took the next piece of work itself.
