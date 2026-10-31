/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train_cmd.cpp Handling of trains. */

#include "stdafx.h"
#include "error.h"
#include "articulated_vehicles.h"
#include "command_func.h"
#include "error_func.h"
#include "pathfinder/yapf/yapf.hpp"
#include "news_func.h"
#include "company_func.h"
#include "newgrf_sound.h"
#include "newgrf_text.h"
#include "strings_func.h"
#include "viewport_func.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "newgrf_station.h"
#include "effectvehicle_func.h"
#include "network/network.h"
#include "core/random_func.hpp"
#include "company_base.h"
#include "newgrf.h"
#include "order_backup.h"
#include "zoom_func.h"
#include "newgrf_debug.h"
#include "framerate_type.h"
#include "train_cmd.h"
#include "misc_cmd.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"

#include "table/strings.h"
#include "table/train_sprites.h"

#include "rust/train_ffi.h"
#include "rust/services_map.h"
#include "rust/train_reservation_ffi.h"
#include "rust/vehicle_services.h"
#include "rust/vehicle_refit_ffi.h"
#include "rust/command_cost_services.h"
#include "rust/engine_services.h"
#include "rust/settings_services.h"
OpenTTDTrainHandle TrainHandle(const Train *v) noexcept
{
	return {const_cast<Train *>(v), v == nullptr ? nullptr : v->GetRustState()};
}
static_assert(sizeof(Order) == sizeof(OpenTTDOrderFields) && alignof(Order) == alignof(OpenTTDOrderFields));
static_assert(std::is_trivially_copyable_v<Order> && std::is_trivially_destructible_v<Order>);


#include "rust/vehicle_effect_ffi.h"
#include "rust/effect_ffi.h"
#include "safeguards.h"

static Track ChooseTrainTrack(Train *v, TileIndex tile, DiagDirection enterdir, TrackBits tracks, bool force_res, bool *got_reservation, bool mark_stuck);
bool TrainController(Train *v, Vehicle *nomove, bool reverse = true); // Also used in vehicle_sl.cpp.
static void CheckIfTrainNeedsService(Train *v);
static void CheckNextTrainTile(Train *v);

static const uint8_t _vehicle_initial_x_fract[4] = {10, 8, 4,  8};
static const uint8_t _vehicle_initial_y_fract[4] = { 8, 4, 8, 10};

template <>
bool IsValidImageIndex<VEH_TRAIN>(uint8_t image_index)
{
	return image_index < lengthof(_engine_sprite_base);
}


/**
 * Return the cargo weight multiplier to use for a rail vehicle
 * @param cargo Cargo type to get multiplier for
 * @return Cargo weight multiplier
 */
uint8_t FreightWagonMult(CargoType cargo)
{
	if (!CargoSpec::Get(cargo)->is_freight) return 1;
	return _settings_game.vehicle.freight_trains;
}

/** Checks if lengths of all rail vehicles are valid. If not, shows an error message. */
void CheckTrainsLengths()
{
	bool first = true;

	for (const Train *v : Train::Iterate()) {
		if (v->First() == v && !v->vehstatus.Test(VehState::Crashed)) {
			for (const Train *u = v, *w = v->Next(); w != nullptr; u = w, w = w->Next()) {
				if (u->GetTrack() != TRACK_BIT_DEPOT) {
					if ((w->GetTrack() != TRACK_BIT_DEPOT &&
							std::max(abs(u->x_pos - w->x_pos), abs(u->y_pos - w->y_pos)) != u->CalcNextVehicleOffset()) ||
							(w->GetTrack() == TRACK_BIT_DEPOT && TicksToLeaveDepot(u) <= 0)) {
						ShowErrorMessage(GetEncodedString(STR_BROKEN_VEHICLE_LENGTH, v->index, v->owner), {}, WL_CRITICAL);

						if (!_networking && first) {
							first = false;
							Command<CMD_PAUSE>::Post(PauseMode::Error, true);
						}
						/* Break so we warn only once for each train. */
						break;
					}
				}
			}
		}
	}
}

/**
 * Recalculates the cached stuff of a train. Should be called each time a vehicle is added
 * to/removed from the chain, and when the game is loaded.
 * Note: this needs to be called too for 'wagon chains' (in the depot, without an engine)
 * @param allowed_changes Stuff that is allowed to change.
 */
void Train::ConsistChanged(ConsistChangeFlags allowed_changes)
{
	openttd_rust_train_consist_changed(TrainHandle(this), static_cast<uint8_t>(allowed_changes.base()), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Get the stop location of (the center) of the front vehicle of a train at
 * a platform of a station.
 * @param station_id     the ID of the station where we're stopping
 * @param tile           the tile where the vehicle currently is
 * @param v              the vehicle to get the stop location of
 * @param station_ahead  'return' the amount of 1/16th tiles in front of the train
 * @param station_length 'return' the station length in 1/16th tiles
 * @return the location, calculated from the begin of the station to stop at.
 */


/**
 * Computes train speed limit caused by curves
 * @return imposed speed limit
 */
uint16_t Train::GetCurveSpeedLimit() const
{
	return static_cast<uint16_t>(openttd_rust_train_curve_limit(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices()));
}

/**
 * Calculates the maximum speed of the vehicle under its current conditions.
 * @return Maximum speed of the vehicle.
 */
int Train::GetCurrentMaxSpeed() const
{
	return static_cast<int>(openttd_rust_train_current_max_speed(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices()));
}

/** Update acceleration of the train from the cached power and weight. */
void Train::UpdateAcceleration()
{
	openttd_rust_train_update_acceleration(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices());
}

int Train::GetCursorImageOffset() const
{
	if (this->gcache.cached_veh_length != 8 && this->GetTrainFlags().Test(VehicleRailFlag::Flipped) && !EngInfo(this->engine_type)->misc_flags.Test(EngineMiscFlag::RailFlips)) {
		int reference_width = TRAININFO_DEFAULT_VEHICLE_WIDTH;

		const Engine *e = this->GetEngine();
		if (e->GetGRF() != nullptr && IsCustomVehicleSpriteNum(e->VehInfo<RailVehicleInfo>().image_index)) {
			reference_width = e->GetGRF()->traininfo_vehicle_width;
		}

		return ScaleSpriteTrad((this->gcache.cached_veh_length - (int)VEHICLE_LENGTH) * reference_width / (int)VEHICLE_LENGTH);
	}
	return 0;
}

/**
 * Get the width of a train vehicle image in the GUI.
 * @param offset Additional offset for positioning the sprite; set to nullptr if not needed
 * @return Width in pixels
 */
int Train::GetDisplayImageWidth(Point *offset) const
{
	int reference_width = TRAININFO_DEFAULT_VEHICLE_WIDTH;
	int vehicle_pitch = 0;

	const Engine *e = this->GetEngine();
	if (e->GetGRF() != nullptr && IsCustomVehicleSpriteNum(e->VehInfo<RailVehicleInfo>().image_index)) {
		reference_width = e->GetGRF()->traininfo_vehicle_width;
		vehicle_pitch = e->GetGRF()->traininfo_vehicle_pitch;
	}

	if (offset != nullptr) {
		if (this->GetTrainFlags().Test(VehicleRailFlag::Flipped) && !EngInfo(this->engine_type)->misc_flags.Test(EngineMiscFlag::RailFlips)) {
			offset->x = ScaleSpriteTrad(((int)this->gcache.cached_veh_length - (int)VEHICLE_LENGTH / 2) * reference_width / (int)VEHICLE_LENGTH);
		} else {
			offset->x = ScaleSpriteTrad(reference_width) / 2;
		}
		offset->y = ScaleSpriteTrad(vehicle_pitch);
	}
	return ScaleSpriteTrad(this->gcache.cached_veh_length * reference_width / VEHICLE_LENGTH);
}

static SpriteID GetDefaultTrainSprite(uint8_t spritenum, Direction direction)
{
	assert(IsValidImageIndex<VEH_TRAIN>(spritenum));
	return ((direction + _engine_sprite_add[spritenum]) & _engine_sprite_and[spritenum]) + _engine_sprite_base[spritenum];
}

/**
 * Get the sprite to display the train.
 * @param direction Direction of view/travel.
 * @param image_type Visualisation context.
 * @return Sprite to display.
 */
void Train::GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const
{
	uint8_t spritenum = this->spritenum;

	if (this->GetTrainFlags().Test(VehicleRailFlag::Flipped)) direction = ReverseDir(direction);

	if (IsCustomVehicleSpriteNum(spritenum)) {
		if (spritenum == CUSTOM_VEHICLE_SPRITENUM_REVERSED) direction = ReverseDir(direction);
		GetCustomVehicleSprite(this, direction, image_type, result);
		if (result->IsValid()) return;

		spritenum = this->GetEngine()->original_image_index;
	}

	assert(IsValidImageIndex<VEH_TRAIN>(spritenum));
	SpriteID sprite = GetDefaultTrainSprite(spritenum, direction);

	if (this->cargo.StoredCount() >= this->cargo_cap / 2U) sprite += _wagon_full_adder[spritenum];

	result->Set(sprite);
}

static void GetRailIcon(EngineID engine, bool rear_head, int &y, EngineImageType image_type, VehicleSpriteSeq *result)
{
	const Engine *e = Engine::Get(engine);
	Direction dir = rear_head ? DIR_E : DIR_W;
	uint8_t spritenum = e->VehInfo<RailVehicleInfo>().image_index;

	if (IsCustomVehicleSpriteNum(spritenum)) {
		GetCustomVehicleIcon(engine, dir, image_type, result);
		if (result->IsValid()) {
			if (e->GetGRF() != nullptr) {
				y += ScaleSpriteTrad(e->GetGRF()->traininfo_vehicle_pitch);
			}
			return;
		}

		spritenum = Engine::Get(engine)->original_image_index;
	}

	if (rear_head) spritenum++;

	result->Set(GetDefaultTrainSprite(spritenum, DIR_W));
}

void DrawTrainEngine(int left, int right, int preferred_x, int y, EngineID engine, PaletteID pal, EngineImageType image_type)
{
	if (RailVehInfo(engine)->railveh_type == RAILVEH_MULTIHEAD) {
		int yf = y;
		int yr = y;

		VehicleSpriteSeq seqf, seqr;
		GetRailIcon(engine, false, yf, image_type, &seqf);
		GetRailIcon(engine, true, yr, image_type, &seqr);

		Rect rectf, rectr;
		seqf.GetBounds(&rectf);
		seqr.GetBounds(&rectr);

		preferred_x = Clamp(preferred_x,
				left - UnScaleGUI(rectf.left) + ScaleSpriteTrad(14),
				right - UnScaleGUI(rectr.right) - ScaleSpriteTrad(15));

		seqf.Draw(preferred_x - ScaleSpriteTrad(14), yf, pal, pal == PALETTE_CRASH);
		seqr.Draw(preferred_x + ScaleSpriteTrad(15), yr, pal, pal == PALETTE_CRASH);
	} else {
		VehicleSpriteSeq seq;
		GetRailIcon(engine, false, y, image_type, &seq);

		Rect rect;
		seq.GetBounds(&rect);
		preferred_x = Clamp(preferred_x,
				left - UnScaleGUI(rect.left),
				right - UnScaleGUI(rect.right));

		seq.Draw(preferred_x, y, pal, pal == PALETTE_CRASH);
	}
}

/**
 * Get the size of the sprite of a train sprite heading west, or both heads (used for lists).
 * @param engine The engine to get the sprite from.
 * @param[out] width The width of the sprite.
 * @param[out] height The height of the sprite.
 * @param[out] xoffs Number of pixels to shift the sprite to the right.
 * @param[out] yoffs Number of pixels to shift the sprite downwards.
 * @param image_type Context the sprite is used in.
 */
void GetTrainSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type)
{
	int y = 0;

	VehicleSpriteSeq seq;
	GetRailIcon(engine, false, y, image_type, &seq);

	Rect rect;
	seq.GetBounds(&rect);

	width  = UnScaleGUI(rect.Width());
	height = UnScaleGUI(rect.Height());
	xoffs  = UnScaleGUI(rect.left);
	yoffs  = UnScaleGUI(rect.top);

	if (RailVehInfo(engine)->railveh_type == RAILVEH_MULTIHEAD) {
		GetRailIcon(engine, true, y, image_type, &seq);
		seq.GetBounds(&rect);

		/* Calculate values relative to an imaginary center between the two sprites. */
		width = ScaleSpriteTrad(TRAININFO_DEFAULT_VEHICLE_WIDTH) + UnScaleGUI(rect.right) - xoffs;
		height = std::max<uint>(height, UnScaleGUI(rect.Height()));
		xoffs  = xoffs - ScaleSpriteTrad(TRAININFO_DEFAULT_VEHICLE_WIDTH) / 2;
		yoffs  = std::min(yoffs, UnScaleGUI(rect.top));
	}
}


/**
 * Move a rail vehicle around inside the depot.
 * @param flags type of operation
 *              Note: DoCommandFlag::AutoReplace is set when autoreplace tries to undo its modifications or moves vehicles to temporary locations inside the depot.
 * @param src_veh source vehicle index
 * @param dest_veh what wagon to put the source wagon AFTER, XXX - VehicleID::Invalid() to make a new line
 * @param move_chain move all vehicles following the source vehicle
 * @return the cost of this operation or an error
 */
CommandCost CmdMoveRailVehicle(DoCommandFlags flags, VehicleID src_veh, VehicleID dest_veh, bool move_chain)
{
	CommandCost cost;
	openttd_rust_train_move(&GetRustVehicleBuildServices(), RustCommandCost(&cost), flags.base(), src_veh.base(), dest_veh.base(), move_chain);
	return cost;
}


void Train::UpdateDeltaXY()
{
	/* Set common defaults. */
	this->bounds = {{-1, -1, 0}, {3, 3, 6}, {}};

	/* Set if flipped and engine is NOT flagged with custom flip handling. */
	int flipped = this->GetTrainFlags().Test(VehicleRailFlag::Flipped) && !EngInfo(this->engine_type)->misc_flags.Test(EngineMiscFlag::RailFlips);
	/* If flipped and vehicle length is odd, we need to adjust the bounding box offset slightly. */
	int flip_offs = flipped && (this->gcache.cached_veh_length & 1);

	Direction dir = this->direction;
	if (flipped) dir = ReverseDir(dir);

	if (!IsDiagonalDirection(dir)) {
		static const Point _sign_table[] = {
			/* x, y */
			{-1, -1}, // DIR_N
			{-1,  1}, // DIR_E
			{ 1,  1}, // DIR_S
			{ 1, -1}, // DIR_W
		};

		int half_shorten = (VEHICLE_LENGTH - this->gcache.cached_veh_length + flipped) / 2;

		/* For all straight directions, move the bound box to the centre of the vehicle, but keep the size. */
		this->bounds.offset.x -= half_shorten * _sign_table[DirToDiagDir(dir)].x;
		this->bounds.offset.y -= half_shorten * _sign_table[DirToDiagDir(dir)].y;
	} else {
		switch (dir) {
				/* Shorten southern corner of the bounding box according the vehicle length
				 * and center the bounding box on the vehicle. */
			case DIR_NE:
				this->bounds.origin.x = -(this->gcache.cached_veh_length + 1) / 2 + flip_offs;
				this->bounds.extent.x = this->gcache.cached_veh_length;
				this->bounds.offset.x = 1;
				break;

			case DIR_NW:
				this->bounds.origin.y = -(this->gcache.cached_veh_length + 1) / 2 + flip_offs;
				this->bounds.extent.y = this->gcache.cached_veh_length;
				this->bounds.offset.y = 1;
				break;

				/* Move northern corner of the bounding box down according to vehicle length
				 * and center the bounding box on the vehicle. */
			case DIR_SW:
				this->bounds.origin.x = -(this->gcache.cached_veh_length) / 2 - flip_offs;
				this->bounds.extent.x = this->gcache.cached_veh_length;
				this->bounds.offset.x = 1 - (VEHICLE_LENGTH - this->gcache.cached_veh_length);
				break;

			case DIR_SE:
				this->bounds.origin.y = -(this->gcache.cached_veh_length) / 2 - flip_offs;
				this->bounds.extent.y = this->gcache.cached_veh_length;
				this->bounds.offset.y = 1 - (VEHICLE_LENGTH - this->gcache.cached_veh_length);
				break;

			default:
				NOT_REACHED();
		}
	}
}

/**
 * Mark a train as stuck and stop it if it isn't stopped right now.
 * @param v %Train to mark as being stuck.
 */
static void MarkTrainAsStuck(Train *v)
{
	openttd_rust_train_mark_stuck(TrainHandle(v), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Swap the two up/down flags in two ways:
 * - Swap values of \a swap_flag1 and \a swap_flag2, and
 * - If going up previously (#GVF_GOINGUP_BIT set), the #GVF_GOINGDOWN_BIT is set, and vice versa.
 * @param[in,out] swap_flag1 First train flag.
 * @param[in,out] swap_flag2 Second train flag.
 */

/**
 * Updates some variables after swapping the vehicle.
 * @param v swapped vehicle
 */

/**
 * Swap vehicles \a l and \a r in consist \a v, and reverse their direction.
 * @param v Consist to change.
 * @param l %Vehicle index in the consist of the first vehicle.
 * @param r %Vehicle index in the consist of the second vehicle.
 */
void ReverseTrainSwapVeh(Train *v, int l, int r)
{
	openttd_rust_train_reverse_swap(TrainHandle(v), static_cast<int32_t>(l), static_cast<int32_t>(r), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Check if the vehicle is a train
 * @param v vehicle on tile
 * @return true if v is a train
 */

/**
 * Check if a level crossing tile has a train on it
 * @param tile tile to test
 * @return true if a train is on the crossing
 * @pre tile is a level crossing
 */
bool TrainOnCrossing(TileIndex tile)
{
	return openttd_rust_train_train_on_tile(static_cast<uint32_t>(tile.base()), &GetTrainServices(), &GetRustSharedServices()) != 0;
}

/**
 * Checks if a train is approaching a rail-road crossing
 * @param v vehicle on tile
 * @param tile tile with crossing we are testing
 * @return true if v is approaching a crossing
 */


/**
 * Finds a vehicle approaching rail-road crossing
 * @param tile tile to test
 * @return true if a vehicle is approaching the crossing
 * @pre tile is a rail-road crossing
 */

/**
 * Check if a level crossing should be barred.
 * @param tile The tile to check.
 * @return True if the crossing should be barred, else false.
 */

/**
 * Sets a level crossing tile to the correct state.
 * @param tile Tile to update.
 * @param sound Should we play sound?
 * @param force_barred Should we set the crossing to barred?
 * @pre tile is a rail-road crossing.
 */

/**
 * Update a level crossing to barred or open (crossing may include multiple adjacent tiles).
 * @param tile Tile which causes the update.
 * @param sound Should we play sound?
 * @param force_bar Should we force the crossing to be barred?
 */
void UpdateLevelCrossing(TileIndex tile, bool sound, bool force_bar)
{
	openttd_rust_train_update_crossing(static_cast<uint32_t>(tile.base()), static_cast<uint8_t>(sound), static_cast<uint8_t>(force_bar), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Find adjacent level crossing tiles in this multi-track crossing and mark them dirty.
 * @param tile The tile which causes the update.
 * @param road_axis The road axis.
 */
void MarkDirtyAdjacentLevelCrossingTiles(TileIndex tile, Axis road_axis)
{
	openttd_rust_train_adjacent_crossing_dirty(static_cast<uint32_t>(tile.base()), static_cast<uint8_t>(road_axis), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Update adjacent level crossing tiles in this multi-track crossing, due to removal of a level crossing tile.
 * @param tile The crossing tile which has been or is about to be removed, and which caused the update.
 * @param road_axis The road axis.
 */
void UpdateAdjacentLevelCrossingTilesOnLevelCrossingRemoval(TileIndex tile, Axis road_axis)
{
	openttd_rust_train_crossing_removed(static_cast<uint32_t>(tile.base()), static_cast<uint8_t>(road_axis), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Bars crossing and plays ding-ding sound if not barred already
 * @param tile tile with crossing
 * @pre tile is a rail-road crossing
 */


/**
 * Advances wagons for train reversing, needed for variable length wagons.
 * This one is called before the train is reversed.
 * @param v First vehicle in chain
 */


/**
 * Advances wagons for train reversing, needed for variable length wagons.
 * This one is called after the train is reversed.
 * @param v First vehicle in chain
 */


/**
 * Turn a train around.
 * @param v %Train to turn around.
 */
void ReverseTrainDirection(Train *v)
{
	openttd_rust_train_reverse(TrainHandle(v), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Reverse train.
 * @param flags type of operation
 * @param veh_id train to reverse
 * @param reverse_single_veh if true, reverse a unit in a train (needs to be in a depot)
 * @return the cost of this operation or an error
 */
CommandCost CmdReverseTrainDirection(DoCommandFlags flags, VehicleID veh_id, bool reverse_single_veh)
{
	Train *v = Train::GetIfValid(veh_id);
	if (v == nullptr) return CMD_ERROR;
	CommandCost ret = CheckOwnership(v->owner);
	if (ret.Failed()) return ret;
	switch (openttd_rust_train_reverse_command(TrainHandle(v), static_cast<uint8_t>(flags.Test(DoCommandFlag::Execute)), static_cast<uint8_t>(reverse_single_veh), &GetTrainServices(), &GetRustSharedServices())) {
		case 0: return CommandCost();
		case 1: return CommandCost(STR_ERROR_CAN_T_REVERSE_DIRECTION_RAIL_VEHICLE_MULTIPLE_UNITS);
		case 2: return CommandCost(STR_ERROR_TRAINS_CAN_ONLY_BE_ALTERED_INSIDE_A_DEPOT);
		default: return CMD_ERROR;
	}
}

/**
 * Determine to what force_proceed should be changed.
 * If we are forced to proceed, cancel that order.
 * If we are marked stuck we would want to force the train to
 * proceed to the next signal unless we are stuck just before
 * the next signal. In the other cases we would like to pass
 * the signal at danger and run till the next signal we encounter.
 * @param t The train to determine the new value of force_proceed for.
 * @return The next state of force_proceed.
 */

/**
 * Force a train through a red signal
 * @param flags type of operation
 * @param veh_id train to ignore the red signal
 * @return the cost of this operation or an error
 */
CommandCost CmdForceTrainProceed(DoCommandFlags flags, VehicleID veh_id)
{
	Train *t = Train::GetIfValid(veh_id);
	if (t == nullptr) return CMD_ERROR;
	/* Preserve primary validation before the ownership shared service. */
	if (!t->IsPrimaryVehicle()) return CMD_ERROR;
	CommandCost ret = CheckOwnership(t->owner);
	if (ret.Failed()) return ret;
	openttd_rust_train_force_command(TrainHandle(t), static_cast<uint8_t>(flags.Test(DoCommandFlag::Execute)), &GetTrainServices(), &GetRustSharedServices());
	return CommandCost();
}

/**
 * Try to find a depot nearby.
 * @param v %Train that wants a depot.
 * @param max_distance Maximal search distance.
 * @return Information where the closest train depot is located.
 * @pre The given vehicle must not be crashed!
 */
static FindDepotData FindClosestTrainDepot(Train *v, int max_distance)
{
	assert(!v->vehstatus.Test(VehState::Crashed));

	return YapfTrainFindNearestDepot(v, max_distance);
}

ClosestDepot Train::FindClosestDepot()
{
	FindDepotData tfdd = FindClosestTrainDepot(this, 0);
	if (tfdd.best_length == UINT_MAX) return ClosestDepot();

	return ClosestDepot(tfdd.tile, GetDepotIndex(tfdd.tile), tfdd.reverse);
}

/** Play a sound for a train leaving the station. */
void Train::PlayLeaveStationSound(bool force) const
{
	static const SoundFx sfx[] = {
		SND_04_DEPARTURE_STEAM,
		SND_0A_DEPARTURE_TRAIN,
		SND_0A_DEPARTURE_TRAIN,
		SND_47_DEPARTURE_MONORAIL,
		SND_41_DEPARTURE_MAGLEV
	};

	if (PlayVehicleSound(this, VSE_START, force)) return;

	SndPlayVehicleFx(sfx[RailVehInfo(this->engine_type)->engclass], this);
}

/**
 * Check if the train is on the last reserved tile and try to extend the path then.
 * @param v %Train that needs its path extended.
 */
static void CheckNextTrainTile(Train *v)
{
	openttd_rust_train_reservation_check_next(TrainHandle(v), RustVehicle(v), &GetTrainReservationServices(), nullptr);
}

/**
 * Will the train stay in the depot the next tick?
 * @param v %Train to check.
 * @return True if it stays in the depot, false otherwise.
 */

/**
 * Clear the reservation of \a tile that was just left by a wagon on \a track_dir.
 * @param v %Train owning the reservation.
 * @param tile Tile with reservation to clear.
 * @param track_dir Track direction to clear.
 */
static void ClearPathReservation(const Train *v, TileIndex tile, Trackdir track_dir)
{
	openttd_rust_train_reservation_clear(TrainHandle(v), RustVehicle(v), static_cast<uint32_t>(tile.base()), static_cast<uint8_t>(track_dir), &GetTrainReservationServices(), nullptr);
}

/**
 * Free the reserved path in front of a vehicle.
 * @param v %Train owning the reserved path.
 */
void FreeTrainTrackReservation(const Train *v)
{
	openttd_rust_train_reservation_free(TrainHandle(v), RustVehicle(v), &GetTrainReservationServices(), nullptr);
}


/**
 * Perform pathfinding for a train.
 *
 * @param v The train
 * @param tile The tile the train is about to enter
 * @param enterdir Diagonal direction the train is coming from
 * @param tracks Usable tracks on the new tile
 * @param[out] path_found Whether a path has been found or not.
 * @param do_track_reservation Path reservation is requested
 * @param[out] dest State and destination of the requested path
 * @param[out] final_dest Final tile of the best path found
 * @return The best track the train should follow
 */

/**
 * Extend a train path as far as possible. Stops on encountering a safe tile,
 * another reservation or a track choice.
 * @return INVALID_TILE indicates that the reservation failed.
 */

/**
 * Try to reserve any path to a safe tile, ignoring the vehicle's destination.
 * Safe tiles are tiles in front of a signal, depots and station tiles at end of line.
 *
 * @param v The vehicle.
 * @param tile The tile the search should start from.
 * @param td The trackdir the search should start from.
 * @param override_railtype Whether all physically compatible railtypes should be followed.
 * @return True if a path to a safe stopping tile could be reserved.
 */

/** This class will save the current order of a vehicle and restore it on destruction. */

/* choose a track */
static Track ChooseTrainTrack(Train *v, TileIndex tile, DiagDirection enterdir, TrackBits tracks, bool force_res, bool *got_reservation, bool mark_stuck)
{
	OpenTTDTrainReservationChoice result = openttd_rust_train_reservation_choose(TrainHandle(v), RustVehicle(v), static_cast<uint32_t>(tile.base()), static_cast<uint8_t>(enterdir), static_cast<uint8_t>(tracks), static_cast<uint8_t>(force_res), static_cast<uint8_t>(mark_stuck), &GetTrainReservationServices(), nullptr);
	if (got_reservation != nullptr) *got_reservation = result.reserved != 0;
	return static_cast<Track>(result.track);
}

/**
 * Try to reserve a path to a safe position.
 *
 * @param v The vehicle
 * @param mark_as_stuck Should the train be marked as stuck on a failed reservation?
 * @param first_tile_okay True if no path should be reserved if the current tile is a safe position.
 * @return True if a path could be reserved.
 */
bool TryPathReserve(Train *v, bool mark_as_stuck, bool first_tile_okay)
{
	return (openttd_rust_train_reservation_try_path(TrainHandle(v), RustVehicle(v), static_cast<uint8_t>(mark_as_stuck), static_cast<uint8_t>(first_tile_okay), &GetTrainReservationServices(), nullptr)) != 0;
}


static bool CheckReverseTrain(const Train *v)
{
	return (openttd_rust_train_reservation_check_reverse(TrainHandle(v), RustVehicle(v), &GetTrainReservationServices(), nullptr)) != 0;
}

/**
 * Get the location of the next station to visit.
 * @param station Next station to visit.
 * @return Location of the new station.
 */
TileIndex Train::GetOrderStationLocation(StationID station)
{
	return TileIndex(static_cast<uint32_t>(openttd_rust_train_reservation_station_location(TrainHandle(this), RustVehicle(this), static_cast<uint16_t>(station.base()), &GetTrainReservationServices(), nullptr)));
}

/** Goods at the consist have changed, update the graphics, cargo, and acceleration. */
void Train::MarkDirty()
{
	openttd_rust_train_mark_dirty(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * This function looks at the vehicle and updates its speed (cur_speed
 * and subspeed) variables. Furthermore, it returns the distance that
 * the train can drive this tick. #Vehicle::GetAdvanceDistance() determines
 * the distance to drive before moving a step on the map.
 * @return distance to drive.
 */
int Train::UpdateSpeed()
{
	return static_cast<int>(openttd_rust_train_update_speed(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices()));
}

/**
 * Trains enters a station, send out a news item if it is the first train, and start loading.
 * @param v Train that entered the station.
 * @param station Station visited.
 */

/* Check if the vehicle is compatible with the specified tile */

/** Data structure for storing engine speed changes of an acceleration type. */

/** Speed update fractions for each acceleration type. */

/**
 * Modify the speed of the vehicle due to a change in altitude.
 * @param v %Train to update.
 * @param old_z Previous height.
 */


/** Tries to reserve track under whole train consist. */
void Train::ReserveTrackUnderConsist() const
{
	openttd_rust_train_reservation_reserve_under(TrainHandle(this), RustVehicle(this), &GetTrainReservationServices(), nullptr);
}

/**
 * The train vehicle crashed!
 * Update its status and other parts around it.
 * @param flooded Crash was caused by flooding.
 * @return Number of people killed.
 */
uint Train::Crash(bool flooded)
{
	return static_cast<uint>(openttd_rust_train_crash(TrainHandle(this), static_cast<uint8_t>(flooded), &GetTrainServices(), &GetRustSharedServices()));
}

/**
 * Marks train as crashed and creates an AI event.
 * Doesn't do anything if the train is crashed already.
 * @param v first vehicle of chain
 * @return number of victims (including 2 drivers; zero if train was already crashed)
 */

/**
 * Collision test function.
 * @param v %Train vehicle to test collision with.
 * @param t %Train being examined.
 * @return Number of victims.
 */

/**
 * Checks whether the specified train has a collision with another vehicle. If
 * so, destroys this vehicle, and the other vehicle if its subtype has TS_Front.
 * Reports the incident in a flashy news item, modifies station ratings and
 * plays a sound.
 * @param v %Train to test.
 */

/**
 * Move a vehicle chain one movement stop forwards.
 * @param v First vehicle to move.
 * @param nomove Stop moving this and all following vehicles.
 * @param reverse Set to false to not execute the vehicle reversing. This does not change any other logic.
 * @return True if the vehicle could be moved forward, false otherwise.
 */
bool TrainController(Train *v, Vehicle *nomove, bool reverse)
{
	return openttd_rust_train_controller(TrainHandle(v), OpenTTDTrainHandle{nomove, nullptr}, static_cast<uint8_t>(reverse), &GetTrainServices(), &GetRustSharedServices()) != 0;
}


/**
 * Deletes/Clears the last wagon of a crashed train. It takes the engine of the
 * train, then goes to the last wagon and deletes that. Each call to this function
 * will remove the last wagon of a crashed train. If this wagon was on a crossing,
 * or inside a tunnel/bridge, recalculate the signals as they might need updating
 * @param v the Vehicle of which last wagon is to be removed
 */

/**
 * Rotate all vehicles of a (crashed) train chain randomly to animate the crash.
 * @param v First crashed vehicle.
 */

/**
 * Handle a crashed train.
 * @param v First train vehicle.
 * @return %Vehicle chain still exists.
 */

/** Maximum speeds for train that is broken down or approaching line end */


/**
 * Train is approaching line end, slow down and possibly reverse
 *
 * @param v front train engine
 * @param signal not line end, just a red signal
 * @param reverse Set to false to not execute the vehicle reversing. This does not change any other logic.
 * @return true iff we did NOT have to reverse
 */


/**
 * Determines whether train would like to leave the tile
 * @param v train to test
 * @return true iff vehicle is NOT entering or inside a depot or tunnel/bridge
 */


/**
 * Determines whether train is approaching a rail-road crossing
 *   (thus making it barred)
 * @param v front engine of train
 * @return TileIndex of crossing the train is approaching, else INVALID_TILE
 * @pre v in non-crashed front engine
 */


/**
 * Checks for line end. Also, bars crossing at next tile if needed
 *
 * @param v vehicle we are checking
 * @param reverse Set to false to not execute the vehicle reversing. This does not change any other logic.
 * @return true iff we did NOT have to reverse
 */



/**
 * Get running cost for the train consist.
 * @return Yearly running costs.
 */
Money Train::GetRunningCost() const
{
	return Money(static_cast<int64_t>(openttd_rust_train_running_cost(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices())));
}

/**
 * Update train vehicle data for a tick.
 * @return True if the vehicle still exists, false if it has ceased to exist (front of consists only).
 */
bool Train::Tick()
{
	if (this->IsFrontEngine()) {
		PerformanceAccumulator framerate(PFE_GL_TRAINS);
		return openttd_rust_train_tick(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices()) != 0;
	}
	return openttd_rust_train_tick(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices()) != 0;
}

/**
 * Check whether a train needs service, and if so, find a depot or service it.
 * @return v %Train to check.
 */
static void CheckIfTrainNeedsService(Train *v)
{
	openttd_rust_train_needs_service(TrainHandle(v), &GetTrainServices(), &GetRustSharedServices());
}

/** Calendar day handler. */
void Train::OnNewCalendarDay()
{
	openttd_rust_train_calendar_day(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices());
}

/** Economy day handler. */
void Train::OnNewEconomyDay()
{
	openttd_rust_train_economy_day(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices());
}

/**
 * Get the tracks of the train vehicle.
 * @return Current tracks of the vehicle.
 */
Trackdir Train::GetVehicleTrackdir() const
{
	return static_cast<Trackdir>(openttd_rust_train_trackdir(TrainHandle(this), &GetTrainServices(), &GetRustSharedServices()));
}

uint16_t Train::GetMaxWeight() const
{
	return openttd_rust_ground_weight(&GetRustGroundPhysicsServices(), this, true);
}

#include "rust/train_services.h"

#include "rust/train_reservation_services.h"



const OpenTTDTrainServices &GetTrainServices() noexcept
{
	static const OpenTTDTrainServices services{
		.physics = &GetRustGroundPhysicsServices(),
		.read_first = TrainReadFirst,
		.read_next = TrainReadNext,
		.read_previous = TrainReadPrevious,
		.read_next_unit = TrainReadNextUnit,
		.read_last = TrainReadLast,
		.read_tile = TrainVehicleRead<uint32_t, RustVehicleTile>,
		.read_dest = TrainVehicleRead<uint32_t, RustVehicleDestTile>,
		.read_order_time = TrainReadOrderTime,
		.read_length = TrainVehicleRead<uint16_t, RustGroundCachedLength<Train>>,
		.read_total_length = TrainReadTotalLength,
		.read_max_track_speed = TrainReadMaxTrackSpeed,
		.read_speed = TrainVehicleRead<uint16_t, RustVehicleSpeed>,
		.read_gv_flags = TrainReadGvFlags,
		.read_refit_cap = TrainReadRefitCap,
		.read_last_station = TrainReadLastStation,
		.read_direction = TrainVehicleRead<uint8_t, RustVehicleDirection>,
		.read_status = TrainVehicleRead<uint8_t, RustVehicleStatus>,
		.read_tick = TrainVehicleRead<uint8_t, RustVehicleTick>,
		.read_running = TrainReadRunning,
		.read_day = TrainVehicleRead<uint8_t, RustVehicleDay>,
		.read_progress = TrainVehicleRead<uint8_t, RustVehicleProgress>,
		.read_order = TrainVehicleRead<uint8_t, RustVehicleOrderType>,
		.read_front = TrainReadFront,
		.read_articulated = TrainVehicleRead<uint8_t, RustVehicleArticulated>,
		.read_multiheaded = TrainReadMultiheaded,
		.read_owner = TrainVehicleRead<uint8_t, RustVehicleOwner>,
		.read_vis_effect = TrainReadVisEffect,
		.read_consist_changed = TrainReadConsistChanged,
		.read_consist_changed_1 = TrainReadConsistChanged1,
		.read_consist_changed_2 = TrainReadConsistChanged2,
		.read_curve_limit = TrainReadCurveLimit,
		.read_stop_location = TrainReadStopLocation,
		.read_current_max_speed = TrainReadCurrentMaxSpeed,
		.read_update_acceleration = TrainReadUpdateAcceleration,
		.read_update_speed = TrainReadUpdateSpeed,
		.read_trackdir = TrainReadTrackdir,
		.read_can_leave = TrainReadCanLeave,
		.read_crossing_approach = TrainReadCrossingApproach,
		.read_next_offset = TrainReadNextOffset,
		.read_after_swap = TrainReadAfterSwap,
		.read_reverse_swap = TrainReadReverseSwap,
		.read_approaching_end = TrainReadApproachingEnd,
		.read_line_ends = TrainReadLineEnds,
		.read_speed_z = TrainReadSpeedZ,
		.read_move_vehicle = TrainReadMoveVehicle,
		.read_move_vehicle_20 = TrainReadMoveVehicle20,
		.read_collision_one = TrainReadCollisionOne,
		.read_collision_one_22 = TrainReadCollisionOne22,
		.read_delete_last = TrainReadDeleteLast,
		.read_stay_depot = TrainReadStayDepot,
		.read_loco = TrainReadLoco,
		.read_loco_26 = TrainReadLoco26,
		.read_tick_state = TrainReadTickState,
		.read_needs_service = TrainReadNeedsService,
		.read_next_force = TrainReadNextForce,
		.read_reverse_command = TrainReadReverseCommand,
		.write_tile = TrainWriteTile,
		.write_dest = TrainWriteDest,
		.write_x = TrainWriteX,
		.write_y = TrainWriteY,
		.write_z = TrainWriteZ,
		.write_direction = TrainWriteDirection,
		.write_speed = TrainWriteSpeed,
		.write_tick = TrainWriteTick,
		.write_running = TrainWriteRunning,
		.write_day = TrainWriteDay,
		.write_order_time = TrainWriteOrderTime,
		.write_progress = TrainWriteProgress,
		.write_subspeed = TrainWriteSubspeed,
		.write_gv_flags = TrainWriteGvFlags,
		.write_acceleration = TrainWriteAcceleration,
		.write_length = TrainWriteLength,
		.write_total_length = TrainWriteTotalLength,
		.write_first_engine = TrainWriteFirstEngine,
		.write_max_speed = TrainWriteMaxSpeed,
		.write_cargo_cap = TrainWriteCargoCap,
		.write_refit_cap = TrainWriteRefitCap,
		.write_cargo_age = TrainWriteCargoAge,
		.write_last_station = TrainWriteLastStation,
		.write_colourmap = TrainWriteColourmap,
		.write_status = TrainWriteStatus,
		.acc_model = TrainAccModel,
		.advance_distance = TrainAdvanceDistance,
		.all_powered = TrainAllPowered,
		.ambient_sound = RustAmbientSoundLegacy<OpenTTDTrainHandle>,
		.arrival_news = TrainArrivalNews,
		.axis_diag = TrainAxisDiag,
		.backoff = RustTrainPathBackoffInterval,
		.base_viewport = TrainBaseViewport,
		.bridge_speed = TrainSharedMap<uint64_t, &OpenTTDSharedServices::bridge_speed>,
		.capabilities = &GetRustEngineCapabilitiesServices(),
		.capacity_error = TrainCapacityError,
		.cargo_age_default = TrainCargoAgeDefault,
		.check_next = TrainCheckNext,
		.check_orders = TrainCheckOrders,
		.check_reverse = TrainCheckReverse,
		.choose_track = TrainChooseTrack,
		.clear_reservation = TrainClearReservation,
		.consist_windows = TrainConsistWindows,
		.cost_divisor = TrainCostDivisor,
		.count_chain = TrainCountChain,
		.crash_event = TrainCrashEvent,
		.crash_news = TrainCrashNews,
		.crash_sound = TrainCrashSound,
		.crossing_barred = OpenTTDLegacyMapQuery<uint64_t, RustCrossingBarred, OpenTTDTrainHandle, uint32_t>,
		.crossing_rail_axis = TrainCrossingRailAxis,
		.crossing_road_axis = OpenTTDLegacyMapQuery<uint64_t, RustCrossingRoadAxis, OpenTTDTrainHandle, uint32_t>,
		.crossing_sound = RustCrossingSound<OpenTTDTrainHandle>,
		.curve_advantage = TrainCurveAdvantage,
		.curve_mod_default = TrainCurveModDefault,
		.day_ticks = TrainDayTicks,
		.delete_vehicle = TrainDeleteVehicle,
		.depot_dir = OpenTTDLegacyMapQuery<uint64_t, RustRailDepotDirection, OpenTTDTrainHandle, uint32_t>,
		.depot_dirty = TrainDepotDirty,
		.depot_index = TrainSharedMap<uint64_t, &OpenTTDSharedServices::depot_index>,
		.depot_track = TrainDepotTrack,
		.depot_window = TrainDepotWindow,
		.diag_axis = TrainDiagAxis,
		.diag_between = TrainDiagBetween,
		.diag_reaches_tracks = TrainDiagReachesTracks,
		.diag_trackdir = TrainDiagTrackdir,
		.dirty_tile = TrainSharedMap<void, &OpenTTDSharedServices::mark_dirty>,
		.dir_diag = TrainDirDiag,
		.disconnect = TrainDisconnect,
		.engine_power = TrainEnginePower,
		.find_depot = TrainFindDepot,
		.first_track = TrainFirstTrack,
		.grf_version = TrainGrfVersion,
		.has_depot_res = OpenTTDLegacyMapQuery<uint64_t, RustDepotReserved, OpenTTDTrainHandle, uint32_t>,

		.has_signal = OpenTTDLegacyMapQuery<uint64_t, RustRailHasSignalTrack, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.has_signals = OpenTTDLegacyMapQuery<uint64_t, RustRailHasSignals, OpenTTDTrainHandle, uint32_t>,
		.has_signal_td = OpenTTDLegacyMapQuery<uint64_t, RustRailHasSignalTrackdir, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.hide_fill = TrainHideFill,
		.invalidate_grf = TrainVehicleInvoke<RustVehicleInvalidateNewGRFCache>,
		.is_bridge = TrainIsBridge,
		.is_crossing = TrainSharedMap<uint64_t, &OpenTTDSharedServices::is_level_crossing>,
		.is_depot = OpenTTDLegacyMapQuery<uint64_t, RustRailDepotTile, OpenTTDTrainHandle, uint32_t>,
		.is_plain_rail = OpenTTDLegacyMapQuery<uint64_t, RustPlainRailTile, OpenTTDTrainHandle, uint32_t>,
		.is_station = OpenTTDLegacyMapQuery<uint64_t, RustIsRailStation, OpenTTDTrainHandle, uint32_t>,
		.is_station_any = TrainIsStationAny,
		.leave_sound = TrainLeaveSound,
		.leave_unbunch = TrainLeaveUnbunch,
		.length_callback = TrainLengthCallback,
		.length_default = TrainLengthDefault,
		.local_company = TrainLocalCompany,
		.lost_warn = TrainLostWarn,
		.map_size = TrainMapSize,
		.max_depot_penalty = TrainMaxDepotPenalty,
		.no_90 = OpenTTDLegacyMapQuery<uint64_t, RustDefaultRail90Disallowed, OpenTTDTrainHandle, uint8_t, uint8_t>,
		.oneway_blocking = OpenTTDLegacyMapQuery<uint64_t, RustOnewaySignalBlocking, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.order_depot_service = TrainOrderDepotService,
		.order_dummy = TrainOrderDummy,
		.order_free = TrainOrderFree,
		.order_max_speed = TrainVehicleRead<uint64_t, RustVehicleOrderMaxSpeed>,
		.other_end = OpenTTDLegacyMapQuery<uint64_t, RustOtherTunnelBridgeEnd, OpenTTDTrainHandle, uint32_t>,
		.pay_running = TrainPayRunning,
		.pbs_signal_type = TrainPbsSignalType,
		.pow_wag_power = TrainPowWagPower,
		.railveh_wagon = TrainRailvehWagon,
		.rail_tilt = TrainRailTilt,
		.rail_type = OpenTTDLegacyMapQuery<uint64_t, RustRailType, OpenTTDTrainHandle, uint32_t>,
		.rail_types = TrainRailTypes,
		.reserve_paths = RustTrainReservePaths,
		.reserve_under = TrainReserveUnder,
		.reset_unbunch = TrainResetUnbunch,
		.reverse_at_signals = TrainReverseAtSignals,
		.reverse_single_blocked = TrainReverseSingleBlocked,
		.reverse_windows = TrainReverseWindows,
		.running_windows = TrainRunningWindows,
		.servint = TrainServint,
		.set_depot_res = OpenTTDLegacyMapQuery<void, RustSetDepotReservation, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.set_next = TrainVehiclePair<RustVehicleSetNext>,

		.set_signal_state = OpenTTDLegacyMapQuery<void, RustRailSetSignalState, OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t>,
		.set_tunnel_res = OpenTTDLegacyMapQuery<void, RustSetTunnelBridgeReservation, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.show_reservation = OpenTTDLegacyMapQuery<uint64_t, RustShowTrackReservation, OpenTTDTrainHandle>,
		.signal_context = GetRustSignalContext,
		.signal_has_pbs = OpenTTDLegacyMapQuery<uint64_t, RustHasPbsSignal, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.signal_pbs = TrainSignalPbs,
		.signal_type = OpenTTDLegacyMapQuery<uint64_t, RustRailSignalType, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.sigseg_full = TrainSigsegFull,
		.sigseg_pbs = TrainSigsegPbs,
		.speed_default = TrainSpeedDefault,
		.start_stop_dirty = TrainStartStopDirty,
		.station = TrainSharedMap<uint64_t, &OpenTTDSharedServices::station_index>,
		.station_axis = OpenTTDLegacyMapQuery<uint64_t, RustRailStationAxis, OpenTTDTrainHandle, uint32_t>,
		.station_compatible = RustStationLegacyCompatible<OpenTTDTrainHandle>,
		.station_dest = TrainStationDest,
		.stopped_in_depot = TrainStoppedInDepot,
		.stop_location = TrainStopLocation,
		.stuck_news = TrainStuckNews,
		.suppress_implicit = TrainSuppressImplicit,

		.tile_add_diag = OpenTTDLegacyMapQuery<uint64_t, RustTileAddByDiagDir, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.tile_offset_axis = TrainTileOffsetAxis,
		.tile_offset_diag = TrainTileOffsetDiag,
		.tile_owner = TrainSharedMap<uint64_t, &OpenTTDSharedServices::tile_owner>,
		.tile_rail_type = OpenTTDLegacyMapQuery<uint64_t, RustTileRailType, OpenTTDTrainHandle, uint32_t>,
		.tile_virt = RustTileVirtXY,
		.trackdir_exit = TrainTrackdirExit,
		.trackdir_reaches = TrainTrackdirReaches,
		.track_bits = OpenTTDLegacyMapQuery<uint64_t, RustRailTracks, OpenTTDTrainHandle, uint32_t>,
		.track_crosses = TrainTrackCrosses,
		.track_direction = TrainTrackDirection,

		.train_list = TrainTrainList,
		.train_visit = TrainTrainVisit,
		.truncate_cargo = TrainTruncateCargo,

		.tunnel_dir = TrainSharedMap<uint64_t, &OpenTTDSharedServices::tunnel_bridge_direction>,

		.update_delta = RustVehicleUpdateDelta<OpenTTDTrainHandle>,
		.user_default = TrainUserDefault,
		.veh_exit_dir = TrainVehExitDir,
		.viewport = TrainViewport,
		.visit_type = TrainVisitType,
		.wagon_override = TrainVehicleRead<uint64_t, RustVehicleUsesWagonOverride>,
		.wagon_speed_limits = RustSettingRead<uint64_t, RustWagonSpeedLimits, OpenTTDTrainHandle>,
		.wait_oneway = TrainWaitOneway,
		.wait_pbs = TrainWaitPbs,
		.wait_twoway = TrainWaitTwoway,
		.wait_unbunch = TrainWaitUnbunch,
		.write_crossing_bar = TrainWriteCrossingBar,
		.write_visit_type = TrainWriteVisitType,
		.vehicle = &GetRustVehicleServices(),
	};
	return services;
}
const OpenTTDTrainReservationLeaves &GetTrainReservationServices() noexcept
{
	static const OpenTTDTrainReservationLeaves services{
		.read_tile = TrainVehicleRead<uint32_t, RustVehicleTile>,
		.read_next = TrainReadNext,
		.read_last_station = TrainReadLastStation,
		.read_direction = TrainVehicleRead<uint8_t, RustVehicleDirection>,
		.read_order = TrainVehicleRead<uint8_t, RustVehicleOrderType>,
		.read_num_orders = TrainVehicleRead<uint8_t, RustVehicleNumOrders>,
		.read_order_index = TrainVehicleRead<uint8_t, RustVehicleRealOrderIndex>,
		.read_free = TrainReservationReadFree,
		.read_free_1 = TrainReservationReadFree1,
		.read_new = TrainReservationReadNew,
		.read_choose = TrainReservationReadChoose,
		.read_choose_4 = TrainReservationReadChoose4,
		.read_check_next = TrainReservationReadCheckNext,
		.all_compat = TrainReservationAllCompat,
		.backoff = RustTrainPathBackoffInterval,
		.bits_track = TrainReservationBitsTrack,
		.blocking = OpenTTDLegacyContextMapQuery<uint64_t, RustOnewaySignalBlocking, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.check_reverse = TrainReservationCheckReverse,
		.compat_station = TrainReservationCompatStation,
		.conditional = TrainReservationConditional,
		.copy_order = TrainReservationCopyOrder,
		.cross_dirs = TrainReservationCrossDirs,
		.cross_tracks = TrainReservationCrossTracks,
		.depot_dir = OpenTTDLegacyContextMapQuery<uint64_t, RustRailDepotDirection, OpenTTDTrainHandle, uint32_t>,
		.depot_reserved = OpenTTDLegacyContextMapQuery<uint64_t, RustDepotReserved, OpenTTDTrainHandle, uint32_t>,
		.diag_reach_dirs = TrainReservationDiagReachDirs,
		.diag_track = TrainReservationDiagTrack,
		.enter_td = TrainReservationEnterTd,
		.exit_dir = TrainReservationExitDir,

		.green = OpenTTDLegacyContextMapQuery<uint64_t, RustRailSignalIsGreen, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.has_pbs = OpenTTDLegacyContextMapQuery<uint64_t, RustHasPbsSignal, OpenTTDTrainHandle, uint32_t, uint8_t>,

		.has_signal = OpenTTDLegacyContextMapQuery<uint64_t, RustRailHasSignalTrackdir, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.increment_order = TrainReservationIncrementOrder,
		.is_bridge = OpenTTDLegacyContextMapQuery<uint64_t, RustTunnelBridgeIsBridge, OpenTTDTrainHandle, uint32_t>,
		.is_depot = OpenTTDLegacyContextMapQuery<uint64_t, RustRailDepotTile, OpenTTDTrainHandle, uint32_t>,
		.is_pbs = TrainReservationIsPbs,
		.is_plain = OpenTTDLegacyContextMapQuery<uint64_t, RustMovementPlainRail, OpenTTDTrainHandle, uint32_t>,
		.is_station = OpenTTDLegacyContextMapQuery<uint64_t, RustIsRailStation, OpenTTDTrainHandle, uint32_t>,
		.is_waypoint = TrainReservationIsWaypoint,
		.line_reverse = TrainReservationLineReverse,
		.mark_tile = TrainReservationSharedMap<void, &OpenTTDSharedServices::mark_dirty>,
		.oneway = OpenTTDLegacyContextMapQuery<uint64_t, RustRailOnewaySignal, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.order_service = TrainReservationOrderService,
		.order_type = TrainReservationOrderType,
		.other_end = OpenTTDLegacyContextMapQuery<uint64_t, RustOtherTunnelBridgeEnd, OpenTTDTrainHandle, uint32_t>,
		.overlap = TrainReservationOverlap,
		.path_result = TrainReservationPathResult,
		.rail90 = OpenTTDLegacyContextMapQuery<uint64_t, RustTileRail90Disallowed, OpenTTDTrainHandle, uint32_t, uint32_t>,
		.reach_dirs = TrainReservationReachDirs,
		.reach_tracks = TrainReservationReachTracks,

		.reserve_paths = RustTrainReservePaths,
		.restore_order = TrainReservationRestoreOrder,
		.save_order = TrainReservationSaveOrder,
		.service = TrainReservationService,
		.set_depot = OpenTTDLegacyContextMapQuery<void, RustSetDepotReservation, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.set_depot_dest = TrainReservationSetDepotDest,

		.set_signal = OpenTTDLegacyContextMapQuery<void, RustRailSetSignalGreen, OpenTTDTrainHandle, uint32_t, uint8_t, uint8_t>,
		.set_tunnel = OpenTTDLegacyContextMapQuery<void, RustSetTunnelBridgeReservation, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.show_res = OpenTTDLegacyContextMapQuery<uint64_t, RustShowTrackReservation, OpenTTDTrainHandle>,
		.signal_context = GetRustSignalContext,
		.vehicle = &GetRustVehicleServices(),
		.shared = &GetRustSharedServices(),
		.start_stop = TrainReservationStartStop,
		.station = TrainReservationSharedMap<uint64_t, &OpenTTDSharedServices::station_index>,
		.station_train = TrainReservationStationTrain,
		.station_xy = TrainReservationStationXy,
		.stuck = TrainReservationStuck,
		.tile_add = OpenTTDLegacyContextMapQuery<uint64_t, RustTileAddByDiagDir, OpenTTDTrainHandle, uint32_t, uint8_t>,
		.tile_offset = TrainReservationTileOffset,
		.trackdir = TrainReservationTrackdir,
		.tunnel_dir = TrainReservationSharedMap<uint64_t, &OpenTTDSharedServices::tunnel_bridge_direction>,
		.write_dest = TrainReservationWriteDest,
		.write_last = TrainReservationWriteLast,
		.write_suppress = TrainReservationWriteSuppress,

		.pathfind = TrainReservationPathfind,
		.safe_track = TrainReservationSafeTrack,
		.process_orders = TrainReservationProcessOrders,
		.update_order_dest = TrainReservationUpdateOrderDest
	};
	return services;
}


#include "rust/rail_construction_services.hpp"
#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDTrainServices.hpp"
#include "abi-visit-OpenTTDTrainReservationLeaves.hpp"
#include "abi-visit-OpenTTDRailConstructionServices.hpp"
bool CheckAbi_train_cmd()
{
	return AbiServicesComplete(GetTrainServices()) &&
		AbiServicesComplete(GetTrainReservationServices()) &&
		AbiServicesComplete(GetRustRailConstructionServices());
}
