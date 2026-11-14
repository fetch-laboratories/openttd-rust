/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file news_ffi.h Native news, string and script event entries that Rust calls, bound once for the process. */
#ifndef RUST_NEWS_FFI_H
#define RUST_NEWS_FFI_H
#include <cstddef>
#include <cstdint>
#include "abi-record-news.h"
#include "abi-record-script_event.h"
#include "encoded_ffi.h"
#include "service_defaults.hpp"

/** Storage for one StringParameter, constructed in place on the Rust caller's stack. */
struct OpenTTDStringParameterSlot {
	alignas(8) uint8_t bytes[64];
};
/** Receives the bytes of a native string during the call that produced it. */
using OpenTTDRustStringSink = void (*)(void *target, const uint8_t *bytes, size_t length) noexcept;

struct OpenTTDNewsServices {
	void (*add_news_item)(const OpenTTDAddNewsItemArgs *args) noexcept OPENTTD_SERVICE_DEFAULT(add_news_item);
	void (*assert_encoded)(uint32_t codepoint, size_t length) noexcept OPENTTD_SERVICE_DEFAULT(assert_encoded);
	void (*company_news_company_name)(const NewsAllocatedData *data, OpenTTDRustStringSink sink, void *target) noexcept OPENTTD_SERVICE_DEFAULT(company_news_company_name);
	NewsAllocatedData *(*company_news_information)(uint32_t title, uint8_t company, uint8_t other) noexcept OPENTTD_SERVICE_DEFAULT(company_news_information);
	void (*delete_industry_news)(uint16_t industry) noexcept OPENTTD_SERVICE_DEFAULT(delete_industry_news);
	void (*delete_station_news)(uint16_t station) noexcept OPENTTD_SERVICE_DEFAULT(delete_station_news);
	void (*delete_vehicle_news)(uint32_t vehicle, uint8_t advice_type) noexcept OPENTTD_SERVICE_DEFAULT(delete_vehicle_news);
	void (*get_string)(uint32_t string, const OpenTTDRustEncodedParameter *args, OpenTTDStringParameterSlot *slots, size_t count, OpenTTDRustStringSink sink, void *target) noexcept OPENTTD_SERVICE_DEFAULT(get_string);
	uint8_t string_assertions;
};
extern "C" void openttd_rust_news_bind(const OpenTTDNewsServices *services);

struct OpenTTDScriptEventServices {
	void (*ai_broadcast)(const OpenTTDScriptEvent *event, uint8_t skip_company) noexcept OPENTTD_SERVICE_DEFAULT(ai_broadcast);
	void (*ai_new)(uint8_t company, const OpenTTDScriptEvent *event) noexcept OPENTTD_SERVICE_DEFAULT(ai_new);
	void (*game_new)(const OpenTTDScriptEvent *event) noexcept OPENTTD_SERVICE_DEFAULT(game_new);
};
extern "C" void openttd_rust_script_event_bind(const OpenTTDScriptEventServices *services);
#endif /* RUST_NEWS_FFI_H */
