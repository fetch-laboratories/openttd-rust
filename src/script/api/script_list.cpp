/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_list.cpp Implementation of ScriptList. */

#include "../../stdafx.h"
#include "script_list.hpp"
#include "../../debug.h"
#include "../../script/squirrel.hpp"

#include "../../safeguards.h"


static_assert(sizeof(SQInteger) == sizeof(int64_t) && std::is_signed_v<SQInteger>);
static_assert(sizeof(int) == sizeof(int32_t));

static_assert(sizeof(SQRESULT) == sizeof(int64_t) && std::is_signed_v<SQRESULT>);
static_assert(sizeof(SQBool) == sizeof(uint64_t) && std::is_unsigned_v<SQBool>);
static_assert(OT_NULL == 0x01000001 && OT_INTEGER == 0x05000002 && OT_BOOL == 0x01000008);
static_assert(OT_TABLE == 0x0A000020 && OT_ARRAY == 0x08000040);
static_assert(OT_CLOSURE == 0x08000100 && OT_NATIVECLOSURE == 0x08000200);
static_assert(SQ_ERROR == -1 && SQTrue == 1 && SQFalse == 0);

ScriptList::VMControl::VMControl(HSQUIRRELVM vm, ScriptList *list, uint8_t operation) :
	vm(vm), list(list), control(openttd_rust_list_control_new(operation), openttd_rust_list_control_destroy)
{
}

SQInteger ScriptList::VMControl::Drive()
{
	this->needs_index = false;
	for (;;) {
		OpenTTDListControlAction action{};
		openttd_rust_list_control_step(this->control.get(), this->list->owner.get(), &this->input, &action);
		this->input = {};
		/* No Rust stack or borrow exists while these real VM/RAII operations
		 * execute. Exceptions retain the original stack and partial list state. */
		switch (action.kind) {
			case LC_RETURN: return action.a;
			case LC_ITEM: return 0;
			case LC_INDEX: this->needs_index = true; return 0;
			case LC_TOP: this->input.a = sq_gettop(this->vm); break;
			case LC_TYPE: this->input.kind = static_cast<uint32_t>(sq_gettype(this->vm, action.a)); break;
			case LC_GET_INT: {
				SQInteger value;
				/* _nexti ignores failure and the value. Do not read that local
				 * on failure; load's historical ignored getters stay below. */
				if (SQ_SUCCEEDED(sq_getinteger(this->vm, action.a, &value))) this->input.a = value;
				break;
			}
			case LC_GET_BOOL: {
				SQBool value;
				sq_getbool(this->vm, action.a, &value);
				this->input.flag = value;
				break;
			}
			case LC_GET_PAIR: {
				/* Preserve the original AND type predicate and ignored getter
				 * results. Defined mixed numeric values convert in the actual VM.
				 * A nonnumeric side paired with integer can read an uninitialized
				 * local upstream, and remains outside the defined-input contract. */
				SQInteger key, value;
				sq_getinteger(this->vm, -2, &key);
				sq_getinteger(this->vm, -1, &value);
				this->input.a = key;
				this->input.b = value;
				break;
			}
			case LC_PUSH: sq_push(this->vm, action.a); break;
			case LC_ROOT: sq_pushroottable(this->vm); break;
			case LC_PUSH_INT: sq_pushinteger(this->vm, action.a); break;
			case LC_PUSH_BOOL: sq_pushbool(this->vm, action.a != 0 ? SQTrue : SQFalse); break;
			case LC_PUSH_NULL: sq_pushnull(this->vm); break;
			case LC_TAG: sq_pushstring(this->vm, "List"); break;
			case LC_NEW_ARRAY: sq_newarray(this->vm, 0); break;
			case LC_NEW_TABLE: sq_newtable(this->vm); break;
			case LC_APPEND: sq_arrayappend(this->vm, action.a); break;
			case LC_RAW_SET: sq_rawset(this->vm, action.a); break;
			case LC_NEXT: this->input.flag = SQ_SUCCEEDED(sq_next(this->vm, action.a)); break;
			case LC_POP: sq_pop(this->vm, action.a); break;
			case LC_POP_TOP: sq_poptop(this->vm); break;
			case LC_CALL: this->input.flag = SQ_SUCCEEDED(sq_call(this->vm, action.a, SQTrue, SQFalse)); break;
			case LC_CHARGE: Squirrel::DecreaseOps(this->vm, static_cast<int>(action.a)); break;
			case LC_DISABLE: this->disabler.emplace(); break;
			case LC_LIMIT: this->limiter.emplace(this->vm, MAX_VALUATE_OPS, action.a == 0 ? "valuator function" : "list filter function"); break;
			case LC_THROW_RESULT: throw static_cast<SQInteger>(action.a);
			case LC_ERROR:
			case LC_THROW_ERROR: {
				const char *message;
				switch (action.a) {
					case 1: message = "You need to give at least a Valuator as parameter to ScriptList::Valuate"; break;
					case 2: message = "parameter 1 has an invalid type (expected function)"; break;
					case 3: message = "return value of valuator is not valid (not integer/bool)"; break;
					case 4: message = "modifying valuated list outside of valuator function"; break;
					case 5: message = "return value of filter is not valid (not bool)"; break;
					case 6: message = "you can only assign integers to this list"; break;
					default: NOT_REACHED();
				}
				SQInteger result = sq_throwerror(this->vm, message);
				if (action.kind == LC_THROW_ERROR) throw result;
				return result;
			}
			case LC_LOG_NEXT: Debug(script, 0, "Next() is invalid as Begin() is never called"); break;
			case LC_LOG_END: Debug(script, 0, "IsEnd() is invalid as Begin() is never called"); break;
			default: NOT_REACHED();
		}
	}
}

void ScriptList::VMControl::Item(bool valid)
{
	this->input.flag = valid ? 2 : 1;
	this->Drive();
}

void ScriptList::VMControl::Index(SQInteger index)
{
	this->input.a = index;
	this->Drive();
}

void ScriptList::VMControl::Finish()
{
	this->input.flag = 0;
	this->Drive();
}

bool ScriptList::SaveObject(HSQUIRRELVM vm)
{
	return VMControl(vm, this, 2).Drive() != 0;
}

bool ScriptList::LoadObject(HSQUIRRELVM vm)
{
	return VMControl(vm, this, 3).Drive() != 0;
}

ScriptObject *ScriptList::CloneObject()
{
	ScriptList *clone = new ScriptList();
	clone->CopyList(this);
	return clone;
}

void ScriptList::CopyList(const ScriptList *list)
{
	openttd_rust_list_combine(this->owner.get(), list->owner.get(), 4);
}

ScriptList::ScriptList() : owner(openttd_rust_list_new(), openttd_rust_list_destroy) {}

ScriptList::~ScriptList()
{
}

bool ScriptList::HasItem(SQInteger item)
{
	int64_t value;
	return openttd_rust_list_get(this->owner.get(), item, &value) != 0;
}

void ScriptList::Clear()
{
	openttd_rust_list_clear(this->owner.get());
}

void ScriptList::AddItem(SQInteger item, SQInteger value)
{
	openttd_rust_list_add(this->owner.get(), item, value);
}

void ScriptList::RemoveItem(SQInteger item)
{
	openttd_rust_list_remove(this->owner.get(), item);
}

SQInteger ScriptList::Begin()
{
	int64_t value;
	openttd_rust_list_iter(this->owner.get(), 0, &value);
	return value;
}

SQInteger ScriptList::Next()
{
	int64_t value;
	if (openttd_rust_list_iter(this->owner.get(), 1, &value) == 0) Debug(script, 0, "Next() is invalid as Begin() is never called");
	return value;
}

bool ScriptList::IsEmpty()
{
	return openttd_rust_list_count(this->owner.get()) == 0;
}

bool ScriptList::IsEnd()
{
	int64_t value;
	if (openttd_rust_list_iter(this->owner.get(), 2, &value) == 0) {
		Debug(script, 0, "IsEnd() is invalid as Begin() is never called");
		return true;
	}
	return value != 0;
}

SQInteger ScriptList::Count()
{
	return openttd_rust_list_count(this->owner.get());
}

SQInteger ScriptList::GetValue(SQInteger item)
{
	int64_t value = 0;
	openttd_rust_list_get(this->owner.get(), item, &value);
	return value;
}

bool ScriptList::SetValue(SQInteger item, SQInteger value)
{
	return openttd_rust_list_set(this->owner.get(), item, value) != 0;
}

void ScriptList::Sort(SorterType sorter, bool ascending)
{
	openttd_rust_list_sort(this->owner.get(), sorter, ascending);
}

void ScriptList::AddList(ScriptList *list)
{
	openttd_rust_list_combine(this->owner.get(), list->owner.get(), 0);
}

void ScriptList::SwapList(ScriptList *list)
{
	openttd_rust_list_combine(this->owner.get(), list->owner.get(), 1);
}

void ScriptList::RemoveAboveValue(SQInteger value)
{
	openttd_rust_list_filter(this->owner.get(), 0, value, 0);
}

void ScriptList::RemoveBelowValue(SQInteger value)
{
	openttd_rust_list_filter(this->owner.get(), 1, value, 0);
}

void ScriptList::RemoveBetweenValue(SQInteger start, SQInteger end)
{
	openttd_rust_list_filter(this->owner.get(), 2, start, end);
}

void ScriptList::RemoveValue(SQInteger value)
{
	openttd_rust_list_filter(this->owner.get(), 3, value, 0);
}

void ScriptList::RemoveTop(SQInteger count)
{
	openttd_rust_list_rank(this->owner.get(), 0, count);
}

void ScriptList::RemoveBottom(SQInteger count)
{
	openttd_rust_list_rank(this->owner.get(), 1, count);
}

void ScriptList::RemoveList(ScriptList *list)
{
	openttd_rust_list_combine(this->owner.get(), list->owner.get(), 2);
}

void ScriptList::KeepAboveValue(SQInteger value)
{
	openttd_rust_list_filter(this->owner.get(), 4, value, 0);
}

void ScriptList::KeepBelowValue(SQInteger value)
{
	openttd_rust_list_filter(this->owner.get(), 5, value, 0);
}

void ScriptList::KeepBetweenValue(SQInteger start, SQInteger end)
{
	openttd_rust_list_filter(this->owner.get(), 6, start, end);
}

void ScriptList::KeepValue(SQInteger value)
{
	openttd_rust_list_filter(this->owner.get(), 7, value, 0);
}

void ScriptList::KeepTop(SQInteger count)
{
	openttd_rust_list_rank(this->owner.get(), 2, count);
}

void ScriptList::KeepBottom(SQInteger count)
{
	openttd_rust_list_rank(this->owner.get(), 3, count);
}

void ScriptList::KeepList(ScriptList *list)
{
	openttd_rust_list_combine(this->owner.get(), list->owner.get(), 3);
}

SQInteger ScriptList::_get(HSQUIRRELVM vm)
{
	return VMControl(vm, this, 4).Drive();
}

SQInteger ScriptList::_set(HSQUIRRELVM vm)
{
	return VMControl(vm, this, 5).Drive();
}

SQInteger ScriptList::_nexti(HSQUIRRELVM vm)
{
	return VMControl(vm, this, 6).Drive();
}

SQInteger ScriptList::Valuate(HSQUIRRELVM vm)
{
	return VMControl(vm, this, 0).Drive();
}
