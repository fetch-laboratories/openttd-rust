/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Station helpers, rectangle modes and shared member readers from `base_station_base.h`.

use crate::station_type::BaseStation;
use crate::tile_type::TileIndex;
use std::sync::OnceLock;

/// Native `StationRect::StationRectMode`, retaining the original byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StationRectMode(pub u8);
impl StationRectMode {
    /// Test the spread without changing the rectangle.
    pub const ADD_TEST: Self = Self(0);
    /// Try to extend the rectangle within the station spread limit.
    pub const ADD_TRY: Self = Self(1);
    /// Extend the rectangle regardless of the station spread limit.
    pub const ADD_FORCE: Self = Self(2);
}

/// `BaseStation::IsInUse`: check whether the base station currently is in use;
/// in use means that it is not scheduled for deletion and that it still has
/// some facilities left.
pub(crate) fn is_in_use(facilities: crate::station_type::StationFacilities) -> bool {
    use crate::station_type::{StationFacilities, StationFacility};
    facilities.any(
        StationFacilities::from_element(StationFacility::Train)
            .with(StationFacilities::from_element(StationFacility::TruckStop))
            .with(StationFacilities::from_element(StationFacility::BusStop))
            .with(StationFacilities::from_element(StationFacility::Airport))
            .with(StationFacilities::from_element(StationFacility::Dock)),
    )
}

/// Native readers of `BaseStation` members that Rust code shares, bound once for the
/// process, as `news_func.rs` binds its table. Each takes the native pool shell of a live
/// base station; every caller holds such a shell for the duration of the call.
#[repr(C)]
pub struct Services {
    /// `BaseStation::xy`.
    pub xy: extern "C" fn(*const BaseStation) -> u32,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDBaseStationServices" ["station_service"] => Services { xy };
}

extern "C" fn unbound_xy(_: *const BaseStation) -> u32 {
    std::process::abort()
}
/// Services before the native table binds; each call aborts.
static UNBOUND: Services = Services { xy: unbound_xy };
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
pub unsafe extern "C" fn openttd_rust_base_station_bind(services: *const Services) {
    // SAFETY: This function's contract; the table binds once.
    let _ = SERVICES.set(unsafe { &*services });
}

/// The bound table, or the aborting services before the native table binds.
fn services() -> &'static Services {
    SERVICES.get().copied().unwrap_or(&UNBOUND)
}

/// `BaseStation::xy`: base tile of the station of the live native shell `st`.
pub(crate) fn xy(st: *const BaseStation) -> TileIndex {
    TileIndex((services().xy)(st))
}

/// A test binding of the shared table: `xy` calls the reader the current thread registered.
#[cfg(test)]
pub(crate) mod testing {
    use super::{BaseStation, Services};
    use std::cell::Cell;
    use std::sync::Once;

    /// A test thread's reader of `BaseStation::xy`.
    type Reader = fn(*const BaseStation) -> u32;
    thread_local! {
        static XY: Cell<Option<Reader>> = const { Cell::new(None) };
    }
    extern "C" fn xy(st: *const BaseStation) -> u32 {
        let reader = XY
            .with(Cell::get)
            .expect("no BaseStation::xy reader registered");
        reader(st)
    }
    static RECORDING: Services = Services { xy };

    /// Bind the test table once for the process and register this thread's `xy` reader.
    #[allow(unsafe_code, reason = "binding the test table for the process")]
    pub(crate) fn set_xy(reader: Reader) {
        static BIND: Once = Once::new();
        BIND.call_once(|| unsafe { super::openttd_rust_base_station_bind(&raw const RECORDING) });
        XY.with(|x| x.set(Some(reader)));
    }
}
