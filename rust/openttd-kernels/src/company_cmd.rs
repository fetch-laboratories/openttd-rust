/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared company-command helpers from `company_cmd.cpp`.
use crate::company_base::CompanyInfrastructure;
use crate::road_type::RoadTramType;

/// Get total sum of all owned road bits.
/// `rtt` is the `RoadTramType` to get the combined total for.
///
/// # Safety
/// The native record is live and serialized throughout this non-reentrant calculation.
#[allow(
    unsafe_code,
    reason = "Read each source-selected road counter from the live native CompanyInfrastructure without retaining a Rust borrow."
)]
pub(crate) unsafe fn get_road_tram_total(
    this: *const CompanyInfrastructure,
    rtt: RoadTramType,
) -> u32 {
    let mut total = 0_u32;
    let mut mask = crate::road::availability::get_mask_for_road_tram_type(rtt);
    while mask != 0 {
        let rt = mask.trailing_zeros() as usize;
        mask &= mask - 1;
        // SAFETY: The canonical road/tram mask contains source-valid road types;
        // raw pointer indexing preserves the native array subscript contract.
        total = total.wrapping_add(unsafe {
            std::ptr::addr_of!((*this).road)
                .cast::<u32>()
                .add(rt)
                .read()
        });
    }
    total
}
