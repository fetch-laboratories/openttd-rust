/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file tgp_ffi.h Named settings for the Rust-owned TerraGenesis generator. */
#ifndef TGP_FFI_H
#define TGP_FFI_H
#include <cstdint>
struct OpenTTDTGPSettings {
	uint32_t log_x;
	uint32_t log_y;
	uint32_t terrain;
	uint32_t custom_height;
	uint32_t height_limit;
	uint32_t smoothness;
	uint32_t landscape;
	uint32_t variety;
	uint32_t sea;
	uint32_t custom_sea;
	uint32_t freeform;
	uint32_t borders;
	uint32_t seed;
	uint32_t creation_log_x;
	uint32_t creation_log_y;
};
OpenTTDTGPSettings RustTGPSettings() noexcept;
#endif /* TGP_FFI_H */
