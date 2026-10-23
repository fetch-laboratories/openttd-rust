/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file animation_binding.hpp Six typed resolver bindings; specs stay native. */
#ifndef RUST_ANIMATION_BINDING_HPP
#define RUST_ANIMATION_BINDING_HPP
#include "animation_ffi.h"
#include "services_newgrf.h"
#include "../newgrf_animation_type.h"
#include "../newgrf_sound.h"
template <typename Base, typename Spec, typename Obj, typename Extra,
	uint16_t (*Callback)(CallbackID, uint32_t, uint32_t, const Spec *, Obj *, TileIndex, Extra)>
struct RustAnimationBinding {
	using Binding = OpenTTDAnimationBinding<Spec, Obj, Extra>;
	static OpenTTDAnimationInfo Info(const Spec *s) noexcept {
		return {s->animation.speed, s->animation.frames, s->animation.status == AnimationStatus::Looping,
			s->callback_mask.Test(Base::cbm_animation_speed), s->callback_mask.Test(Base::cbm_animation_next_frame), Base::Random(s), Base::Enabled(s)};
	}
	static uint16_t Resolve(uint16_t cb, uint32_t a, uint32_t b, const Spec *s, Obj *o, uint32_t t, Extra extra) noexcept { return Callback(static_cast<CallbackID>(cb), a, b, s, o, TileIndex{t}, extra); }
	static void Sound(const Spec *s, uint8_t sound, uint32_t t) noexcept { PlayTileSound(s->grf_prop.grffile, sound, TileIndex{t}); }
	static void Error(const Spec *s, uint16_t result) noexcept {
		if (result >= 0x100 && s->grf_prop.grffile->grf_version >= 8) RustUnknownCallbackResult(s->grf_prop.grfid, Base::cb_animation_speed, result);
	}
	static const Binding &Get() noexcept {
		static const Binding binding{.entry = Base::Entry, .info = Info, .callback = Resolve, .sound = Sound, .speed_error = Error,
			.speed_callback = Base::cb_animation_speed, .next_callback = Base::cb_animation_next_frame};
		return binding;
	}
};
#endif /* RUST_ANIMATION_BINDING_HPP */
