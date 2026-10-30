/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file heightmap_services.hpp Native codec resources for one Rust heightmap read; no input policy. */
#ifndef RUST_HEIGHTMAP_SERVICES_HPP
#define RUST_HEIGHTMAP_SERVICES_HPP
#include "heightmap_ffi.h"
#include "../fios.h"
#include "../fileio_func.h"
#include "../bmp.h"
#include "../random_access_file_type.h"
#ifdef WITH_PNG
#include <png.h>
#endif
/* Codec state for one decode, allocated by png or bmp_header and released by
 * finish, so generation frames hold only the input. Rust borrows decoded rows,
 * palettes and bitmaps synchronously until finish. */
struct RustHeightmapCodec {
	std::optional<FileHandle> file;
#ifdef WITH_PNG
	png_structp png = nullptr;
	png_infop info = nullptr;
#endif
	std::optional<RandomAccessFile> bmp_file;
	BmpInfo bmp_info{};
	BmpData bmp_data{};
	~RustHeightmapCodec();
};
/* Without a fixed file, source reads _file_to_saveload, so no filename view
 * crosses generation progress. */
struct RustHeightmapInput {
	bool saveload;
	DetailedFileType type;
	std::string_view filename;
	std::unique_ptr<RustHeightmapCodec> codec;
	OpenTTDHeightmapInput view;
	RustHeightmapInput();
	RustHeightmapInput(DetailedFileType type, std::string_view filename);
	RustHeightmapInput(const RustHeightmapInput &) = delete;
	RustHeightmapInput &operator=(const RustHeightmapInput &) = delete;
};
#endif /* RUST_HEIGHTMAP_SERVICES_HPP */
