/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file admin_conversion.cpp Typed VM/JSON effects between Rust traversal steps. */
#include "../stdafx.h"
#include "admin_conversion.hpp"

#include "../rust/admin_conversion_ffi.h"
#include "api/script_log.hpp"
#include "script_instance.hpp"

#include "../safeguards.h"

static_assert(sizeof(SQInteger) == 8 && std::is_signed_v<SQInteger>);
static_assert(sizeof(SQRESULT) == 8 && std::is_signed_v<SQRESULT>);
static_assert(sizeof(SQBool) == 8 && !std::is_signed_v<SQBool>);
static_assert(sizeof(int) == 4 && SQUIRREL_MAX_DEPTH == 25);
static_assert(OT_NULL == 0x01000001 && OT_BOOL == 0x01000008 && OT_STRING == 0x08000010);
static_assert(OT_INTEGER == 0x05000002 && OT_TABLE == 0x0A000020 && OT_ARRAY == 0x08000040);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::null) == 0);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::object) == 1);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::array) == 2);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::string) == 3);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::boolean) == 4);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::number_integer) == 5);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::number_unsigned) == 6);
static_assert(static_cast<uint8_t>(nlohmann::json::value_t::number_float) == 7);
static_assert(offsetof(OpenTTDRustAdminAction, frame) == 0 && offsetof(OpenTTDRustAdminAction, index) == 8);
static_assert(offsetof(OpenTTDRustAdminAction, kind) == 16 && offsetof(OpenTTDRustAdminAction, argument) == 17);

namespace {

/** Stable heap storage; vector growth moves owners, never JSON/iterators/keys. */
struct AdminFrame {
	nlohmann::json temporary;
	nlohmann::json *value;
	nlohmann::json::iterator iterator;
	nlohmann::json::iterator end;
	std::optional<std::string> key;

	explicit AdminFrame(nlohmann::json *value = nullptr) : value(value == nullptr ? &this->temporary : value) {}
};

/** Match recursive temporary/key destruction on exceptions, without VM effects. */
struct AdminFrames {
	std::vector<std::unique_ptr<AdminFrame>> entries;

	~AdminFrames()
	{
		while (!this->entries.empty()) this->entries.pop_back();
	}
};

bool Convert(nlohmann::json &json, HSQUIRRELVM vm, uint8_t direction, SQInteger index, int depth)
{
	/* On C++ exceptions this destroys only control/temporary storage. No pending
	 * VM action runs, and no exception crosses a Rust call or callback frame. */
	using Owner = std::unique_ptr<OpenTTDRustAdminConversion, decltype(&openttd_rust_admin_conversion_destroy)>;
	Owner owner{openttd_rust_admin_conversion_create(direction, index, depth), openttd_rust_admin_conversion_destroy};
	AdminFrames owned_frames;
	auto &frames = owned_frames.entries;
	frames.emplace_back(std::make_unique<AdminFrame>(&json));
	auto frame_at = [&frames](uint64_t slot) -> AdminFrame & {
		assert(slot <= std::numeric_limits<size_t>::max() && slot < frames.size());
		return *frames[static_cast<size_t>(slot)];
	};
	uint64_t response = 0;
	for (;;) {
		const auto action = openttd_rust_admin_conversion_advance(owner.get(), response);
		response = 0;
		switch (action.kind) {
			case OTTD_ADMIN_DONE: return action.argument != 0;
			case OTTD_ADMIN_INSPECT_OUT: response = sq_gettype(vm, action.index); break;
			case OTTD_ADMIN_INSPECT_IN: response = static_cast<uint8_t>(frame_at(action.frame).value->type()); break;
			case OTTD_ADMIN_COPY_OUT_SCALAR: {
				auto &target = *frame_at(action.frame).value;
				switch (action.argument) {
					case 0: target = nullptr; break;
					case 1: {
						SQBool value;
						sq_getbool(vm, action.index, &value);
						target = value ? true : false;
						break;
					}
					case 2: {
						std::string_view value;
						sq_getstring(vm, action.index, value);
						target = value;
						break;
					}
					case 3: {
						SQInteger value;
						sq_getinteger(vm, action.index, &value);
						target = value;
						break;
					}
					default: NOT_REACHED();
				}
				break;
			}
			case OTTD_ADMIN_PUSH_IN_SCALAR: {
				auto &source = *frame_at(action.frame).value;
				switch (action.argument) {
					case 0: sq_pushnull(vm); break;
					case 1: sq_pushbool(vm, source.get<bool>() ? 1 : 0); break;
					case 2: {
						auto value = source.get<std::string>();
						sq_pushstring(vm, value);
						break;
					}
					case 3: sq_pushinteger(vm, source.get<int64_t>()); break;
					default: NOT_REACHED();
				}
				break;
			}
			case OTTD_ADMIN_BEGIN_OUT:
				*frame_at(action.frame).value = action.argument != 0 ? nlohmann::json::object() : nlohmann::json::array();
				sq_pushnull(vm);
				break;
			case OTTD_ADMIN_BEGIN_IN: {
				if (action.argument != 0) { sq_newtable(vm); } else { sq_newarray(vm, 0); }
				auto &frame = frame_at(action.frame);
				frame.iterator = frame.value->begin();
				frame.end = frame.value->end();
				break;
			}
			case OTTD_ADMIN_NEXT_OUT: response = SQ_SUCCEEDED(sq_next(vm, action.index)) ? 1 : 0; break;
			case OTTD_ADMIN_CHECK_IN_ITERATOR: {
				auto &frame = frame_at(action.frame);
				response = frame.iterator != frame.end ? 1 : 0;
				break;
			}
			case OTTD_ADMIN_COPY_OUT_KEY: {
				sq_tostring(vm, -2);
				std::string_view view;
				sq_getstring(vm, -1, view);
				frame_at(action.frame).key.emplace(view);
				sq_pop(vm, 1);
				break;
			}
			case OTTD_ADMIN_PUSH_IN_KEY: sq_pushstring(vm, frame_at(action.frame).iterator.key()); break;
			case OTTD_ADMIN_CHILD_OUT:
				assert(action.frame == frames.size());
				frames.emplace_back(std::make_unique<AdminFrame>());
				break;
			case OTTD_ADMIN_CHILD_IN:
				assert(action.frame == frames.size() && action.frame != 0);
				frames.emplace_back(std::make_unique<AdminFrame>(&frame_at(action.frame - 1).iterator.value()));
				break;
			case OTTD_ADMIN_POP_PAIR: sq_pop(vm, 2); break;
			case OTTD_ADMIN_POP_ITERATOR: sq_pop(vm, 1); break;
			case OTTD_ADMIN_COMMIT_OUT_ARRAY:
				frame_at(action.frame).value->push_back(*frame_at(action.frame + 1).value);
				break;
			case OTTD_ADMIN_COMMIT_OUT_TABLE:
				(*frame_at(action.frame).value)[std::move(*frame_at(action.frame).key)] = std::move(*frame_at(action.frame + 1).value);
				break;
			case OTTD_ADMIN_COMMIT_IN_ARRAY: sq_arrayappend(vm, -2); break;
			case OTTD_ADMIN_COMMIT_IN_TABLE: sq_rawset(vm, -3); break;
			case OTTD_ADMIN_ADVANCE_IN_ITERATOR: ++frame_at(action.frame).iterator; break;
			case OTTD_ADMIN_RELEASE_FRAME:
				assert(action.frame + 1 == frames.size());
				frames.pop_back();
				if (action.argument != 0) frames.back()->key.reset();
				break;
			case OTTD_ADMIN_ERROR_DEPTH: ScriptLog::Error("Send parameters can only be nested to 25 deep. No data sent."); break;
			case OTTD_ADMIN_ERROR_OUT_TYPE: ScriptLog::Error("You tried to send an unsupported type. No data sent."); break;
			case OTTD_ADMIN_ERROR_ROOT: ScriptLog::Error("The root element in the JSON data from AdminPort has to be an object."); break;
			case OTTD_ADMIN_ERROR_IN_TYPE: ScriptLog::Error("Received invalid JSON data from AdminPort."); break;
			case OTTD_ADMIN_SAVE_TOP: response = static_cast<uint64_t>(sq_gettop(vm)); break;
			case OTTD_ADMIN_RESTORE_TOP: sq_settop(vm, action.index); break;
			case OTTD_ADMIN_PUSH_NULL: sq_pushnull(vm); break;
			default: NOT_REACHED();
		}
	}
}

} // namespace

bool ScriptAdminConvertToJSON(nlohmann::json &json, HSQUIRRELVM vm, SQInteger index, int depth)
{
	return Convert(json, vm, 0, index, depth);
}

void ScriptAdminConvertFromJSON(nlohmann::json &json, HSQUIRRELVM vm)
{
	Convert(json, vm, 1, 0, 0);
}
