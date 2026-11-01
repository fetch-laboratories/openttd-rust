/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file league_cmd.cpp Handling of league tables. */

#include "stdafx.h"
#include "league_cmd.h"
#include "league_base.h"
#include "command_type.h"
#include "command_func.h"
#include "industry.h"
#include "story_base.h"
#include "town.h"
#include "window_func.h"
#include "core/pool_func.hpp"

#include "rust/services_objectives.h"

#include "safeguards.h"

LeagueTableElementPool _league_table_element_pool("LeagueTableElement");
INSTANTIATE_POOL_METHODS(LeagueTableElement)

LeagueTablePool _league_table_pool("LeagueTable");
INSTANTIATE_POOL_METHODS(LeagueTable)

/**
 * Checks whether a link is valid, i.e. has a valid target.
 * @param link the link to check
 * @return true iff the link is valid
 */
bool IsValidLink(Link link)
{
	return openttd_rust_valid_link(&GetRustObjectivesServices(), link.type, link.target);
}

/**
 * Create a new league table.
 * @param flags type of operation
 * @param title Title of the league table
 * @param header Text to show above the table
 * @param footer Text to show below the table
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, LeagueTableID> CmdCreateLeagueTable(DoCommandFlags flags, const EncodedString &title, const EncodedString &header, const EncodedString &footer)
{
	uint8_t result;
	bool success = openttd_rust_create_league_table(&GetRustObjectivesServices(), flags.base(), &title, &header, &footer, &result);
	return { success ? CommandCost() : CMD_ERROR, LeagueTableID(result) };
}


/**
 * Create a new element in a league table.
 * @param flags type of operation
 * @param table Id of the league table this element belongs to
 * @param rating Value that elements are ordered by
 * @param company Company to show the colour blob for or CompanyID::Invalid()
 * @param text Text of the element
 * @param score String representation of the score associated with the element
 * @param link_type Type of the referenced object
 * @param link_target Id of the referenced object
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, LeagueTableElementID> CmdCreateLeagueTableElement(DoCommandFlags flags, LeagueTableID table, int64_t rating, CompanyID company, const EncodedString &text, const EncodedString &score, LinkType link_type, LinkTargetID link_target)
{
	uint16_t result;
	bool success = openttd_rust_create_league_table_element(&GetRustObjectivesServices(), flags.base(), table.base(), rating, company.base(), &text, &score, link_type, link_target, &result);
	return { success ? CommandCost() : CMD_ERROR, LeagueTableElementID(result) };
}

/**
 * Update the attributes of a league table element.
 * @param flags type of operation
 * @param element Id of the element to update
 * @param company Company to show the colour blob for or CompanyID::Invalid()
 * @param text Text of the element
 * @param link_type Type of the referenced object
 * @param link_target Id of the referenced object
 * @return the cost of this operation or an error
 */
CommandCost CmdUpdateLeagueTableElementData(DoCommandFlags flags, LeagueTableElementID element, CompanyID company, const EncodedString &text, LinkType link_type, LinkTargetID link_target)
{
	return openttd_rust_update_league_table_element_data(&GetRustObjectivesServices(), flags.base(), element.base(), company.base(), &text, link_type, link_target) ? CommandCost() : CMD_ERROR;
}

/**
 * Update the score of a league table element.
 * @param flags type of operation
 * @param element Id of the element to update
 * @param rating Value that elements are ordered by
 * @param score String representation of the score associated with the element
 * @return the cost of this operation or an error
 */
CommandCost CmdUpdateLeagueTableElementScore(DoCommandFlags flags, LeagueTableElementID element, int64_t rating, const EncodedString &score)
{
	return openttd_rust_update_league_table_element_score(&GetRustObjectivesServices(), flags.base(), element.base(), rating, &score) ? CommandCost() : CMD_ERROR;
}

/**
 * Remove a league table element.
 * @param flags type of operation
 * @param element Id of the element to update
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveLeagueTableElement(DoCommandFlags flags, LeagueTableElementID element)
{
	return openttd_rust_remove_league_table_element(&GetRustObjectivesServices(), flags.base(), element.base()) ? CommandCost() : CMD_ERROR;
}
