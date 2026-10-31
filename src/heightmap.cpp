/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file heightmap.cpp Creating of maps from heightmaps. */

#include "stdafx.h"
#include "heightmap.h"
#include "landscape.h"
#include "clear_map.h"
#include "strings_func.h"
#include "void_map.h"
#include "error.h"
#include "saveload/saveload.h"
#include "bmp.h"
#include "gfx_func.h"
#include "fios.h"
#include "fileio_func.h"

#include "table/strings.h"

#ifdef WITH_PNG
#include <png.h>
#endif /* WITH_PNG */

#include "safeguards.h"


/**
 * Converts a given greyscale map to something that fits in OTTD map system
 * and create a map of that data.
 * @param img_width  the with of the image in pixels/tiles
 * @param img_height the height of the image in pixels/tiles
 * @param map        the input map
 */

/**
 * This function takes care of the fact that land in OpenTTD can never differ
 * more than 1 in height
 */
#include "rust/heightmap_services.hpp"

/**
 * Reads the heightmap with the correct file reader.
 * @param dft Type of image file.
 * @param filename Name of the file to load.
 * @param[out] x Length of the image.
 * @param[out] y Height of the image.
 * @param[in,out] map If not \c nullptr, destination to store the loaded block of image data.
 * @return Whether loading was successful.
 */
bool GetHeightmapDimensions(DetailedFileType dft, std::string_view filename, uint *x, uint *y)
{
	RustHeightmapInput input(dft, filename);
	return openttd_rust_heightmap_dimensions(&input.view, x, y);
}

/**
 * Load a heightmap from file and change the map in its current dimensions
 *  to a landscape representing the heightmap.
 * It converts pixels to height. The brighter, the higher.
 * @param dft Type of image file.
 * @param filename of the heightmap file to be imported
 */
static_assert(HM_COUNTER_CLOCKWISE == 0 && HM_CLOCKWISE == 1);
static_assert(DFT_HEIGHTMAP_BMP == 2 && DFT_HEIGHTMAP_PNG == 3 && MAX_MAP_SIZE == 4096);
static uint8_t HeightmapSource(void *ctx) noexcept
{
	auto &input = *static_cast<RustHeightmapInput *>(ctx);
	if (input.saveload) {
		input.type = _file_to_saveload.ftype.detailed;
		input.filename = _file_to_saveload.name;
	}
	return input.type;
}
#ifdef WITH_PNG
/* The setjmp frame covers every libpng call that can fail; getters after png_read_png cannot. */
static uint8_t HeightmapPng(void *ctx, OpenTTDHeightmapPng *out) noexcept
{
	auto &input = *static_cast<RustHeightmapInput *>(ctx);
	auto &codec = input.codec.emplace();
	codec.file = FioFOpenFile(input.filename, "rb", HEIGHTMAP_DIR);
	if (!codec.file.has_value()) return OPENTTD_HEIGHTMAP_NOT_FOUND;
	codec.png = png_create_read_struct(PNG_LIBPNG_VER_STRING, nullptr, nullptr, nullptr);
	if (codec.png == nullptr) return OPENTTD_HEIGHTMAP_FAILED;
	codec.info = png_create_info_struct(codec.png);
	if (codec.info == nullptr || setjmp(png_jmpbuf(codec.png))) return OPENTTD_HEIGHTMAP_FAILED;
	png_init_io(codec.png, *codec.file);
	png_set_packing(codec.png);
	png_read_png(codec.png, codec.info, PNG_TRANSFORM_PACKING | PNG_TRANSFORM_STRIP_ALPHA | PNG_TRANSFORM_STRIP_16, nullptr);
	png_color *palette = nullptr;
	int palette_size = 0;
	bool has_palette = png_get_color_type(codec.png, codec.info) == PNG_COLOR_TYPE_PALETTE;
	if (has_palette) png_get_PLTE(codec.png, codec.info, &palette, &palette_size);
	static_assert(sizeof(png_color) == 3);
	*out = {
		.width = png_get_image_width(codec.png, codec.info), .height = png_get_image_height(codec.png, codec.info),
		.rows = png_get_rows(codec.png, codec.info), .palette = reinterpret_cast<const uint8_t *>(palette), .palette_size = palette_size,
		.channels = png_get_channels(codec.png, codec.info), .bit_depth = png_get_bit_depth(codec.png, codec.info), .has_palette = has_palette,
	};
	return OPENTTD_HEIGHTMAP_OK;
}
#else
static uint8_t HeightmapPng(void *, OpenTTDHeightmapPng *) noexcept { NOT_REACHED(); }
#endif /* WITH_PNG */
RustHeightmapCodec::~RustHeightmapCodec()
{
#ifdef WITH_PNG
	if (this->png != nullptr) png_destroy_read_struct(&this->png, &this->info, nullptr);
#endif
}
static void HeightmapBmpResult(const RustHeightmapCodec &codec, OpenTTDHeightmapBmp *out)
{
	const auto &info = codec.bmp_info;
	const auto &data = codec.bmp_data;
	*out = {.width = info.width, .height = info.height, .palette_size = info.palette_size, .palette = data.palette.data(), .palette_len = data.palette.size(), .bitmap = data.bitmap.data(), .bpp = info.bpp};
}
static uint8_t HeightmapBmpHeader(void *ctx, OpenTTDHeightmapBmp *out) noexcept
{
	auto &input = *static_cast<RustHeightmapInput *>(ctx);
	auto &codec = input.codec.emplace();
	codec.file = FioFOpenFile(input.filename, "rb", HEIGHTMAP_DIR);
	if (!codec.file.has_value()) return OPENTTD_HEIGHTMAP_NOT_FOUND;
	codec.bmp_file.emplace(input.filename, HEIGHTMAP_DIR);
	if (!BmpReadHeader(*codec.bmp_file, codec.bmp_info, codec.bmp_data)) return OPENTTD_HEIGHTMAP_FAILED;
	HeightmapBmpResult(codec, out);
	return OPENTTD_HEIGHTMAP_OK;
}
static bool HeightmapBmpBitmap(void *ctx, OpenTTDHeightmapBmp *out) noexcept
{
	auto &codec = *static_cast<RustHeightmapInput *>(ctx)->codec;
	if (!BmpReadBitmap(*codec.bmp_file, codec.bmp_info, codec.bmp_data)) return false;
	HeightmapBmpResult(codec, out);
	return true;
}
static void HeightmapFinish(void *ctx) noexcept { static_cast<RustHeightmapInput *>(ctx)->codec.reset(); }
#include "rust/presentation_services.h"
#include "rust/services_assert.h"
RustHeightmapInput::RustHeightmapInput(DetailedFileType type, std::string_view filename) : saveload(false), type(type), filename(filename), view{
	.shared = &GetRustSharedServices(),
	.context = this,
	.strings = {
		.png = STR_ERROR_PNGMAP, .bmp = STR_ERROR_BMPMAP, .not_found = STR_ERROR_PNGMAP_FILE_NOT_FOUND, .png_misc = STR_ERROR_PNGMAP_MISC,
		.png_image_type = STR_ERROR_PNGMAP_IMAGE_TYPE, .bmp_image_type = STR_ERROR_BMPMAP_IMAGE_TYPE, .too_large = STR_ERROR_HEIGHTMAP_TOO_LARGE,
	},
	.source = HeightmapSource,
	.png = HeightmapPng,
	.bmp_header = HeightmapBmpHeader,
	.bmp_bitmap = HeightmapBmpBitmap,
	.finish = HeightmapFinish,
} {}
RustHeightmapInput::RustHeightmapInput() : RustHeightmapInput(DFT_HEIGHTMAP_BMP, {}) { this->saveload = true; }


/**
 * Make an empty world where all tiles are of height 'tile_height'.
 * @param tile_height of the desired new empty world
 */
