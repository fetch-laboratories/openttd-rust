/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file generation_sea_set.hpp Native identity only for observable flattening order. */
#ifndef RUST_GENERATION_SEA_SET_HPP
#define RUST_GENERATION_SEA_SET_HPP
#include "../tile_type.h"
#include <optional>
#include <unordered_set>
/* Exactly one search is live. Rust ends it before FlowRiver recursion. Fresh
 * emplace reproduces the source's initial buckets; clear would retain them.
 * No traversal, sorting, copying, generation or terraforming policy lives here. */
struct RustGenerationSeaSet {
	std::optional<std::unordered_set<TileIndex>> tiles;
	std::optional<std::unordered_set<TileIndex>::const_iterator> cursor;
	void Construct() noexcept { this->tiles.emplace(); }
	void Destroy() noexcept { this->cursor.reset(); this->tiles.reset(); }
	void Insert(uint32_t tile) noexcept { this->tiles->insert(TileIndex{tile}); }
	bool Contains(uint32_t tile) const noexcept { return this->tiles->contains(TileIndex{tile}); }
	size_t Size() const noexcept { return this->tiles->size(); }
	uint32_t Value() const noexcept { return *this->cursor == this->tiles->end() ? INVALID_TILE.base() : (**this->cursor).base(); }
	uint32_t First() noexcept { this->cursor = this->tiles->begin(); return this->Value(); }
	uint32_t Next() noexcept { ++*this->cursor; return this->Value(); }
};
#endif /* RUST_GENERATION_SEA_SET_HPP */
