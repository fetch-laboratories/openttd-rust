/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_loading_ffi.h Canonical loading-state aliases. */
#ifndef RUST_NEWGRF_LOADING_FFI_H
#define RUST_NEWGRF_LOADING_FFI_H
struct SpriteGroup;
struct GRFFile;
struct GRFConfig;
class SpriteFile;
enum GrfLoadingStage : uint8_t;
struct OpenTTDGrfExecution {
	GrfLoadingStage stage;
	uint32_t spriteid;
	SpriteFile *file;
	GRFFile *grffile;
	GRFConfig *grfconfig;
	uint32_t nfo_line;
	int32_t skip_sprites;
	std::array<const SpriteGroup *, 256> spritegroups;
};
extern "C" {
OpenTTDGrfExecution *openttd_rust_grf_execution();
void openttd_rust_grf_clear_file();
void openttd_rust_grf_add_sets(uint8_t, uint32_t, uint32_t, uint32_t, uint32_t);
bool openttd_rust_grf_has_sets(uint8_t);
bool openttd_rust_grf_has_set(uint8_t, uint32_t);
uint32_t openttd_rust_grf_set_sprite(uint8_t, uint32_t);
uint32_t openttd_rust_grf_set_size(uint8_t, uint32_t);
}
#endif
