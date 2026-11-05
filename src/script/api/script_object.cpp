/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_object.cpp Implementation of ScriptObject. */

#include "../../stdafx.h"
#include "../../script/squirrel.hpp"
#include "../../company_func.h"
#include "../../company_base.h"
#include "../../network/network.h"
#include "../../genworld.h"
#include "../../string_func.h"
#include "../../strings_func.h"
#include "../../command_func.h"
#include "../../rust/command_cost_services.h"

#include "../script_instance.hpp"
#include "../script_fatalerror.hpp"
#include "script_error.hpp"
#include "../../debug.h"
#include "../squirrel_helper.hpp"

#include "../../safeguards.h"

void SimpleCountedObject::Release()
{
	int32_t res = --this->ref_count;
	assert(res >= 0);
	if (res == 0) {
		try {
			this->FinalRelease(); // may throw, for example ScriptTest/ExecMode
		} catch (...) {
			delete this;
			throw;
		}
		delete this;
	}
}

ScriptObject::ActiveInstance::ActiveInstance(ScriptInstance &instance) : alc_scope(instance.engine.get())
{
	this->last_active = std::exchange(GetRustScriptContext().active, &instance.GetStorage());
}

ScriptObject::ActiveInstance::~ActiveInstance()
{
	GetRustScriptContext().active = this->last_active;
}

ScriptObject::DisableDoCommandScope::DisableDoCommandScope()
	: AutoRestoreBackup(GetStorage().allow_do_command, false)
{}

/* static */ ScriptInstance &ScriptObject::GetActiveInstance()
{
	assert(GetRustScriptContext().active != nullptr);
	return *GetRustScriptContext().active->instance;
}


/* static */ void ScriptObject::SetLastCommand(const CommandDataBuffer &data, Commands cmd)
{
	OpenTTDScriptHost &s = GetStorage();
	Debug(script, 6, "SetLastCommand company={:02d} cmd={} data={}", s.root_company, cmd, FormatArrayAsHex(data));
	openttd_rust_script_set_last_command(&s, data.data(), data.size(), cmd);
}

/* static */ bool ScriptObject::CanSuspend()
{
	Squirrel &squirrel = *ScriptObject::GetActiveInstance().engine;
	return GetStorage().allow_do_command && squirrel.CanSuspend();
}

/* static */ ScriptLogTypes::LogData &ScriptObject::GetLogData()
{
	return GetActiveInstance().log_data;
}

/* static */ CommandCallbackData *ScriptObject::GetDoCommandCallback()
{
	return ScriptObject::GetActiveInstance().GetDoCommandCallback();
}

/* static */ std::tuple<bool, bool, bool, bool> ScriptObject::DoCommandPrep()
{
	if (!ScriptObject::CanSuspend()) {
		throw Script_FatalError("You are not allowed to execute any DoCommand (even indirect) in your constructor, Save(), Load(), and any valuator.");
	}

	auto [rejected, estimate_only, asynchronous, networking] = openttd_rust_script_command_prep(&GetStorage(), &GetRustScriptHostServices());
	return { rejected, estimate_only, asynchronous, networking };
}

/* static */ bool ScriptObject::DoCommandProcessResult(const CommandCost &res, Script_SuspendCallbackProc *callback, bool estimate_only, bool asynchronous)
{
	/* Set the default callback to return a true/false result of the DoCommand */
	if (callback == nullptr) callback = &ScriptInstance::DoCommandReturn;

	auto [outcome, suspend] = openttd_rust_script_command_result(&GetStorage(), &GetRustScriptHostServices(), RustCommandCost(&res), estimate_only, asynchronous);
	switch (outcome) {
		case OpenTTDScriptOutcome::Failed: return false;
		case OpenTTDScriptOutcome::Done: return true;
		case OpenTTDScriptOutcome::ChargedCallback:
			/* Charge a nominal fee for asynchronously executed commands */
			Squirrel::DecreaseOps(ScriptObject::GetActiveInstance().engine->GetVM(), 100);
			[[fallthrough]];
		case OpenTTDScriptOutcome::Callback:
			/* Insert return value into to stack and throw a control code that
			 * the return value in the stack should be used. */
			callback(GetActiveInstance());
			throw SQInteger(1);
		case OpenTTDScriptOutcome::Suspend:
			/* Suspend the script till the command is really executed, or for 1+ ticks
			 * to simulate multiplayer. */
			throw Script_Suspend(suspend, callback);
	}

	NOT_REACHED();
}


/* static */ ScriptObject::RandomizerArray ScriptObject::random_states;

/* static */ Randomizer &ScriptObject::GetRandomizer(Owner owner)
{
	return ScriptObject::random_states[owner];
}

/* static */ void ScriptObject::InitializeRandomizers()
{
	Randomizer random = _random;
	for (Owner owner = OWNER_BEGIN; owner < OWNER_END; ++owner) {
		ScriptObject::GetRandomizer(owner).SetSeed(random.Next());
	}
}

/* static */ SQInteger ScriptObject::Constructor(HSQUIRRELVM)
{
	throw Script_FatalError("This class is not instantiatable");
}

/* static */ SQInteger ScriptObject::_cloned(HSQUIRRELVM vm)
{
	ScriptObject *original = static_cast<ScriptObject *>(Squirrel::GetRealInstance(vm, 2, "Object"));
	if (ScriptObject *clone = original->CloneObject(); clone != nullptr) {
		clone->AddRef();
		sq_setinstanceup(vm, 1, clone);
		sq_setreleasehook(vm, 1, SQConvert::DefSQDestructorCallback<ScriptObject>);
		return 0;
	}

	throw Script_FatalError("This instance is not cloneable");
}
