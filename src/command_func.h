/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command_func.h Functions related to commands. */

#ifndef COMMAND_FUNC_H
#define COMMAND_FUNC_H

#include "command_type.h"
#include "network/network_type.h"
#include "company_type.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "misc/endian_buffer.hpp"
#include "tile_map.h"
#include "rust/command_services_ffi.h"
#include <exception>
#include <source_location>

/**
 * Define a default return value for a failed command.
 *
 * This variable contains a CommandCost object with is declared as "failed".
 * Other functions just need to return this error if there is an error,
 * which doesn't need to specific by a StringID.
 */
static const CommandCost CMD_ERROR = CommandCost(INVALID_STRING_ID);

void NetworkSendCommand(Commands cmd, StringID err_message, CommandCallback *callback, CompanyID company, const CommandDataBuffer &cmd_data);
bool IsNetworkRegisteredCallback(CommandCallback *callback);

bool IsValidCommand(Commands cmd);
CommandFlags GetCommandFlags(Commands cmd);
std::string_view GetCommandName(Commands cmd);
bool IsCommandAllowedWhilePaused(Commands cmd);

template <Commands Tcmd>
constexpr CommandFlags GetCommandFlags()
{
	return CommandTraits<Tcmd>::flags;
}

/**
 * Extracts the DC flags needed for DoCommand from the flags returned by GetCommandFlags
 * @param cmd_flags Flags from GetCommandFlags
 * @return flags for DoCommand
 */
static constexpr inline DoCommandFlags CommandFlagsToDCFlags(CommandFlags cmd_flags)
{
	DoCommandFlags flags = {};
	if (cmd_flags.Test(CommandFlag::NoWater)) flags.Set(DoCommandFlag::NoWater);
	if (cmd_flags.Test(CommandFlag::Auto)) flags.Set(DoCommandFlag::Auto);
	if (cmd_flags.Test(CommandFlag::AllTiles)) flags.Set(DoCommandFlag::AllTiles);
	return flags;
}

#if defined(__GNUC__) && !defined(__clang__)
/*
 * We cast specialized function pointers to a generic one, but don't use the
 * converted value to call the function, which is safe, except that GCC
 * helpfully thinks it is not.
 *
 * "Any pointer to function can be converted to a pointer to a different function type.
 * Calling the function through a pointer to a different function type is undefined,
 * but converting such pointer back to pointer to the original function type yields
 * the pointer to the original function." */
#	pragma GCC diagnostic push
#	pragma GCC diagnostic ignored "-Wcast-function-type"
#	define SILENCE_GCC_FUNCTION_POINTER_CAST
#endif

template <Commands TCmd, typename T, bool THasTile> struct CommandHelper;

class CommandHelperBase {
protected:
	static void LogCommandExecution(Commands cmd, StringID err_message, const CommandDataBuffer &args, bool failed);
	static bool DesyncLogging();
	[[noreturn]] static void AbandonExecution(std::exception_ptr exception, std::source_location location = std::source_location::current());
};

/**
 * Templated wrapper that exposes the command parameter arguments
 * for the various Command::Do/Post calls.
 * @tparam Tcmd The command-id to execute.
 * @tparam Tret Return type of the command.
 * @tparam Targs The command parameter types.
 */
template <Commands Tcmd, typename Tret, typename... Targs>
struct CommandHelper<Tcmd, Tret(*)(DoCommandFlags, Targs...), true> : protected CommandHelperBase {
private:
	/** Extract the \c CommandCost from a command proc result. */
	static inline CommandCost &ExtractCommandCost(Tret &ret)
	{
		if constexpr (std::is_same_v<Tret, CommandCost>) {
			return ret;
		} else {
			return std::get<0>(ret);
		}
	}

	/**
	 * Native frame of one invocation: the argument tuple and the typed result
	 * tuples whose \c CommandCost Rust reconciles. Rust selects the result slot.
	 * @tparam Ttuple Argument tuple, by value or of references.
	 * @tparam Tslots Result slots: test, execution and untouched default.
	 */
	template <typename Ttuple, size_t Tslots>
	struct Frame : OpenTTDCommandFrame {
		Ttuple *args;
		StringID err_message;
		CommandCallback *user_callback;
		TileIndex tile;
		std::exception_ptr exception{};
		std::array<Tret, Tslots> res{};

		Frame(Ttuple &args, StringID err_message, CommandCallback *callback, TileIndex tile) : OpenTTDCommandFrame{}, args(&args), err_message(err_message), user_callback(callback), tile(tile)
		{
			for (size_t i = 0; i < Tslots; i++) this->results[i] = reinterpret_cast<OpenTTDCommandCost *>(&ExtractCommandCost(this->res[i]));
			this->run = &Run<Ttuple, Tslots>;
		}
		Frame(const Frame &) = delete;
		Frame &operator=(const Frame &) = delete;
	};

	using ExecuteFrame = Frame<std::tuple<Targs...>, 3>;

	/** Run the trait procedure into a result slot, capturing an exception for the caller to rethrow. */
	template <typename Ttuple, size_t Tslots>
	static bool Run(OpenTTDCommandFrame *base, uint16_t flags, uint8_t slot) noexcept
	{
		auto &f = *static_cast<Frame<Ttuple, Tslots> *>(base);
		try {
			f.res[slot] = std::apply(CommandTraits<Tcmd>::proc, std::tuple_cat(std::make_tuple(DoCommandFlags{flags}), *f.args));
		} catch (...) {
			f.exception = std::current_exception();
			return false;
		}
		return true;
	}

	static int64_t AdditionalMoney(OpenTTDCommandFrame *base) noexcept
	{
		/* Convention: If the second result element is of type Money,
		 * this is the additional cash required for the command. */
		if constexpr (!std::is_same_v<Tret, CommandCost>) {
			return ExtractAdditionalMoney(static_cast<ExecuteFrame *>(base)->res[1]);
		} else {
			return 0;
		}
	}

	static void Log(OpenTTDCommandFrame *base, bool failed) noexcept
	{
		/* Debug(desync, 1) prints nothing below level 1; skip serializing then. */
		if (!DesyncLogging()) return;
		auto &f = *static_cast<ExecuteFrame *>(base);
		LogCommandExecution(Tcmd, f.err_message, EndianBufferWriter<CommandDataBuffer>::FromValue(*f.args), failed);
	}

	static void Send(OpenTTDCommandFrame *base, uint8_t company) noexcept
	{
		auto &f = *static_cast<ExecuteFrame *>(base);
		::NetworkSendCommand(Tcmd, f.err_message, f.user_callback, CompanyID(company), EndianBufferWriter<CommandDataBuffer>::FromValue(*f.args));
	}

	static void SetClientIdsStep(OpenTTDCommandFrame *base) noexcept
	{
		SetClientIds(*static_cast<ExecuteFrame *>(base)->args, std::index_sequence_for<Targs...>{});
	}

	static void AssertClientIds([[maybe_unused]] OpenTTDCommandFrame *base) noexcept
	{
		/* Make sure arguments are properly set to a ClientID also when processing external commands. */
		assert(AllClientIdsSet(*static_cast<ExecuteFrame *>(base)->args, std::index_sequence_for<Targs...>{}));
	}

	template <typename Tcallback>
	static void Callback(OpenTTDCommandFrame *base, uint8_t slot) noexcept
	{
		auto &f = *static_cast<ExecuteFrame *>(base);
		Tcallback *callback = reinterpret_cast<Tcallback *>(reinterpret_cast<void(*)()>(f.user_callback));
		Tret &res = f.res[slot];
		auto &args = *f.args;
		TileIndex tile = f.tile;
		if constexpr (std::is_same_v<Tcallback, CommandCallback>) {
			/* Callback that doesn't need any command arguments. */
			callback(Tcmd, ExtractCommandCost(res), tile);
		} else if constexpr (std::is_same_v<Tcallback, CommandCallbackData>) {
			/* Generic callback that takes packed arguments as a buffer. */
			if constexpr (std::is_same_v<Tret, CommandCost>) {
				callback(Tcmd, ExtractCommandCost(res), EndianBufferWriter<CommandDataBuffer>::FromValue(args), {});
			} else {
				callback(Tcmd, ExtractCommandCost(res), EndianBufferWriter<CommandDataBuffer>::FromValue(args), EndianBufferWriter<CommandDataBuffer>::FromValue(RemoveFirstTupleElement(res)));
			}
		} else if constexpr (!std::is_same_v<Tret, CommandCost> && std::is_same_v<Tcallback *, typename CommandTraits<Tcmd>::RetCallbackProc>) {
			std::apply(callback, std::tuple_cat(std::make_tuple(Tcmd), res));
		} else {
			/* Callback with arguments. We assume that the tile is only interesting if it actually is in the command arguments. */
			if constexpr (std::is_same_v<Tret, CommandCost>) {
				std::apply(callback, std::tuple_cat(std::make_tuple(Tcmd, res), args));
			} else {
				std::apply(callback, std::tuple_cat(std::make_tuple(Tcmd), res, args));
			}
		}
	}

	/** Frame of \c Execute and \c InternalPost with every native step. */
	static void PrepareExecute(ExecuteFrame &frame)
	{
		frame.additional_money = &AdditionalMoney;
		frame.log = &Log;
		frame.send = &Send;
		frame.set_client_ids = &SetClientIdsStep;
		frame.assert_client_ids = &AssertClientIds;
	}

public:
	/**
	 * This function executes a given command with the parameters from the #CommandProc parameter list.
	 * Depending on the flags parameter it executes or tests a command.
	 *
	 * @note This function is to be called from the StateGameLoop or from within the execution of a Command.
	 * This function must not be called from the context of a "player" (real person, AI, game script).
	 * Use ::Post for commands directly triggered by "players".
	 *
	 * @param flags Flags for the command and how to execute the command
	 * @param args Parameters for the command
	 * @see CommandProc
	 * @return the cost
	 */
	static Tret Do(DoCommandFlags flags, Targs... args)
	{
		auto arguments = std::forward_as_tuple(args...);
		/* Rust rejects out-of-bounds tile-commands before running anything. */
		TileIndex tile{};
		if constexpr (std::is_same_v<TileIndex, std::tuple_element_t<0, std::tuple<Targs...>>>) tile = std::get<0>(arguments);
		Frame<decltype(arguments), 1> frame(arguments, 0, nullptr, tile);
		if (!openttd_rust_command_do(&GetRustCommandServices(), &frame, tile.base(), flags.base())) std::rethrow_exception(frame.exception);
		return std::move(frame.res[0]);
	}

	/**
	 * Shortcut for the long Post when not using a callback.
	 * @param err_message Message prefix to show on error
	 * @param args Parameters for the command
	 */
	static inline bool Post(StringID err_message, Targs... args) { return Post<CommandCallback>(err_message, nullptr, std::forward<Targs>(args)...); }
	/**
	 * Shortcut for the long Post when not using an error message.
	 * @param callback A callback function to call after the command is finished
	 * @param args Parameters for the command
	 */
	template <typename Tcallback>
	static inline bool Post(Tcallback *callback, Targs... args) { return Post((StringID)0, callback, std::forward<Targs>(args)...); }
	/**
	 * Shortcut for the long Post when not using a callback or an error message.
	 * @param args Parameters for the command
	 */
	static inline bool Post(Targs... args) { return Post<CommandCallback>((StringID)0, nullptr, std::forward<Targs>(args)...); }

	/**
	 * Top-level network safe command execution for the current company.
	 * Must not be called recursively. The callback is called when the
	 * command succeeded or failed.
	 *
	 * @param err_message Message prefix to show on error
	 * @param callback A callback function to call after the command is finished
	 * @param args Parameters for the command
	 * @return \c true if the command succeeded, else \c false.
	 */
	template <typename Tcallback>
	static bool Post(StringID err_message, Tcallback *callback, Targs... args)
	{
		assert(::IsNetworkRegisteredCallback(reinterpret_cast<CommandCallback *>(reinterpret_cast<void(*)()>(callback))));
		return InternalPost(err_message, callback, true, false, std::forward_as_tuple(args...));
	}

	/**
	 * Execute a command coming from the network.
	 * @param err_message Message prefix to show on error
	 * @param callback A callback function to call after the command is finished
	 * @param my_cmd indicator if the command is from a company or server (to display error messages for a user)
	 * @param args Parameters for the command
	 * @return \c true if the command succeeded, else \c false.
	 */
	template <typename Tcallback>
	static bool PostFromNet(StringID err_message, Tcallback *callback, bool my_cmd, std::tuple<Targs...> args)
	{
		return InternalPost(err_message, callback, my_cmd, true, std::move(args));
	}

	/**
	 * Prepare a command to be send over the network
	 * @param cmd The command to execute (a CMD_* value)
	 * @param err_message Message prefix to show on error
	 * @param company The company that wants to send the command
	 * @param args Parameters for the command
	 */
	static void SendNet(StringID err_message, CompanyID company, Targs... args)
	{
		auto args_tuple = std::forward_as_tuple(args...);

		::NetworkSendCommand(Tcmd, err_message, nullptr, company, EndianBufferWriter<CommandDataBuffer>::FromValue(args_tuple));
	}

	/**
	 * Top-level network safe command execution without safety checks.
	 * @param err_message Message prefix to show on error
	 * @param callback A callback function to call after the command is finished
	 * @param my_cmd indicator if the command is from a company or server (to display error messages for a user)
	 * @param estimate_only whether to give only the estimate or also execute the command
	 * @param location Tile location for user feedback.
	 * @param args Parameters for the command
	 * @return the command cost of this function.
	 */
	template <typename Tcallback>
	static Tret Unsafe(StringID err_message, Tcallback *callback, bool, bool estimate_only, TileIndex location, std::tuple<Targs...> args)
	{
		ExecuteFrame frame(args, err_message, reinterpret_cast<CommandCallback *>(reinterpret_cast<void(*)()>(callback)), location);
		PrepareExecute(frame);
		uint8_t slot = openttd_rust_command_execute(&GetRustCommandServices(), &frame, Tcmd, to_underlying(CommandTraits<Tcmd>::type), GetCommandFlags<Tcmd>().base(), estimate_only, location.base());
		if (slot == UINT8_MAX) AbandonExecution(frame.exception);
		return std::move(frame.res[slot]);
	}

protected:
	/** Helper to process a single ClientID argument. */
	template <class T>
	static inline void SetClientIdHelper([[maybe_unused]] T &data)
	{
		if constexpr (std::is_same_v<ClientID, T>) {
			if (data == INVALID_CLIENT_ID) data = CLIENT_ID_SERVER;
		}
	}

	/** Set all invalid ClientID's to the proper value. */
	template <class Ttuple, size_t... Tindices>
	static inline void SetClientIds(Ttuple &values, std::index_sequence<Tindices...>)
	{
		((SetClientIdHelper(std::get<Tindices>(values))), ...);
	}

	/** Remove the first element of a tuple. */
	template <template <typename...> typename Tt, typename T1, typename... Ts>
	static inline Tt<Ts...> RemoveFirstTupleElement(const Tt<T1, Ts...> &tuple)
	{
		return std::apply([](auto &&, const auto&... args) { return std::tie(args...); }, tuple);
	}

	template <typename Tcallback>
	static bool InternalPost(StringID err_message, Tcallback *callback, bool my_cmd, bool network_command, std::tuple<Targs...> args)
	{
		/* Where to show the message? */
		TileIndex tile{};
		if constexpr (std::is_same_v<TileIndex, std::tuple_element_t<0, decltype(args)>>) {
			tile = std::get<0>(args);
		}

		return InternalPost(err_message, callback, my_cmd, network_command, tile, std::move(args));
	}

	template <typename Tcallback>
	static bool InternalPost(StringID err_message, Tcallback *callback, bool my_cmd, bool network_command, TileIndex tile, std::tuple<Targs...> args)
	{
		ExecuteFrame frame(args, err_message, reinterpret_cast<CommandCallback *>(reinterpret_cast<void(*)()>(callback)), tile);
		PrepareExecute(frame);
		frame.callback = &Callback<Tcallback>;
		uint8_t result = openttd_rust_command_post(&GetRustCommandServices(), &frame, Tcmd, to_underlying(CommandTraits<Tcmd>::type), GetCommandFlags<Tcmd>().base(), err_message, my_cmd, network_command, tile.base(), callback != nullptr);
		if (result == UINT8_MAX) AbandonExecution(frame.exception);
		return result != 0;
	}

	/** Helper to process a single ClientID argument. */
	template <class T>
	static inline bool ClientIdIsSet([[maybe_unused]] T &data)
	{
		if constexpr (std::is_same_v<ClientID, T>) {
			return data != INVALID_CLIENT_ID;
		} else {
			return true;
		}
	}

	/** Check if all ClientID arguments are set to valid values. */
	template <class Ttuple, size_t... Tindices>
	static inline bool AllClientIdsSet(Ttuple &values, std::index_sequence<Tindices...>)
	{
		return (ClientIdIsSet(std::get<Tindices>(values)) && ...);
	}

	template <class Ttuple>
	static inline Money ExtractAdditionalMoney([[maybe_unused]] Ttuple &values)
	{
		if constexpr (std::is_same_v<std::tuple_element_t<1, Tret>, Money>) {
			return std::get<1>(values);
		} else {
			return {};
		}
	}

};

/**
 * Overload for #CommandHelper that exposes additional \c Post variants
 * for commands that don't take a TileIndex themselves.
 * @tparam Tcmd The command-id to execute.
 * @tparam Tret Return type of the command.
 * @tparam Targs The command parameter types.
 */
template <Commands Tcmd, typename Tret, typename... Targs>
struct CommandHelper<Tcmd, Tret(*)(DoCommandFlags, Targs...), false> : CommandHelper<Tcmd, Tret(*)(DoCommandFlags, Targs...), true>
{
	/* Do not allow Post without explicit location. */
	static inline bool Post(StringID err_message, Targs... args) = delete;
	template <typename Tcallback>
	static inline bool Post(Tcallback *callback, Targs... args) = delete;
	static inline bool Post(Targs... args) = delete;
	template <typename Tcallback>
	static bool Post(StringID err_message, Tcallback *callback, Targs... args) = delete;

	/**
	 * Shortcut for Post when not using a callback.
	 * @param err_message Message prefix to show on error
	 * @param location Tile location for user feedback.
	 * @param args Parameters for the command
	 */
	static inline bool Post(StringID err_message, TileIndex location, Targs... args) { return Post<CommandCallback>(err_message, nullptr, location, std::forward<Targs>(args)...); }
	/**
	 * Shortcut for Post when not using an error message.
	 * @param callback A callback function to call after the command is finished
	 * @param location Tile location for user feedback.
	 * @param args Parameters for the command
	 */
	template <typename Tcallback>
	static inline bool Post(Tcallback *callback, TileIndex location, Targs... args) { return Post((StringID)0, callback, location, std::forward<Targs>(args)...); }
	/**
	 * Shortcut for Post when not using a callback or an error message.
	 * @param location Tile location for user feedback.
	 * @param args Parameters for the command*
	 */
	static inline bool Post(TileIndex location, Targs... args) { return Post<CommandCallback>((StringID)0, nullptr, location, std::forward<Targs>(args)...); }

	/**
	 * Post variant that takes a TileIndex (for error window location and text effects) for
	 * commands that don't take a TileIndex by themselves.
	 * @param err_message Message prefix to show on error
	 * @param callback A callback function to call after the command is finished
	 * @param args Parameters for the command
	 * @return \c true if the command succeeded, else \c false.
	 */
	template <typename Tcallback>
	static inline bool Post(StringID err_message, Tcallback *callback, TileIndex location, Targs... args)
	{
		return CommandHelper<Tcmd, Tret(*)(DoCommandFlags, Targs...), true>::InternalPost(err_message, callback, true, false, location, std::forward_as_tuple(args...));
	}
};

#ifdef SILENCE_GCC_FUNCTION_POINTER_CAST
#	pragma GCC diagnostic pop
#endif

template <Commands Tcmd>
using Command = CommandHelper<Tcmd, typename CommandTraits<Tcmd>::ProcType, !GetCommandFlags<Tcmd>().Test(CommandFlag::Location)>;

#endif /* COMMAND_FUNC_H */
