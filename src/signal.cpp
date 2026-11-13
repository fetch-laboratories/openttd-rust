/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file signal.cpp functions related to rail signals updating */

#include "stdafx.h"
#include "debug.h"
#include "station_map.h"
#include "tunnelbridge_map.h"
#include "vehicle_func.h"
#include "viewport_func.h"
#include "train.h"
#include "company_base.h"
#include "pbs.h"

#include "table/signal_data.h"

#include "safeguards.h"


#include "rust/signal_ffi.h"
#include "rust/command_services_ffi.h"
#include "rust/command_cost_services.h"
#include "rust/services_assert.h"
#include "rust/services_rail.h"
#include "rust/company_services.h"
#include "rust/environment_services.h"
#include "rust/vehicle_services.h"

static const OpenTTDSignalServices _signal_services{
#ifdef WITH_ASSERT
	.assertions = true,
#else
	.assertions = false,
#endif
	.assert_condition = RustAssertCondition,
	.vehicles = &_rust_vehicle_services,
};

const OpenTTDSignalServices &GetRustSignalServices() noexcept
{
	return _signal_services;
}

void UpdateSignalsInBuffer()
{
	openttd_rust_signal_flush(&GetRustSharedServices(), &GetRustRailMapServices(), &_signal_services);
}
void AddTrackToSignalBuffer(TileIndex tile, Track track, Owner owner)
{
	openttd_rust_signal_add_track(&GetRustSharedServices(), &GetRustRailMapServices(), &_signal_services, tile.base(), track, owner.base());
}
void AddSideToSignalBuffer(TileIndex tile, DiagDirection side, Owner owner)
{
	openttd_rust_signal_add_side(&GetRustSharedServices(), &GetRustRailMapServices(), &_signal_services, tile.base(), side, owner.base());
}
SigSegState UpdateSignalsOnSegment(TileIndex tile, DiagDirection side, Owner owner)
{
	return static_cast<SigSegState>(openttd_rust_signal_segment(&GetRustSharedServices(), &GetRustRailMapServices(), &_signal_services, tile.base(), side, owner.base()));
}
void SetSignalsOnBothDir(TileIndex tile, Track track, Owner owner)
{
	openttd_rust_signal_both(&GetRustSharedServices(), &GetRustRailMapServices(), &_signal_services, tile.base(), track, owner.base());
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDSignalServices.hpp"
bool CheckAbi_signal() { return AbiServicesComplete(_signal_services); }
