/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file town_data_ffi.h Town-data import: native file and JSON operations, Rust import policy. */
#ifndef RUST_TOWN_DATA_FFI_H
#define RUST_TOWN_DATA_FFI_H
#include "town_founding_ffi.h"
#include "byte_span_ffi.h"
struct OpenTTDTownDataFrame;
enum OpenTTDTownDataFeatureStatus : uint8_t { OPENTTD_TOWN_DATA_OK, OPENTTD_TOWN_DATA_END, OPENTTD_TOWN_DATA_MALFORMED, OPENTTD_TOWN_DATA_NAMED_MALFORMED };
/* One validated feature; name lives in the frame until the next feature call. */
struct OpenTTDTownDataFeature { OpenTTDByteSpan name; uint32_t population; float x, y; bool city; };
struct OpenTTDTownDataStrings { uint32_t load_failed, json_format, town_format, bad_coordinate, failed_towns; };
/* Every JSON operation, including parse exceptions, completes inside load or
 * feature; the commands raise no ordinary exception. */
struct OpenTTDTownDataServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDTownFoundingServices *founding;
	OpenTTDTownDataStrings strings;
	bool (*load)(OpenTTDTownDataFrame *) noexcept OPENTTD_SERVICE_DEFAULT(load);
	uint8_t (*feature)(OpenTTDTownDataFrame *, size_t, OpenTTDTownDataFeature *) noexcept OPENTTD_SERVICE_DEFAULT(feature);
	void (*error)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(error);
	void (*named_error)(uint32_t, uint32_t, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(named_error);
	void (*failed_warning)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(failed_warning);
	uint16_t (*found)(uint32_t, TownSize, bool, TownLayout, bool, uint32_t, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(found);
	void (*place_sign)(uint32_t, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(place_sign);
	void (*expand)(uint16_t, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(expand);
	void (*not_reached)() noexcept OPENTTD_SERVICE_DEFAULT(not_reached);
};
extern "C" void openttd_rust_load_town_data(const OpenTTDTownDataServices *, OpenTTDTownDataFrame *, uint8_t);
#endif /* RUST_TOWN_DATA_FFI_H */
