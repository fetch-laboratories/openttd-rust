/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file bitmap_type.hpp Bitmap functions. */

#ifndef BITMAP_TYPE_HPP
#define BITMAP_TYPE_HPP


#include "rust/bitmap_ffi.h"
/**
 * Facade over a Rust-owned bitmap, either allocated for this object or
 * borrowed from its owner. The live geometry is the storage's leading TileArea.
 */
class BitmapTileArea {
	OpenTTDBitmapTileArea *bitmap;
	bool owned;
public:
	BitmapTileArea() : bitmap(openttd_rust_bitmap_new()), owned(true) {}
	BitmapTileArea(const TileArea &ta) : BitmapTileArea() { this->Initialize(ta); }
	explicit BitmapTileArea(OpenTTDBitmapTileArea *storage) : bitmap(storage), owned(false) {}
	BitmapTileArea(const BitmapTileArea &) = delete;
	BitmapTileArea &operator=(const BitmapTileArea &) = delete;
	~BitmapTileArea() { if (this->owned) openttd_rust_bitmap_destroy(this->bitmap); }

	/** The live inherited geometry. */
	operator const TileArea &() const { return *reinterpret_cast<const TileArea *>(this->bitmap); }
	void Initialize(const TileArea &ta) { openttd_rust_bitmap_initialize(this->bitmap, {ta.tile.base(), ta.w, ta.h}); }
	void SetTile(TileIndex tile) { openttd_rust_bitmap_set(this->bitmap, tile.base(), &GetRustSharedServices()); }
	bool HasTile(TileIndex tile) const { return openttd_rust_bitmap_has(this->bitmap, tile.base(), &GetRustSharedServices()); }
};


/** Iterator to iterate over all tiles belonging to a bitmaptilearea. */
class BitmapTileIterator : public OrthogonalTileIterator {
protected:
	const BitmapTileArea *bitmap;
public:
	/**
	 * Construct the iterator.
	 * @param bitmap BitmapTileArea to iterate.
	 */
	BitmapTileIterator(const BitmapTileArea &bitmap) : OrthogonalTileIterator(bitmap), bitmap(&bitmap)
	{
		if (!this->bitmap->HasTile(TileIndex(this->tile))) ++(*this);
	}

	inline TileIterator& operator ++() override
	{
		this->OrthogonalTileIterator::operator++();
		while (this->tile != INVALID_TILE && !this->bitmap->HasTile(TileIndex(this->tile))) {
			this->OrthogonalTileIterator::operator++();
		}
		return *this;
	}

	std::unique_ptr<TileIterator> Clone() const override
	{
		return std::make_unique<BitmapTileIterator>(*this);
	}
};



#endif /* BITMAP_TYPE_HPP */
