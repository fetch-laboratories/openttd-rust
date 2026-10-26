/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_layout_services.h Checked calls to the canonical native layout parsers. */
#ifndef RUST_NEWGRF_LAYOUT_SERVICES_H
#define RUST_NEWGRF_LAYOUT_SERVICES_H
#include "newgrf_bytereader_services.h"

inline bool RustGRFReadSpriteLayoutSprite(ByteReader *reader, bool flags, bool invert, bool current, GrfSpecFeature feature, PalSpriteID *sprite, uint8_t *result) noexcept
{
	return RustGRFReadChecked([&] { *result = ReadSpriteLayoutSprite(*reader, flags, invert, current, feature, sprite); });
}
inline bool RustGRFReadSpriteLayout(ByteReader *reader, uint32_t count, bool current, GrfSpecFeature feature, bool var10, bool no_z, NewGRFSpriteLayout *layout, bool *disabled) noexcept
{
	return RustGRFReadChecked([&] { *disabled = ReadSpriteLayout(*reader, count, current, feature, var10, no_z, layout); });
}
#endif
