/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file signs_cmd.cpp Handling of sign related commands. */

#include "stdafx.h"
#include "landscape.h"
#include "company_func.h"
#include "signs_base.h"
#include "signs_func.h"
#include "command_func.h"
#include "tilehighlight_func.h"
#include "viewport_kdtree.h"
#include "window_func.h"
#include "string_func.h"
#include "signs_cmd.h"
#include "rust/command_cost_services.h"
#include "rust/services_ffi.h"
#include "rust/services_objectives.h"
#include "rust/signs_ffi.h"

#include "table/strings.h"

#include "safeguards.h"

static_assert(MAX_LENGTH_SIGN_NAME_CHARS == 32);

static void RustSignRemove(Sign *si) noexcept
{
	si->sign.MarkDirty();
	if (si->sign.kdtree_valid) _viewport_sign_kdtree.Remove(ViewportSignKdtreeItem::MakeSign(si->index));
	delete si;

}

const OpenTTDSignServices &GetRustSignServices() noexcept
{
	static const OpenTTDSignServices services{
		.shared = &GetRustSharedServices(),
		.costs = &GetRustCommandCostServices(),
		.create = []() noexcept { return (new Sign())->record.get(); },
		.position = [](Sign *si) noexcept { si->UpdateVirtCoord(); },
		.list_changed = [](int32_t data) noexcept { GetRustObjectivesServices().window_data(WC_SIGN_LIST, 0, data); },
		.remove = RustSignRemove,
		.close_rename = [](uint16_t id) noexcept { DeleteRenameSignWindow(SignID{id}); },
		.too_many_signs = STR_ERROR_TOO_MANY_SIGNS,
	};
	return services;
}

static OpenTTDByteSpan RustSpan(const std::string &text) noexcept { return {reinterpret_cast<const uint8_t *>(text.data()), text.size()}; }

/**
 * Place a sign at the given coordinates. Ownership of sign has
 * no effect whatsoever except for the colour the sign gets for easy recognition,
 * but everybody is able to rename/remove it.
 * @param tile tile to place sign at
 * @param flags type of operation
 * @param text contents of the sign
 * @return the cost of this operation + the ID of the new sign or an error
 */
std::tuple<CommandCost, SignID> CmdPlaceSign(DoCommandFlags flags, TileIndex tile, const std::string &text)
{
	CommandCost cost;
	SignID id{openttd_rust_sign_place(&GetRustSignServices(), RustCommandCost(&cost), flags.base(), tile.base(), RustSpan(text))};
	return { std::move(cost), id };
}

/**
 * Rename a sign. If the new name of the sign is empty, we assume
 * the user wanted to delete it. So delete it. Ownership of signs
 * has no meaning/effect whatsoever except for eyecandy
 * @param flags type of operation
 * @param sign_id index of the sign to be renamed/removed
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameSign(DoCommandFlags flags, SignID sign_id, const std::string &text)
{
	CommandCost cost;
	openttd_rust_sign_rename(&GetRustSignServices(), RustCommandCost(&cost), flags.base(), sign_id.base(), RustSpan(text));
	return cost;
}

/**
 * Move a sign to the given coordinates. Ownership of signs
 * has no meaning/effect whatsoever except for eyecandy.
 * @param flags type of operation
 * @param sign_id index of the sign to be moved
 * @param tile tile to place the sign at
 * @return the cost of this operation or an error
 */
CommandCost CmdMoveSign(DoCommandFlags flags, SignID sign_id, TileIndex tile)
{
	CommandCost cost;
	openttd_rust_sign_move(&GetRustSignServices(), RustCommandCost(&cost), flags.base(), sign_id.base(), tile.base());
	return cost;
}

/**
 * Callback function that is called after a sign is placed
 * @param result of the operation
 * @param new_sign ID of the placed sign.
 */
void CcPlaceSign(Commands, const CommandCost &result, SignID new_sign)
{
	if (result.Failed()) return;

	ShowRenameSignWindow(Sign::Get(new_sign));
	ResetObjectToPlace();
}

/**
 *
 * PlaceProc function, called when someone pressed the button if the
 *  sign-tool is selected
 * @param tile on which to place the sign
 */
void PlaceProc_Sign(TileIndex tile)
{
	Command<CMD_PLACE_SIGN>::Post(STR_ERROR_CAN_T_PLACE_SIGN_HERE, CcPlaceSign, tile, {});
}
