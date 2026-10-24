/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file animation_types.hpp Native typed animation resolver tables. */
#ifndef RUST_ANIMATION_TYPES_HPP
#define RUST_ANIMATION_TYPES_HPP
template <typename Spec, typename Obj, typename Extra> struct OpenTTDAnimationBinding {
	OpenTTDAnimationEntry<Spec, Obj> (*entry)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(entry);
	OpenTTDAnimationInfo (*info)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(info);
	uint16_t (*callback)(uint16_t, uint32_t, uint32_t, const Spec *, Obj *, uint32_t, Extra) noexcept OPENTTD_SERVICE_DEFAULT(callback);
	void (*sound)(const Spec *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);
	bool (*version8)(const Spec *) noexcept OPENTTD_SERVICE_DEFAULT(version8);
	void (*speed_error)(const Spec *, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(speed_error);
};

struct OpenTTDAirportAnimation {
	OpenTTDAnimationInfo (*info)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(info);
	void (*sound)(const AirportTileSpec *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);
	bool (*version8)(const AirportTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(version8);
	void (*speed_error)(const AirportTileSpec *, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(speed_error);
};
#endif /* RUST_ANIMATION_TYPES_HPP */
