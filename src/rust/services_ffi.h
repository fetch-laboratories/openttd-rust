/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_ffi.h Synchronous shared game services for Rust owners. */
#ifndef RUST_SERVICES_FFI_H
#define RUST_SERVICES_FFI_H
#include "byte_span_ffi.h"
#include "../tile_storage_type.h"
#include "../newgrf_badge_type.h"
#include "abi-record-probe.h"
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include <memory>
#include <type_traits>
#include "station_ffi.h"
#include "snow_line_ffi.h"

/* Distinct trivial date carriers. Strong native Date classes stay inside C++;
 * wrappers convert explicitly so MSVC class-return rules cannot alter this ABI. */
struct OpenTTDCalendarDate { int32_t value; };
/** A `std::span<const CargoLabel>`, live until its owner changes. */
struct OpenTTDCargoLabels { const uint32_t *data; size_t size; };
struct OpenTTDEconomyDate { int32_t value; };
static_assert(std::is_trivially_copyable_v<OpenTTDCalendarDate> && std::is_standard_layout_v<OpenTTDCalendarDate>);
static_assert(std::is_trivially_copyable_v<OpenTTDEconomyDate> && std::is_standard_layout_v<OpenTTDEconomyDate>);

/* Addresses of the canonical Tile and Map statics. Rust loads them at entry
 * instead of calling; only field-sized raw reads follow, ending before a map
 * reset or mutex yield. C++ owns allocation, construction, reset and every write. */
struct OpenTTDMapStatics {
	const std::unique_ptr<TileBaseStorage[]> *base;
	const std::unique_ptr<TileExtendedStorage[]> *extended;
	const uint32_t *size;
	const uint32_t *log_x;
	const uint32_t *log_y;
	bool assertions;
};
/* Rust's TileAdd projects native _DEBUG at build time (cargo feature native-debug). */
#if defined(_DEBUG) != defined(OPENTTD_RUST_NATIVE_DEBUG)
#	error "The Rust archive's native-debug feature must match this configuration's _DEBUG"
#endif
extern const OpenTTDMapStatics _rust_map_statics;

enum Slope : uint8_t;
/** Pool ID and its coordinate on the dimension being split. */
struct OpenTTDKdtreeKey { uint16_t coordinate; uint16_t id; };
struct OpenTTDCalendarYMD { int32_t year; uint8_t month, day; };
struct OpenTTDFoundationSlope { int32_t z; Slope slope; };
struct OpenTTDEnvironmentServices {
	bool (*editor)() noexcept OPENTTD_SERVICE_DEFAULT(editor);
	bool (*freeform_edges)() noexcept OPENTTD_SERVICE_DEFAULT(freeform_edges);
};
const OpenTTDEnvironmentServices &GetRustEnvironmentServices() noexcept;
/* The tables C++ fills for Rust are defined in shared_services_table.h, which only the C++ that fills or calls them includes. */
struct OpenTTDSharedServices;
struct OpenTTDRandomServices;
struct OpenTTDRailMapServices;
extern const OpenTTDSharedServices _rust_shared_services;
struct OpenTTDSubsidyServices;
const OpenTTDSubsidyServices &GetRustSubsidyServices() noexcept;
inline const OpenTTDSubsidyServices *RustSubsidyServices() noexcept { return &GetRustSubsidyServices(); }
const OpenTTDSharedServices &GetRustSharedServices() noexcept;
const OpenTTDRandomServices &GetRustRandomServices() noexcept;
const OpenTTDRailMapServices &GetRustRailMapServices() noexcept;
#endif /* RUST_SERVICES_FFI_H */
