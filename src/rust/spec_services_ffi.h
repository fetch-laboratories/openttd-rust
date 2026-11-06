/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file spec_services_ffi.h Shared typed NewGRF specification leaves, one table per spec type. */
#ifndef RUST_SPEC_SERVICES_FFI_H
#define RUST_SPEC_SERVICES_FFI_H
#include "service_defaults.hpp"
#include "spec_identity.h"
#include "../newgrf_badge_type.h"
#include "abi-record-probe.h"
#include <cstdint>
#include <vector>
struct AirportSpec; struct AirportTileSpec; struct GRFFile;
struct HouseSpec; struct ObjectSpec; struct ResolverObject; struct RoadStopSpec; struct SpriteGroup; struct StationSpec;
/* Specs carrying `StandardGRFFileProps`. */
template <typename Spec> struct OpenTTDStandardSpecServices {
	bool (*has_sprite_groups)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(has_sprite_groups);
	void (*set_sprite_group)(Spec *, uint8_t, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(set_sprite_group);
	const SpriteGroup *(*sprite_group)(const Spec *, bool) noexcept OPENTTD_SERVICE_DEFAULT(sprite_group);
};
/* Specs carrying `SubstituteGRFFileProps` and an `enabled` flag. */
template <typename Spec> struct OpenTTDSubstituteSpecServices {
	bool (*enabled)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(enabled);
	OpenTTDSubstituteSpecIdentity (*identity)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(identity);
	uint16_t (*override_id)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(override_id);
	void (*set_enabled)(Spec *, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_enabled);
	void (*set_override)(Spec *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_override);
	uint16_t (*substitute)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(substitute);
};
/* Specs carrying `AnimationInfo`; `Triggers` is its trigger bitset's storage type. */
template <typename Spec, typename Triggers> struct OpenTTDSpecAnimationServices {
	void (*frames)(Spec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(frames);
	void (*speed)(Spec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(speed);
	void (*status)(Spec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(status);
	void (*triggers)(Spec *, Triggers) noexcept OPENTTD_SERVICE_DEFAULT(triggers);
};
/* Every spec; the capability tables are null where the spec lacks the capability. */
template <typename Spec, typename Triggers> struct OpenTTDSpecServices {
	const OpenTTDSpecAnimationServices<Spec, Triggers> *animation;
	std::vector<BadgeID> *(*badge_list)(Spec *) noexcept OPENTTD_SERVICE_DEFAULT(badge_list);
	OpenTTDBadgeSpan (*badges)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	uint8_t (*cargo_map)(const Spec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_map);
	const GRFFile *(*file)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(file);
	uint32_t (*grfid)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(grfid);
	bool (*has_file)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(has_file);
	uint16_t (*local_id)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(local_id);
	uint32_t *(*name)(Spec *) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (*set_file)(Spec *, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(set_file);
	void (*set_local_id)(Spec *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_local_id);
	const OpenTTDStandardSpecServices<Spec> *standard;
	const OpenTTDSubstituteSpecServices<Spec> *substitute;
};
using OpenTTDAirportSpecServices = OpenTTDSpecServices<AirportSpec, uint8_t>;
using OpenTTDAirportTileSpecServices = OpenTTDSpecServices<AirportTileSpec, uint8_t>;
using OpenTTDHouseSpecServices = OpenTTDSpecServices<HouseSpec, uint8_t>;
using OpenTTDRoadStopSpecServices = OpenTTDSpecServices<RoadStopSpec, uint16_t>;
using OpenTTDStationSpecServices = OpenTTDSpecServices<StationSpec, uint16_t>;
using OpenTTDAirportTileSpecAnimation = OpenTTDSpecAnimationServices<AirportTileSpec, uint8_t>;
using OpenTTDRoadStopSpecAnimation = OpenTTDSpecAnimationServices<RoadStopSpec, uint16_t>;
using OpenTTDStationSpecAnimation = OpenTTDSpecAnimationServices<StationSpec, uint16_t>;
using OpenTTDAirportSpecStandard = OpenTTDStandardSpecServices<AirportSpec>;
using OpenTTDAirportTileSpecStandard = OpenTTDStandardSpecServices<AirportTileSpec>;
using OpenTTDHouseSpecStandard = OpenTTDStandardSpecServices<HouseSpec>;
using OpenTTDAirportSpecSubstitute = OpenTTDSubstituteSpecServices<AirportSpec>;
using OpenTTDAirportTileSpecSubstitute = OpenTTDSubstituteSpecServices<AirportTileSpec>;
using OpenTTDHouseSpecSubstitute = OpenTTDSubstituteSpecServices<HouseSpec>;
struct OpenTTDNewGRFSpecs {
	const OpenTTDAirportSpecServices *airport;
	const OpenTTDAirportTileSpecServices *airport_tile;
	const OpenTTDHouseSpecServices *house;
	const OpenTTDRoadStopSpecServices *road_stop;
	const OpenTTDStationSpecServices *station;
};
#endif /* RUST_SPEC_SERVICES_FFI_H */
