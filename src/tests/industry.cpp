/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry.cpp Canonical industry slot/history and native ABI checks. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../industry.h"
#include "../effectvehicle_func.h"
#include "../sound_type.h"
#include "abi_layout.hpp"
#include "../rust/industry_generation_adapter.hpp"
#include "../object_base.h"
#include "../industrytype.h"
#include "../water_map.h"
static_assert(EV_CHIMNEY_SMOKE == 0 && EV_BUBBLE == 9 && EV_COPPER_MINE_SMOKE == 11);
static_assert(SND_0B_MINE == 9 && SND_0C_POWER_STATION == 10 && SND_29_SUGAR_MINE_2 == 41);
static_assert(SND_2A_TOY_FACTORY_3 == 42 && SND_2B_TOY_FACTORY_2 == 43 && SND_2C_TOY_FACTORY_1 == 44);
static_assert(SND_2D_SUGAR_MINE_1 == 45 && SND_2E_BUBBLE_GENERATOR == 46 && SND_30_TOFFEE_QUARRY == 48);
static_assert(OWNER_NONE.base() == 16 && INVALID_OWNER.base() == 255);
static_assert(ICT_NORMAL_GAMEPLAY == 1 && ICT_MAP_GENERATION == 2 && ICT_SCENARIO_EDITOR == 3);
static_assert(INDUSTRY_NUM_INPUTS == 16 && INDUSTRY_NUM_OUTPUTS == 16);
static_assert(sizeof(ClearedObjectArea) == sizeof(OpenTTDIndustryClearArea));
static_assert(alignof(ClearedObjectArea) == alignof(OpenTTDIndustryClearArea));
static_assert(offsetof(ClearedObjectArea, first_tile) == offsetof(OpenTTDIndustryClearArea, first_tile));
static_assert(offsetof(ClearedObjectArea, area) == offsetof(OpenTTDIndustryClearArea, area));
static_assert(sizeof(CargoArray::value_type) == sizeof(uint32_t) && sizeof(CargoTypes) == sizeof(uint64_t));
static_assert(std::is_trivially_destructible_v<TileArea>);
static_assert(sizeof(TileArea) == sizeof(OpenTTDIndustryLocation));
static_assert(alignof(TileArea) == alignof(OpenTTDIndustryMetadata));
static_assert(offsetof(TileArea, tile) == offsetof(OpenTTDIndustryMetadata, tile));
static_assert(offsetof(TileArea, w) == offsetof(OpenTTDIndustryMetadata, width));
static_assert(offsetof(TileArea, h) == offsetof(OpenTTDIndustryMetadata, height));
static_assert(sizeof(IndustryID) == sizeof(uint16_t) && alignof(IndustryID) == alignof(uint16_t));
static_assert(sizeof(Owner) == sizeof(uint8_t) && alignof(Owner) == alignof(uint8_t));
static_assert(sizeof(Colours) == sizeof(uint8_t));
static_assert(sizeof(TimerGameCalendar::Date) == sizeof(int32_t));

TEST_CASE("Industry - canonical native layouts")
{
	CHECK(AbiLayoutMismatch(586, {sizeof(OpenTTDIndustryLocationResult), alignof(OpenTTDIndustryLocationResult), offsetof(OpenTTDIndustryLocationResult, callback), offsetof(OpenTTDIndustryLocationResult, registers)}) == -1);
	CHECK(AbiLayoutMismatch(579, {sizeof(OpenTTDIndustryClearArea), alignof(OpenTTDIndustryClearArea), offsetof(OpenTTDIndustryClearArea, first_tile), offsetof(OpenTTDIndustryClearArea, area)}) == -1);
	CHECK(AbiLayoutMismatch(580, {sizeof(OpenTTDIndustryClearAreas), alignof(OpenTTDIndustryClearAreas), offsetof(OpenTTDIndustryClearAreas, data), offsetof(OpenTTDIndustryClearAreas, size)}) == -1);
	CHECK(AbiLayoutMismatch(581, {sizeof(OpenTTDIndustryAllocation), alignof(OpenTTDIndustryAllocation), offsetof(OpenTTDIndustryAllocation, shell), offsetof(OpenTTDIndustryAllocation, owner), offsetof(OpenTTDIndustryAllocation, id)}) == -1);
	CHECK(AbiLayoutMismatch(582, {sizeof(OpenTTDIndustryConflictKinds), alignof(OpenTTDIndustryConflictKinds), offsetof(OpenTTDIndustryConflictKinds, kinds)}) == -1);
	CHECK(AbiLayoutMismatch(583, {sizeof(OpenTTDIndustryConstructionErrors), alignof(OpenTTDIndustryConstructionErrors), offsetof(OpenTTDIndustryConstructionErrors, site), offsetof(OpenTTDIndustryConstructionErrors, forest), offsetof(OpenTTDIndustryConstructionErrors, positioned), offsetof(OpenTTDIndustryConstructionErrors, desert), offsetof(OpenTTDIndustryConstructionErrors, rainforest), offsetof(OpenTTDIndustryConstructionErrors, lowland), offsetof(OpenTTDIndustryConstructionErrors, one_per_town), offsetof(OpenTTDIndustryConstructionErrors, town), offsetof(OpenTTDIndustryConstructionErrors, population), offsetof(OpenTTDIndustryConstructionErrors, near_town), offsetof(OpenTTDIndustryConstructionErrors, conflict), offsetof(OpenTTDIndustryConstructionErrors, too_many)}) == -1);
	CHECK(AbiLayoutMismatch(584, {sizeof(OpenTTDIndustryTransactionServices), alignof(OpenTTDIndustryTransactionServices), offsetof(OpenTTDIndustryTransactionServices, enabled), offsetof(OpenTTDIndustryTransactionServices, layout_count), offsetof(OpenTTDIndustryTransactionServices, layout), offsetof(OpenTTDIndustryTransactionServices, raw), offsetof(OpenTTDIndustryTransactionServices, raw_setting), offsetof(OpenTTDIndustryTransactionServices, check_proc), offsetof(OpenTTDIndustryTransactionServices, conflicting), offsetof(OpenTTDIndustryTransactionServices, multiple_per_town), offsetof(OpenTTDIndustryTransactionServices, closest_town), offsetof(OpenTTDIndustryTransactionServices, town), offsetof(OpenTTDIndustryTransactionServices, population), offsetof(OpenTTDIndustryTransactionServices, town_position), offsetof(OpenTTDIndustryTransactionServices, industry), offsetof(OpenTTDIndustryTransactionServices, highest_snow_line), offsetof(OpenTTDIndustryTransactionServices, edge_distance), offsetof(OpenTTDIndustryTransactionServices, refinery_limit), offsetof(OpenTTDIndustryTransactionServices, tile_height), offsetof(OpenTTDIndustryTransactionServices, has_water_class), offsetof(OpenTTDIndustryTransactionServices, ensure_no_vehicle), offsetof(OpenTTDIndustryTransactionServices, set_company), offsetof(OpenTTDIndustryTransactionServices, temporary_shell), offsetof(OpenTTDIndustryTransactionServices, resolve_location), offsetof(OpenTTDIndustryTransactionServices, resolve_shape), offsetof(OpenTTDIndustryTransactionServices, location_error), offsetof(OpenTTDIndustryTransactionServices, shape_error), offsetof(OpenTTDIndustryTransactionServices, tile_grf_version), offsetof(OpenTTDIndustryTransactionServices, shape_callback), offsetof(OpenTTDIndustryTransactionServices, terragenesis), offsetof(OpenTTDIndustryTransactionServices, ignore_restrictions), offsetof(OpenTTDIndustryTransactionServices, platform), offsetof(OpenTTDIndustryTransactionServices, terraform), offsetof(OpenTTDIndustryTransactionServices, can_allocate), offsetof(OpenTTDIndustryTransactionServices, allocate), offsetof(OpenTTDIndustryTransactionServices, prospecting_chance), offsetof(OpenTTDIndustryTransactionServices, local_company), offsetof(OpenTTDIndustryTransactionServices, prospect_no_place), offsetof(OpenTTDIndustryTransactionServices, prospect_unlucky), offsetof(OpenTTDIndustryTransactionServices, construction_cost)}) == -1);
	CHECK(AbiLayoutMismatch(585, {sizeof(OpenTTDIndustryTransactionContext), alignof(OpenTTDIndustryTransactionContext), offsetof(OpenTTDIndustryTransactionContext, map), offsetof(OpenTTDIndustryTransactionContext, builder), offsetof(OpenTTDIndustryTransactionContext, production), offsetof(OpenTTDIndustryTransactionContext, generation), offsetof(OpenTTDIndustryTransactionContext, construction), offsetof(OpenTTDIndustryTransactionContext, lifecycle), offsetof(OpenTTDIndustryTransactionContext, tiles), offsetof(OpenTTDIndustryTransactionContext, tile_policy), offsetof(OpenTTDIndustryTransactionContext, commands), offsetof(OpenTTDIndustryTransactionContext, costs), offsetof(OpenTTDIndustryTransactionContext, transaction), offsetof(OpenTTDIndustryTransactionContext, errors), offsetof(OpenTTDIndustryTransactionContext, clear_areas), offsetof(OpenTTDIndustryTransactionContext, restore_clear_areas)}) == -1);
	CHECK(AbiLayoutMismatch(577, {sizeof(OpenTTDIndustryCostServices), alignof(OpenTTDIndustryCostServices), offsetof(OpenTTDIndustryCostServices, zero), offsetof(OpenTTDIndustryCostServices, error), offsetof(OpenTTDIndustryCostServices, expense_money), offsetof(OpenTTDIndustryCostServices, success)}) == -1);
	CHECK(AbiLayoutMismatch(578, {sizeof(OpenTTDIndustryTilePolicyServices), alignof(OpenTTDIndustryTilePolicyServices), offsetof(OpenTTDIndustryTilePolicyServices, current_company), offsetof(OpenTTDIndustryTilePolicyServices, magic_bulldozer), offsetof(OpenTTDIndustryTilePolicyServices, object_in_way), offsetof(OpenTTDIndustryTilePolicyServices, slopes_refused), offsetof(OpenTTDIndustryTilePolicyServices, close_ai), offsetof(OpenTTDIndustryTilePolicyServices, close_game), offsetof(OpenTTDIndustryTilePolicyServices, destroy), offsetof(OpenTTDIndustryTilePolicyServices, removal_cost), offsetof(OpenTTDIndustryTilePolicyServices, tile_max_z), offsetof(OpenTTDIndustryTilePolicyServices, autoslope_callback), offsetof(OpenTTDIndustryTilePolicyServices, callback_boolean), offsetof(OpenTTDIndustryTilePolicyServices, landscape_clear)}) == -1);
	CHECK(AbiLayoutMismatch(576, {sizeof(OpenTTDIndustryCommandServices), alignof(OpenTTDIndustryCommandServices), offsetof(OpenTTDIndustryCommandServices, deity), offsetof(OpenTTDIndustryCommandServices, get), offsetof(OpenTTDIndustryCommandServices, owner), offsetof(OpenTTDIndustryCommandServices, valid_company), offsetof(OpenTTDIndustryCommandServices, invalidate_view)}) == -1);
	CHECK(AbiLayoutMismatch(575, {sizeof(OpenTTDIndustryTileAcceptance), alignof(OpenTTDIndustryTileAcceptance), offsetof(OpenTTDIndustryTileAcceptance, cargo), offsetof(OpenTTDIndustryTileAcceptance, amount), offsetof(OpenTTDIndustryTileAcceptance, accepts_all), offsetof(OpenTTDIndustryTileAcceptance, accepts_callback), offsetof(OpenTTDIndustryTileAcceptance, amount_callback)}) == -1);
	CHECK(AbiLayoutMismatch(573, {sizeof(OpenTTDIndustryTileOwner), alignof(OpenTTDIndustryTileOwner), offsetof(OpenTTDIndustryTileOwner, shell), offsetof(OpenTTDIndustryTileOwner, owner)}) == -1);
	CHECK(AbiLayoutMismatch(574, {sizeof(OpenTTDIndustryTileServices), alignof(OpenTTDIndustryTileServices), offsetof(OpenTTDIndustryTileServices, gfx), offsetof(OpenTTDIndustryTileServices, set_gfx), offsetof(OpenTTDIndustryTileServices, frame), offsetof(OpenTTDIndustryTileServices, set_frame), offsetof(OpenTTDIndustryTileServices, animation_loop), offsetof(OpenTTDIndustryTileServices, set_animation_loop), offsetof(OpenTTDIndustryTileServices, ambient), offsetof(OpenTTDIndustryTileServices, animate_new), offsetof(OpenTTDIndustryTileServices, construction_counter), offsetof(OpenTTDIndustryTileServices, construction_stage), offsetof(OpenTTDIndustryTileServices, completed), offsetof(OpenTTDIndustryTileServices, set_completed), offsetof(OpenTTDIndustryTileServices, reset_construction), offsetof(OpenTTDIndustryTileServices, industry), offsetof(OpenTTDIndustryTileServices, industry_index), offsetof(OpenTTDIndustryTileServices, toffee_offset), offsetof(OpenTTDIndustryTileServices, on_water), offsetof(OpenTTDIndustryTileServices, water_loop), offsetof(OpenTTDIndustryTileServices, transport), offsetof(OpenTTDIndustryTileServices, anim_production), offsetof(OpenTTDIndustryTileServices, anim_next), offsetof(OpenTTDIndustryTileServices, build_oilrig), offsetof(OpenTTDIndustryTileServices, pixel_max_z), offsetof(OpenTTDIndustryTileServices, create_effect), offsetof(OpenTTDIndustryTileServices, create_effect_above), offsetof(OpenTTDIndustryTileServices, set_effect_substate), offsetof(OpenTTDIndustryTileServices, spec), offsetof(OpenTTDIndustryTileServices, acceptance), offsetof(OpenTTDIndustryTileServices, callback), offsetof(OpenTTDIndustryTileServices, cargo_translation)}) == -1);
	CHECK(AbiLayoutMismatch(572, {sizeof(OpenTTDByteSpan), alignof(OpenTTDByteSpan), offsetof(OpenTTDByteSpan, data), offsetof(OpenTTDByteSpan, size)}) == -1);
	CHECK(AbiLayoutMismatch(568, {sizeof(OpenTTDIndustryCargoLists), alignof(OpenTTDIndustryCargoLists), offsetof(OpenTTDIndustryCargoLists, produced), offsetof(OpenTTDIndustryCargoLists, accepted)}) == -1);
	CHECK(AbiLayoutMismatch(569, {sizeof(OpenTTDIndustryLayoutTile), alignof(OpenTTDIndustryLayoutTile), offsetof(OpenTTDIndustryLayoutTile, x), offsetof(OpenTTDIndustryLayoutTile, y), offsetof(OpenTTDIndustryLayoutTile, gfx)}) == -1);
	CHECK(AbiLayoutMismatch(570, {sizeof(OpenTTDIndustryConstructionRequest), alignof(OpenTTDIndustryConstructionRequest), offsetof(OpenTTDIndustryConstructionRequest, layout), offsetof(OpenTTDIndustryConstructionRequest, town), offsetof(OpenTTDIndustryConstructionRequest, layout_index), offsetof(OpenTTDIndustryConstructionRequest, tile), offsetof(OpenTTDIndustryConstructionRequest, size_x), offsetof(OpenTTDIndustryConstructionRequest, size_y), offsetof(OpenTTDIndustryConstructionRequest, id), offsetof(OpenTTDIndustryConstructionRequest, random_bits), offsetof(OpenTTDIndustryConstructionRequest, type), offsetof(OpenTTDIndustryConstructionRequest, founder)}) == -1);
	CHECK(AbiLayoutMismatch(571, {sizeof(OpenTTDIndustryConstructionServices), alignof(OpenTTDIndustryConstructionServices), offsetof(OpenTTDIndustryConstructionServices, cargo_lists), offsetof(OpenTTDIndustryConstructionServices, callback_mask), offsetof(OpenTTDIndustryConstructionServices, set_town), offsetof(OpenTTDIndustryConstructionServices, calendar_date), offsetof(OpenTTDIndustryConstructionServices, generating), offsetof(OpenTTDIndustryConstructionServices, callback), offsetof(OpenTTDIndustryConstructionServices, callback_error), offsetof(OpenTTDIndustryConstructionServices, grf_version), offsetof(OpenTTDIndustryConstructionServices, cargo_translation), offsetof(OpenTTDIndustryConstructionServices, layout_size), offsetof(OpenTTDIndustryConstructionServices, layout_tile), offsetof(OpenTTDIndustryConstructionServices, water_class), offsetof(OpenTTDIndustryConstructionServices, clear_tile), offsetof(OpenTTDIndustryConstructionServices, make_industry), offsetof(OpenTTDIndustryConstructionServices, set_construction_counter), offsetof(OpenTTDIndustryConstructionServices, set_construction_stage), offsetof(OpenTTDIndustryConstructionServices, translated_gfx), offsetof(OpenTTDIndustryConstructionServices, animation_enabled), offsetof(OpenTTDIndustryConstructionServices, add_animated), offsetof(OpenTTDIndustryConstructionServices, build_window_dirty), offsetof(OpenTTDIndustryConstructionServices, serve_neutral), offsetof(OpenTTDIndustryConstructionServices, insert_near_station), offsetof(OpenTTDIndustryConstructionServices, clear_near_industries), offsetof(OpenTTDIndustryConstructionServices, insert_neutral_industry), offsetof(OpenTTDIndustryConstructionServices, stations_around), offsetof(OpenTTDIndustryConstructionServices, add_delivery)}) == -1);
	CHECK(AbiLayoutMismatch(566, {sizeof(OpenTTDIndustryGenerationInfo), alignof(OpenTTDIndustryGenerationInfo), offsetof(OpenTTDIndustryGenerationInfo, appear), offsetof(OpenTTDIndustryGenerationInfo, enabled), offsetof(OpenTTDIndustryGenerationInfo, layouts)}) == -1);
	CHECK(AbiLayoutMismatch(567, {sizeof(OpenTTDIndustryGenerationServices), alignof(OpenTTDIndustryGenerationServices), offsetof(OpenTTDIndustryGenerationServices, spec), offsetof(OpenTTDIndustryGenerationServices, spec_behaviour), offsetof(OpenTTDIndustryGenerationServices, info), offsetof(OpenTTDIndustryGenerationServices, coastal), offsetof(OpenTTDIndustryGenerationServices, editor), offsetof(OpenTTDIndustryGenerationServices, density), offsetof(OpenTTDIndustryGenerationServices, custom_number), offsetof(OpenTTDIndustryGenerationServices, scale_by_size_1d), offsetof(OpenTTDIndustryGenerationServices, scale_by_land_proportion), offsetof(OpenTTDIndustryGenerationServices, set_progress), offsetof(OpenTTDIndustryGenerationServices, increase_progress), offsetof(OpenTTDIndustryGenerationServices, missing_warning)}) == -1);
	CHECK(AbiLayoutMismatch(560, {sizeof(OpenTTDIndustryMetadata), alignof(OpenTTDIndustryMetadata), offsetof(OpenTTDIndustryMetadata, tile), offsetof(OpenTTDIndustryMetadata, width), offsetof(OpenTTDIndustryMetadata, height), offsetof(OpenTTDIndustryMetadata, construction_date), offsetof(OpenTTDIndustryMetadata, random), offsetof(OpenTTDIndustryMetadata, type), offsetof(OpenTTDIndustryMetadata, owner), offsetof(OpenTTDIndustryMetadata, random_colour), offsetof(OpenTTDIndustryMetadata, founder), offsetof(OpenTTDIndustryMetadata, construction_type), offsetof(OpenTTDIndustryMetadata, selected_layout), offsetof(OpenTTDIndustryMetadata, exclusive_supplier), offsetof(OpenTTDIndustryMetadata, exclusive_consumer)}) == -1);
	CHECK(AbiLayoutMismatch(561, {sizeof(OpenTTDIndustryCleanupTile), alignof(OpenTTDIndustryCleanupTile), offsetof(OpenTTDIndustryCleanupTile, industry), offsetof(OpenTTDIndustryCleanupTile, type), offsetof(OpenTTDIndustryCleanupTile, oilrig)}) == -1);
	CHECK(AbiLayoutMismatch(562, {sizeof(OpenTTDIndustryReseed), alignof(OpenTTDIndustryReseed), offsetof(OpenTTDIndustryReseed, self), offsetof(OpenTTDIndustryReseed, parent), offsetof(OpenTTDIndustryReseed, used)}) == -1);
	CHECK(AbiLayoutMismatch(563, {sizeof(OpenTTDIndustryRandomisationSpec), alignof(OpenTTDIndustryRandomisationSpec), offsetof(OpenTTDIndustryRandomisationSpec, gfx), offsetof(OpenTTDIndustryRandomisationSpec, enabled)}) == -1);
	CHECK(AbiLayoutMismatch(564, {sizeof(OpenTTDIndustryAnimationSpec), alignof(OpenTTDIndustryAnimationSpec), offsetof(OpenTTDIndustryAnimationSpec, spec), offsetof(OpenTTDIndustryAnimationSpec, triggers)}) == -1);
	CHECK(AbiLayoutMismatch(565, {sizeof(OpenTTDIndustryLifecycleServices), alignof(OpenTTDIndustryLifecycleServices), offsetof(OpenTTDIndustryLifecycleServices, neutral_station), offsetof(OpenTTDIndustryLifecycleServices, cleanup_tile), offsetof(OpenTTDIndustryLifecycleServices, delete_tile_inspect), offsetof(OpenTTDIndustryLifecycleServices, make_water), offsetof(OpenTTDIndustryLifecycleServices, delete_oilrig), offsetof(OpenTTDIndustryLifecycleServices, clear_docking), offsetof(OpenTTDIndustryLifecycleServices, industry_field), offsetof(OpenTTDIndustryLifecycleServices, disassociate_field), offsetof(OpenTTDIndustryLifecycleServices, release_disasters), offsetof(OpenTTDIndustryLifecycleServices, delete_storage), offsetof(OpenTTDIndustryLifecycleServices, delete_news), offsetof(OpenTTDIndustryLifecycleServices, close_view), offsetof(OpenTTDIndustryLifecycleServices, close_production), offsetof(OpenTTDIndustryLifecycleServices, delete_inspect), offsetof(OpenTTDIndustryLifecycleServices, delete_subsidies), offsetof(OpenTTDIndustryLifecycleServices, invalidate_cargo), offsetof(OpenTTDIndustryLifecycleServices, next_near_station), offsetof(OpenTTDIndustryLifecycleServices, remove_delivery), offsetof(OpenTTDIndustryLifecycleServices, belongs), offsetof(OpenTTDIndustryLifecycleServices, randomisation_spec), offsetof(OpenTTDIndustryLifecycleServices, random_triggers), offsetof(OpenTTDIndustryLifecycleServices, set_random_triggers), offsetof(OpenTTDIndustryLifecycleServices, resolve_reseed), offsetof(OpenTTDIndustryLifecycleServices, random_bits), offsetof(OpenTTDIndustryLifecycleServices, set_random_bits), offsetof(OpenTTDIndustryLifecycleServices, animation_spec), offsetof(OpenTTDIndustryLifecycleServices, change_frame)}) == -1);
	CHECK(AbiLayoutMismatch(130, {sizeof(OpenTTDIndustryFields), alignof(OpenTTDIndustryFields), offsetof(OpenTTDIndustryFields, valid_history), offsetof(OpenTTDIndustryFields, last_prod_year), offsetof(OpenTTDIndustryFields, counter), offsetof(OpenTTDIndustryFields, prod_level), offsetof(OpenTTDIndustryFields, was_cargo_delivered), offsetof(OpenTTDIndustryFields, ctlflags)}) == -1);
	CHECK(AbiLayoutMismatch(131, {sizeof(OpenTTDIndustryBuildFields), alignof(OpenTTDIndustryBuildFields), offsetof(OpenTTDIndustryBuildFields, probability), offsetof(OpenTTDIndustryBuildFields, min_number), offsetof(OpenTTDIndustryBuildFields, target_count), offsetof(OpenTTDIndustryBuildFields, max_wait), offsetof(OpenTTDIndustryBuildFields, wait_count)}) == -1);
	CHECK(AbiLayoutMismatch(132, {sizeof(OpenTTDIndustryBuilderFields), alignof(OpenTTDIndustryBuilderFields), offsetof(OpenTTDIndustryBuilderFields, builddata), offsetof(OpenTTDIndustryBuilderFields, wanted_inds), offsetof(OpenTTDIndustryBuilderFields, daily_counter), offsetof(OpenTTDIndustryBuilderFields, daily_increment), offsetof(OpenTTDIndustryBuilderFields, sound_tile), offsetof(OpenTTDIndustryBuilderFields, sound_ctr)}) == -1);
	CHECK(AbiLayoutMismatch(133, {sizeof(OpenTTDIndustrySlots), alignof(OpenTTDIndustrySlots), offsetof(OpenTTDIndustrySlots, data), offsetof(OpenTTDIndustrySlots, size)}) == -1);
	CHECK(AbiLayoutMismatch(134, {sizeof(Industry::ProducedCargo), alignof(Industry::ProducedCargo), offsetof(Industry::ProducedCargo, cargo), offsetof(Industry::ProducedCargo, waiting), offsetof(Industry::ProducedCargo, rate), offsetof(Industry::ProducedCargo, history)}) == -1);
	CHECK(AbiLayoutMismatch(135, {sizeof(Industry::AcceptedCargo), alignof(Industry::AcceptedCargo), offsetof(Industry::AcceptedCargo, cargo), offsetof(Industry::AcceptedCargo, waiting), offsetof(Industry::AcceptedCargo, accumulated_waiting), offsetof(Industry::AcceptedCargo, last_accepted), offsetof(Industry::AcceptedCargo, history)}) == -1);
	CHECK(AbiLayoutMismatch(136, {sizeof(OpenTTDIndustryObservation), alignof(OpenTTDIndustryObservation), offsetof(OpenTTDIndustryObservation, owner), offsetof(OpenTTDIndustryObservation, behaviour), offsetof(OpenTTDIndustryObservation, callbacks), offsetof(OpenTTDIndustryObservation, life), offsetof(OpenTTDIndustryObservation, original), offsetof(OpenTTDIndustryObservation, type), offsetof(OpenTTDIndustryObservation, up_text), offsetof(OpenTTDIndustryObservation, down_text), offsetof(OpenTTDIndustryObservation, closure_text)}) == -1);
	CHECK(AbiLayoutMismatch(137, {sizeof(OpenTTDIndustryServices), alignof(OpenTTDIndustryServices), offsetof(OpenTTDIndustryServices, shared), offsetof(OpenTTDIndustryServices, next_tick), offsetof(OpenTTDIndustryServices, sound_count), offsetof(OpenTTDIndustryServices, behaviour), offsetof(OpenTTDIndustryServices, location), offsetof(OpenTTDIndustryServices, industry_sound), offsetof(OpenTTDIndustryServices, play_sound), offsetof(OpenTTDIndustryServices, special_effect), offsetof(OpenTTDIndustryServices, tick_trigger), offsetof(OpenTTDIndustryServices, production_callback), offsetof(OpenTTDIndustryServices, scale_cargo), offsetof(OpenTTDIndustryServices, move_goods), offsetof(OpenTTDIndustryServices, tile_add_wrap), offsetof(OpenTTDIndustryServices, farm_tile), offsetof(OpenTTDIndustryServices, snow_line), offsetof(OpenTTDIndustryServices, make_field), offsetof(OpenTTDIndustryServices, fence_wanted), offsetof(OpenTTDIndustryServices, set_fence), offsetof(OpenTTDIndustryServices, tile_completed), offsetof(OpenTTDIndustryServices, harvest), offsetof(OpenTTDIndustryServices, observe), offsetof(OpenTTDIndustryServices, production_rate), offsetof(OpenTTDIndustryServices, change_callback), offsetof(OpenTTDIndustryServices, custom_text), offsetof(OpenTTDIndustryServices, news), offsetof(OpenTTDIndustryServices, rate_news), offsetof(OpenTTDIndustryServices, callback_error), offsetof(OpenTTDIndustryServices, set_dirty), offsetof(OpenTTDIndustryServices, destroy), offsetof(OpenTTDIndustryServices, advertise), offsetof(OpenTTDIndustryServices, create), offsetof(OpenTTDIndustryServices, random_industry), offsetof(OpenTTDIndustryServices, get), offsetof(OpenTTDIndustryServices, type_count), offsetof(OpenTTDIndustryServices, total), offsetof(OpenTTDIndustryServices, type_info), offsetof(OpenTTDIndustryServices, probability_callback), offsetof(OpenTTDIndustryServices, scale_by_map_size), offsetof(OpenTTDIndustryServices, company_none), offsetof(OpenTTDIndustryServices, restore_company), offsetof(OpenTTDIndustryServices, directory_dirty), offsetof(OpenTTDIndustryServices, economy_month), offsetof(OpenTTDIndustryServices, economy_year), offsetof(OpenTTDIndustryServices, days_since_last_month), offsetof(OpenTTDIndustryServices, landscape), offsetof(OpenTTDIndustryServices, economy_type), offsetof(OpenTTDIndustryServices, passengers), offsetof(OpenTTDIndustryServices, fund_only), offsetof(OpenTTDIndustryServices, calendar_year), offsetof(OpenTTDIndustryServices, deity)}) == -1);
	CHECK(AbiLayoutMismatch(138, {sizeof(OpenTTDIndustryProductionResult), alignof(OpenTTDIndustryProductionResult), offsetof(OpenTTDIndustryProductionResult, subtract), offsetof(OpenTTDIndustryProductionResult, add), offsetof(OpenTTDIndustryProductionResult, again), offsetof(OpenTTDIndustryProductionResult, cargo_input), offsetof(OpenTTDIndustryProductionResult, cargo_output), offsetof(OpenTTDIndustryProductionResult, version), offsetof(OpenTTDIndustryProductionResult, num_input), offsetof(OpenTTDIndustryProductionResult, num_output), offsetof(OpenTTDIndustryProductionResult, present)}) == -1);
	CHECK(AbiLayoutMismatch(139, {sizeof(OpenTTDIndustryMap), alignof(OpenTTDIndustryMap), offsetof(OpenTTDIndustryMap, size_x), offsetof(OpenTTDIndustryMap, size_y), offsetof(OpenTTDIndustryMap, landscape)}) == -1);
	CHECK(AbiLayoutMismatch(140, {sizeof(OpenTTDIndustryTickRecord), alignof(OpenTTDIndustryTickRecord), offsetof(OpenTTDIndustryTickRecord, counter), offsetof(OpenTTDIndustryTickRecord, map), offsetof(OpenTTDIndustryTickRecord, interval), offsetof(OpenTTDIndustryTickRecord, ambient), offsetof(OpenTTDIndustryTickRecord, editor)}) == -1);
	CHECK(AbiLayoutMismatch(141, {sizeof(OpenTTDIndustryEntry), alignof(OpenTTDIndustryEntry), offsetof(OpenTTDIndustryEntry, industry), offsetof(OpenTTDIndustryEntry, owner), offsetof(OpenTTDIndustryEntry, id), offsetof(OpenTTDIndustryEntry, callbacks)}) == -1);
	CHECK(AbiLayoutMismatch(142, {sizeof(OpenTTDIndustryLocation), alignof(OpenTTDIndustryLocation), offsetof(OpenTTDIndustryLocation, tile), offsetof(OpenTTDIndustryLocation, width), offsetof(OpenTTDIndustryLocation, height)}) == -1);
	CHECK(AbiLayoutMismatch(143, {sizeof(OpenTTDIndustryFarmTile), alignof(OpenTTDIndustryFarmTile), offsetof(OpenTTDIndustryFarmTile, type), offsetof(OpenTTDIndustryFarmTile, snow), offsetof(OpenTTDIndustryFarmTile, ground), offsetof(OpenTTDIndustryFarmTile, grown)}) == -1);
	CHECK(AbiLayoutMismatch(144, {sizeof(OpenTTDIndustryChange), alignof(OpenTTDIndustryChange), offsetof(OpenTTDIndustryChange, result), offsetof(OpenTTDIndustryChange, reg)}) == -1);
	CHECK(AbiLayoutMismatch(145, {sizeof(OpenTTDIndustryTypeInfo), alignof(OpenTTDIndustryTypeInfo), offsetof(OpenTTDIndustryTypeInfo, behaviour), offsetof(OpenTTDIndustryTypeInfo, enabled), offsetof(OpenTTDIndustryTypeInfo, layouts), offsetof(OpenTTDIndustryTypeInfo, appear)}) == -1);
	REQUIRE(NUM_INDUSTRYTYPES == 240);
	REQUIRE(std::size(OpenTTDIndustryBuilderFields{}.builddata) == NUM_INDUSTRYTYPES);
	REQUIRE(HISTORY_RECORDS == 61);
	REQUIRE(sizeof(Industry::ProducedHistory) == 4);
	REQUIRE(sizeof(Industry::AcceptedHistory) == 4);
	REQUIRE(sizeof(TimerGameEconomy::Year) == sizeof(int32_t));
	REQUIRE(sizeof(TimerGameEconomy::Date) == sizeof(int32_t));
	REQUIRE(sizeof(IndustryControlFlags) == sizeof(uint8_t));
	REQUIRE(sizeof(IndustryTypeBuildData) == sizeof(OpenTTDIndustryBuildFields));
}
TEST_CASE("Industry - canonical allocation and history lifetime")
{
	RustIndustryOwner owner(openttd_rust_industry_new(), openttd_rust_industry_destroy);
	RustIndustryVector<Industry::AcceptedCargo, false> accepted(owner.get());
	RustIndustryVector<Industry::ProducedCargo, true> produced(owner.get());
	accepted.emplace_back().waiting = 65535;
	auto &history = accepted[0].GetOrCreateHistory();
	history[THIS_MONTH].accepted = 65535;
	auto *allocation = accepted[0].history.get();
	accepted.reserve(256);
	CHECK(accepted[0].history.get() == allocation);
	CHECK(accepted[0].waiting == 65535);
	CHECK(accepted[0].GetOrCreateHistory()[THIS_MONTH].accepted == 65535);
	accepted.emplace_back().cargo = INVALID_CARGO;
	produced.emplace_back().cargo = 3;
	produced.emplace_back().cargo = INVALID_CARGO;
	openttd_rust_industry_trim(owner.get());
	CHECK(accepted.size() == 1);
	CHECK(produced.size() == 1);
	CHECK(accepted[0].history.get() == allocation);
	accepted.clear();
	CHECK(accepted.empty());
	CHECK(accepted.emplace_back().history == nullptr);
}
TEST_CASE("Industry - construction owner and ascending type registry")
{
	RustIndustryOwner owner{openttd_rust_industry_new(), openttd_rust_industry_destroy};
	auto *metadata = openttd_rust_industry_metadata(owner.get());
	CHECK(metadata->tile == INVALID_TILE.base());
	CHECK(metadata->width == 0);
	CHECK(metadata->owner == INVALID_OWNER);
	CHECK(metadata->founder == INVALID_OWNER);
	CHECK(metadata->exclusive_supplier == INVALID_OWNER);
	CHECK(metadata->exclusive_consumer == INVALID_OWNER);
	std::construct_at(reinterpret_cast<TileArea *>(&metadata->tile), TileIndex(456), 7, 9);
	CHECK(metadata->tile == 456);
	CHECK(metadata->width == 7);
	CHECK(metadata->height == 9);
	const uint8_t text[] = {0, 0xFF, 0x1E, 'a'};
	openttd_rust_industry_set_text(owner.get(), text, sizeof(text));
	auto text_view = openttd_rust_industry_text(owner.get());
	CHECK(text_view.size == sizeof(text));
	CHECK(std::equal(std::begin(text), std::end(text), text_view.data));
	openttd_rust_industry_set_text(owner.get(), nullptr, 0);
	CHECK(openttd_rust_industry_text(owner.get()).size == 0);
	RustIndustryBuilderOwner builder{openttd_rust_industry_builder_new(), openttd_rust_industry_builder_destroy};
	for (uint16_t id : {9, 1, 9, 5}) openttd_rust_industry_registry_insert(builder.get(), 3, id);
	const uint16_t expected[] = {1, 5, 9};
	auto view = openttd_rust_industry_registry_view(builder.get(), 3);
	CHECK(view.size == 3);
	CHECK(std::equal(std::begin(expected), std::end(expected), static_cast<uint16_t *>(view.data)));
	openttd_rust_industry_registry_erase(builder.get(), 3, 5);
	openttd_rust_industry_registry_erase(builder.get(), 3, 99);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 3).size == 2);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 4).size == 0);
	openttd_rust_industry_registry_insert(builder.get(), 3, 5);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 3).size == 3);
	openttd_rust_industry_registry_clear(builder.get(), 3);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 3).size == 0);
}

TEST_CASE("Industry - native generation handoff retains named cancellation")
{
	OpenTTDIndustryGenerationContext context;
	CHECK(RunGenerationProgress(&context, [] { }));
	CHECK(context.cancellation == nullptr);
	CHECK_FALSE(RunGenerationProgress(&context, [] { throw AbortGenerateWorldSignal(); }));
	REQUIRE(context.cancellation != nullptr);
	bool aborted = false;
	try { std::rethrow_exception(context.cancellation); }
	catch (AbortGenerateWorldSignal &) { aborted = true; }
	CHECK(aborted);
}

namespace {
struct IndustryGenerationFixture {
	static inline uint8_t company;
	static inline uint32_t draws, created, increases, sets;
	static inline int cancel_set, cancel_increase;
	static inline int shell;

	static OpenTTDIndustryServices World()
	{
		auto world = GetIndustryRustServices();
		world.create = [](uint8_t, uint8_t creation) noexcept -> void * { CHECK(creation == 0); ++created; return &shell; };
		world.type_count = [](uint8_t) noexcept -> uint16_t { return 0; };
		world.total = []() noexcept -> uint32_t { return 0; };
		world.probability_callback = [](uint8_t, uint8_t creation, uint32_t chance) noexcept -> uint32_t { CHECK(creation == 0); return chance; };
		world.scale_by_map_size = [](uint32_t n) noexcept -> uint32_t { return n; };
		static auto shared = GetRustSharedServices();
		shared.random = [](void *) noexcept -> uint32_t { ++draws; return 0; };
		world.shared = &shared;
		world.company_none = []() noexcept -> uint8_t { uint8_t previous = company; company = 255; return previous; };
		world.restore_company = [](uint8_t previous) noexcept { CHECK(company == 255); company = previous; };
		return world;
	}
	static OpenTTDIndustryGenerationServices Generation()
	{
		return {
			.spec = [](uint8_t type) noexcept -> const OpenTTDIndustrySpecHandle * { return reinterpret_cast<const OpenTTDIndustrySpecHandle *>(uintptr_t(type) + 1); },
			.spec_behaviour = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> uint32_t { return uintptr_t(spec) == 2 ? 1U << 2 : 0; },
			.info = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> OpenTTDIndustryGenerationInfo { return {1, uintptr_t(spec) <= 2, true}; },
			.coastal = [](const OpenTTDIndustrySpecHandle *) noexcept -> bool { return false; },
			.editor = []() noexcept -> bool { return false; },
			.density = []() noexcept -> uint8_t { return 4; },
			.custom_number = []() noexcept -> uint32_t { return 0; },
			.scale_by_size_1d = [](uint32_t n) noexcept -> uint32_t { return n; },
			.scale_by_land_proportion = [](uint32_t n) noexcept -> uint32_t { return n; },
			.set_progress = [](OpenTTDIndustryGenerationContext *, bool water, uint32_t total) noexcept -> bool { CHECK(company == 9); CHECK(total == 27); ++sets; return cancel_set != int(water); },
			.increase_progress = [](OpenTTDIndustryGenerationContext *, bool) noexcept -> bool { CHECK(company == 255); return int(++increases) != cancel_increase; },
			.missing_warning = [](const OpenTTDIndustrySpecHandle *) noexcept { },
		};
	}
};
}

TEST_CASE("Industry - generation cancellation restores company before return")
{
	using F = IndustryGenerationFixture;
	const auto world = F::World();
	const auto generation = F::Generation();
	for (const auto [cancel_set, cancel_increase, draws, created, increases, sets] : {
		std::tuple{0, -1, 0U, 0U, 0U, 1U},
		std::tuple{-1, 1, 0U, 0U, 1U, 1U},
		std::tuple{-1, 2, 1U, 1U, 2U, 1U},
		std::tuple{1, -1, 26U, 27U, 27U, 2U},
		std::tuple{-1, 28, 26U, 27U, 28U, 2U},
		std::tuple{-1, -1, 52U, 54U, 54U, 2U},
	}) {
		F::company = 9;
		F::draws = F::created = F::increases = F::sets = 0;
		F::cancel_set = cancel_set;
		F::cancel_increase = cancel_increase;
		auto *builder = openttd_rust_industry_builder_new();
		auto *fields = openttd_rust_industry_builder_fields(builder);
		fields->wanted_inds = 123;
		const bool complete = openttd_rust_industry_generate(builder, nullptr, &world, &generation);
		CHECK(complete == (cancel_set == -1 && cancel_increase == -1));
		CHECK(F::company == 9);
		CHECK(F::draws == draws);
		CHECK(F::created == created);
		CHECK(F::increases == increases);
		CHECK(F::sets == sets);
		CHECK(fields->wanted_inds == (complete ? 0 : 123));
		openttd_rust_industry_builder_destroy(builder);
	}
}

TEST_CASE("Industry - shared plain water predicate excludes depot and lock parts")
{
	Map::Allocate(64, 64);
	const auto &shared = GetRustSharedServices();
	TileIndex tile = TileXY(20, 20);
	SetTileType(tile, MP_CLEAR);
	CHECK_FALSE(shared.is_water_tile(tile.base()));
	MakeSea(tile);
	CHECK(shared.is_water_tile(tile.base()));
	MakeCanal(tile, Owner(0), 0);
	CHECK(shared.is_water_tile(tile.base()));
	MakeRiver(tile, 0);
	CHECK(shared.is_water_tile(tile.base()));
	MakeShore(tile);
	CHECK_FALSE(shared.is_water_tile(tile.base()));
	for (DepotPart part : {DepotPart::North, DepotPart::South}) {
		MakeShipDepot(tile, Owner(0), DepotID(0), part, AXIS_X, WaterClass::Sea);
		CHECK_FALSE(shared.is_water_tile(tile.base()));
	}
	for (LockPart part : {LockPart::Lower, LockPart::Middle, LockPart::Upper}) {
		MakeLockTile(tile, Owner(0), part, DIAGDIR_NE, WaterClass::Sea);
		CHECK_FALSE(shared.is_water_tile(tile.base()));
	}
}

namespace IndustryTemporaryFixture {
static std::vector<uint8_t> trace;
static uint16_t result;
static uint8_t slope, refused, version;
static IndustrySpec industry_spec;
static IndustryTileSpec tile_spec;
static OpenTTDIndustry *active_owner;
static OpenTTDIndustryLocationResult Observe(Industry *shell)
{
	CHECK(shell->production_owner.get() == active_owner);
	CHECK(shell->index == IndustryID::Invalid());
	CHECK(shell->location.tile == TileIndex(123));
	CHECK(shell->location.w == 0);
	CHECK(shell->type == 7);
	CHECK(shell->random == 0xABCD);
	CHECK(shell->founder == Owner(2));
	CHECK(shell->accepted.empty());
	CHECK(shell->produced.empty());
	trace.push_back(3);
	OpenTTDIndustryLocationResult output{};
	output.callback = result;
	output.registers[3] = 456;
	return output;
}
}
TEST_CASE("Industry - borrowed stack resolver shell and callback policy")
{
	using namespace IndustryTemporaryFixture;
	auto shared = GetRustSharedServices();
	OpenTTDIndustryServices production{};
	production.shared = &shared;
	OpenTTDIndustryGenerationServices generation{};
	OpenTTDIndustryConstructionServices construction{};
	OpenTTDIndustryLifecycleServices lifecycle{};
	OpenTTDIndustryTileServices tiles{};
	OpenTTDIndustryTilePolicyServices policy{};
	OpenTTDIndustryCommandServices commands{};
	OpenTTDIndustryCostServices costs{};
	OpenTTDIndustryTransactionServices transaction{};
	OpenTTDIndustryConstructionErrors errors{};
	errors.site = 1234;
	generation.spec = [](uint8_t kind) noexcept -> const OpenTTDIndustrySpecHandle * { CHECK(kind == 7); trace.push_back(0); return reinterpret_cast<const OpenTTDIndustrySpecHandle *>(&industry_spec); };
	commands.owner = [](Industry *shell) noexcept -> OpenTTDIndustry * { return shell->production_owner.get(); };
	construction.set_town = [](void *shell, const OpenTTDTownHandle *town) noexcept { CHECK(town == nullptr); static_cast<Industry *>(shell)->town = nullptr; };
	transaction.closest_town = [](uint32_t tile, uint32_t threshold) noexcept -> const OpenTTDTownHandle * { CHECK(tile == 123); CHECK(threshold == UINT32_MAX); CHECK(openttd_rust_industry_metadata(active_owner)->random == 0); trace.push_back(2); return nullptr; };
	transaction.temporary_shell = [](OpenTTDIndustry *owner, void *context, OpenTTDIndustryTemporaryVisit visit) noexcept {
		active_owner = owner;
		trace.push_back(1);
		{ Industry shell(owner); shell.index = IndustryID::Invalid(); visit(context, &shell); }
		trace.push_back(5);
		/* The owner still lives after the actual native destructor returns. */
		CHECK(openttd_rust_industry_metadata(owner)->width == 0);
		active_owner = nullptr;
	};
	transaction.resolve_location = [](Industry *shell, uint32_t tile, uint8_t kind, uint32_t seed, uint8_t creation) noexcept -> OpenTTDIndustryLocationResult { CHECK(tile == 123); CHECK(kind == 7); CHECK(seed == 0x12345678); CHECK(creation == 2); CHECK(shell->selected_layout == 2); return Observe(shell); };
	transaction.resolve_shape = [](Industry *shell, uint32_t tile, uint16_t gfx, uint32_t parameter) noexcept -> OpenTTDIndustryLocationResult { CHECK(tile == 456); CHECK(gfx == 8); CHECK(parameter == 0x302); CHECK(shell->selected_layout == 0); return Observe(shell); };
	transaction.location_error = [](CommandCost *out, const OpenTTDIndustryLocationResult *callback, const OpenTTDIndustrySpecHandle *spec) noexcept { CHECK(spec == reinterpret_cast<const OpenTTDIndustrySpecHandle *>(&industry_spec)); CHECK(callback->registers[3] == 456); trace.push_back(4); *out = CommandCost(EXPENSES_OTHER, Money(789)); };
	transaction.shape_error = [](CommandCost *out, const OpenTTDIndustryLocationResult *callback, const IndustryTileSpec *spec) noexcept { CHECK(spec == &tile_spec); CHECK(callback->registers[3] == 456); trace.push_back(4); *out = CommandCost(EXPENSES_OTHER, Money(789)); };
	transaction.tile_grf_version = [](const IndustryTileSpec *spec) noexcept -> uint8_t { CHECK(spec == &tile_spec); trace.push_back(6); return version; };
	shared.tile_slope = [](uint32_t tile) noexcept -> uint8_t { CHECK(tile == 456); trace.push_back(7); return slope; };
	policy.slopes_refused = [](const IndustryTileSpec *spec) noexcept -> uint8_t { CHECK(spec == &tile_spec); trace.push_back(8); return refused; };
	costs.zero = [](CommandCost *out) noexcept { *out = CommandCost(); };
	costs.error = [](CommandCost *out, uint32_t message) noexcept { *out = CommandCost(message); };
	OpenTTDIndustryTransactionContext context{
		.map = {},
		.builder = nullptr, .production = &production, .generation = &generation,
		.construction = &construction, .lifecycle = &lifecycle, .tiles = &tiles,
		.tile_policy = &policy, .commands = &commands, .costs = &costs,
		.transaction = &transaction, .errors = &errors,
		.clear_areas = nullptr, .restore_clear_areas = nullptr,
	};
	const auto pool_count = Industry::GetNumItems();
	for (uint16_t callback : {uint16_t{0}, uint16_t{1}, uint16_t{0xFFFF}}) {
		result = callback; trace.clear(); CommandCost cost;
		openttd_rust_industry_location_check(&cost, 123, 7, 258, 0x12345678, 0xABCD, 2, 2, &context);
		CHECK(cost.Succeeded());
		CHECK(cost.GetCost() == (callback == 0xFFFF ? 0 : 789));
		CHECK(trace == (callback == 0xFFFF ? std::vector<uint8_t>{0, 1, 2, 3, 5} : std::vector<uint8_t>{0, 1, 2, 3, 4, 5}));
		for (uint8_t grf_version : {6, 7, 8}) for (uint8_t current_slope : {0, 16}) {
			version = grf_version; slope = current_slope; refused = 0; trace.clear();
			openttd_rust_industry_shape_check(&cost, 123, 456, &tile_spec, 7, 8, 258, 0xABCD, 2, 3, &context);
			CHECK(cost.Succeeded() == (callback == 0xFFFF ? slope == 0 : version >= 7 || callback != 0));
			CHECK(cost.GetCost() == (callback != 0xFFFF && version >= 7 ? 789 : 0));
			if (callback == 0xFFFF) { CHECK(cost.GetErrorMessage() == (slope == 0 ? INVALID_STRING_ID : errors.site)); CHECK(trace == (std::vector<uint8_t>{1, 3, 7, 8, 5})); }
			else if (version < 7) { CHECK(trace == (std::vector<uint8_t>{1, 3, 6, 5})); }
			else { CHECK(trace == (std::vector<uint8_t>{1, 3, 6, 4, 5})); }
		}
	}
	CHECK(Industry::GetNumItems() == pool_count);
}
#endif
