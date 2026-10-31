/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file story.cpp Handling of stories. */

#include "stdafx.h"
#include "story_base.h"
#include "core/pool_func.hpp"
#include "command_func.h"
#include "company_base.h"
#include "company_func.h"
#include "string_func.h"
#include "timer/timer_game_calendar.h"
#include "tile_map.h"
#include "goal_type.h"
#include "goal_base.h"
#include "window_func.h"
#include "gui.h"
#include "vehicle_base.h"
#include "game/game.hpp"
#include "script/api/script_story_page.hpp"
#include "script/api/script_event_types.hpp"
#include "story_cmd.h"

#include "rust/services_objectives.h"

#include "safeguards.h"



StoryPageElementPool _story_page_element_pool("StoryPageElement");
StoryPagePool _story_page_pool("StoryPage");
INSTANTIATE_POOL_METHODS(StoryPageElement)
INSTANTIATE_POOL_METHODS(StoryPage)

StoryPage::~StoryPage()
{
	openttd_rust_story_page_destruct(&GetRustObjectivesServices(), this->index.base());
}

/**
 * This helper for Create/Update PageElement Cmd procedure verifies if the page
 * element parameters are correct for the given page element type.
 * @param page_id The page id of the page which the page element (will) belong to
 * @param type The type of the page element to create/update
 * @param tile The tile parameter of the DoCommand proc
 * @param reference The reference parameter of the DoCommand proc (p2)
 * @param text The text parameter of the DoCommand proc
 * @return true, if and only if the given parameters are valid for the given page element type and page id.
 */

/** Set the button background colour. */
void StoryPageButtonData::SetColour(Colours button_colour)
{
	assert(button_colour < COLOUR_END);
	SB(this->referenced_id, 0, 8, button_colour);
}

void StoryPageButtonData::SetFlags(StoryPageButtonFlags flags)
{
	SB(this->referenced_id, 24, 8, flags);
}

/** Set the mouse cursor used while waiting for input for the button. */
void StoryPageButtonData::SetCursor(StoryPageButtonCursor cursor)
{
	assert(cursor < SPBC_END);
	SB(this->referenced_id, 8, 8, cursor);
}

/** Set the type of vehicles that are accepted by the button */
void StoryPageButtonData::SetVehicleType(VehicleType vehtype)
{
	assert(vehtype == VEH_INVALID || vehtype < VEH_COMPANY_END);
	SB(this->referenced_id, 16, 8, vehtype);
}

/** Get the button background colour. */
Colours StoryPageButtonData::GetColour() const
{
	Colours colour = static_cast<Colours>(GB(this->referenced_id, 0, 8));
	if (!IsValidColours(colour)) return INVALID_COLOUR;
	return colour;
}

StoryPageButtonFlags StoryPageButtonData::GetFlags() const
{
	return (StoryPageButtonFlags)GB(this->referenced_id, 24, 8);
}

/** Get the mouse cursor used while waiting for input for the button. */
StoryPageButtonCursor StoryPageButtonData::GetCursor() const
{
	StoryPageButtonCursor cursor = (StoryPageButtonCursor)GB(this->referenced_id, 8, 8);
	if (!IsValidStoryPageButtonCursor(cursor)) return INVALID_SPBC;
	return cursor;
}

/** Get the type of vehicles that are accepted by the button */
VehicleType StoryPageButtonData::GetVehicleType() const
{
	return (VehicleType)GB(this->referenced_id, 16, 8);
}

/** Verify that the data stored a valid Colour value */
bool StoryPageButtonData::ValidateColour() const
{
	return openttd_rust_story_button_validate_colour(this->referenced_id);
}

bool StoryPageButtonData::ValidateFlags() const
{
	return openttd_rust_story_button_validate_flags(this->referenced_id);
}

/** Verify that the data stores a valid StoryPageButtonCursor value */
bool StoryPageButtonData::ValidateCursor() const
{
	return openttd_rust_story_button_validate_cursor(this->referenced_id);
}

/** Verity that the data stored a valid VehicleType value */
bool StoryPageButtonData::ValidateVehicleType() const
{
	return openttd_rust_story_button_validate_vehicle_type(this->referenced_id);
}

/**
 * Create a new story page.
 * @param flags type of operation
 * @param company Company for which this story page belongs to.
 * @param text Title of the story page. Null is allowed in which case a generic page title is provided by OpenTTD.
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, StoryPageID> CmdCreateStoryPage(DoCommandFlags flags, CompanyID company, const EncodedString &text)
{
	uint16_t result;
	bool success = openttd_rust_create_story_page(&GetRustObjectivesServices(), flags.base(), company.base(), &text, &result);
	return { success ? CommandCost() : CMD_ERROR, StoryPageID(result) };
}

/**
 * Create a new story page element.
 * @param flags type of operation
 * @param tile Tile location if it is a location page element, otherwise unused.
 * @param page_id The page which the element belongs to.
 * @param type Page element type
 * @param reference Id of referenced object
 * @param text Text content in case it is a text or location page element
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, StoryPageElementID> CmdCreateStoryPageElement(DoCommandFlags flags, TileIndex tile, StoryPageID page_id, StoryPageElementType type, uint32_t reference, const EncodedString &text)
{
	uint16_t result;
	bool success = openttd_rust_create_story_page_element(&GetRustObjectivesServices(), flags.base(), tile.base(), page_id.base(), type, reference, &text, &result);
	return { success ? CommandCost() : CMD_ERROR, StoryPageElementID(result) };
}

/**
 * Update a new story page element.
 * @param flags type of operation
 * @param tile Tile location if it is a location page element, otherwise unused.
 * @param page_element_id The page element to update.
 * @param reference Id of referenced object
 * @param text Text content in case it is a text or location page element
 * @return the cost of this operation or an error
 */
CommandCost CmdUpdateStoryPageElement(DoCommandFlags flags, TileIndex tile, StoryPageElementID page_element_id, uint32_t reference, const EncodedString &text)
{
	return openttd_rust_update_story_page_element(&GetRustObjectivesServices(), flags.base(), tile.base(), page_element_id.base(), reference, &text) ? CommandCost() : CMD_ERROR;
}

/**
 * Update title of a story page.
 * @param flags type of operation
 * @param page_id StoryPageID to update.
 * @param text title text of the story page.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetStoryPageTitle(DoCommandFlags flags, StoryPageID page_id, const EncodedString &text)
{
	return openttd_rust_set_story_page_title(&GetRustObjectivesServices(), flags.base(), page_id.base(), &text) ? CommandCost() : CMD_ERROR;
}

/**
 * Update date of a story page.
 * @param flags type of operation
 * @param page_id StoryPageID to update.
 * @param date date
 * @return the cost of this operation or an error
 */
CommandCost CmdSetStoryPageDate(DoCommandFlags flags, StoryPageID page_id, TimerGameCalendar::Date date)
{
	return openttd_rust_set_story_page_date(&GetRustObjectivesServices(), flags.base(), page_id.base(), date.base()) ? CommandCost() : CMD_ERROR;
}

/**
 * Display a story page for all clients that are allowed to
 * view the story page.
 * @param flags type of operation
 * @param page_id StoryPageID to show.
 * @return the cost of this operation or an error
 */
CommandCost CmdShowStoryPage(DoCommandFlags flags, StoryPageID page_id)
{
	return openttd_rust_show_story_page(&GetRustObjectivesServices(), flags.base(), page_id.base()) ? CommandCost() : CMD_ERROR;
}
/**
 * Remove a story page and associated story page elements.
 * @param flags type of operation
 * @param page_id StoryPageID to remove.
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveStoryPage(DoCommandFlags flags, StoryPageID page_id)
{
	return openttd_rust_remove_story_page(&GetRustObjectivesServices(), flags.base(), page_id.base()) ? CommandCost() : CMD_ERROR;
}

/**
 * Remove a story page element
 * @param flags type of operation
 * @param page_element_id StoryPageElementID to remove.
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveStoryPageElement(DoCommandFlags flags, StoryPageElementID page_element_id)
{
	return openttd_rust_remove_story_page_element(&GetRustObjectivesServices(), flags.base(), page_element_id.base()) ? CommandCost() : CMD_ERROR;
}

/**
 * Clicked/used a button on a story page.
 * @param flags  Type of operation.
 * @param tile   Tile selected, for tile selection buttons, otherwise unused.
 * @param page_element_id story page element id of button.
 * @param reference ID of selected item for buttons that select an item (e.g. vehicle), otherwise unused.
 * @return The cost of the operation, or an error.
 */
CommandCost CmdStoryPageButton(DoCommandFlags flags, TileIndex tile, StoryPageElementID page_element_id, VehicleID reference)
{
	return openttd_rust_story_page_button(&GetRustObjectivesServices(), flags.base(), tile.base(), page_element_id.base(), reference.base()) ? CommandCost() : CMD_ERROR;
}
