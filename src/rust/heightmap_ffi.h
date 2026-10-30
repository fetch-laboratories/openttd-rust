/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file heightmap_ffi.h One heightmap read: native codecs, Rust input policy. */
#ifndef RUST_HEIGHTMAP_FFI_H
#define RUST_HEIGHTMAP_FFI_H
#include "service_defaults.hpp"
#include <cstddef>
#include <cstdint>
#include "../gfx_type.h"
enum OpenTTDHeightmapDecode : uint8_t { OPENTTD_HEIGHTMAP_OK, OPENTTD_HEIGHTMAP_NOT_FOUND, OPENTTD_HEIGHTMAP_FAILED };
struct OpenTTDHeightmapStrings { uint32_t png, bmp, not_found, png_misc, png_image_type, bmp_image_type, too_large; };
/* libpng results after png_read_png; rows and RGB palette triples live until finish. */
struct OpenTTDHeightmapPng {
	uint32_t width, height;
	const uint8_t *const *rows;
	const uint8_t *palette;
	int32_t palette_size;
	uint8_t channels, bit_depth;
	bool has_palette;
};
/* BmpInfo and BmpData fields; palette and bitmap live until finish. */
struct OpenTTDHeightmapBmp {
	uint32_t width, height, palette_size;
	const Colour *palette;
	size_t palette_len;
	const uint8_t *bitmap;
	uint16_t bpp;
};
/* Each decode is one native call; no libpng longjmp or C++ exception leaves it.
 * source fixes the file kind and name for the read; finish releases every codec
 * resource and may run when none was acquired. */
struct OpenTTDHeightmapInput {
	void *context;
	OpenTTDHeightmapStrings strings;
	uint8_t (*source)(void *) noexcept OPENTTD_SERVICE_DEFAULT(source);
	uint8_t (*png)(void *, OpenTTDHeightmapPng *) noexcept OPENTTD_SERVICE_DEFAULT(png);
	uint8_t (*bmp_header)(void *, OpenTTDHeightmapBmp *) noexcept OPENTTD_SERVICE_DEFAULT(bmp_header);
	bool (*bmp_bitmap)(void *, OpenTTDHeightmapBmp *) noexcept OPENTTD_SERVICE_DEFAULT(bmp_bitmap);
	void (*finish)(void *) noexcept OPENTTD_SERVICE_DEFAULT(finish);
	void (*error)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(error);
	void (*not_reached)() noexcept OPENTTD_SERVICE_DEFAULT(not_reached);
};
extern "C" bool openttd_rust_heightmap_dimensions(const OpenTTDHeightmapInput *, uint32_t *, uint32_t *);
#endif /* RUST_HEIGHTMAP_FFI_H */
