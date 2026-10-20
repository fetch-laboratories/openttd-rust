/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_station.h Typed airport fields shared by native Rust adapters. */
#ifndef RUST_SERVICES_STATION_H
#define RUST_SERVICES_STATION_H
#include "station_services.h"

inline uint16_t RustStationAirportWidth(const Station *station) noexcept { return station->airport.w; }
inline uint16_t RustStationAirportHeight(const Station *station) noexcept { return station->airport.h; }
inline const AirportFTAClass *RustStationAirportFTA(const Station *station) noexcept { return station->airport.GetFTA(); }
inline uint8_t RustAirportDeltaZ(const AirportFTAClass *airport) noexcept { return airport->delta_z; }

/* The existing aircraft table retains its opaque pointers and return widths. */
inline const void *OpenTTDStationAirportFTA(const OpenTTDStation *station) noexcept { return RustStationAirportFTA(StationOf(station)); }
inline uint8_t OpenTTDAirportDeltaZ(const void *airport) noexcept { return RustAirportDeltaZ(static_cast<const AirportFTAClass *>(airport)); }
#endif /* RUST_SERVICES_STATION_H */
