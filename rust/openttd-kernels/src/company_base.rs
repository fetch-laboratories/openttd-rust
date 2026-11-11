/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Live native company infrastructure from `company_base.h`.
//! The company pool owns this record. Raw accesses are serialized and finish before callbacks;
//! neither a Rust reference nor a copy of the record is retained across reentry.
use crate::company_type::CompanyID;
use crate::rail_type::{RAILTYPE_END, RailType};
use crate::raw_field::read_field;
use crate::road_type::{ROADTYPE_END, RoadTramType};

/// Counts of company-owned infrastructure, at its original native address.
#[repr(C)]
pub struct CompanyInfrastructure {
    /// Count of company owned track bits for each rail type.
    pub rail: [u32; RAILTYPE_END as usize],
    /// Count of company owned track bits for each road type.
    pub road: [u32; ROADTYPE_END as usize],
    /// Count of company owned signals.
    pub signal: u32,
    /// Count of company owned track bits for canals.
    pub water: u32,
    /// Count of company owned station tiles.
    pub station: u32,
    /// Count of company owned airports.
    pub airport: u32,
}

#[allow(
    unsafe_code,
    reason = "Access the live native CompanyInfrastructure with field-sized raw loads and stores, ending each access before callbacks."
)]
impl CompanyInfrastructure {
    /// Resolve `Company::Get(id)->infrastructure` once at the original entry point.
    pub(crate) fn get(shared: &crate::services::Services, id: CompanyID) -> *mut Self {
        (shared.company_infrastructure)(id.0)
    }
    /// Add a wrapping count to the original rail field.
    ///
    /// # Safety
    /// `this` points to live, writable, serialized native infrastructure and `kind` meets the original array subscript precondition.
    pub(crate) unsafe fn add_rail(this: *mut Self, kind: crate::rail_type::RailType, delta: u32) {
        // SAFETY: The caller preserves native lifetime, serialization and indexing contracts.
        unsafe {
            let field = std::ptr::addr_of_mut!((*this).rail)
                .cast::<u32>()
                .add(usize::from(kind.0));
            field.write(field.read().wrapping_add(delta));
        }
    }
    /// Add a wrapping count to the original road field.
    ///
    /// # Safety
    /// `this` points to live, writable, serialized native infrastructure and `kind` meets the original array subscript precondition.
    pub(crate) unsafe fn add_road(this: *mut Self, kind: crate::road_type::RoadType, delta: u32) {
        // SAFETY: The caller preserves native lifetime, serialization and indexing contracts.
        unsafe {
            let field = std::ptr::addr_of_mut!((*this).road)
                .cast::<u32>()
                .add(usize::from(kind.0));
            field.write(field.read().wrapping_add(delta));
        }
    }
    /// Add a wrapping count to the original signal field.
    ///
    /// # Safety
    /// `this` points to live, writable, serialized native infrastructure.
    pub(crate) unsafe fn add_signal(this: *mut Self, delta: u32) {
        // SAFETY: The caller preserves native lifetime, serialization and indexing contracts.
        unsafe {
            let field = std::ptr::addr_of_mut!((*this).signal);
            field.write(field.read().wrapping_add(delta));
        }
    }
    /// Add a wrapping count to the original station field.
    ///
    /// # Safety
    /// `this` points to live, writable, serialized native infrastructure.
    pub(crate) unsafe fn add_station(this: *mut Self, delta: u32) {
        // SAFETY: The caller preserves native lifetime, serialization and indexing contracts.
        unsafe {
            let field = std::ptr::addr_of_mut!((*this).station);
            field.write(field.read().wrapping_add(delta));
        }
    }
    /// Add a wrapping count to the original airport field.
    ///
    /// # Safety
    /// `this` points to live, writable, serialized native infrastructure.
    pub(crate) unsafe fn add_airport(this: *mut Self, delta: u32) {
        // SAFETY: The caller preserves native lifetime, serialization and indexing contracts.
        unsafe {
            let field = std::ptr::addr_of_mut!((*this).airport);
            field.write(field.read().wrapping_add(delta));
        }
    }
    /// Add a wrapping count to the original water field.
    ///
    /// # Safety
    /// `this` points to live, writable, serialized native infrastructure.
    pub(crate) unsafe fn add_water(this: *mut Self, delta: u32) {
        // SAFETY: The caller keeps the native record live and serialized; no reference escapes.
        unsafe {
            let field = std::ptr::addr_of_mut!((*this).water);
            field.write(field.read().wrapping_add(delta));
        }
    }
    /// Get total sum of all owned track bits (`GetRailTotal`).
    ///
    /// # Safety
    /// The native record is live and serialized throughout this non-reentrant calculation.
    pub(crate) unsafe fn get_rail_total(this: *const Self) -> u32 {
        (RailType::RAILTYPE_BEGIN.0..RAILTYPE_END).fold(0_u32, |total, rt| {
            total.wrapping_add(read_field!(this, rail[usize::from(rt)]))
        })
    }
    /// `GetRoadTotal`.
    ///
    /// # Safety
    /// The native record is live and serialized throughout this non-reentrant calculation.
    pub(crate) unsafe fn get_road_total(this: *const Self) -> u32 {
        unsafe { crate::company_cmd::get_road_tram_total(this, RoadTramType::RTT_ROAD) }
    }
    /// `GetTramTotal`.
    ///
    /// # Safety
    /// The native record is live and serialized throughout this non-reentrant calculation.
    pub(crate) unsafe fn get_tram_total(this: *const Self) -> u32 {
        unsafe { crate::company_cmd::get_road_tram_total(this, RoadTramType::RTT_TRAM) }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "CompanyInfrastructure" ["company_economy"] => CompanyInfrastructure {
        rail, road, signal, water, station, airport,
    };
}
