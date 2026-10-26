/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_act2_services.inc Native canonical Action2 pool and payload storage. */
#include "newgrf_act2_ffi.h"
#include "newgrf_bytereader_services.h"
static_assert(TLF_DODRAW == 1 && TLF_SPRITE == 2 && TLF_PALETTE == 4 && TLF_CUSTOM_PALETTE == 8);
static_assert(TLF_BB_XY_OFFSET == 16 && TLF_BB_Z_OFFSET == 32 && TLF_CHILD_X_OFFSET == 16 && TLF_CHILD_Y_OFFSET == 32);
static_assert(TLF_SPRITE_VAR10 == 64 && TLF_PALETTE_VAR10 == 128 && TLF_KNOWN_FLAGS == 255 && TLR_MAX_VAR10 == 7);
static_assert(SPR_IMG_QUERY == 723 && PAL_NONE == 0 && SPRITE_WIDTH == 24);
static_assert(SPRITE_MODIFIER_CUSTOM_SPRITE == 29 && SPRITE_MODIFIER_OPAQUE == 28 && PALETTE_MODIFIER_TRANSPARENT == 31 && PALETTE_MODIFIER_COLOUR == 30);
static_assert(INDUSTRY_NUM_INPUTS == 16 && INDUSTRY_NUM_OUTPUTS == 16 && INDUSTRY_ORIGINAL_NUM_INPUTS == 3 && INDUSTRY_ORIGINAL_NUM_OUTPUTS == 2);

const OpenTTDAction2Services &GetRustAction2Services() noexcept
{
	static const OpenTTDAction2Services services{
		.invalid_layout = STR_NEWGRF_ERROR_INVALID_SPRITE_LAYOUT,
		.invalid_production = STR_NEWGRF_ERROR_INDPROD_CALLBACK,
		.callback_new = [](uint16_t value) noexcept -> const SpriteGroup * { assert(CallbackResultSpriteGroup::CanAllocateItem()); return new CallbackResultSpriteGroup(value); },
		.group_index = [](const SpriteGroup *group) noexcept { return group->index.base(); },
		.group_get = [](uint32_t index) noexcept -> const SpriteGroup * { return SpriteGroup::Get(SpriteGroupID(index)); },
		.result_new = [](SpriteID sprite, uint8_t count) noexcept -> const SpriteGroup * { assert(ResultSpriteGroup::CanAllocateItem()); return new ResultSpriteGroup(sprite, count); },
		.assert_loaded = [](uint32_t end, SpriteID current) noexcept { assert(end <= current); },
		.deterministic_new = [](uint32_t line, VarSpriteGroupScope scope, DeterministicSpriteGroupSize size) noexcept -> const SpriteGroup * { assert(DeterministicSpriteGroup::CanAllocateItem()); auto *group = new DeterministicSpriteGroup(); group->nfo_line = line; group->var_scope = scope; group->size = size; return group; },
		.random_new = [](uint32_t line) noexcept -> const SpriteGroup * { assert(RandomizedSpriteGroup::CanAllocateItem()); auto *group = new RandomizedSpriteGroup(); group->nfo_line = line; return group; },
		.real_new = [](uint32_t line) noexcept -> const SpriteGroup * { assert(RealSpriteGroup::CanAllocateItem()); auto *group = new RealSpriteGroup(); group->nfo_line = line; return group; },
		.layout_new = [](uint32_t line) noexcept -> const SpriteGroup * { assert(TileLayoutSpriteGroup::CanAllocateItem()); auto *group = new TileLayoutSpriteGroup(); group->nfo_line = line; return group; },
		.production_new = [](uint32_t line) noexcept -> const SpriteGroup * { assert(IndustryProductionSpriteGroup::CanAllocateItem()); auto *group = new IndustryProductionSpriteGroup(); group->nfo_line = line; return group; },
		.adjust_begin = [](const SpriteGroup *handle) noexcept { auto &a = const_cast<DeterministicSpriteGroup *>(static_cast<const DeterministicSpriteGroup *>(handle))->adjusts.emplace_back(); return OpenTTDAdjustStorage{reinterpret_cast<uint8_t *>(&a.operation), reinterpret_cast<uint8_t *>(&a.type), &a.variable, &a.parameter, &a.shift_num, &a.and_mask, &a.add_val, &a.divmod_val, &a.subroutine}; },
		.deterministic_default = [](const SpriteGroup *handle, OpenTTDGroupResult value) noexcept { auto *g = const_cast<DeterministicSpriteGroup *>(static_cast<const DeterministicSpriteGroup *>(handle)); g->default_result = {value.calculated_result, value.group}; },
		.deterministic_error = [](const SpriteGroup *handle, const SpriteGroup *error) noexcept { const_cast<DeterministicSpriteGroup *>(static_cast<const DeterministicSpriteGroup *>(handle))->error_group = error; },
		.range_append = [](const SpriteGroup *handle, OpenTTDGroupRange range) noexcept { auto &r = const_cast<DeterministicSpriteGroup *>(static_cast<const DeterministicSpriteGroup *>(handle))->ranges.emplace_back(); r.result = {range.result.calculated_result, range.result.group}; r.low = range.low; r.high = range.high; },
		.random_storage = [](const SpriteGroup *handle) noexcept { auto *g = const_cast<RandomizedSpriteGroup *>(static_cast<const RandomizedSpriteGroup *>(handle)); return OpenTTDRandomStorage{reinterpret_cast<uint8_t *>(&g->var_scope), reinterpret_cast<uint8_t *>(&g->cmp_mode), &g->triggers, &g->count, &g->lowest_randbit}; },
		.random_reserve = [](const SpriteGroup *handle, size_t count) noexcept { const_cast<RandomizedSpriteGroup *>(static_cast<const RandomizedSpriteGroup *>(handle))->groups.reserve(count); },
		.random_append = [](const SpriteGroup *handle, const SpriteGroup *result) noexcept { const_cast<RandomizedSpriteGroup *>(static_cast<const RandomizedSpriteGroup *>(handle))->groups.push_back(result); },
		.loaded_reserve = [](const SpriteGroup *handle, size_t count) noexcept { const_cast<RealSpriteGroup *>(static_cast<const RealSpriteGroup *>(handle))->loaded.reserve(count); },
		.loaded_append = [](const SpriteGroup *handle, const SpriteGroup *result) noexcept { const_cast<RealSpriteGroup *>(static_cast<const RealSpriteGroup *>(handle))->loaded.push_back(result); },
		.loading_reserve = [](const SpriteGroup *handle, size_t count) noexcept { const_cast<RealSpriteGroup *>(static_cast<const RealSpriteGroup *>(handle))->loading.reserve(count); },
		.loading_append = [](const SpriteGroup *handle, const SpriteGroup *result) noexcept { const_cast<RealSpriteGroup *>(static_cast<const RealSpriteGroup *>(handle))->loading.push_back(result); },
		.production_storage = [](const SpriteGroup *handle) noexcept { auto *g = const_cast<IndustryProductionSpriteGroup *>(static_cast<const IndustryProductionSpriteGroup *>(handle)); return OpenTTDProductionStorage{&g->version, &g->num_input, g->subtract_input.data(), g->cargo_input.data(), &g->num_output, g->add_output.data(), g->cargo_output.data(), &g->again}; },
		.group_layout = [](const SpriteGroup *handle) noexcept { return &const_cast<TileLayoutSpriteGroup *>(static_cast<const TileLayoutSpriteGroup *>(handle))->dts; },
		.layout_allocate = [](NewGRFSpriteLayout *layout, uint32_t count) noexcept { layout->Allocate(count); },
		.layout_storage = [](NewGRFSpriteLayout *layout) noexcept { return OpenTTDLayoutStorage{&layout->ground, &layout->consistent_max_offset}; },
		.layout_sequence = [](NewGRFSpriteLayout *layout, uint32_t index) noexcept { return &layout->seq[index]; },
		.sequence_storage = [](DrawTileSeqStruct *seq) noexcept { return OpenTTDSequenceStorage{&seq->image, &seq->origin.x, &seq->origin.y, &seq->origin.z, &seq->extent.x, &seq->extent.y, &seq->extent.z}; },
		.is_parent = [](const DrawTileSeqStruct *seq) noexcept { return seq->IsParentSprite(); },
		.has_registers = [](const NewGRFSpriteLayout *layout) noexcept { return !layout->registers.empty(); },
		.allocate_registers = [](NewGRFSpriteLayout *layout) noexcept { layout->AllocateRegisters(); },
		.registers = [](NewGRFSpriteLayout *layout, uint32_t index) noexcept { return &layout->registers[index]; },
		.assert_offsets = [](bool use_sets, bool consistent, uint32_t maximum) noexcept { assert(use_sets || (consistent && (maximum == 0 || maximum == UINT16_MAX))); },
		.error_data = [](GRFError *error, const uint8_t *data, size_t size) noexcept { error->data.assign(reinterpret_cast<const char *>(data), size); },
		.missing_group = [](uint8_t setid, uint8_t kind, uint16_t group) noexcept { GrfMsg(1, "GetGroupFromGroupID(0x{:02X}:0x{:02X}): Groupid 0x{:04X} does not exist, leaving empty", setid, kind, group); },
		.missing_set = [](uint8_t setid, uint8_t kind, uint16_t set) noexcept { GrfMsg(1, "CreateGroupFromGroupID(0x{:02X}:0x{:02X}): Sprite set {} invalid", setid, kind, set); },
		.unsupported_feature = [](uint8_t feature) noexcept { GrfMsg(1, "NewSpriteGroup: Unsupported feature 0x{:02X}, skipping", feature); },
		.reserved_type = [](uint8_t kind) noexcept { GrfMsg(0, "NewSpriteGroup: Reserved group type 0x{:02X}, skipping", kind); },
		.random_count = []() noexcept { GrfMsg(1, "NewSpriteGroup: Random Action 2 nrand should be power of 2"); },
		.no_sets = []() noexcept { GrfMsg(0, "NewSpriteGroup: No sprite set to work on! Skipping"); },
		.real_counts = [](uint8_t setid, uint8_t loaded, uint8_t loading) noexcept { GrfMsg(6, "NewSpriteGroup: New SpriteGroup 0x{:02X}, {} loaded, {} loading", setid, loaded, loading); },
		.real_empty = []() noexcept { GrfMsg(1, "NewSpriteGroup: no result, skipping invalid RealSpriteGroup"); },
		.real_one = [](uint16_t sprite) noexcept { GrfMsg(8, "NewSpriteGroup: one result, skipping RealSpriteGroup = subset {}", sprite); },
		.real_same = [](uint16_t sprite) noexcept { GrfMsg(8, "NewSpriteGroup: same result, skipping RealSpriteGroup = subset {}", sprite); },
		.loaded_entry = [](uint32_t index, uint16_t sprite) noexcept { GrfMsg(8, "NewSpriteGroup: + rg->loaded[{}]  = subset {}", index, sprite); },
		.loading_entry = [](uint32_t index, uint16_t sprite) noexcept { GrfMsg(8, "NewSpriteGroup: + rg->loading[{}] = subset {}", index, sprite); },
		.production_version = [](uint8_t version) noexcept { GrfMsg(1, "NewSpriteGroup: Unsupported industry production version {}, skipping", version); },
		.sprite_set = [](uint32_t index) noexcept { GrfMsg(1, "ReadSpriteLayoutSprite: Spritelayout uses undefined custom spriteset {}", index); },
		.palette_set = [](uint32_t index) noexcept { GrfMsg(1, "ReadSpriteLayoutSprite: Spritelayout uses undefined custom spriteset {} for 'palette'", index); },
		.sprite_var10 = []() noexcept { GrfMsg(1, "ReadSpriteLayoutSprite: Spritelayout specifies var10 value for non-action-1 sprite"); },
		.palette_var10 = []() noexcept { GrfMsg(1, "ReadSpriteLayoutRegisters: Spritelayout specifies var10 value for non-action-1 palette"); },
		.register_var10 = [](uint8_t value, uint8_t maximum) noexcept { GrfMsg(1, "ReadSpriteLayoutRegisters: Spritelayout specifies var10 ({}) exceeding the maximal allowed value {}", value, maximum); },
		.ground_flags = [](uint8_t flags) noexcept { GrfMsg(1, "ReadSpriteLayout: Spritelayout uses invalid flag 0x{:X} for ground sprite", flags); },
		.unknown_flags = [](uint8_t flags) noexcept { GrfMsg(1, "ReadSpriteLayout: Spritelayout uses unknown flag 0x{:X}", flags); },
	};
	return services;
}
