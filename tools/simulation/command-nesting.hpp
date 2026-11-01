/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file command-nesting.hpp Run a command procedure at nesting depth one in both games. */
#ifndef SIMULATION_COMMAND_NESTING_HPP
#define SIMULATION_COMMAND_NESTING_HPP
#include "command_func.h"
#ifdef WITH_RUST
/**
 * Enter Rust's canonical command counter through a top-level bankrupt execution
 * whose test run does nothing; bankruptcy skips both money steps. Its other
 * effects, clearing the cleared-object areas before each run and a balanced
 * town-rating test toggle, precede the procedure.
 */
template <typename F>
void NestedCommand(F &&body)
{
	struct Frame : OpenTTDCommandFrame {
		CommandCost result;
		F *body;
	} frame{};
	frame.body = &body;
	frame.results[0] = reinterpret_cast<OpenTTDCommandCost *>(&frame.result);
	frame.run = [](OpenTTDCommandFrame *base, uint16_t flags, uint8_t) noexcept {
		auto &f = *static_cast<Frame *>(base);
		if (DoCommandFlags{flags}.Test(DoCommandFlag::Execute)) (*f.body)();
		return true;
	};
	openttd_rust_command_do(&GetRustCommandServices(), &frame, 0, DoCommandFlags{DoCommandFlag::Execute, DoCommandFlag::Bankrupt}.base());
}
#else
template <typename F>
void NestedCommand(F &&body)
{
	RecursiveCommandCounter nested;
	body();
}
#endif
#endif /* SIMULATION_COMMAND_NESTING_HPP */
