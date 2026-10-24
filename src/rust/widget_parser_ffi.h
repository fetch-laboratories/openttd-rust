/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file widget_parser_ffi.h Widget-only pull parser; no widget layout crosses ABI. */
#ifndef RUST_WIDGET_PARSER_FFI_H
#define RUST_WIDGET_PARSER_FFI_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct OpenTTDRustWidgetParser OpenTTDRustWidgetParser;
typedef struct {
	uint64_t part; ///< Original borrowed descriptor offset.
	uint64_t slot; ///< Stable C++ owner slot, never an object pointer.
	uint64_t other; ///< Source owner for attachment/assignment.
	uint8_t kind;
} OpenTTDRustWidgetAction;

enum OpenTTDRustWidgetOperation {
	OTTD_WIDGET_DONE, OTTD_WIDGET_CHECK_NONNULL, OTTD_WIDGET_CREATE_VERTICAL,
	OTTD_WIDGET_OBSERVE_PARENT, OTTD_WIDGET_READ_PART, OTTD_WIDGET_CREATE_NODE,
	OTTD_WIDGET_APPLY_ATTRIBUTE, OTTD_WIDGET_OBSERVE_TYPE, OTTD_WIDGET_ADD_CONTAINER,
	OTTD_WIDGET_ADD_BACKGROUND, OTTD_WIDGET_ASSIGN_PARENT, OTTD_WIDGET_RELEASE,
	OTTD_WIDGET_ASSERT_END, OTTD_WIDGET_ERROR_ATTRIBUTE, OTTD_WIDGET_ERROR_TRAILING,
	OTTD_WIDGET_ASSERT_FIRST, OTTD_WIDGET_OBSERVE_HORIZONTAL, OTTD_WIDGET_QUERY_CAPTION,
	OTTD_WIDGET_QUERY_SHADE, OTTD_WIDGET_CREATE_SHADE, OTTD_WIDGET_WRITE_SHADE,
};

/** Return bit 0 attribute / bit 1 declared container, using exact raw uint8 tags. */
uint8_t openttd_rust_widget_part_classify(uint8_t type);

/** Window mode is 0 MakeNWidgets or 1 MakeWindowNWidgetTree; trailing_check is the C++ WITH_ASSERT
 * policy. Typed C++ operations and callbacks run only after advance returns, so reentrant parsers
 * own independent engines and exceptions never cross a Rust frame. Supplied unique_ptr&& stays a
 * C++ reference until the final return; destroying on an exception performs no pending actions. */
OpenTTDRustWidgetParser *openttd_rust_widget_parser_create(uint64_t length, uint8_t window, uint8_t trailing_check);

/**
 * Response is the preceding action's raw uint8 part/object tag, boolean, or
 * parent capability bits (bit 0 nonnull, bit 1 container, bit 2 background), else 0.
 * Initial response 0. C++ keeps RTTI observations and typed owners in stable slots.
 * Done slot identifies the owner moved only at the final successful return.
 */
OpenTTDRustWidgetAction openttd_rust_widget_parser_advance(OpenTTDRustWidgetParser *parser, uint8_t response);
void openttd_rust_widget_parser_destroy(OpenTTDRustWidgetParser *parser);
#ifdef __cplusplus
}
#endif
#endif /* RUST_WIDGET_PARSER_FFI_H */
