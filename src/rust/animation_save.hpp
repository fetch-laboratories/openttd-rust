/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file animation_save.hpp Native-only decoding scope for Rust ANIT storage. */
#ifndef RUST_ANIMATION_SAVE_HPP
#define RUST_ANIMATION_SAVE_HPP
#include "animation_ffi.h"
struct RustAnimationScope {
	const OpenTTDAnimationServices *previous;
	explicit RustAnimationScope(const OpenTTDAnimationServices &services) : previous(openttd_rust_animation_exchange(&services)) {}
	~RustAnimationScope() { openttd_rust_animation_exchange(this->previous); }
};
struct RustAnimationSave {
	std::vector<TileIndex> tiles;
	bool loading;
	inline static std::vector<TileIndex> *active = nullptr;
	std::vector<TileIndex> *previous;
	static std::vector<TileIndex> &Tiles() { return *active; }
	explicit RustAnimationSave(bool loading) : loading(loading), previous(active) {
		const size_t count = openttd_rust_animation_count();
		this->tiles.reserve(count);
		for (size_t i = 0; i < count; ++i) this->tiles.emplace_back(openttd_rust_animation_at(i));
		active = &this->tiles;
	}
	~RustAnimationSave() {
		if (this->loading) {
			openttd_rust_animation_initialize();
			for (TileIndex tile : this->tiles) openttd_rust_animation_append(tile.base());
		}
		active = this->previous;
	}
};
#endif /* RUST_ANIMATION_SAVE_HPP */
