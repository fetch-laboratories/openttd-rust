/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act7_9.cpp NewGRF Action 0x07 and Action 0x09 handler. */

#include "../stdafx.h"
#include "../debug.h"
#include "../genworld.h"
#include "../network/network.h"
#include "../newgrf_engine.h"
#include "../newgrf_cargo.h"
#include "../rail.h"
#include "../road.h"
#include "../settings_type.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"

#include "../safeguards.h"
