/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Base functions for all Games (`game/game.hpp`).
//!
//! `Game::NewEvent` stays native until the script API port.

use crate::script_event::services;
use crate::script_event_types::ScriptEvent;

/// `Game::NewEvent(new ScriptEvent...(...))`: queue a new event for a Game Script.
pub(crate) fn new_event(event: &ScriptEvent) {
    (services().game_new_event)(event);
}
