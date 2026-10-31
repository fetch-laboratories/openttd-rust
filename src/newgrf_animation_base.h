/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_animation_base.h Function implementations related to NewGRF animation. */

/* No inclusion guards as this file must only be included from .cpp files. */

#include "animated_tile_func.h"
#include "core/random_func.hpp"
#include "timer/timer_game_tick.h"
#include "viewport_func.h"
#include "newgrf_animation_type.h"
#include "newgrf_callbacks.h"
#include "tile_map.h"
