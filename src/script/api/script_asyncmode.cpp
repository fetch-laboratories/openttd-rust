/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_asyncmode.cpp Implementation of ScriptAsyncMode. */

#include "../../stdafx.h"
#include "script_asyncmode.hpp"
#include "../script_instance.hpp"
#include "../script_fatalerror.hpp"

#include "../../safeguards.h"

ScriptAsyncMode::ScriptAsyncMode(HSQUIRRELVM vm)
{
	int nparam = sq_gettop(vm) - 1;
	if (nparam < 1) {
		throw sq_throwerror(vm, "You need to pass a boolean to the constructor");
	}

	SQBool sqasync;
	if (SQ_FAILED(sq_getbool(vm, 2, &sqasync))) {
		throw sq_throwerror(vm, "Argument must be a boolean");
	}

	this->last_mode     = this->GetDoCommandAsyncMode();
	this->last_instance = this->GetDoCommandAsyncModeInstance();

	this->SetDoCommandAsyncMode(sqasync ? SCRIPT_MODE_EXEC : SCRIPT_MODE_TEST, this);
}

void ScriptAsyncMode::FinalRelease()
{
	if (this->GetDoCommandAsyncModeInstance() != this) {
		/* Ignore this error if the script is not alive. */
		if (ScriptObject::GetActiveInstance().IsAlive()) {
			throw Script_FatalError("Asyncmode object was removed while it was not the latest *Mode object created.");
		}
	}
}

ScriptAsyncMode::~ScriptAsyncMode()
{
	this->SetDoCommandAsyncMode(this->last_mode, this->last_instance);
}
