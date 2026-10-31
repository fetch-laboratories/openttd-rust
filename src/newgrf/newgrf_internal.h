/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_internal.h NewGRF internal processing state. */

#ifndef NEWGRF_INTERNAL_H
#define NEWGRF_INTERNAL_H

#include "../newgrf.h"
#include "../newgrf_commons.h"
#include "../newgrf_config.h"
#include "../spriteloader/sprite_file_type.hpp"
#include "newgrf_bytereader.h"

/** Possible return values for the GrfChangeInfoHandler functions */
enum ChangeInfoResult : uint8_t {
	CIR_SUCCESS,    ///< Variable was parsed and read
	CIR_DISABLED,   ///< GRF was disabled due to error
	CIR_UNHANDLED,  ///< Variable was parsed but unread
	CIR_UNKNOWN,    ///< Variable is unknown
	CIR_INVALID_ID, ///< Attempt to modify an invalid ID
};

/** GRF feature handler */
template <GrfSpecFeature TFeature>
struct GrfChangeInfoHandler {
	static ChangeInfoResult Reserve(uint first, uint last, int prop, ByteReader &buf);
	static ChangeInfoResult Activation(uint first, uint last, int prop, ByteReader &buf);
};

/** GRF action handler */
template <uint8_t TAction>
struct GrfActionHandler {
	static void FileScan(ByteReader &buf);
	static void SafetyScan(ByteReader &buf);
	static void LabelScan(ByteReader &buf);
	static void Init(ByteReader &buf);
	static void Reserve(ByteReader &buf);
	static void Activation(ByteReader &buf);
};

static constexpr uint MAX_SPRITEGROUP = UINT8_MAX; ///< Maximum GRF-local ID for a spritegroup.

#include "../rust/newgrf_loading_ffi.h"
/** Nonowning references into the process-wide Rust execution carrier. */
struct GrfProcessingState {
	GrfLoadingStage &stage = openttd_rust_grf_execution()->stage;
	SpriteID &spriteid = openttd_rust_grf_execution()->spriteid;
	SpriteFile *&file = openttd_rust_grf_execution()->file;
	GRFFile *&grffile = openttd_rust_grf_execution()->grffile;
	GRFConfig *&grfconfig = openttd_rust_grf_execution()->grfconfig;
	uint32_t &nfo_line = openttd_rust_grf_execution()->nfo_line;
	int &skip_sprites = openttd_rust_grf_execution()->skip_sprites;
	std::array<const SpriteGroup *, MAX_SPRITEGROUP + 1> &spritegroups = openttd_rust_grf_execution()->spritegroups;
	std::string *&translated_text = openttd_rust_grf_execution()->translated_text;
	void AddSpriteSets(GrfSpecFeature feature, SpriteID first_sprite, uint first_set, uint numsets, uint numents)
	{
		assert(feature < GSF_END);
		openttd_rust_grf_add_sets(feature, first_sprite, first_set, numsets, numents);
	}
	bool HasValidSpriteSets(GrfSpecFeature feature) const
	{
		assert(feature < GSF_END);
		return openttd_rust_grf_has_sets(feature);
	}
	bool IsValidSpriteSet(GrfSpecFeature feature, uint set) const
	{
		assert(feature < GSF_END);
		return openttd_rust_grf_has_set(feature, set);
	}
	SpriteID GetSprite(GrfSpecFeature feature, uint set) const
	{
		assert(IsValidSpriteSet(feature, set));
		return openttd_rust_grf_set_sprite(feature, set);
	}
	uint GetNumEnts(GrfSpecFeature feature, uint set) const
	{
		assert(IsValidSpriteSet(feature, set));
		return openttd_rust_grf_set_size(feature, set);
	}
};

extern GrfProcessingState _cur_gps;


extern GrfMiscBits _misc_grf_features;

void SetNewGRFOverride(uint32_t source_grfid, uint32_t target_grfid);
GRFFile *GetCurrentGRFOverride();

std::span<const CargoLabel> GetCargoTranslationTable(const GRFFile &grffile);
CargoTypes TranslateRefitMask(uint32_t refit_mask);

void SkipBadgeList(ByteReader &buf);

std::vector<BadgeID> ReadBadgeList(ByteReader &buf, GrfSpecFeature feature);

void MapSpriteMappingRecolour(PalSpriteID *grf_sprite);

GRFFile *GetFileByGRFID(uint32_t grfid);
GRFError *DisableGrf(StringID message = {}, GRFConfig *config = nullptr);
bool HandleChangeInfoResult(std::string_view caller, ChangeInfoResult cir, GrfSpecFeature feature, uint8_t property);
void GRFUnsafe(ByteReader &);


#endif /* NEWGRF_INTERNAL_H */
