/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file heightmap_services.hpp Native decoding scratch; no terrain policy. */
#ifndef RUST_HEIGHTMAP_SERVICES_HPP
#define RUST_HEIGHTMAP_SERVICES_HPP
#include "landscape_generation_ffi.h"
#include "../fios.h"
/* The decoded byte buffer belongs only to the native decoder. Rust borrows it
 * synchronously after read, before destruction; map services cannot resize it. */
struct RustHeightmapDecoder {
	DetailedFileType type;
	std::string_view filename;
	std::vector<uint8_t> image;
	OpenTTDHeightmapDecoder view;
	RustHeightmapDecoder(DetailedFileType type, std::string_view filename);
};
#endif /* RUST_HEIGHTMAP_SERVICES_HPP */
