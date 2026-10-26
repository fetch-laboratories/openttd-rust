/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_ffi.h Canonical live station handles and association results shared by Rust ports. */
#ifndef RUST_STATION_FFI_H
#define RUST_STATION_FFI_H
#include <cstdint>

struct OpenTTDStationSpecIndex { uint8_t value; bool valid; };

struct Station;
struct BaseStation;
using OpenTTDStation = Station;
using OpenTTDBaseStation = BaseStation;

/** Rust-owned queue node; identity remains valid until the node is erased. */
struct OpenTTDStationQueueEntry;

#endif /* RUST_STATION_FFI_H */
