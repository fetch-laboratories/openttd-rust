---
status: accepted
may change: user
---
# Porting stays paused until the cleanup is finished

On 2026-11-06 an agent measured the Rust code against the original. The Rust had 7 comment lines per 100 lines of code, against 35 in the original C++. Function parameters were bare integers 9,165 times and named types 2,341 times. The original's named values had become raw numbers. Long functions had grown longer, with more parameters. The checks all passed, but passing checks say nothing about code like this.

The cleanup has five parts: use the original's types, names and comments in the Rust code ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734)); replace long parameter lists with argument structs (#735); move tests out of C++ where that removes more C++ than it adds (#736); reduce the calling layer between C++ and Rust (#737); clean up the rule docs (#738). Porting stays paused until all five are finished.

The reason is the user's: agents copy the standards of the code they find, so bad existing code produces more bad code. On 2026-11-06 the user wrote: "agents pick up standards, conventions, and habits from existing code. … This will pay dividends." In the same message they answered the agent's warning that porting would stall: "your costs and risks are noted, but I am not concerned." The user then approved the plan.

The cleanup has already cost many agent-days of work. At merge 32, 99 of #734's 319 files were finished. Stopping halfway would leave the code partly raw and partly cleaned, which defeats the reason for the cleanup.
