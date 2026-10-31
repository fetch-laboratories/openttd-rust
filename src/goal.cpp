/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file goal.cpp Handling of goals. */

#include "stdafx.h"
#include "company_func.h"
#include "industry.h"
#include "town.h"
#include "window_func.h"
#include "goal_base.h"
#include "core/pool_func.hpp"
#include "game/game.hpp"
#include "command_func.h"
#include "company_base.h"
#include "story_base.h"
#include "string_func.h"
#include "gui.h"
#include "network/network.h"
#include "network/network_base.h"
#include "network/network_func.h"
#include "goal_cmd.h"

#include "rust/services_objectives.h"

#include "safeguards.h"


GoalPool _goal_pool("Goal");
INSTANTIATE_POOL_METHODS(Goal)


/**
 * Create a new goal.
 * @param flags type of operation
 * @param company Company for which this goal is.
 * @param type GoalType of destination.
 * @param dest GoalTypeID of destination.
 * @param text Text of the goal.
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, GoalID> CmdCreateGoal(DoCommandFlags flags, CompanyID company, GoalType type, GoalTypeID dest, const EncodedString &text)
{
	uint16_t result;
	bool success = openttd_rust_create_goal(&GetRustObjectivesServices(), flags.base(), company.base(), type, dest, &text, &result);
	return { success ? CommandCost() : CMD_ERROR, GoalID(result) };
}

/**
 * Remove a goal.
 * @param flags type of operation
 * @param goal GoalID to remove.
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveGoal(DoCommandFlags flags, GoalID goal)
{
	return openttd_rust_remove_goal(&GetRustObjectivesServices(), flags.base(), goal.base()) ? CommandCost() : CMD_ERROR;
}

/**
 * Update goal destination of a goal.
 * @param flags type of operation
 * @param goal GoalID to update.
 * @param type GoalType of destination.
 * @param dest GoalTypeID of destination.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetGoalDestination(DoCommandFlags flags, GoalID goal, GoalType type, GoalTypeID dest)
{
	return openttd_rust_set_goal_destination(&GetRustObjectivesServices(), flags.base(), goal.base(), type, dest) ? CommandCost() : CMD_ERROR;
}

/**
 * Update goal text of a goal.
 * @param flags type of operation
 * @param goal GoalID to update.
 * @param text Text of the goal.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetGoalText(DoCommandFlags flags, GoalID goal, const EncodedString &text)
{
	return openttd_rust_set_goal_text(&GetRustObjectivesServices(), flags.base(), goal.base(), &text) ? CommandCost() : CMD_ERROR;
}

/**
 * Update progress text of a goal.
 * @param flags type of operation
 * @param goal GoalID to update.
 * @param text Progress text of the goal.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetGoalProgress(DoCommandFlags flags, GoalID goal, const EncodedString &text)
{
	return openttd_rust_set_goal_progress(&GetRustObjectivesServices(), flags.base(), goal.base(), &text) ? CommandCost() : CMD_ERROR;
}

/**
 * Update completed state of a goal.
 * @param flags type of operation
 * @param goal GoalID to update.
 * @param completed completed state of goal.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetGoalCompleted(DoCommandFlags flags, GoalID goal, bool completed)
{
	return openttd_rust_set_goal_completed(&GetRustObjectivesServices(), flags.base(), goal.base(), completed) ? CommandCost() : CMD_ERROR;
}

/**
 * Ask a goal related question
 * @param flags type of operation
 * @param uniqueid Unique ID to use for this question.
 * @param target Company or client for which this question is.
 * @param is_client Question target: false - company, true - client.
 * @param button_mask Buttons of the question.
 * @param type Question type.
 * @param text Text of the question.
 * @return the cost of this operation or an error
 */
CommandCost CmdGoalQuestion(DoCommandFlags flags, uint16_t uniqueid, uint32_t target, bool is_client, uint32_t button_mask, GoalQuestionType type, const EncodedString &text)
{
	return openttd_rust_goal_question(&GetRustObjectivesServices(), flags.base(), uniqueid, target, is_client, button_mask, type, &text) ? CommandCost() : CMD_ERROR;
}

/**
 * Reply to a goal question.
 * @param flags type of operation
 * @param uniqueid Unique ID to use for this question.
 * @param button Button the company pressed
 * @return the cost of this operation or an error
 */
CommandCost CmdGoalQuestionAnswer(DoCommandFlags flags, uint16_t uniqueid, uint8_t button)
{
	return openttd_rust_goal_question_answer(&GetRustObjectivesServices(), flags.base(), uniqueid, button) ? CommandCost() : CMD_ERROR;
}
