/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_act2_ffi.h Typed construction storage and native pool services. */
#ifndef RUST_NEWGRF_ACT2_FFI_H
#define RUST_NEWGRF_ACT2_FFI_H
#include "newgrf_spritegroup_ffi.h"
#include "newgrf_bytereader_ffi.h"
#include "../newgrf_commons.h"
#include "../newgrf_spritegroup.h"
struct OpenTTDGrfLoadingServices;
using OpenTTDLayoutRegisters = TileLayoutRegisters;
struct OpenTTDPalSpriteStorage {
	SpriteID *sprite;
	PaletteID *pal;
};
struct OpenTTDAdjustStorage {
	uint8_t *operation;
	uint8_t *kind;
	uint8_t *variable;
	uint8_t *parameter;
	uint8_t *shift_num;
	uint32_t *and_mask;
	uint32_t *add_val;
	uint32_t *divmod_val;
	const SpriteGroup * *subroutine;
};
struct OpenTTDRandomStorage {
	uint8_t *scope;
	uint8_t *compare;
	uint8_t *triggers;
	uint8_t *count;
	uint8_t *lowest_bit;
};
struct OpenTTDProductionStorage {
	uint8_t *version;
	uint8_t *num_input;
	int16_t *subtract_input;
	uint8_t *cargo_input;
	uint8_t *num_output;
	uint16_t *add_output;
	uint8_t *cargo_output;
	uint8_t *again;
};
struct OpenTTDLayoutStorage {
	PalSpriteID *ground;
	uint32_t *consistent_max_offset;
};
struct OpenTTDSequenceStorage {
	PalSpriteID *image;
	int8_t *x;
	int8_t *y;
	int8_t *z;
	uint8_t *extent_x;
	uint8_t *extent_y;
	uint8_t *extent_z;
};
struct OpenTTDAction2Services {
	OpenTTDPalSpriteStorage (*pal_sprite_storage)(PalSpriteID *) noexcept OPENTTD_SERVICE_DEFAULT(pal_sprite_storage);
	StringID invalid_layout, invalid_production;
	const SpriteGroup * (*callback_new)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(callback_new);
	uint32_t (*group_index)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(group_index);
	const SpriteGroup * (*group_get)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(group_get);
	const SpriteGroup * (*result_new)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(result_new);
	void (*assert_loaded)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(assert_loaded);
	const SpriteGroup * (*deterministic_new)(uint32_t, VarSpriteGroupScope, DeterministicSpriteGroupSize) noexcept OPENTTD_SERVICE_DEFAULT(deterministic_new);
	const SpriteGroup * (*random_new)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random_new);
	const SpriteGroup * (*real_new)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(real_new);
	const SpriteGroup * (*layout_new)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_new);
	const SpriteGroup * (*production_new)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(production_new);
	OpenTTDAdjustStorage (*adjust_begin)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(adjust_begin);
	void (*deterministic_default)(const SpriteGroup *, OpenTTDGroupResult) noexcept OPENTTD_SERVICE_DEFAULT(deterministic_default);
	void (*deterministic_error)(const SpriteGroup *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(deterministic_error);
	void (*range_append)(const SpriteGroup *, OpenTTDGroupRange) noexcept OPENTTD_SERVICE_DEFAULT(range_append);
	OpenTTDRandomStorage (*random_storage)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(random_storage);
	void (*random_reserve)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(random_reserve);
	void (*random_append)(const SpriteGroup *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(random_append);
	void (*loaded_reserve)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(loaded_reserve);
	void (*loaded_append)(const SpriteGroup *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(loaded_append);
	void (*loading_reserve)(const SpriteGroup *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(loading_reserve);
	void (*loading_append)(const SpriteGroup *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(loading_append);
	OpenTTDProductionStorage (*production_storage)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(production_storage);
	NewGRFSpriteLayout * (*group_layout)(const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(group_layout);
	void (*layout_allocate)(NewGRFSpriteLayout *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_allocate);
	OpenTTDLayoutStorage (*layout_storage)(NewGRFSpriteLayout *) noexcept OPENTTD_SERVICE_DEFAULT(layout_storage);
	DrawTileSeqStruct * (*layout_sequence)(NewGRFSpriteLayout *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(layout_sequence);
	OpenTTDSequenceStorage (*sequence_storage)(DrawTileSeqStruct *) noexcept OPENTTD_SERVICE_DEFAULT(sequence_storage);
	bool (*is_parent)(const DrawTileSeqStruct *) noexcept OPENTTD_SERVICE_DEFAULT(is_parent);
	bool (*has_registers)(const NewGRFSpriteLayout *) noexcept OPENTTD_SERVICE_DEFAULT(has_registers);
	void (*allocate_registers)(NewGRFSpriteLayout *) noexcept OPENTTD_SERVICE_DEFAULT(allocate_registers);
	TileLayoutRegisters * (*registers)(NewGRFSpriteLayout *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(registers);
	void (*assert_offsets)(bool, bool, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(assert_offsets);
	void (*error_data)(GRFError *, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(error_data);
};
const OpenTTDAction2Services &GetRustAction2Services() noexcept;
extern "C" {
void openttd_rust_action2_recolour(const OpenTTDAction2Services *, PalSpriteID *);
void openttd_rust_action2_reset(bool);
}
#endif
