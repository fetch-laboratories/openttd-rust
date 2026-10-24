/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file animation_binding.hpp Six typed resolver bindings; specs stay native. */
#ifndef RUST_ANIMATION_BINDING_HPP
#define RUST_ANIMATION_BINDING_HPP
#include "animation_ffi.h"
#include "services_newgrf.h"
#include "../newgrf_animation_type.h"
#include "../newgrf_sound.h"
/* Configuration is immutable during synchronous Action2 evaluation: resolvers
 * mutate PSA/owner state, not specs. Keep live GRF-version reads branch-local. */
template <typename Spec, typename Obj, typename Extra,
	uint16_t (*Callback)(CallbackID, uint32_t, uint32_t, const Spec *, Obj *, TileIndex, Extra)>
OpenTTDAnimationBinding<Spec, Obj, Extra> MakeRustAnimationBinding(
	OpenTTDAnimationEntry<Spec, Obj> (*entry)(uint32_t) noexcept,
	OpenTTDAnimationInfo (*info)(const Spec *) noexcept) noexcept
{
	return {.entry = entry, .info = info,
		.callback = [](uint16_t cb, uint32_t a, uint32_t b, const Spec *s, Obj *o, uint32_t t, Extra extra) noexcept { return Callback(static_cast<CallbackID>(cb), a, b, s, o, TileIndex{t}, extra); },
		.sound = [](const Spec *s, uint8_t sound, uint32_t t) noexcept { PlayTileSound(s->grf_prop.grffile, sound, TileIndex{t}); },
		.version8 = [](const Spec *s) noexcept { return s->grf_prop.grffile->grf_version >= 8; },
		.speed_error = RustSpecCallbackError<Spec, Spec>};
}
#endif /* RUST_ANIMATION_BINDING_HPP */
