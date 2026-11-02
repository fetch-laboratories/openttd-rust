/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file spec_services.hpp The single binding of the typed NewGRF specification leaves. */
#ifndef RUST_SPEC_SERVICES_HPP
#define RUST_SPEC_SERVICES_HPP
#include "spec_services_ffi.h"
#include "services_cargo.h"
#include "services_newgrf.h"
#include "../newgrf_airport.h"
#include "../newgrf_airporttiles.h"
#include "../newgrf_object.h"
#include "../newgrf_roadstop.h"
#include "../newgrf_station.h"
#include "../industrytype.h"
#include <type_traits>
template <typename Spec, typename Triggers>
inline constexpr OpenTTDSpecAnimationServices<Spec, Triggers> _rust_spec_animation{
	.frames = RustSpecAnimationFrames<Spec>,
	.speed = RustSpecAnimationSpeed<Spec>,
	.status = RustSpecAnimationStatus<Spec>,
	.triggers = RustSpecAnimationTriggers<Spec, Triggers>,
};
template <typename Spec>
inline constexpr OpenTTDStandardSpecServices<Spec> _rust_spec_standard{
	.has_sprite_groups = RustSpecHasSpriteGroups<Spec>,
	.sprite_group = RustSpecSpriteGroup<Spec>,
};
template <typename Spec>
inline constexpr OpenTTDSubstituteSpecServices<Spec> _rust_spec_substitute{
	.enabled = RustSpecEnabled<Spec>,
	.identity = RustSpecSubstituteIdentity<Spec>,
	.override_id = RustSpecOverride<Spec>,
	.set_enabled = RustSpecSetEnabled<Spec>,
	.set_override = RustSpecSetOverride<Spec>,
	.substitute = RustSpecSubstitute<Spec>,
};
template <typename Spec, typename Triggers>
constexpr const OpenTTDSpecAnimationServices<Spec, Triggers> *RustSpecAnimationTable() noexcept
{
	if constexpr (requires(Spec *spec) { spec->animation.triggers; }) {
		static_assert(std::is_same_v<Triggers, typename decltype(Spec::animation.triggers)::BaseType>);
		return &_rust_spec_animation<Spec, Triggers>;
	} else {
		return nullptr;
	}
}
template <typename Spec>
constexpr const OpenTTDStandardSpecServices<Spec> *RustSpecStandardTable() noexcept
{
	if constexpr (std::is_base_of_v<StandardGRFFileProps, decltype(Spec::grf_prop)>) {
		return &_rust_spec_standard<Spec>;
	} else {
		return nullptr;
	}
}
template <typename Spec>
constexpr const OpenTTDSubstituteSpecServices<Spec> *RustSpecSubstituteTable() noexcept
{
	if constexpr (requires(Spec *spec) { spec->grf_prop.subst_id; spec->enabled; }) {
		return &_rust_spec_substitute<Spec>;
	} else {
		return nullptr;
	}
}
template <typename Spec, typename Triggers>
inline constexpr OpenTTDSpecServices<Spec, Triggers> _rust_spec_services{
	.animation = RustSpecAnimationTable<Spec, Triggers>(),
	.badge_list = RustSpecBadgeList<Spec>,
	.badges = RustSpecBadges<Spec>,
	.boolean_callback = RustSpecBooleanCallback<Spec>,
	.cargo_map = RustSpecCargoMap<Spec>,
	.file = RustSpecGRFFile<Spec>,
	.grfid = RustSpecGRFID<Spec>,
	.has_file = RustSpecHasGRFFile<Spec>,
	.local_id = RustSpecLocalID<Spec>,
	.set_file = RustSpecSetGRFFile<Spec>,
	.set_local_id = RustSpecSetLocalID<Spec>,
	.standard = RustSpecStandardTable<Spec>(),
	.substitute = RustSpecSubstituteTable<Spec>(),
};
static constexpr OpenTTDNewGRFSpecs _rust_newgrf_specs{
	.airport = &_rust_spec_services<AirportSpec, uint8_t>,
	.airport_tile = &_rust_spec_services<AirportTileSpec, uint8_t>,
	.industry = &_rust_spec_services<IndustrySpec, uint8_t>,
	.industry_tile = &_rust_spec_services<IndustryTileSpec, uint8_t>,
	.object = &_rust_spec_services<ObjectSpec, uint16_t>,
	.road_stop = &_rust_spec_services<RoadStopSpec, uint16_t>,
	.station = &_rust_spec_services<StationSpec, uint16_t>,
};
#endif /* RUST_SPEC_SERVICES_HPP */
