/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine_type.h Types related to engines. */

#ifndef ENGINE_TYPE_H
#define ENGINE_TYPE_H

#include "core/pool_type.hpp"
#include "economy_type.h"
#include "landscape_type.h"
#include "newgrf_callbacks.h"
#include "rail_type.h"
#include "road_type.h"
#include "cargo_type.h"
#include "timer/timer_game_calendar.h"
#include "sound_type.h"
#include "strings_type.h"
#include "newgrf_badge_type.h"

/** Unique identification number of an engine. */
using EngineID = PoolID<uint16_t, struct EngineIDTag, 64000, 0xFFFF>;

class Engine;

/** Available types of rail vehicles. */
enum RailVehicleTypes : uint8_t {
	RAILVEH_SINGLEHEAD,  ///< indicates a "standalone" locomotive
	RAILVEH_MULTIHEAD,   ///< indicates a combination of two locomotives
	RAILVEH_WAGON,       ///< simple wagon, not motorized
};

/** Type of rail engine. */
enum EngineClass : uint8_t {
	EC_STEAM,    ///< Steam rail engine.
	EC_DIESEL,   ///< Diesel rail engine.
	EC_ELECTRIC, ///< Electric rail engine.
	EC_MONORAIL, ///< Mono rail engine.
	EC_MAGLEV,   ///< Maglev engine.
};

/** Acceleration model of a vehicle. */
enum class VehicleAccelerationModel : uint8_t {
	Normal,   ///< Default acceleration model.
	Monorail, ///< Monorail acceleration model.
	Maglev,   ///< Maglev acceleration model.
};

/** Meaning of the various bits of the visual effect. */
enum VisualEffect : uint8_t {
	VE_OFFSET_START        = 0, ///< First bit that contains the offset (0 = front, 8 = centre, 15 = rear)
	VE_OFFSET_COUNT        = 4, ///< Number of bits used for the offset
	VE_OFFSET_CENTRE       = 8, ///< Value of offset corresponding to a position above the centre of the vehicle

	VE_TYPE_START          = 4, ///< First bit used for the type of effect
	VE_TYPE_COUNT          = 2, ///< Number of bits used for the effect type
	VE_TYPE_DEFAULT        = 0, ///< Use default from engine class
	VE_TYPE_STEAM          = 1, ///< Steam plumes
	VE_TYPE_DIESEL         = 2, ///< Diesel fumes
	VE_TYPE_ELECTRIC       = 3, ///< Electric sparks

	VE_DISABLE_EFFECT      = 6, ///< Flag to disable visual effect
	VE_ADVANCED_EFFECT     = VE_DISABLE_EFFECT, ///< Flag for advanced effects
	VE_DISABLE_WAGON_POWER = 7, ///< Flag to disable wagon power

	VE_DEFAULT = 0xFF,          ///< Default value to indicate that visual effect should be based on engine class
};

/**
 * AircraftVehicleInfo subtypes, bitmask type.
 * If bit 0 is 0 then it is a helicopter, otherwise it is a plane
 * in which case bit 1 tells us whether it's a big(fast) plane or not.
 */
enum AircraftSubTypeBits : uint8_t {
	AIR_HELI = 0,
	AIR_CTOL = 1, ///< Conventional Take Off and Landing, i.e. planes
	AIR_FAST = 2
};

/**
 * Extra engine flags for NewGRF features.
 * This is defined in the specification a 32 bit value, but most bits are not currently used.
 */
enum class ExtraEngineFlag : uint8_t {
	NoNews          = 0, ///< No 'new vehicle' news will be generated.
	NoPreview       = 1, ///< No exclusive preview will be offered.
	JoinPreview     = 2, ///< Engine will join exclusive preview with variant parent.
	SyncReliability = 3, ///< Engine reliability will be synced with variant parent.
};
using ExtraEngineFlags = EnumBitSet<ExtraEngineFlag, uint8_t>;

/**
 * EngineInfo.misc_flags is a bitmask, with the following values
 */
enum class EngineMiscFlag : uint8_t {
	RailTilts  = 0, ///< Rail vehicle tilts in curves
	RoadIsTram = 0, ///< Road vehicle is a tram/light rail vehicle

	Uses2CC    = 1, ///< Vehicle uses two company colours

	RailIsMU   = 2, ///< Rail vehicle is a multiple-unit (DMU/EMU)
	RailFlips  = 3, ///< Rail vehicle has old depot-flip handling

	AutoRefit                = 4, ///< Automatic refitting is allowed
	NoDefaultCargoMultiplier = 5, ///< Use the new capacity algorithm. The default cargotype of the vehicle does not affect capacity multipliers. CB 15 is also called in purchase list.
	NoBreakdownSmoke         = 6, ///< Do not show black smoke during a breakdown.
	SpriteStack              = 7, ///< Draw vehicle by stacking multiple sprites.
};
using EngineMiscFlags = EnumBitSet<EngineMiscFlag, uint8_t>;

/**
 * Engine.flags is a bitmask, with the following values.
 */
enum class EngineFlag : uint8_t {
	Available        = 0, ///< This vehicle is available to everyone.
	ExclusivePreview = 1, ///< This vehicle is in the exclusive preview stage, either being used or being offered to a company.
};
using EngineFlags = EnumBitSet<EngineFlag, uint8_t>;

#include "rust/cargo_label_ffi.h"
#include "abi-record-engine_spec.h"
constexpr OpenTTDNewGRFCargoLabel EngineCargoLabel(CargoLabel label) { return {label.base(), false}; }
constexpr OpenTTDNewGRFCargoLabel EngineCargoLabel(MixedCargoType cargo) { return {cargo, true}; }
inline uint ApplyWaterClassSpeedFrac(const ShipVehicleInfo &info, uint raw_speed, bool is_ocean)
{
	return raw_speed * (256 - (is_ocean ? info.ocean_speed_frac : info.canal_speed_frac)) / 256;
}

/**
 * Contexts an engine name can be shown in.
 */
enum class EngineNameContext : uint8_t {
	Generic                 = 0x00, ///< No specific context available.
	VehicleDetails          = 0x11, ///< Name is shown in the vehicle details GUI.
	PurchaseList            = 0x20, ///< Name is shown in the purchase list (including autoreplace window 'Available vehicles' panel).
	PreviewNews             = 0x21, ///< Name is shown in exclusive preview or newspaper.
	AutoreplaceVehicleInUse = 0x22, ///< Name is show in the autoreplace window 'Vehicles in use' panel.
};

/** Combine an engine ID and a name context to an engine name dparam. */
inline uint64_t PackEngineNameDParam(EngineID engine_id, EngineNameContext context, uint32_t extra_data = 0)
{
	return engine_id.base() | (static_cast<uint64_t>(context) << 32) | (static_cast<uint64_t>(extra_data) << 40);
}

static const uint MAX_LENGTH_ENGINE_NAME_CHARS = 32; ///< The maximum length of an engine name in characters including '\0'

#endif /* ENGINE_TYPE_H */
