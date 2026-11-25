/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Everything to handle events from the game (`script/api/script_event.hpp`).

use crate::company_type::CompanyID;
use crate::script_event_types::ScriptEvent;
use std::sync::OnceLock;

/// The type of an event (`ScriptEvent::ScriptEventType`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptEventType(pub i32);
impl ScriptEventType {
    /// `ET_SUBSIDY_OFFER`.
    pub const ET_SUBSIDY_OFFER: Self = Self(2);
    /// `ET_SUBSIDY_OFFER_EXPIRED`.
    pub const ET_SUBSIDY_OFFER_EXPIRED: Self = Self(3);
    /// `ET_SUBSIDY_AWARDED`.
    pub const ET_SUBSIDY_AWARDED: Self = Self(4);
    /// `ET_SUBSIDY_EXPIRED`.
    pub const ET_SUBSIDY_EXPIRED: Self = Self(5);
    /// `ET_VEHICLE_CRASHED`.
    pub const ET_VEHICLE_CRASHED: Self = Self(12);
    /// `ET_VEHICLE_WAITING_IN_DEPOT`.
    pub const ET_VEHICLE_WAITING_IN_DEPOT: Self = Self(14);
    /// `ET_INDUSTRY_OPEN`.
    pub const ET_INDUSTRY_OPEN: Self = Self(16);
    /// `ET_INDUSTRY_CLOSE`.
    pub const ET_INDUSTRY_CLOSE: Self = Self(17);
    /// `ET_STATION_FIRST_VEHICLE`.
    pub const ET_STATION_FIRST_VEHICLE: Self = Self(19);
    /// `ET_TOWN_FOUNDED`.
    pub const ET_TOWN_FOUNDED: Self = Self(22);
    /// `ET_AIRCRAFT_DEST_TOO_FAR`.
    pub const ET_AIRCRAFT_DEST_TOO_FAR: Self = Self(23);
    /// `ET_EXCLUSIVE_TRANSPORT_RIGHTS`.
    pub const ET_EXCLUSIVE_TRANSPORT_RIGHTS: Self = Self(27);
    /// `ET_ROAD_RECONSTRUCTION`.
    pub const ET_ROAD_RECONSTRUCTION: Self = Self(28);
    /// `ET_COMPANY_RENAMED`.
    pub const ET_COMPANY_RENAMED: Self = Self(33);
    /// `ET_PRESIDENT_RENAMED`.
    pub const ET_PRESIDENT_RENAMED: Self = Self(34);
}

/// Native script event entries. The table is bound once for the process; each service
/// constructs the native event the record names and passes it to its original entry. The
/// slots are private to this module: a record reaches them only through the dispatches
/// below, which send a plain record without text and form a record with text only from a
/// live `ScriptTextEvent`.
#[repr(C)]
pub struct Services {
    /// `AI::BroadcastNewEvent(event, skip_company)`.
    ai_broadcast: extern "C" fn(*const ScriptEvent, u8),
    /// `AI::NewEvent(company, event)`.
    ai_new: extern "C" fn(u8, *const ScriptEvent),
    /// `Game::NewEvent(event)`.
    game_new: extern "C" fn(*const ScriptEvent),
}

#[cfg(test)]
impl Services {
    /// A table over the given entries, for the test recorders.
    pub(crate) const fn new(
        ai_broadcast: extern "C" fn(*const ScriptEvent, u8),
        ai_new: extern "C" fn(u8, *const ScriptEvent),
        game_new: extern "C" fn(*const ScriptEvent),
    ) -> Self {
        Self {
            ai_broadcast,
            ai_new,
            game_new,
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDScriptEventServices" ["script_event"] => Services {
        ai_broadcast,
        ai_new,
        game_new,
    };
}

extern "C" fn unbound_ai_broadcast_new_event(_: *const ScriptEvent, _: u8) {
    std::process::abort()
}
extern "C" fn unbound_ai_new_event(_: u8, _: *const ScriptEvent) {
    std::process::abort()
}
extern "C" fn unbound_game_new_event(_: *const ScriptEvent) {
    std::process::abort()
}
/// Services before the native table binds; each call aborts.
static UNBOUND: Services = Services {
    ai_broadcast: unbound_ai_broadcast_new_event,
    ai_new: unbound_ai_new_event,
    game_new: unbound_game_new_event,
};
static SERVICES: OnceLock<&'static Services> = OnceLock::new();

/// Bind the native table during static initialization.
///
/// # Safety
/// `services` lives for the process.
#[allow(
    unsafe_code,
    reason = "the binding export and its process-lifetime pointer contract"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_event_bind(services: *const Services) {
    // SAFETY: This function's contract; the table binds once.
    let _ = SERVICES.set(unsafe { &*services });
}

/// The bound table, or the aborting services before the native table binds.
fn services() -> &'static Services {
    SERVICES.get().copied().unwrap_or(&UNBOUND)
}

/// `AI::NewEvent(company, event)` of an event without a name.
pub(crate) fn ai_new(company: u8, event: &ScriptEvent) {
    (services().ai_new)(company, &event.without_text());
}

/// `AI::BroadcastNewEvent(event, skip_company)` of an event without a name.
pub(crate) fn ai_broadcast(event: &ScriptEvent, skip_company: u8) {
    (services().ai_broadcast)(&event.without_text(), skip_company);
}

/// `Game::NewEvent(event)` of an event without a name.
pub(crate) fn game_new(event: &ScriptEvent) {
    (services().game_new)(&event.without_text());
}

/// `AI::BroadcastNewEvent(event, skip_company)` of an event with a name: the native
/// factory copies the bytes during the call.
pub(crate) fn ai_broadcast_text(event: &ScriptTextEvent<'_>, skip_company: u8) {
    (services().ai_broadcast)(&event.record(), skip_company);
}

/// `Game::NewEvent(event)` of an event with a name: the native factory copies the bytes
/// during the call.
pub(crate) fn game_new_text(event: &ScriptTextEvent<'_>) {
    (services().game_new)(&event.record());
}

/// A script event carrying a name, `ScriptEventCompanyRenamed` or
/// `ScriptEventPresidentRenamed`. It holds the name's borrow; the raw record that points at
/// the bytes is formed only inside the synchronous dispatch (`ai::broadcast_new_text_event`,
/// `game::new_text_event`), so no record outlives them.
#[derive(Clone, Copy)]
pub struct ScriptTextEvent<'a> {
    event: ScriptEvent,
    text: &'a [u8],
}

impl<'a> ScriptTextEvent<'a> {
    /// `ScriptEventCompanyRenamed(company, new_name)`.
    #[must_use]
    pub const fn company_renamed(company: CompanyID, new_name: &'a [u8]) -> Self {
        Self {
            event: ScriptEvent {
                company: company.0,
                ..ScriptEvent::of(ScriptEventType::ET_COMPANY_RENAMED)
            },
            text: new_name,
        }
    }

    /// `ScriptEventPresidentRenamed(company, new_name)`.
    #[must_use]
    pub const fn president_renamed(company: CompanyID, new_name: &'a [u8]) -> Self {
        Self {
            event: ScriptEvent {
                company: company.0,
                ..ScriptEvent::of(ScriptEventType::ET_PRESIDENT_RENAMED)
            },
            text: new_name,
        }
    }

    /// The raw record borrowing the name, for this module's two text dispatches only;
    /// the plain dispatches clear `text`, so no record pointing at the name outlives it.
    const fn record(&self) -> ScriptEvent {
        ScriptEvent {
            text: self.text.as_ptr(),
            text_length: self.text.len(),
            ..self.event
        }
    }
}
