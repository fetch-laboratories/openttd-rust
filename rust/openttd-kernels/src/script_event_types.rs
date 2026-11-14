/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The detailed types of events (`script/api/script_event_types.hpp`).
//!
//! An event crosses as one [`ScriptEvent`] record: its type and the values its class holds.
//! The native factory constructs the class the type names; one constructor below per class
//! fills the fields that class takes.

use crate::company_type::CompanyID;
use crate::industry_type::IndustryID;
use crate::script_event::ScriptEventType;
use crate::station_type::StationID;
use crate::subsidy_type::SubsidyID;
use crate::tile_type::TileIndex;
use crate::town_type::TownId;
use crate::vehicle_type::VehicleID;

/// The reasons for vehicle crashes (`ScriptEventVehicleCrashed::CrashReason`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrashReason(pub i32);
impl CrashReason {
    /// Plane crashed on landing
    pub const CRASH_PLANE_LANDING: Self = Self(3);
    /// Aircraft crashed after it found not a single airport for landing
    pub const CRASH_AIRCRAFT_NO_AIRPORT: Self = Self(4);
}

crate::abi::shared_records! {
    fn abi_record;
    /// One `ScriptEvent` to construct natively: its type and the constructor arguments of
    /// its class, as base integers. Fields the class does not take are zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    "OpenTTDScriptEvent" ["script_event"] => pub struct ScriptEvent {
        /// `ScriptEvent::GetEventType`.
        pub r#type: ScriptEventType,
        /// A vehicle.
        pub vehicle: u32,
        /// A tile.
        pub tile: u32,
        /// `ScriptEventVehicleCrashed::CrashReason`.
        pub crash_reason: CrashReason,
        /// Number of victims.
        pub victims: u32,
        /// A station.
        pub station: u16,
        /// A subsidy.
        pub subsidy: u16,
        /// An industry.
        pub industry: u16,
        /// A town.
        pub town: u16,
        /// A company.
        pub company: u8,
    }
}

impl ScriptEvent {
    const fn of(r#type: ScriptEventType) -> Self {
        Self {
            r#type,
            vehicle: 0,
            tile: 0,
            crash_reason: CrashReason(0),
            victims: 0,
            station: 0,
            subsidy: 0,
            industry: 0,
            town: 0,
            company: 0,
        }
    }

    /// `ScriptEventVehicleCrashed(vehicle, crash_site, crash_reason, victims, company)`.
    #[must_use]
    pub const fn vehicle_crashed(
        vehicle: VehicleID,
        crash_site: TileIndex,
        crash_reason: CrashReason,
        victims: u32,
        company: CompanyID,
    ) -> Self {
        Self {
            vehicle: vehicle.0,
            tile: crash_site.0,
            crash_reason,
            victims,
            company: company.0,
            ..Self::of(ScriptEventType::ET_VEHICLE_CRASHED)
        }
    }

    /// `ScriptEventSubsidyOffer(subsidy_id)`.
    #[must_use]
    pub const fn subsidy_offer(subsidy_id: SubsidyID) -> Self {
        Self {
            subsidy: subsidy_id.0,
            ..Self::of(ScriptEventType::ET_SUBSIDY_OFFER)
        }
    }

    /// `ScriptEventSubsidyOfferExpired(subsidy_id)`.
    #[must_use]
    pub const fn subsidy_offer_expired(subsidy_id: SubsidyID) -> Self {
        Self {
            subsidy: subsidy_id.0,
            ..Self::of(ScriptEventType::ET_SUBSIDY_OFFER_EXPIRED)
        }
    }

    /// `ScriptEventSubsidyAwarded(subsidy_id)`.
    #[must_use]
    pub const fn subsidy_awarded(subsidy_id: SubsidyID) -> Self {
        Self {
            subsidy: subsidy_id.0,
            ..Self::of(ScriptEventType::ET_SUBSIDY_AWARDED)
        }
    }

    /// `ScriptEventSubsidyExpired(subsidy_id)`.
    #[must_use]
    pub const fn subsidy_expired(subsidy_id: SubsidyID) -> Self {
        Self {
            subsidy: subsidy_id.0,
            ..Self::of(ScriptEventType::ET_SUBSIDY_EXPIRED)
        }
    }

    /// `ScriptEventIndustryOpen(industry_id)`.
    #[must_use]
    pub const fn industry_open(industry_id: IndustryID) -> Self {
        Self {
            industry: industry_id.0,
            ..Self::of(ScriptEventType::ET_INDUSTRY_OPEN)
        }
    }

    /// `ScriptEventIndustryClose(industry_id)`.
    #[must_use]
    pub const fn industry_close(industry_id: IndustryID) -> Self {
        Self {
            industry: industry_id.0,
            ..Self::of(ScriptEventType::ET_INDUSTRY_CLOSE)
        }
    }

    /// `ScriptEventStationFirstVehicle(station, vehicle)`.
    #[must_use]
    pub const fn station_first_vehicle(station: StationID, vehicle: VehicleID) -> Self {
        Self {
            station: station.0,
            vehicle: vehicle.0,
            ..Self::of(ScriptEventType::ET_STATION_FIRST_VEHICLE)
        }
    }

    /// `ScriptEventTownFounded(town)`.
    #[must_use]
    pub const fn town_founded(town: TownId) -> Self {
        Self {
            town: town.0,
            ..Self::of(ScriptEventType::ET_TOWN_FOUNDED)
        }
    }

    /// `ScriptEventAircraftDestTooFar(vehicle_id)`.
    #[must_use]
    pub const fn aircraft_dest_too_far(vehicle_id: VehicleID) -> Self {
        Self {
            vehicle: vehicle_id.0,
            ..Self::of(ScriptEventType::ET_AIRCRAFT_DEST_TOO_FAR)
        }
    }

    /// `ScriptEventExclusiveTransportRights(company, town)`.
    #[must_use]
    pub const fn exclusive_transport_rights(company: CompanyID, town: TownId) -> Self {
        Self {
            company: company.0,
            town: town.0,
            ..Self::of(ScriptEventType::ET_EXCLUSIVE_TRANSPORT_RIGHTS)
        }
    }

    /// `ScriptEventRoadReconstruction(company, town)`.
    #[must_use]
    pub const fn road_reconstruction(company: CompanyID, town: TownId) -> Self {
        Self {
            company: company.0,
            town: town.0,
            ..Self::of(ScriptEventType::ET_ROAD_RECONSTRUCTION)
        }
    }
}
