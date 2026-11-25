/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `GetPrice` of `economy.cpp` and `ScaleByInverseCargoScale` of `economy_func.h`.
//! A setting address is a process-lifetime native field, read under the game-state mutex.
use crate::economy_type::Price;
use crate::newgrf::GRFFileRef;
use crate::services::Services;
use crate::settings_type::SettingAddresses;
use std::ffi::c_void;

/// Determine a certain price.
/// `index` is the price base, `cost_factor` the price factor, `grf_file` the `NewGRF` to use
/// local price multipliers from (null for none) and `shift` the extra bit shifting after the
/// computation. Returns the price.
pub(crate) fn get_price(
    shared: &Services,
    index: Price,
    cost_factor: u32,
    grf_file: Option<GRFFileRef<'_>>,
    mut shift: i32,
) -> i64 {
    if index >= Price::PR_END {
        return 0;
    }
    let cost = multiply_price(index, cost_factor);
    if let Some(grf_file) = grf_file {
        shift = shift.wrapping_add((shared.grf_price_shift)(
            grf_file.as_ptr().cast(),
            u32::from(index.0),
        ));
    }
    shift_price(cost, shift)
}

/// Raw adapter of `get_price` for the unfinished callers that read the file's multiplier
/// themselves: `grf_shift` is the file's `price_base_multipliers[index]`, zero without one.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Below `PR_END`, the index is a `Price`, an enum over `uint8_t`."
)]
pub(crate) fn get_price_raw(index: u32, cost_factor: u32, grf_shift: i32, shift: i32) -> i64 {
    if index >= u32::from(Price::PR_END.0) {
        return 0;
    }
    let cost = multiply_price(Price(index as u8), cost_factor);
    shift_price(cost, shift.wrapping_add(grf_shift))
}

/// `_price[index] * cost_factor`: `Money` is an `OverflowSafeInt64`, whose `uint` product
/// saturates.
#[expect(
    clippy::cast_possible_wrap,
    reason = "`OverflowSafeInt::operator *(const uint)` passes `(int64_t)factor` to `operator *=(const int)`, which narrows it to `int`."
)]
fn multiply_price(index: Price, cost_factor: u32) -> i64 {
    let factor = cost_factor as i32;
    crate::company::price(usize::from(index.0)).saturating_mul(i64::from(factor))
}

/// The shift that ends `GetPrice`: left for a non-negative `shift`, else arithmetic right.
fn shift_price(cost: i64, shift: i32) -> i64 {
    if shift >= 0 {
        cost.wrapping_shl(shift.unsigned_abs())
    } else {
        cost >> (-shift)
    }
}

/// `GetPrice` for its native callers.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute and the caller's file.
pub extern "C" fn openttd_rust_economy_get_price(
    shared: &Services,
    index: u8,
    cost_factor: u32,
    grf_file: *const c_void,
    shift: i32,
) -> i64 {
    // SAFETY: A native caller's file is live for the call.
    let grf_file = unsafe { GRFFileRef::new(shared, grf_file.cast()) };
    get_price(shared, Price(index), cost_factor, grf_file, shift)
}

/// `ScaleByInverseCargoScale` for its native caller, `ScaleByCargoScale` (`economy_func.h`).
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_economy_scale_by_inverse_cargo_scale(
    shared: &Services,
    num: u32,
    town: bool,
) -> u32 {
    scale_by_inverse_cargo_scale(shared.settings(), num, town)
}

/// Scale a number by the inverse of the cargo scale setting, e.g. a scale of 25% multiplies
/// the number by 4. `town` says whether town or industry production is scaled. Returns the
/// number scaled by the inverse of the cargo scale setting, minimum of 1.
#[allow(unsafe_code)] // The two setting reads.
pub(crate) fn scale_by_inverse_cargo_scale(
    settings: &SettingAddresses,
    num: u32,
    town: bool,
) -> u32 {
    // SAFETY: The module contract: both addresses are live native settings.
    let percentage = unsafe {
        if town {
            settings.town_cargo_scale.read()
        } else {
            settings.industry_cargo_scale.read()
        }
    };
    // We might not need to do anything.
    if percentage == 100 {
        return num;
    }
    // Never return 0, since we often divide by this number.
    (num.wrapping_mul(100) / u32::from(percentage)).max(1)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::test_tables::SHARED;

    extern "C" fn file_shift(file: *const c_void, index: u32) -> i32 {
        assert_eq!(index, u32::from(Price::PR_BUILD_OBJECT.0));
        assert!(!file.is_null());
        -3
    }
    extern "C" fn no_shift(_: *const c_void, _: u32) -> i32 {
        panic!("GetPrice reads no multiplier of a null file or at PR_END")
    }

    /// A shared table whose only `GRFFile` reader, `grf_price_shift`, is one of this module's
    /// stubs, which receive a record's address and never read it.
    struct StubTable(Services);

    fn shared(grf_price_shift: extern "C" fn(*const c_void, u32) -> i32) -> StubTable {
        let mut shared = SHARED;
        shared.grf_price_shift = grf_price_shift;
        StubTable(shared)
    }

    /// The record the multiplier stubs stand for; they receive only its address.
    static RECORD: u8 = 0;
    /// The handle of `RECORD` over a stub table.
    #[allow(unsafe_code)] // A live stand-in record the stub readers never read.
    fn stub_file(table: &StubTable) -> GRFFileRef<'_> {
        // SAFETY: `RECORD` lives for the process, and the table's only `GRFFile` reader is a stub
        // that does not dereference it (`GRFFileRef::of`'s stand-in case).
        unsafe { GRFFileRef::of(&table.0, (&raw const RECORD).cast()) }
    }

    /// Store `_price[index]` and return its earlier value, under `GLOBAL_LOCK`.
    #[allow(unsafe_code)] // One store to the process-lifetime price array.
    pub(crate) fn set_price(index: Price, value: i64) -> i64 {
        let earlier = crate::company::price(usize::from(index.0));
        // SAFETY: The caller holds `GLOBAL_LOCK`, which serializes the price array.
        unsafe {
            crate::company::openttd_rust_economy_prices()
                .add(usize::from(index.0))
                .write(value);
        }
        earlier
    }

    /// `economy.cpp`: `index >= PR_END` returns 0 before `_price` or the file is read.
    #[test]
    fn get_price_end_returns_zero_without_the_file() {
        let shared = shared(no_shift);
        let file = stub_file(&shared);
        assert_eq!(get_price(&shared.0, Price::PR_END, 1, Some(file), 0), 0);
        assert_eq!(
            get_price(&shared.0, Price(Price::PR_END.0 + 1), 1, Some(file), 0),
            0
        );
        assert_eq!(get_price_raw(u32::from(Price::PR_END.0), 1, 0, 0), 0);
    }

    /// A null file adds nothing; a file's multiplier adds to `shift`, and a negative total
    /// shifts right arithmetically.
    #[test]
    fn get_price_shifts_by_the_file_multiplier() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let earlier = set_price(Price::PR_BUILD_OBJECT, -800);
        let none = shared(no_shift);
        assert_eq!(
            get_price(&none.0, Price::PR_BUILD_OBJECT, 3, None, 2),
            -9600
        );
        assert_eq!(
            get_price(&none.0, Price::PR_BUILD_OBJECT, 3, None, -4),
            -150
        );
        let with = shared(file_shift);
        let file = stub_file(&with);
        assert_eq!(
            get_price(&with.0, Price::PR_BUILD_OBJECT, 3, Some(file), 2),
            -1200
        );
        assert_eq!(
            get_price_raw(u32::from(Price::PR_BUILD_OBJECT.0), 3, -3, 2),
            -1200
        );
        set_price(Price::PR_BUILD_OBJECT, earlier);
    }

    /// The `uint` factor narrows to `int` and the product saturates as `OverflowSafeInt64`.
    #[test]
    fn get_price_multiplies_as_overflow_safe_int() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let earlier = set_price(Price::PR_BUILD_OBJECT, i64::MAX / 2);
        let none = shared(no_shift);
        assert_eq!(
            get_price(&none.0, Price::PR_BUILD_OBJECT, 3, None, 0),
            i64::MAX
        );
        set_price(Price::PR_BUILD_OBJECT, 5);
        assert_eq!(
            get_price(&none.0, Price::PR_BUILD_OBJECT, u32::MAX, None, 0),
            -5
        );
        set_price(Price::PR_BUILD_OBJECT, earlier);
    }

    /// `ScaleByInverseCargoScale` reads the town or the industry setting by `town`, returns
    /// `num` at 100% and never less than 1.
    #[test]
    fn scale_by_inverse_cargo_scale_reads_the_selected_setting() {
        let town_scale: u16 = 25;
        let industry_scale: u16 = 400;
        let mut settings = crate::settings_type::SettingAddresses::NULL;
        settings.town_cargo_scale = &raw const town_scale;
        settings.industry_cargo_scale = &raw const industry_scale;
        assert_eq!(scale_by_inverse_cargo_scale(&settings, 256, true), 1024);
        assert_eq!(scale_by_inverse_cargo_scale(&settings, 256, false), 64);
        assert_eq!(scale_by_inverse_cargo_scale(&settings, 3, false), 1);
        let hundred: u16 = 100;
        settings.town_cargo_scale = &raw const hundred;
        assert_eq!(scale_by_inverse_cargo_scale(&settings, 0, true), 0);
    }
}
