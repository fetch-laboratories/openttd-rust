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

#ifndef WITH_RUST
/**
 * Maximum number of pixels for one dimension of a heightmap image.
 * Do not allow images for which the longest side is twice the maximum number of
 * tiles along the longest side of the (tile) map.
 */
static const uint MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS = 2 * MAX_MAP_SIZE;

/*
 * Maximum size in pixels of the heightmap image.
 */
static const uint MAX_HEIGHTMAP_SIZE_PIXELS = 256 << 20; // ~256 million
/*
 * When loading a PNG or BMP the 24 bpp variant requires at least 4 bytes per pixel
 * of memory to load the data. Make sure the "reasonable" limit is well within the
 * maximum amount of memory allocatable on 32 bit platforms.
 */
static_assert(MAX_HEIGHTMAP_SIZE_PIXELS < UINT32_MAX / 8);

/**
 * Check whether the loaded dimension of the heightmap image are considered valid enough
 * to attempt to load the image. In other words, the width and height are not beyond the
 * #MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS limit and the total number of pixels does not
 * exceed #MAX_HEIGHTMAP_SIZE_PIXELS. A width or height less than 1 are disallowed too.
 * @param width The width of the to be loaded height map.
 * @param height The height of the to be loaded height map.
 * @return True iff the dimensions are within the limits.
 */
static inline bool IsValidHeightmapDimension(size_t width, size_t height)
{
	return (uint64_t)width * height <= MAX_HEIGHTMAP_SIZE_PIXELS &&
		width > 0 && width <= MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS &&
		height > 0 && height <= MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS;
}

/**
 * Convert RGB colours to Greyscale using 29.9% Red, 58.7% Green, 11.4% Blue
 *  (average luminosity formula, NTSC Colour Space)
 */
static inline uint8_t RGBToGreyscale(uint8_t red, uint8_t green, uint8_t blue)
{
	/* To avoid doubles and stuff, multiply it with a total of 65536 (16bits), then
	 *  divide by it to normalize the value to a byte again. */
	return ((red * 19595) + (green * 38470) + (blue * 7471)) / 65536;
}


#ifdef WITH_PNG

/**
 * The PNG Heightmap loader.
 */
static void ReadHeightmapPNGImageData(std::span<uint8_t> map, png_structp png_ptr, png_infop info_ptr)
{
	uint x, y;
	uint8_t gray_palette[256];
	png_bytep *row_pointers = nullptr;
	bool has_palette = png_get_color_type(png_ptr, info_ptr) == PNG_COLOR_TYPE_PALETTE;
	uint channels = png_get_channels(png_ptr, info_ptr);

	/* Get palette and convert it to greyscale */
	if (has_palette) {
		int i;
		int palette_size;
		png_color *palette;
		bool all_gray = true;

		png_get_PLTE(png_ptr, info_ptr, &palette, &palette_size);
		for (i = 0; i < palette_size && (palette_size != 16 || all_gray); i++) {
			all_gray &= palette[i].red == palette[i].green && palette[i].red == palette[i].blue;
			gray_palette[i] = RGBToGreyscale(palette[i].red, palette[i].green, palette[i].blue);
		}

		/**
		 * For a non-gray palette of size 16 we assume that
		 * the order of the palette determines the height;
		 * the first entry is the sea (level 0), the second one
		 * level 1, etc.
		 */
		if (palette_size == 16 && !all_gray) {
			for (i = 0; i < palette_size; i++) {
				gray_palette[i] = 256 * i / palette_size;
			}
		}
	}

	row_pointers = png_get_rows(png_ptr, info_ptr);

	/* Read the raw image data and convert in 8-bit greyscale */
	for (x = 0; x < png_get_image_width(png_ptr, info_ptr); x++) {
		for (y = 0; y < png_get_image_height(png_ptr, info_ptr); y++) {
			uint8_t *pixel = &map[y * png_get_image_width(png_ptr, info_ptr) + x];
			uint x_offset = x * channels;

			if (has_palette) {
				*pixel = gray_palette[row_pointers[y][x_offset]];
			} else if (channels == 3) {
				*pixel = RGBToGreyscale(row_pointers[y][x_offset + 0],
						row_pointers[y][x_offset + 1], row_pointers[y][x_offset + 2]);
			} else {
				*pixel = row_pointers[y][x_offset];
			}
		}
	}
}

/**
 * Reads the heightmap and/or size of the heightmap from a PNG file.
 * If map == nullptr only the size of the PNG is read, otherwise a map
 * with greyscale pixels is allocated and assigned to *map.
 */
static bool ReadHeightmapPNG(std::string_view filename, uint *x, uint *y, std::vector<uint8_t> *map)
{
	png_structp png_ptr = nullptr;
	png_infop info_ptr  = nullptr;

	auto fp = FioFOpenFile(filename, "rb", HEIGHTMAP_DIR);
	if (!fp.has_value()) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_PNGMAP), GetEncodedString(STR_ERROR_PNGMAP_FILE_NOT_FOUND), WL_ERROR);
		return false;
	}

	png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, nullptr, nullptr, nullptr);
	if (png_ptr == nullptr) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_PNGMAP), GetEncodedString(STR_ERROR_PNGMAP_MISC), WL_ERROR);
		return false;
	}

	info_ptr = png_create_info_struct(png_ptr);
	if (info_ptr == nullptr || setjmp(png_jmpbuf(png_ptr))) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_PNGMAP), GetEncodedString(STR_ERROR_PNGMAP_MISC), WL_ERROR);
		png_destroy_read_struct(&png_ptr, &info_ptr, nullptr);
		return false;
	}

	png_init_io(png_ptr, *fp);

	/* Allocate memory and read image, without alpha or 16-bit samples
	 * (result is either 8-bit indexed/greyscale or 24-bit RGB) */
	png_set_packing(png_ptr);
	png_read_png(png_ptr, info_ptr, PNG_TRANSFORM_PACKING | PNG_TRANSFORM_STRIP_ALPHA | PNG_TRANSFORM_STRIP_16, nullptr);

	/* Maps of wrong colour-depth are not used.
	 * (this should have been taken care of by stripping alpha and 16-bit samples on load) */
	if ((png_get_channels(png_ptr, info_ptr) != 1) && (png_get_channels(png_ptr, info_ptr) != 3) && (png_get_bit_depth(png_ptr, info_ptr) != 8)) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_PNGMAP), GetEncodedString(STR_ERROR_PNGMAP_IMAGE_TYPE), WL_ERROR);
		png_destroy_read_struct(&png_ptr, &info_ptr, nullptr);
		return false;
	}

	uint width = png_get_image_width(png_ptr, info_ptr);
	uint height = png_get_image_height(png_ptr, info_ptr);

	if (!IsValidHeightmapDimension(width, height)) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_PNGMAP), GetEncodedString(STR_ERROR_HEIGHTMAP_TOO_LARGE), WL_ERROR);
		png_destroy_read_struct(&png_ptr, &info_ptr, nullptr);
		return false;
	}

	if (map != nullptr) {
		map->resize(static_cast<size_t>(width) * height);
		ReadHeightmapPNGImageData(*map, png_ptr, info_ptr);
	}

	*x = width;
	*y = height;

	png_destroy_read_struct(&png_ptr, &info_ptr, nullptr);
	return true;
}

#endif /* WITH_PNG */


/**
 * The BMP Heightmap loader.
 */
static void ReadHeightmapBMPImageData(std::span<uint8_t> map, const BmpInfo &info, const BmpData &data)
{
	uint8_t gray_palette[256];

	if (!data.palette.empty()) {
		bool all_gray = true;

		if (info.palette_size != 2) {
			for (uint i = 0; i < info.palette_size && (info.palette_size != 16 || all_gray); i++) {
				all_gray &= data.palette[i].r == data.palette[i].g && data.palette[i].r == data.palette[i].b;
				gray_palette[i] = RGBToGreyscale(data.palette[i].r, data.palette[i].g, data.palette[i].b);
			}

			/**
			 * For a non-gray palette of size 16 we assume that
			 * the order of the palette determines the height;
			 * the first entry is the sea (level 0), the second one
			 * level 1, etc.
			 */
			if (info.palette_size == 16 && !all_gray) {
				for (uint i = 0; i < info.palette_size; i++) {
					gray_palette[i] = 256 * i / info.palette_size;
				}
			}
		} else {
			/**
			 * For a palette of size 2 we assume that the order of the palette determines the height;
			 * the first entry is the sea (level 0), the second one is the land (level 1)
			 */
			gray_palette[0] = 0;
			gray_palette[1] = 16;
		}
	}

	/* Read the raw image data and convert in 8-bit greyscale */
	for (uint y = 0; y < info.height; y++) {
		uint8_t *pixel = &map[y * static_cast<size_t>(info.width)];
		const uint8_t *bitmap = &data.bitmap[y * static_cast<size_t>(info.width) * (info.bpp == 24 ? 3 : 1)];

		for (uint x = 0; x < info.width; x++) {
			if (info.bpp != 24) {
				*pixel++ = gray_palette[*bitmap++];
			} else {
				*pixel++ = RGBToGreyscale(*bitmap, *(bitmap + 1), *(bitmap + 2));
				bitmap += 3;
			}
		}
	}
}

/**
 * Reads the heightmap and/or size of the heightmap from a BMP file.
 * If map == nullptr only the size of the BMP is read, otherwise a map
 * with greyscale pixels is allocated and assigned to *map.
 */
static bool ReadHeightmapBMP(std::string_view filename, uint *x, uint *y, std::vector<uint8_t> *map)
{
	auto f = FioFOpenFile(filename, "rb", HEIGHTMAP_DIR);
	if (!f.has_value()) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_BMPMAP), GetEncodedString(STR_ERROR_PNGMAP_FILE_NOT_FOUND), WL_ERROR);
		return false;
	}

	RandomAccessFile file(filename, HEIGHTMAP_DIR);
	BmpInfo info{};
	BmpData data{};

	if (!BmpReadHeader(file, info, data)) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_BMPMAP), GetEncodedString(STR_ERROR_BMPMAP_IMAGE_TYPE), WL_ERROR);
		return false;
	}

	if (!IsValidHeightmapDimension(info.width, info.height)) {
		ShowErrorMessage(GetEncodedString(STR_ERROR_BMPMAP), GetEncodedString(STR_ERROR_HEIGHTMAP_TOO_LARGE), WL_ERROR);
		return false;
	}

	if (map != nullptr) {
		if (!BmpReadBitmap(file, info, data)) {
			ShowErrorMessage(GetEncodedString(STR_ERROR_BMPMAP), GetEncodedString(STR_ERROR_BMPMAP_IMAGE_TYPE), WL_ERROR);
			return false;
		}

		map->resize(static_cast<size_t>(info.width) * info.height);
		ReadHeightmapBMPImageData(*map, info, data);
	}

	*x = info.width;
	*y = info.height;

	return true;
}
#endif /* WITH_RUST */

/**
 * Converts a given greyscale map to something that fits in OTTD map system
 * and create a map of that data.
 * @param img_width  the with of the image in pixels/tiles
 * @param img_height the height of the image in pixels/tiles
 * @param map        the input map
 */
#ifndef WITH_RUST
static void GreyscaleToMapHeights(uint img_width, uint img_height, std::span<const uint8_t> map)
{
	/* Defines the detail of the aspect ratio (to avoid doubles) */
	const uint num_div = 16384;
	/* Ensure multiplication with num_div does not cause overflows. */
	static_assert(num_div <= std::numeric_limits<uint>::max() / MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS);

	uint width, height;
	uint row, col;
	uint row_pad = 0, col_pad = 0;
	uint img_scale;
	uint img_row, img_col;
	TileIndex tile;

	/* Get map size and calculate scale and padding values */
	switch (_settings_game.game_creation.heightmap_rotation) {
		default: NOT_REACHED();
		case HM_COUNTER_CLOCKWISE:
			width   = Map::SizeX();
			height  = Map::SizeY();
			break;
		case HM_CLOCKWISE:
			width   = Map::SizeY();
			height  = Map::SizeX();
			break;
	}

	if ((img_width * num_div) / img_height > ((width * num_div) / height)) {
		/* Image is wider than map - center vertically */
		img_scale = (width * num_div) / img_width;
		row_pad = (1 + height - ((img_height * img_scale) / num_div)) / 2;
	} else {
		/* Image is taller than map - center horizontally */
		img_scale = (height * num_div) / img_height;
		col_pad = (1 + width - ((img_width * img_scale) / num_div)) / 2;
	}

	if (_settings_game.construction.freeform_edges) {
		for (uint x = 0; x < Map::SizeX(); x++) MakeVoid(TileXY(x, 0));
		for (uint y = 0; y < Map::SizeY(); y++) MakeVoid(TileXY(0, y));
	}

	/* Form the landscape */
	for (row = 0; row < height; row++) {
		for (col = 0; col < width; col++) {
			switch (_settings_game.game_creation.heightmap_rotation) {
				default: NOT_REACHED();
				case HM_COUNTER_CLOCKWISE: tile = TileXY(col, row); break;
				case HM_CLOCKWISE:         tile = TileXY(row, col); break;
			}

			/* Check if current tile is within the 1-pixel map edge or padding regions */
			if ((!_settings_game.construction.freeform_edges && DistanceFromEdge(tile) <= 1) ||
					(row < row_pad) || (row >= (height - row_pad - (_settings_game.construction.freeform_edges ? 0 : 1))) ||
					(col < col_pad) || (col >= (width  - col_pad - (_settings_game.construction.freeform_edges ? 0 : 1)))) {
				SetTileHeight(tile, 0);
			} else {
				/* Use nearest neighbour resizing to scale map data.
				 *  We rotate the map 45 degrees (counter)clockwise */
				img_row = (((row - row_pad) * num_div) / img_scale);
				switch (_settings_game.game_creation.heightmap_rotation) {
					default: NOT_REACHED();
					case HM_COUNTER_CLOCKWISE:
						img_col = (((width - 1 - col - col_pad) * num_div) / img_scale);
						break;
					case HM_CLOCKWISE:
						img_col = (((col - col_pad) * num_div) / img_scale);
						break;
				}

				assert(img_row < img_height);
				assert(img_col < img_width);

				uint heightmap_height = map[img_row * img_width + img_col];

				if (heightmap_height > 0) {
					/* 0 is sea level.
					 * Other grey scales are scaled evenly to the available height levels > 0.
					 * (The coastline is independent from the number of height levels) */
					heightmap_height = 1 + (heightmap_height - 1) * _settings_game.game_creation.heightmap_height / 255;
				}

				SetTileHeight(tile, heightmap_height);
			}
			/* Only clear the tiles within the map area. */
			if (IsInnerTile(tile)) {
				MakeClear(tile, CLEAR_GRASS, 3);
			}
		}
	}
}

#endif

/**
 * This function takes care of the fact that land in OpenTTD can never differ
 * more than 1 in height
 */
#ifdef WITH_RUST
#include "rust/heightmap_services.hpp"
#include "rust/landscape_generation_ffi.h"
#endif
void FixSlopes()
{
#ifdef WITH_RUST
	openttd_rust_fix_slopes(&GetRustLandscapeGenerationServices(), _settings_game.construction.map_height_limit);
#else
	uint width, height;
	int row, col;
	uint8_t current_height;
	uint8_t max_height = _settings_game.construction.map_height_limit;

	/* Adjust height difference to maximum one horizontal/vertical change. */
	width   = Map::SizeX();
	height  = Map::SizeY();

	/* Top and left edge */
	for (row = 0; (uint)row < height; row++) {
		for (col = 0; (uint)col < width; col++) {
			current_height = MAX_TILE_HEIGHT;
			if (col != 0) {
				/* Find lowest tile; either the top or left one */
				current_height = TileHeight(TileXY(col - 1, row)); // top edge
			}
			if (row != 0) {
				if (TileHeight(TileXY(col, row - 1)) < current_height) {
					current_height = TileHeight(TileXY(col, row - 1)); // left edge
				}
			}

			/* Does the height differ more than one? */
			TileIndex tile = TileXY(col, row);
			if (TileHeight(tile) >= (uint)current_height + 2) {
				/* Then change the height to be no more than one */
				SetTileHeight(tile, current_height + 1);
				/* Height was changed so now there's a chance, more likely at higher altitude, of the
				 * tile turning into rock. */
				if (IsInnerTile(tile) && RandomRange(max_height) <= current_height) {
					MakeClear(tile, CLEAR_ROCKS, 3);
				}
			}
		}
	}

	/* Bottom and right edge */
	for (row = height - 1; row >= 0; row--) {
		for (col = width - 1; col >= 0; col--) {
			current_height = MAX_TILE_HEIGHT;
			if ((uint)col != width - 1) {
				/* Find lowest tile; either the bottom and right one */
				current_height = TileHeight(TileXY(col + 1, row)); // bottom edge
			}

			if ((uint)row != height - 1) {
				if (TileHeight(TileXY(col, row + 1)) < current_height) {
					current_height = TileHeight(TileXY(col, row + 1)); // right edge
				}
			}

			/* Does the height differ more than one? */
			TileIndex tile = TileXY(col, row);
			if (TileHeight(tile) >= (uint)current_height + 2) {
				/* Then change the height to be no more than one */
				SetTileHeight(tile, current_height + 1);
				/* Height was changed so now there's a chance, more likely at higher altitude, of the
				 * tile turning into rock. */
				if (IsInnerTile(tile) && RandomRange(max_height) <= current_height) {
					MakeClear(tile, CLEAR_ROCKS, 3);
				}
			}
		}
	}
#endif
}

/**
 * Reads the heightmap with the correct file reader.
 * @param dft Type of image file.
 * @param filename Name of the file to load.
 * @param[out] x Length of the image.
 * @param[out] y Height of the image.
 * @param[in,out] map If not \c nullptr, destination to store the loaded block of image data.
 * @return Whether loading was successful.
 */
#ifndef WITH_RUST
static bool ReadHeightMap(DetailedFileType dft, std::string_view filename, uint *x, uint *y, std::vector<uint8_t> *map)
{
	switch (dft) {
		default:
			NOT_REACHED();

#ifdef WITH_PNG
		case DFT_HEIGHTMAP_PNG:
			return ReadHeightmapPNG(filename, x, y, map);
#endif /* WITH_PNG */

		case DFT_HEIGHTMAP_BMP:
			return ReadHeightmapBMP(filename, x, y, map);
	}
}

/**
 * Get the dimensions of a heightmap.
 * @param dft Type of image file.
 * @param filename to query
 * @param x dimension x
 * @param y dimension y
 * @return Returns false if loading of the image failed.
 */
#endif /* WITH_RUST */
bool GetHeightmapDimensions(DetailedFileType dft, std::string_view filename, uint *x, uint *y)
{
#ifdef WITH_RUST
	RustHeightmapInput input(dft, filename);
	return openttd_rust_heightmap_dimensions(&input.view, x, y);
#else
	return ReadHeightMap(dft, filename, x, y, nullptr);
#endif
}

/**
 * Load a heightmap from file and change the map in its current dimensions
 *  to a landscape representing the heightmap.
 * It converts pixels to height. The brighter, the higher.
 * @param dft Type of image file.
 * @param filename of the heightmap file to be imported
 */
#ifdef WITH_RUST
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
	auto &codec = *(input.codec = std::make_unique<RustHeightmapCodec>());
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
	auto &codec = *(input.codec = std::make_unique<RustHeightmapCodec>());
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
	.error = RustShowErrorMessage,
	.not_reached = RustNotReached,
} {}
RustHeightmapInput::RustHeightmapInput() : RustHeightmapInput(DFT_HEIGHTMAP_BMP, {}) { this->saveload = true; }
#endif

#ifndef WITH_RUST
bool LoadHeightmap(DetailedFileType dft, std::string_view filename)
{
	uint x, y;
	std::vector<uint8_t> map;

	if (!ReadHeightMap(dft, filename, &x, &y, &map)) {
		return false;
	}

	GreyscaleToMapHeights(x, y, map);

	FixSlopes();

	/* If all map borders are water, we will draw infinite water. */
	_settings_game.construction.freeform_edges = !IsMapSurroundedByWater();

	MarkWholeScreenDirty();

	return true;
}
#endif /* WITH_RUST */

/**
 * Make an empty world where all tiles are of height 'tile_height'.
 * @param tile_height of the desired new empty world
 */
#ifndef WITH_RUST
void FlatEmptyWorld(uint8_t tile_height)
{
	int edge_distance = _settings_game.construction.freeform_edges ? 0 : 2;
	for (uint row = edge_distance; row < Map::SizeY() - edge_distance; row++) {
		for (uint col = edge_distance; col < Map::SizeX() - edge_distance; col++) {
			SetTileHeight(TileXY(col, row), tile_height);
		}
	}

	FixSlopes();
	MarkWholeScreenDirty();
}
#endif /* WITH_RUST */
