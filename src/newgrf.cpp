/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf.cpp Base of all NewGRF support. */

#include "stdafx.h"
#include "core/backup_type.hpp"
#include "core/container_func.hpp"
#include "company_manager_face.h"
#include "debug.h"
#include "fileio_func.h"
#include "engine_func.h"
#include "engine_base.h"
#include "bridge.h"
#include "town.h"
#include "newgrf_engine.h"
#include "newgrf_text.h"
#include "spritecache.h"
#include "currency.h"
#include "landscape.h"
#include "newgrf_badge.h"
#include "newgrf_badge_config.h"
#include "newgrf_cargo.h"
#include "newgrf_sound.h"
#include "newgrf_station.h"
#include "industrytype.h"
#include "newgrf_canal.h"
#include "newgrf_townname.h"
#include "newgrf_industries.h"
#include "newgrf_airporttiles.h"
#include "newgrf_airport.h"
#include "newgrf_object.h"
#include "network/core/config.h"
#include "smallmap_gui.h"
#include "genworld.h"
#include "error_func.h"
#include "vehicle_base.h"
#include "road.h"
#include "newgrf_roadstop.h"
#include "newgrf/newgrf_bytereader.h"
#include "newgrf/newgrf_internal_vehicle.h"
#include "newgrf/newgrf_internal.h"
#include "newgrf/newgrf_stringmapping.h"

#include "table/strings.h"

#include "rust/engine_loading_services.hpp"
#include "rust/vehicle_properties_services.hpp"
#include "rust/cargo_registry_ffi.h"
#include "rust/house_definition_services.hpp"

#include "rust/type_registry_ffi.h"

#include "safeguards.h"

/* TTDPatch extended GRF format codec
 * (c) Petr Baudis 2004 (GPL'd)
 * Changes by Florian octo Forster are (c) by the OpenTTD development team.
 *
 * Contains portions of documentation by TTDPatch team.
 * Thanks especially to Josef Drexler for the documentation as well as a lot
 * of help at #tycoon. Also thanks to Michael Blunck for his GRF files which
 * served as subject to the initial testing of this codec. */

/** List of all loaded GRF files */
static std::vector<GRFFile> _grf_files;

std::span<const GRFFile> GetAllGRFFiles()
{
	return _grf_files;
}

/** Miscellaneous GRF features, set by Action 0x0D, parameter 0x9E */
GrfMiscBits _misc_grf_features{};

/** Indicates which are the newgrf features currently loaded ingame */
GRFLoadedFeatures _loaded_newgrf_features;

GrfProcessingState _cur_gps;


/**
 * Debug() function dedicated to newGRF debugging messages
 * Function is essentially the same as Debug(grf, severity, ...) with the
 * addition of file:line information when parsing grf files.
 * NOTE: for the above reason(s) GrfMsg() should ONLY be used for
 * loading/parsing grf files, not for runtime debug messages as there
 * is no file information available during that time.
 * @param severity debugging severity level, see debug.h
 * @param msg the message
 */
void GrfMsgI(int severity, const std::string &msg)
{
	if (_cur_gps.grfconfig == nullptr) {
		Debug(grf, severity, "{}", msg);
	} else {
		Debug(grf, severity, "[{}:{}] {}", _cur_gps.grfconfig->filename, _cur_gps.nfo_line, msg);
	}
}

/**
 * Obtain a NewGRF file by its grfID
 * @param grfid The grfID to obtain the file for
 * @return The file.
 */
GRFFile *GetFileByGRFID(uint32_t grfid)
{
	auto it = std::ranges::find(_grf_files, grfid, &GRFFile::grfid);
	if (it != std::end(_grf_files)) return &*it;
	return nullptr;
}

/**
 * Obtain a NewGRF file by its filename
 * @param filename The filename to obtain the file for.
 * @return The file.
 */
static GRFFile *GetFileByFilename(const std::string &filename)
{
	auto it = std::ranges::find(_grf_files, filename, &GRFFile::filename);
	if (it != std::end(_grf_files)) return &*it;
	return nullptr;
}

/** Reset all NewGRFData that was used only while processing data */

/**
 * Disable a GRF
 * @param message Error message or STR_NULL.
 * @param config GRFConfig to disable, nullptr for current.
 * @return Error message of the GRF for further customisation.
 */
GRFError *DisableGrf(StringID message, GRFConfig *config)
{
	return openttd_rust_grf_disable(&GetRustGrfLoadingServices(), message, config);
}

/**
 * Disable a static NewGRF when it is influencing another (non-static)
 * NewGRF as this could cause desyncs.
 *
 * We could just tell the NewGRF querying that the file doesn't exist,
 * but that might give unwanted results. Disabling the NewGRF gives the
 * best result as no NewGRF author can complain about that.
 * @param c The NewGRF to disable.
 */


static bool GetNewGRFOverride(uint32_t source, uint32_t &target)
{
	return openttd_rust_engine_override_get(source, &target);
}

/**
 * Set the override for a NewGRF
 * @param source_grfid The grfID which wants to override another NewGRF.
 * @param target_grfid The grfID which is being overridden.
 */
void SetNewGRFOverride(uint32_t source_grfid, uint32_t target_grfid)
{
	openttd_rust_engine_set_override(&GetRustEngineLoadingServices(), source_grfid, target_grfid);
}

/**
 * Get overridden GRF for current GRF if present.
 * @return Overridden GRFFile if present, or nullptr.
 */
GRFFile *GetCurrentGRFOverride()
{
	return const_cast<GRFFile *>(openttd_rust_engine_current_override(&GetRustEngineLoadingServices(), _cur_gps.grffile));
}

/**
 * Returns the engine associated to a certain internal_id, resp. allocates it.
 * @param file NewGRF that wants to change the engine.
 * @param type Vehicle type.
 * @param internal_id Engine ID inside the NewGRF.
 * @param static_access If the engine is not present, return nullptr instead of allocating a new engine. (Used for static Action 0x04).
 * @return The requested engine.
 */
Engine *GetNewEngine(const GRFFile *file, VehicleType type, uint16_t internal_id, bool static_access)
{
	return const_cast<Engine *>(EngineOf(openttd_rust_engine_definition(&GetRustEngineLoadingServices(), file, type, internal_id, static_access)));
}

/**
 * Return the ID of a new engine
 * @param file The NewGRF file providing the engine.
 * @param type The Vehicle type.
 * @param internal_id NewGRF-internal ID of the engine.
 * @return The new EngineID.
 * @note depending on the dynamic_engine setting and a possible override
 *       property the grfID may be unique or overwriting or partially re-defining
 *       properties of an existing engine.
 */
EngineID GetNewEngineID(const GRFFile *file, VehicleType type, uint16_t internal_id)
{
	return EngineID{openttd_rust_engine_new_id(&GetRustEngineLoadingServices(), file, type, internal_id)};
}




/**
 * Translate the refit mask. refit_mask is uint32_t as it has not been mapped to CargoTypes.
 */
CargoTypes TranslateRefitMask(uint32_t refit_mask)
{
	return openttd_rust_engine_translate_refit(refit_mask, _cur_gps.grffile);
}

/**
 * Converts TTD(P) Base Price pointers into the enum used by OTTD
 * See http://wiki.ttdpatch.net/tiki-index.php?page=BaseCosts
 * @param base_pointer TTD(P) Base Price Pointer
 * @param error_location Function name for grf error messages
 * @param[out] index If \a base_pointer is valid, \a index is assigned to the matching price; else it is left unchanged
 */

/**
 * Get the language map associated with a given NewGRF and language.
 * @param grfid       The NewGRF to get the map for.
 * @param language_id The (NewGRF) language ID to get the map for.
 * @return The LanguageMap, or nullptr if it couldn't be found.
 */
/* static */ const LanguageMap *LanguageMap::GetLanguageMap(uint32_t grfid, uint8_t language_id)
{
	/* LanguageID "MAX_LANG", i.e. 7F is any. This language can't have a gender/case mapping, but has to be handled gracefully. */
	const GRFFile *grffile = GetFileByGRFID(grfid);
	if (grffile == nullptr) return nullptr;

	auto it = grffile->language_map.find(language_id);
	if (it == std::end(grffile->language_map)) return nullptr;

	return &it->second;
}

/**
 * Set the current NewGRF as unsafe for static use
 * @note Used during safety scan on unsafe actions.
 */
void GRFUnsafe(ByteReader &)
{
	openttd_rust_grf_unsafe(&GetRustGrfLoadingServices());
}

/** Reset and clear all NewGRFs */

/** Clear all NewGRF errors */


/**
 * Reset all NewGRF loaded data
 */
void ResetNewGRFData()
{
	openttd_rust_grf_reset(&GetRustGrfLoadingServices());
}

/**
 * Reset NewGRF data which is stored persistently in savegames.
 */
void ResetPersistentNewGRFData()
{
	/* Reset override managers */
	_engine_mngr.ResetToDefaultMapping();
	_house_mngr.ResetMapping();
	_industry_mngr.ResetMapping();
	_industile_mngr.ResetMapping();
	_airport_mngr.ResetMapping();
	_airporttile_mngr.ResetMapping();
}

/**
 * Get the cargo translation table to use for the given GRF file.
 * @param grffile GRF file.
 * @returns Readonly cargo translation table to use.
 */
 std::span<const CargoLabel> GetCargoTranslationTable(const GRFFile &grffile)
 {
	auto labels = openttd_rust_cargo_translation_table(&grffile);
	return {reinterpret_cast<const CargoLabel *>(labels.data), labels.size};
}

/**
 * Construct the Cargo Mapping
 * @note This is the reverse of a cargo translation table
 */

/**
 * Prepare loading a NewGRF file with its config
 * @param config The NewGRF configuration struct with name, id, parameters and alike.
 */
static void InitNewGRFFile(const GRFConfig &config)
{
	GRFFile *newfile = GetFileByFilename(config.filename);
	if (newfile != nullptr) {
		/* We already loaded it once. */
		_cur_gps.grffile = newfile;
		return;
	}

	assert(_grf_files.size() < _grf_files.capacity()); // We must not invalidate pointers.
	_cur_gps.grffile = &_grf_files.emplace_back(config);
}

/**
 * Constructor for GRFFile
 * @param config GRFConfig to copy name, grfid and parameters from.
 */
GRFFile::GRFFile(const GRFConfig &config)
{
	this->filename = config.filename;
	this->grfid = config.ident.grfid;

	/* Initialise local settings to defaults */
	this->traininfo_vehicle_pitch = 0;
	this->traininfo_vehicle_width = TRAININFO_DEFAULT_VEHICLE_WIDTH;

	/* Mark price_base_multipliers as 'not set' */
	this->price_base_multipliers.fill(INVALID_PRICE_MODIFIER);

	/* Initialise rail type map with default rail types */
	this->railtype_map.fill(INVALID_RAILTYPE);
	this->railtype_map[0] = RAILTYPE_RAIL;
	this->railtype_map[1] = RAILTYPE_ELECTRIC;
	this->railtype_map[2] = RAILTYPE_MONO;
	this->railtype_map[3] = RAILTYPE_MAGLEV;

	/* Initialise road type map with default road types */
	this->roadtype_map.fill(INVALID_ROADTYPE);
	this->roadtype_map[0] = ROADTYPE_ROAD;

	/* Initialise tram type map with default tram types */
	this->tramtype_map.fill(INVALID_ROADTYPE);
	this->tramtype_map[0] = ROADTYPE_TRAM;

	/* Copy the initial parameter list */
	this->param = config.param;
}

/* Some compilers get confused about vectors of unique_ptrs. */
GRFFile::GRFFile() = default;
GRFFile::GRFFile(GRFFile &&other) = default;
GRFFile::~GRFFile() = default;

/**
 * Find first cargo label that exists and is active from a list of cargo labels.
 * @param labels List of cargo labels.
 * @returns First cargo label in list that exists, or CT_INVALID if none exist.
 */

/**
 * Precalculate refit masks from cargo classes for all vehicles.
 */

/** Set to use the correct action0 properties for each canal feature */
static void FinaliseCanals()
{
	for (uint i = 0; i < CF_END; i++) {
		if (_water_feature[i].grffile != nullptr) {
			_water_feature[i].callback_mask = _water_feature[i].grffile->canal_local_properties[i].callback_mask;
			_water_feature[i].flags = _water_feature[i].grffile->canal_local_properties[i].flags;
		}
	}
}

/** Check for invalid engines */

/** Check for invalid cargoes */


/**
 * Add all new industries to the industry array. Industry properties can be set at any
 * time in the GRF file, so we can only add a industry spec to the industry array
 * after the file has finished loading.
 */

/**
 * Add all new objects to the object array. Object properties can be set at any
 * time in the GRF file, so we can only add an object spec to the object array
 * after the file has finished loading.
 */
static void FinaliseObjectsArray()
{
	for (auto &file : _grf_files) {
		for (auto &objectspec : file.objectspec) {
			if (objectspec != nullptr && objectspec->grf_prop.HasGrfFile() && objectspec->IsEnabled()) {
				_object_mngr.SetEntitySpec(std::move(*objectspec));
			}
		}

		/* Won't be used again */
		file.objectspec.clear();
		file.objectspec.shrink_to_fit();
	}

	ObjectSpec::BindToClasses();
}

/**
 * Add all new airports to the airport array. Airport properties can be set at any
 * time in the GRF file, so we can only add a airport spec to the airport array
 * after the file has finished loading.
 */


/**
 * Load a particular NewGRF.
 * @param config     The configuration of the to be loaded NewGRF.
 * @param stage      The loading stage of the NewGRF.
 * @param subdir     The sub directory to find the NewGRF in.
 * @param temporary  The NewGRF/sprite file is to be loaded temporarily and should be closed immediately,
 *                   contrary to loading the SpriteFile and having it cached by the SpriteCache.
 */
void LoadNewGRFFile(GRFConfig &config, GrfLoadingStage stage, Subdirectory subdir, bool temporary)
{
	std::optional<SpriteFile> temporary_file;
	std::string translated_text;
	AutoRestoreBackup text_frame(_cur_gps.translated_text, &translated_text);
	openttd_rust_grf_load_file(&GetRustGrfLoadingServices(), &config, stage, subdir, temporary ? &temporary_file : nullptr);
}

/**
 * Relocates the old shore sprites at new positions.
 *
 * 1. If shore sprites are neither loaded by Action5 nor ActionA, the extra sprites from openttd(w/d).grf are used. (SHORE_REPLACE_ONLY_NEW)
 * 2. If a newgrf replaces some shore sprites by ActionA. The (maybe also replaced) grass tiles are used for corner shores. (SHORE_REPLACE_ACTION_A)
 * 3. If a newgrf replaces shore sprites by Action5 any shore replacement by ActionA has no effect. (SHORE_REPLACE_ACTION_5)
 */
static void ActivateOldShore()
{
	/* Use default graphics, if no shore sprites were loaded.
	 * Should not happen, as the base set's extra grf should include some. */
	if (_loaded_newgrf_features.shore == SHORE_REPLACE_NONE) _loaded_newgrf_features.shore = SHORE_REPLACE_ACTION_A;

	if (_loaded_newgrf_features.shore != SHORE_REPLACE_ACTION_5) {
		DupSprite(SPR_ORIGINALSHORE_START +  1, SPR_SHORE_BASE +  1); // SLOPE_W
		DupSprite(SPR_ORIGINALSHORE_START +  2, SPR_SHORE_BASE +  2); // SLOPE_S
		DupSprite(SPR_ORIGINALSHORE_START +  6, SPR_SHORE_BASE +  3); // SLOPE_SW
		DupSprite(SPR_ORIGINALSHORE_START +  0, SPR_SHORE_BASE +  4); // SLOPE_E
		DupSprite(SPR_ORIGINALSHORE_START +  4, SPR_SHORE_BASE +  6); // SLOPE_SE
		DupSprite(SPR_ORIGINALSHORE_START +  3, SPR_SHORE_BASE +  8); // SLOPE_N
		DupSprite(SPR_ORIGINALSHORE_START +  7, SPR_SHORE_BASE +  9); // SLOPE_NW
		DupSprite(SPR_ORIGINALSHORE_START +  5, SPR_SHORE_BASE + 12); // SLOPE_NE
	}

	if (_loaded_newgrf_features.shore == SHORE_REPLACE_ACTION_A) {
		DupSprite(SPR_FLAT_GRASS_TILE + 16, SPR_SHORE_BASE +  0); // SLOPE_STEEP_S
		DupSprite(SPR_FLAT_GRASS_TILE + 17, SPR_SHORE_BASE +  5); // SLOPE_STEEP_W
		DupSprite(SPR_FLAT_GRASS_TILE +  7, SPR_SHORE_BASE +  7); // SLOPE_WSE
		DupSprite(SPR_FLAT_GRASS_TILE + 15, SPR_SHORE_BASE + 10); // SLOPE_STEEP_N
		DupSprite(SPR_FLAT_GRASS_TILE + 11, SPR_SHORE_BASE + 11); // SLOPE_NWS
		DupSprite(SPR_FLAT_GRASS_TILE + 13, SPR_SHORE_BASE + 13); // SLOPE_ENW
		DupSprite(SPR_FLAT_GRASS_TILE + 14, SPR_SHORE_BASE + 14); // SLOPE_SEN
		DupSprite(SPR_FLAT_GRASS_TILE + 18, SPR_SHORE_BASE + 15); // SLOPE_STEEP_E

		/* XXX - SLOPE_EW, SLOPE_NS are currently not used.
		 *       If they would be used somewhen, then these grass tiles will most like not look as needed */
		DupSprite(SPR_FLAT_GRASS_TILE +  5, SPR_SHORE_BASE + 16); // SLOPE_EW
		DupSprite(SPR_FLAT_GRASS_TILE + 10, SPR_SHORE_BASE + 17); // SLOPE_NS
	}
}

/**
 * Relocate the old tram depot sprites to the new position, if no new ones were loaded.
 */
static void ActivateOldTramDepot()
{
	if (_loaded_newgrf_features.tram == TRAMWAY_REPLACE_DEPOT_WITH_TRACK) {
		DupSprite(SPR_ROAD_DEPOT               + 0, SPR_TRAMWAY_DEPOT_NO_TRACK + 0); // use road depot graphics for "no tracks"
		DupSprite(SPR_TRAMWAY_DEPOT_WITH_TRACK + 1, SPR_TRAMWAY_DEPOT_NO_TRACK + 1);
		DupSprite(SPR_ROAD_DEPOT               + 2, SPR_TRAMWAY_DEPOT_NO_TRACK + 2); // use road depot graphics for "no tracks"
		DupSprite(SPR_TRAMWAY_DEPOT_WITH_TRACK + 3, SPR_TRAMWAY_DEPOT_NO_TRACK + 3);
		DupSprite(SPR_TRAMWAY_DEPOT_WITH_TRACK + 4, SPR_TRAMWAY_DEPOT_NO_TRACK + 4);
		DupSprite(SPR_TRAMWAY_DEPOT_WITH_TRACK + 5, SPR_TRAMWAY_DEPOT_NO_TRACK + 5);
	}
}

/**
 * Decide whether price base multipliers of grfs shall apply globally or only to the grf specifying them
 */
static void FinalisePriceBaseMultipliers()
{
	extern const PriceBaseSpec _price_base_specs[];
	/** Features, to which '_grf_id_overrides' applies. Currently vehicle features only. */
	static constexpr GrfSpecFeatures override_features{GSF_TRAINS, GSF_ROADVEHICLES, GSF_SHIPS, GSF_AIRCRAFT};

	/* Evaluate grf overrides */
	int num_grfs = (uint)_grf_files.size();
	std::vector<int> grf_overrides(num_grfs, -1);
	for (int i = 0; i < num_grfs; i++) {
		GRFFile &source = _grf_files[i];
		uint32_t override_grfid;
		if (!GetNewGRFOverride(source.grfid, override_grfid)) continue;

		auto dest = std::ranges::find(_grf_files, override_grfid, &GRFFile::grfid);
		if (dest == std::end(_grf_files)) continue;

		grf_overrides[i] = static_cast<int>(std::ranges::distance(std::begin(_grf_files), dest));
		assert(grf_overrides[i] >= 0);
	}

	/* Override features and price base multipliers of earlier loaded grfs */
	for (int i = 0; i < num_grfs; i++) {
		if (grf_overrides[i] < 0 || grf_overrides[i] >= i) continue;
		GRFFile &source = _grf_files[i];
		GRFFile &dest = _grf_files[grf_overrides[i]];

		GrfSpecFeatures features = (source.grf_features | dest.grf_features) & override_features;
		source.grf_features.Set(features);
		dest.grf_features.Set(features);

		for (Price p = PR_BEGIN; p < PR_END; p++) {
			/* No price defined -> nothing to do */
			if (!features.Test(_price_base_specs[p].grf_feature) || source.price_base_multipliers[p] == INVALID_PRICE_MODIFIER) continue;
			Debug(grf, 3, "'{}' overrides price base multiplier {} of '{}'", source.filename, p, dest.filename);
			dest.price_base_multipliers[p] = source.price_base_multipliers[p];
		}
	}

	/* Propagate features and price base multipliers of afterwards loaded grfs, if none is present yet */
	for (int i = num_grfs - 1; i >= 0; i--) {
		if (grf_overrides[i] < 0 || grf_overrides[i] <= i) continue;
		GRFFile &source = _grf_files[i];
		GRFFile &dest = _grf_files[grf_overrides[i]];

		GrfSpecFeatures features = (source.grf_features | dest.grf_features) & override_features;
		source.grf_features.Set(features);
		dest.grf_features.Set(features);

		for (Price p = PR_BEGIN; p < PR_END; p++) {
			/* Already a price defined -> nothing to do */
			if (!features.Test(_price_base_specs[p].grf_feature) || dest.price_base_multipliers[p] != INVALID_PRICE_MODIFIER) continue;
			Debug(grf, 3, "Price base multiplier {} from '{}' propagated to '{}'", p, source.filename, dest.filename);
			dest.price_base_multipliers[p] = source.price_base_multipliers[p];
		}
	}

	/* The 'master grf' now have the correct multipliers. Assign them to the 'addon grfs' to make everything consistent. */
	for (int i = 0; i < num_grfs; i++) {
		if (grf_overrides[i] < 0) continue;
		GRFFile &source = _grf_files[i];
		GRFFile &dest = _grf_files[grf_overrides[i]];

		GrfSpecFeatures features = (source.grf_features | dest.grf_features) & override_features;
		source.grf_features.Set(features);
		dest.grf_features.Set(features);

		for (Price p = PR_BEGIN; p < PR_END; p++) {
			if (!features.Test(_price_base_specs[p].grf_feature)) continue;
			if (source.price_base_multipliers[p] != dest.price_base_multipliers[p]) {
				Debug(grf, 3, "Price base multiplier {} from '{}' propagated to '{}'", p, dest.filename, source.filename);
			}
			source.price_base_multipliers[p] = dest.price_base_multipliers[p];
		}
	}

	/* Apply fallback prices for grf version < 8 */
	for (auto &file : _grf_files) {
		if (file.grf_version >= 8) continue;
		PriceMultipliers &price_base_multipliers = file.price_base_multipliers;
		for (Price p = PR_BEGIN; p < PR_END; p++) {
			Price fallback_price = _price_base_specs[p].fallback_price;
			if (fallback_price != INVALID_PRICE && price_base_multipliers[p] == INVALID_PRICE_MODIFIER) {
				/* No price multiplier has been set.
				 * So copy the multiplier from the fallback price, maybe a multiplier was set there. */
				price_base_multipliers[p] = price_base_multipliers[fallback_price];
			}
		}
	}

	/* Decide local/global scope of price base multipliers */
	for (auto &file : _grf_files) {
		PriceMultipliers &price_base_multipliers = file.price_base_multipliers;
		for (Price p = PR_BEGIN; p < PR_END; p++) {
			if (price_base_multipliers[p] == INVALID_PRICE_MODIFIER) {
				/* No multiplier was set; set it to a neutral value */
				price_base_multipliers[p] = 0;
			} else {
				if (!file.grf_features.Test(_price_base_specs[p].grf_feature)) {
					/* The grf does not define any objects of the feature,
					 * so it must be a difficulty setting. Apply it globally */
					Debug(grf, 3, "'{}' sets global price base multiplier {}", file.filename, p);
					SetPriceBaseMultiplier(p, price_base_multipliers[p]);
					price_base_multipliers[p] = 0;
				} else {
					Debug(grf, 3, "'{}' sets local price base multiplier {}", file.filename, p);
				}
			}
		}
	}
}

template <typename T>
void AddBadgeToSpecs(T &specs, GrfSpecFeature feature, Badge &badge)
{
	for (auto &spec : specs) {
		if (spec == nullptr) continue;
		spec->badges.push_back(badge.index);
		badge.features.Set(feature);
	}
}

/** Finish up applying badges to things */
static void FinaliseBadges()
{
	for (const auto &file : _grf_files) {
		Badge *badge = GetBadgeByLabel(fmt::format("newgrf/{:08x}", std::byteswap(file.grfid)));
		if (badge == nullptr) continue;

		for (Engine *e : Engine::Iterate()) {
			if (e->grf_prop.grffile != &file) continue;
			e->badges.push_back(badge->index);
			badge->features.Set(static_cast<GrfSpecFeature>(GSF_TRAINS + e->type));
		}

		AddBadgeToSpecs(file.stations, GSF_STATIONS, *badge);
		AddBadgeToSpecs(file.housespec, GSF_HOUSES, *badge);
		AddBadgeToSpecs(file.industryspec, GSF_INDUSTRIES, *badge);
		AddBadgeToSpecs(file.indtspec, GSF_INDUSTRYTILES, *badge);
		AddBadgeToSpecs(file.objectspec, GSF_OBJECTS, *badge);
		AddBadgeToSpecs(file.airportspec, GSF_AIRPORTS, *badge);
		AddBadgeToSpecs(file.airtspec, GSF_AIRPORTTILES, *badge);
		AddBadgeToSpecs(file.roadstops, GSF_ROADSTOPS, *badge);
	}

	ApplyBadgeFeaturesToClassBadges();
	AddBadgeClassesToConfiguration();
}


/**
 * Load all the NewGRFs.
 * @param load_index The offset for the first sprite to add.
 * @param num_baseset Number of NewGRFs at the front of the list to look up in the baseset dir instead of the newgrf dir.
 */
void LoadNewGRF(SpriteID load_index, uint num_baseset)
{
	std::string translated_text;
	AutoRestoreBackup text_frame(_cur_gps.translated_text, &translated_text);
	openttd_rust_grf_load(&GetRustGrfLoadingServices(), load_index, num_baseset);
}

const OpenTTDTypeRegistryServices &GetRustTypeRegistryServices() noexcept
{
	static const OpenTTDTypeRegistryServices table = {
		.rail = &GetRustRailRegistryServices(),
		.road = &GetRustRoadRegistryServices(),
		.rail_labels = [](const GRFFile *file) noexcept { return OpenTTDTypeLabels{.data = file->railtype_list.data(), .size = file->railtype_list.size()}; },
		.road_labels = [](const GRFFile *file) noexcept { return OpenTTDTypeLabels{.data = file->roadtype_list.data(), .size = file->roadtype_list.size()}; },
		.tram_labels = [](const GRFFile *file) noexcept { return OpenTTDTypeLabels{.data = file->tramtype_list.data(), .size = file->tramtype_list.size()}; },
		.rail_map = [](GRFFile *file) noexcept { return OpenTTDTypeMap{.data = reinterpret_cast<uint8_t *>(file->railtype_map.data()), .size = file->railtype_map.size()}; },
		.road_map = [](GRFFile *file) noexcept { return OpenTTDTypeMap{.data = reinterpret_cast<uint8_t *>(file->roadtype_map.data()), .size = file->roadtype_map.size()}; },
		.tram_map = [](GRFFile *file) noexcept { return OpenTTDTypeMap{.data = reinterpret_cast<uint8_t *>(file->tramtype_map.data()), .size = file->tramtype_map.size()}; },
	};
	return table;
}
#include "rust/newgrf_loading_services.inc"
