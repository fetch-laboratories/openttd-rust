/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Functions related to news (`news_func.h`).
//!
//! The inline wrappers of `news_func.h` are ported here; they call the native `AddNewsItem`
//! (`news_gui.cpp`), which owns the news list until the GUI is ported. The native table is
//! bound once for the process, as `debug.rs` binds its table; it also carries the string
//! services of `strings_func.rs` and the `CompanyNewsInformation` services of
//! `news_type.rs`. Every service is a synchronous `noexcept` call that holds no Rust borrow.
#![allow(unsafe_code)]

use crate::encoded::Descriptor;
use crate::industry_type::IndustryID;
use crate::news_type::{
    AdviceType, NewsAllocatedData, NewsFlag, NewsFlags, NewsReference, NewsReferenceRecord,
    NewsStyle, NewsType,
};
use crate::station_type::StationID;
use crate::strings_type::EncodedString;
use crate::tile_type::TileIndex;
use crate::vehicle_type::VehicleID;
use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Receives the bytes of a native `std::string` during the call that produced it.
pub type Sink = extern "C" fn(*mut c_void, *const u8, usize);

/// Native news and string services.
#[repr(C)]
pub struct Services {
    /// `AddNewsItem`.
    pub add_news_item: extern "C" fn(*const AddNewsItemArgs),
    /// The `WITH_ASSERT` check of `GetEncodedStringWithArgs` on a string parameter beginning
    /// with this codepoint, of this encoded length (`AssertRustEncoded`).
    pub assert_encoded: extern "C" fn(u32, usize),
    /// Sink `CompanyNewsInformation::company_name`.
    pub company_news_company_name: extern "C" fn(*const NewsAllocatedData, Sink, *mut c_void),
    /// `new CompanyNewsInformation(title, Company::Get(company), other)`; an invalid
    /// `other` passes `nullptr`.
    pub company_news_information: extern "C" fn(u32, u8, u8) -> *mut NewsAllocatedData,
    /// `DeleteIndustryNews`.
    pub delete_industry_news: extern "C" fn(u16),
    /// `DeleteStationNews`.
    pub delete_station_news: extern "C" fn(u16),
    /// `DeleteVehicleNews`.
    pub delete_vehicle_news: extern "C" fn(u32, u8),
    /// `GetStringWithArgs`, sinking the result.
    pub get_string: extern "C" fn(u32, *const Descriptor, usize, Sink, *mut c_void),
    /// One when the native build defines `WITH_ASSERT`.
    pub string_assertions: u8,
}
// SAFETY: An immutable table of native functions that run on the calling thread.
unsafe impl Sync for Services {}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNewsServices" ["news"] => Services { add_news_item, assert_encoded, company_news_company_name, company_news_information, delete_industry_news, delete_station_news, delete_vehicle_news, get_string, string_assertions };
}

crate::abi::shared_records! {
    fn abi_record;
    /// The parameters of `AddNewsItem`, in the original's order.
    "OpenTTDAddNewsItemArgs" ["news"] => pub struct AddNewsItemArgs {
        /// The encoded headline's bytes, borrowed for the call.
        pub headline: *const u8,
        /// Byte length of `headline`.
        pub headline_length: usize,
        /// Type of the news.
        pub r#type: NewsType,
        /// Window style for the news.
        pub style: NewsStyle,
        /// `NewsFlags` bits.
        pub flags: NewsFlags,
        /// Reference 1 to some object.
        pub ref1: NewsReferenceRecord,
        /// Reference 2 to some object.
        pub ref2: NewsReferenceRecord,
        /// Owned custom data, or null; the native news item takes ownership.
        pub data: *mut NewsAllocatedData,
        /// The type of advice, to be able to remove specific advices later on.
        pub advice_type: AdviceType,
    }
}

impl AddNewsItemArgs {
    /// The arguments of an item with this headline and the original's defaults: no
    /// references, no data and `AdviceType::Invalid`. The record borrows the headline.
    #[must_use]
    pub(crate) fn new(
        headline: &EncodedString,
        r#type: NewsType,
        style: NewsStyle,
        flags: NewsFlags,
    ) -> Self {
        let headline = headline.as_bytes();
        Self {
            headline: headline.as_ptr(),
            headline_length: headline.len(),
            r#type,
            style,
            flags,
            ref1: NewsReference::None.record(),
            ref2: NewsReference::None.record(),
            data: std::ptr::null_mut(),
            advice_type: AdviceType::Invalid,
        }
    }
}

extern "C" fn unbound_add_news_item(_: *const AddNewsItemArgs) {
    unreachable!("news services are not bound")
}
extern "C" fn unbound_assert_encoded(_: u32, _: usize) {
    unreachable!("news services are not bound")
}
extern "C" fn unbound_company_news_company_name(
    _: *const NewsAllocatedData,
    _: Sink,
    _: *mut c_void,
) {
    unreachable!("news services are not bound")
}
extern "C" fn unbound_company_news_information(_: u32, _: u8, _: u8) -> *mut NewsAllocatedData {
    unreachable!("news services are not bound")
}
extern "C" fn unbound_delete_id(_: u16) {
    unreachable!("news services are not bound")
}
extern "C" fn unbound_delete_vehicle_news(_: u32, _: u8) {
    unreachable!("news services are not bound")
}
extern "C" fn unbound_get_string(_: u32, _: *const Descriptor, _: usize, _: Sink, _: *mut c_void) {
    unreachable!("news services are not bound")
}
/// Services before the native table binds; each call aborts.
static UNBOUND: Services = Services {
    add_news_item: unbound_add_news_item,
    assert_encoded: unbound_assert_encoded,
    company_news_company_name: unbound_company_news_company_name,
    company_news_information: unbound_company_news_information,
    delete_industry_news: unbound_delete_id,
    delete_station_news: unbound_delete_id,
    delete_vehicle_news: unbound_delete_vehicle_news,
    get_string: unbound_get_string,
    string_assertions: 0,
};
static SERVICES: AtomicPtr<Services> = AtomicPtr::new((&raw const UNBOUND).cast_mut());

/// Bind the native table during static initialization.
///
/// # Safety
/// `services` lives for the process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_news_bind(services: *const Services) {
    SERVICES.store(services.cast_mut(), Ordering::Release);
}

/// The bound table.
pub(crate) fn services() -> &'static Services {
    // SAFETY: Either `UNBOUND` or a process-lifetime table from `openttd_rust_news_bind`.
    unsafe { &*SERVICES.load(Ordering::Acquire) }
}

/// `AddNewsItem`: add a new newsitem to be shown.
///
/// The headline the arguments borrow lives for the call; a non-null `data` passes its
/// ownership to the news item, which deletes it at its end of life.
pub(crate) fn add_news_item(args: &AddNewsItemArgs) {
    (services().add_news_item)(args);
}

/// The flags of the thin news items below.
const THIN_FLAGS: NewsFlags = NewsFlags::new(&[NewsFlag::NoTransparency, NewsFlag::Shaded]);

/// `station == StationID::Invalid() ? NewsReference{} : station`.
fn station_reference(station: StationID) -> NewsReference {
    if station == StationID::INVALID {
        NewsReference::None
    } else {
        NewsReference::Station(station)
    }
}

/// `AddVehicleNewsItem`: adds a newsitem referencing a vehicle; pass
/// `StationID::INVALID` for the original's default `station`.
///
/// The DParams may not reference the vehicle due to autoreplace stuff. See
/// [`add_vehicle_advice_news_item`] for how that can be done.
pub(crate) fn add_vehicle_news_item(
    headline: &EncodedString,
    r#type: NewsType,
    vehicle: VehicleID,
    station: StationID,
) {
    add_news_item(&AddNewsItemArgs {
        ref1: NewsReference::Vehicle(vehicle).record(),
        ref2: station_reference(station).record(),
        ..AddNewsItemArgs::new(headline, r#type, NewsStyle::Thin, THIN_FLAGS)
    });
}

/// `AddVehicleAdviceNewsItem`: adds a vehicle-advice news item.
///
/// DParam 0 must reference the vehicle!
pub(crate) fn add_vehicle_advice_news_item(
    advice_type: AdviceType,
    headline: &EncodedString,
    vehicle: VehicleID,
) {
    add_news_item(&AddNewsItemArgs {
        ref1: NewsReference::Vehicle(vehicle).record(),
        advice_type,
        ..AddNewsItemArgs::new(
            headline,
            NewsType::Advice,
            NewsStyle::Small,
            NewsFlags::new(&[NewsFlag::InColour, NewsFlag::VehicleParam0]),
        )
    });
}

/// `AddTileNewsItem`; pass `StationID::INVALID` for the original's default `station`.
pub(crate) fn add_tile_news_item(
    headline: &EncodedString,
    r#type: NewsType,
    tile: TileIndex,
    station: StationID,
) {
    add_news_item(&AddNewsItemArgs {
        ref1: NewsReference::Tile(tile).record(),
        ref2: station_reference(station).record(),
        ..AddNewsItemArgs::new(headline, r#type, NewsStyle::Thin, THIN_FLAGS)
    });
}

/// `AddIndustryNewsItem`.
pub(crate) fn add_industry_news_item(
    headline: &EncodedString,
    r#type: NewsType,
    industry: IndustryID,
) {
    add_news_item(&AddNewsItemArgs {
        ref1: NewsReference::Industry(industry).record(),
        ..AddNewsItemArgs::new(headline, r#type, NewsStyle::Thin, THIN_FLAGS)
    });
}

/// `DeleteVehicleNews`: remove news regarding given vehicle; `AdviceType::Invalid`, the
/// original's default, removes all of them.
pub(crate) fn delete_vehicle_news(vid: VehicleID, advice_type: AdviceType) {
    (services().delete_vehicle_news)(vid.0, advice_type.0);
}

/// `DeleteStationNews`: remove news regarding given station.
pub(crate) fn delete_station_news(sid: StationID) {
    (services().delete_station_news)(sid.0);
}

/// `DeleteIndustryNews`: remove news regarding given industry.
pub(crate) fn delete_industry_news(iid: IndustryID) {
    (services().delete_industry_news)(iid.0);
}
