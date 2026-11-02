/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file spec_services_ffi.h Shared typed NewGRF specification leaves, one table per spec type. */
#ifndef RUST_SPEC_SERVICES_FFI_H
#define RUST_SPEC_SERVICES_FFI_H
#include "service_defaults.hpp"
#include "spec_identity.h"
#include "../newgrf_badge_type.h"
#include <cstdint>
#include <vector>
struct AirportSpec; struct AirportTileSpec; struct GRFFile; struct IndustrySpec; struct IndustryTileSpec;
struct ObjectSpec; struct ResolverObject; struct RoadStopSpec; struct SpriteGroup; struct StationSpec;
/* Specs carrying `StandardGRFFileProps`. */
template <typename Spec> struct OpenTTDStandardSpecServices {
	bool (*has_sprite_groups)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(has_sprite_groups);
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
	uint32_t (*badges)(const ResolverObject *, const Spec *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	bool (*boolean_callback)(const Spec *, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(boolean_callback);
	uint8_t (*cargo_map)(const Spec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_map);
	const GRFFile *(*file)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(file);
	uint32_t (*grfid)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(grfid);
	bool (*has_file)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(has_file);
	uint16_t (*local_id)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(local_id);
	void (*set_file)(Spec *, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(set_file);
	void (*set_local_id)(Spec *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_local_id);
	const OpenTTDStandardSpecServices<Spec> *standard;
	const OpenTTDSubstituteSpecServices<Spec> *substitute;
};
using OpenTTDAirportSpecServices = OpenTTDSpecServices<AirportSpec, uint8_t>;
using OpenTTDAirportTileSpecServices = OpenTTDSpecServices<AirportTileSpec, uint8_t>;
using OpenTTDIndustrySpecServices = OpenTTDSpecServices<IndustrySpec, uint8_t>;
using OpenTTDIndustryTileSpecServices = OpenTTDSpecServices<IndustryTileSpec, uint8_t>;
using OpenTTDObjectSpecServices = OpenTTDSpecServices<ObjectSpec, uint16_t>;
using OpenTTDRoadStopSpecServices = OpenTTDSpecServices<RoadStopSpec, uint16_t>;
using OpenTTDStationSpecServices = OpenTTDSpecServices<StationSpec, uint16_t>;
using OpenTTDAirportTileSpecAnimation = OpenTTDSpecAnimationServices<AirportTileSpec, uint8_t>;
using OpenTTDIndustryTileSpecAnimation = OpenTTDSpecAnimationServices<IndustryTileSpec, uint8_t>;
using OpenTTDRoadStopSpecAnimation = OpenTTDSpecAnimationServices<RoadStopSpec, uint16_t>;
using OpenTTDStationSpecAnimation = OpenTTDSpecAnimationServices<StationSpec, uint16_t>;
using OpenTTDAirportSpecStandard = OpenTTDStandardSpecServices<AirportSpec>;
using OpenTTDAirportTileSpecStandard = OpenTTDStandardSpecServices<AirportTileSpec>;
using OpenTTDIndustrySpecStandard = OpenTTDStandardSpecServices<IndustrySpec>;
using OpenTTDIndustryTileSpecStandard = OpenTTDStandardSpecServices<IndustryTileSpec>;
using OpenTTDObjectSpecStandard = OpenTTDStandardSpecServices<ObjectSpec>;
using OpenTTDAirportSpecSubstitute = OpenTTDSubstituteSpecServices<AirportSpec>;
using OpenTTDAirportTileSpecSubstitute = OpenTTDSubstituteSpecServices<AirportTileSpec>;
using OpenTTDIndustrySpecSubstitute = OpenTTDSubstituteSpecServices<IndustrySpec>;
using OpenTTDIndustryTileSpecSubstitute = OpenTTDSubstituteSpecServices<IndustryTileSpec>;
struct OpenTTDNewGRFSpecs {
	const OpenTTDAirportSpecServices *airport;
	const OpenTTDAirportTileSpecServices *airport_tile;
	const OpenTTDIndustrySpecServices *industry;
	const OpenTTDIndustryTileSpecServices *industry_tile;
	const OpenTTDObjectSpecServices *object;
	const OpenTTDRoadStopSpecServices *road_stop;
	const OpenTTDStationSpecServices *station;
};
#endif /* RUST_SPEC_SERVICES_FFI_H */
