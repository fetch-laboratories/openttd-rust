/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_sound.h Shared actual-width sound settings. */
#ifndef RUST_SERVICES_SOUND_H
#define RUST_SERVICES_SOUND_H
#include "../settings_type.h"
inline bool OpenTTDAmbientSound() noexcept { return _settings_client.sound.ambient; }
/* Existing train callback retains its unused handle and legacy result width. */
template <typename Context>
inline uint64_t RustAmbientSoundLegacy(Context) noexcept { return OpenTTDAmbientSound(); }
#endif
