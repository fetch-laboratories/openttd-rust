/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file animation_save.hpp Native-only decoding scope for Rust ANIT storage. */
#ifndef RUST_ANIMATION_SAVE_HPP
#define RUST_ANIMATION_SAVE_HPP
#include "animation_ffi.h"
struct RustAnimationSave {
	std::vector<TileIndex> tiles;
	bool loading;
	explicit RustAnimationSave(bool loading) : loading(loading) {
		const size_t count = openttd_rust_animation_count();
		this->tiles.reserve(count);
		for (size_t i = 0; i < count; ++i) this->tiles.emplace_back(openttd_rust_animation_at(i));
	}
	~RustAnimationSave() {
		if (this->loading) {
			openttd_rust_animation_initialize();
			for (TileIndex tile : this->tiles) openttd_rust_animation_append(tile.base());
		}
	}
};
#endif /* RUST_ANIMATION_SAVE_HPP */
