/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_object.h Canonical native object identity and town association. */
#ifndef RUST_SERVICES_OBJECT_H
#define RUST_SERVICES_OBJECT_H
#include "../object_base.h"
inline Object *OpenTTDObjectByTile(uint32_t tile) noexcept { return Object::GetByTile(TileIndex(tile)); }
inline Town *OpenTTDObjectTown(const Object *object) noexcept { return object->town; }
inline ObjectType OpenTTDObjectType(const Object *object) noexcept { return object->type; }
inline uint32_t OpenTTDObjectTile(const Object *object) noexcept { return object->location.tile.base(); }
inline void OpenTTDSetObjectTown(Object *object, Town *town) noexcept { object->town = town; }
#endif /* RUST_SERVICES_OBJECT_H */
