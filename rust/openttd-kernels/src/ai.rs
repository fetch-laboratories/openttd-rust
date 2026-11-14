/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Base functions for all AIs (`ai/ai.hpp`).
//!
//! `AI::NewEvent` and `AI::BroadcastNewEvent` stay native until the script API port: they
//! hold the network and company checks, the company backup and the active-instance scope.

use crate::company_type::CompanyID;
use crate::script_event::services;
use crate::script_event_types::ScriptEvent;

/// `AI::NewEvent(company, new ScriptEvent...(...))`: queue a new event for an AI.
pub(crate) fn new_event(company: CompanyID, event: &ScriptEvent) {
    (services().ai_new)(company.0, event);
}

/// `AI::BroadcastNewEvent(new ScriptEvent...(...), skip_company)`: broadcast a new event to
/// all active AIs; pass `CompanyID::INVALID` for the original's default `skip_company`.
pub(crate) fn broadcast_new_event(event: &ScriptEvent, skip_company: CompanyID) {
    (services().ai_broadcast)(event, skip_company.0);
}
