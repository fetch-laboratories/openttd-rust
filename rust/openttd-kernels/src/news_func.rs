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
/// The `DParams` may not reference the vehicle due to autoreplace stuff. See
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
/// `DParam` 0 must reference the vehicle!
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

/// Recording news, string and script event tables shared by the tests of every caller.
#[cfg(test)]
pub(crate) mod testing {
    use super::{AddNewsItemArgs, Services, Sink};
    use crate::encoded::Descriptor;
    use crate::news_type::NewsAllocatedData;
    use crate::script_event_types::ScriptEvent;
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::sync::Once;

    /// One observable native call, in the order the code under test made it.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) enum Effect {
        /// `AddNewsItem(headline, type, style, flags, ref1, ref2, data, advice_type)`;
        /// `data` is the company name of recorded `CompanyNewsInformation` data.
        News {
            headline: Vec<u8>,
            r#type: crate::news_type::NewsType,
            style: crate::news_type::NewsStyle,
            flags: crate::news_type::NewsFlags,
            ref1: crate::news_type::NewsReference,
            ref2: crate::news_type::NewsReference,
            data: Option<Vec<u8>>,
            advice_type: crate::news_type::AdviceType,
        },
        /// The native `WITH_ASSERT` check of an encoded string parameter.
        AssertEncoded(u32, usize),
        DeleteVehicleNews(u32, u8),
        DeleteStationNews(u16),
        DeleteIndustryNews(u16),
        /// `GetString(string, args...)`: the string and its numeric parameters.
        GetString(u32, Vec<u64>),
        /// `new CompanyNewsInformation(title, Company::Get(company), other)`.
        CompanyNewsInformation(u32, u8, u8),
        AiNewEvent(u8, ScriptEvent),
        AiBroadcastNewEvent(ScriptEvent, u8),
        GameNewEvent(ScriptEvent),
    }

    thread_local! {
        static EFFECTS: RefCell<Vec<Effect>> = const { RefCell::new(Vec::new()) };
    }
    fn push(effect: Effect) {
        EFFECTS.with(|e| e.borrow_mut().push(effect));
    }

    /// The text the recording `GetString` returns: `S<string>:<params>`.
    pub(crate) fn formatted(string: u32, params: &[u64]) -> Vec<u8> {
        let params: Vec<String> = params.iter().map(u64::to_string).collect();
        format!("S{string}:{}", params.join(",")).into_bytes()
    }
    /// The company name of recorded `CompanyNewsInformation` data.
    pub(crate) fn company_news_name(company: u8) -> Vec<u8> {
        format!("company {company}").into_bytes()
    }

    fn reference(record: crate::news_type::NewsReferenceRecord) -> crate::news_type::NewsReference {
        use crate::news_type::NewsReference as R;
        let value = record.value;
        match record.index {
            0 => R::None,
            1 => R::Tile(crate::tile_type::TileIndex(value)),
            2 => R::Vehicle(crate::vehicle_type::VehicleID(value)),
            3 => R::Station(crate::station_type::StationID(
                u16::try_from(value).unwrap(),
            )),
            4 => R::Industry(crate::industry_type::IndustryID(
                u16::try_from(value).unwrap(),
            )),
            5 => R::Town(crate::town_type::TownId(u16::try_from(value).unwrap())),
            6 => R::Engine(crate::engine_type::EngineID(u16::try_from(value).unwrap())),
            _ => unreachable!(),
        }
    }
    extern "C" fn add_news_item(args: *const AddNewsItemArgs) {
        let a = unsafe { &*args };
        let data = (!a.data.is_null()).then(|| unsafe { (*a.data.cast::<Vec<u8>>()).clone() });
        push(Effect::News {
            headline: unsafe { std::slice::from_raw_parts(a.headline, a.headline_length) }.to_vec(),
            r#type: a.r#type,
            style: a.style,
            flags: a.flags,
            ref1: reference(a.ref1),
            ref2: reference(a.ref2),
            data,
            advice_type: a.advice_type,
        });
    }
    extern "C" fn assert_encoded(codepoint: u32, length: usize) {
        push(Effect::AssertEncoded(codepoint, length));
    }
    extern "C" fn company_news_company_name(
        data: *const NewsAllocatedData,
        sink: Sink,
        target: *mut c_void,
    ) {
        let name = unsafe { &*data.cast::<Vec<u8>>() };
        sink(target, name.as_ptr(), name.len());
    }
    extern "C" fn company_news_information(
        title: u32,
        company: u8,
        other: u8,
    ) -> *mut NewsAllocatedData {
        push(Effect::CompanyNewsInformation(title, company, other));
        Box::into_raw(Box::new(company_news_name(company))).cast()
    }
    extern "C" fn delete_industry_news(id: u16) {
        push(Effect::DeleteIndustryNews(id));
    }
    extern "C" fn delete_station_news(id: u16) {
        push(Effect::DeleteStationNews(id));
    }
    extern "C" fn delete_vehicle_news(id: u32, advice_type: u8) {
        push(Effect::DeleteVehicleNews(id, advice_type));
    }
    extern "C" fn get_string(
        string: u32,
        args: *const Descriptor,
        count: usize,
        sink: Sink,
        target: *mut c_void,
    ) {
        let params: Vec<u64> = unsafe { std::slice::from_raw_parts(args, count) }
            .iter()
            .map(|d| {
                assert_eq!(
                    d.kind, 1,
                    "the recording GetString takes numeric parameters"
                );
                d.value
            })
            .collect();
        let text = formatted(string, &params);
        push(Effect::GetString(string, params));
        sink(target, text.as_ptr(), text.len());
    }
    static NEWS: Services = Services {
        add_news_item,
        assert_encoded,
        company_news_company_name,
        company_news_information,
        delete_industry_news,
        delete_station_news,
        delete_vehicle_news,
        get_string,
        string_assertions: 1,
    };
    extern "C" fn ai_broadcast(event: *const ScriptEvent, skip_company: u8) {
        push(Effect::AiBroadcastNewEvent(unsafe { *event }, skip_company));
    }
    extern "C" fn ai_new(company: u8, event: *const ScriptEvent) {
        push(Effect::AiNewEvent(company, unsafe { *event }));
    }
    extern "C" fn game_new(event: *const ScriptEvent) {
        push(Effect::GameNewEvent(unsafe { *event }));
    }
    static EVENTS: crate::script_event::Services = crate::script_event::Services {
        ai_broadcast,
        ai_new,
        game_new,
    };

    /// Bind the recording tables once for the process; return and clear this thread's
    /// recorded effects.
    pub(crate) fn take() -> Vec<Effect> {
        static BIND: Once = Once::new();
        BIND.call_once(|| unsafe {
            super::openttd_rust_news_bind(&raw const NEWS);
            crate::script_event::openttd_rust_script_event_bind(&raw const EVENTS);
        });
        EFFECTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
    }

    /// The bytes the original `GetEncodedStringWithArgs` writes, built independently of
    /// `encoded.rs`: `SCC_ENCODED_INTERNAL` and the ID in lower-case hexadecimal, then per
    /// parameter `SCC_RECORD_SEPARATOR` and `SCC_ENCODED_NUMERIC` with hexadecimal digits or
    /// `SCC_ENCODED_STRING` with the bytes.
    pub(crate) fn encoded(id: u32, params: &[Param<'_>]) -> Vec<u8> {
        let mut out = "\u{E001}".as_bytes().to_vec();
        out.extend(format!("{id:x}").bytes());
        for param in params {
            out.push(0x1E);
            match param {
                Param::N(value) => {
                    out.extend("\u{E002}".bytes());
                    out.extend(format!("{value:x}").bytes());
                }
                Param::S(bytes) => {
                    out.extend("\u{E003}".bytes());
                    out.extend_from_slice(bytes);
                }
            }
        }
        out
    }
    /// One parameter of [`encoded`].
    pub(crate) enum Param<'a> {
        N(u64),
        S(&'a [u8]),
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{Effect, Param, encoded, take};
    use crate::news_type::{AdviceType, NewsFlag, NewsFlags, NewsReference, NewsStyle, NewsType};
    use crate::station_type::StationID;
    use crate::strings_type::{StringID, StringParameterData};
    use crate::tile_type::TileIndex;
    use crate::vehicle_type::VehicleID;

    fn headline() -> crate::strings_type::EncodedString {
        crate::strings_func::get_encoded_string(StringID(7), &[])
    }
    fn news(
        r#type: NewsType,
        style: NewsStyle,
        flags: &[NewsFlag],
        ref1: NewsReference,
        ref2: NewsReference,
        advice_type: AdviceType,
    ) -> Effect {
        Effect::News {
            headline: encoded(7, &[]),
            r#type,
            style,
            flags: NewsFlags::new(flags),
            ref1,
            ref2,
            data: None,
            advice_type,
        }
    }

    /// The inline wrappers of `news_func.h`: their styles, flags, references and defaults.
    #[test]
    fn wrappers_pass_the_original_styles_flags_and_references() {
        take();
        let thin = [NewsFlag::NoTransparency, NewsFlag::Shaded];
        super::add_vehicle_news_item(
            &headline(),
            NewsType::ArrivalOther,
            VehicleID(5),
            StationID(9),
        );
        super::add_vehicle_news_item(
            &headline(),
            NewsType::ArrivalCompany,
            VehicleID(5),
            StationID::INVALID,
        );
        super::add_vehicle_advice_news_item(AdviceType::VehicleWaiting, &headline(), VehicleID(6));
        super::add_tile_news_item(&headline(), NewsType::Accident, TileIndex(77), StationID(3));
        super::add_tile_news_item(
            &headline(),
            NewsType::IndustryOpen,
            TileIndex(78),
            StationID::INVALID,
        );
        super::add_industry_news_item(
            &headline(),
            NewsType::IndustryClose,
            crate::industry_type::IndustryID(4),
        );
        super::delete_vehicle_news(VehicleID(5), AdviceType::AircraftDestinationTooFar);
        super::delete_station_news(StationID(3));
        super::delete_industry_news(crate::industry_type::IndustryID(4));
        assert_eq!(
            take(),
            [
                news(
                    NewsType::ArrivalOther,
                    NewsStyle::Thin,
                    &thin,
                    NewsReference::Vehicle(VehicleID(5)),
                    NewsReference::Station(StationID(9)),
                    AdviceType::Invalid
                ),
                news(
                    NewsType::ArrivalCompany,
                    NewsStyle::Thin,
                    &thin,
                    NewsReference::Vehicle(VehicleID(5)),
                    NewsReference::None,
                    AdviceType::Invalid
                ),
                news(
                    NewsType::Advice,
                    NewsStyle::Small,
                    &[NewsFlag::InColour, NewsFlag::VehicleParam0],
                    NewsReference::Vehicle(VehicleID(6)),
                    NewsReference::None,
                    AdviceType::VehicleWaiting
                ),
                news(
                    NewsType::Accident,
                    NewsStyle::Thin,
                    &thin,
                    NewsReference::Tile(TileIndex(77)),
                    NewsReference::Station(StationID(3)),
                    AdviceType::Invalid
                ),
                news(
                    NewsType::IndustryOpen,
                    NewsStyle::Thin,
                    &thin,
                    NewsReference::Tile(TileIndex(78)),
                    NewsReference::None,
                    AdviceType::Invalid
                ),
                news(
                    NewsType::IndustryClose,
                    NewsStyle::Thin,
                    &thin,
                    NewsReference::Industry(crate::industry_type::IndustryID(4)),
                    NewsReference::None,
                    AdviceType::Invalid
                ),
                Effect::DeleteVehicleNews(5, 0),
                Effect::DeleteStationNews(3),
                Effect::DeleteIndustryNews(4),
            ]
        );
    }

    /// `GetEncodedString` writes the original's encoding; `GetString` passes its parameters.
    #[test]
    fn strings_encode_and_format_through_the_table() {
        take();
        let name =
            crate::strings_func::get_string(StringID(0x55), [StringParameterData::Integer(3)]);
        assert_eq!(name, super::testing::formatted(0x55, &[3]));
        let headline = crate::strings_func::get_encoded_string(
            StringID(0x1234),
            &[
                StringParameterData::Integer(0xAB),
                StringParameterData::String(&name),
            ],
        );
        assert_eq!(
            headline.as_bytes(),
            encoded(0x1234, &[Param::N(0xAB), Param::S(&name)])
        );
        assert_eq!(take(), [Effect::GetString(0x55, vec![3])]);
    }

    /// Under `WITH_ASSERT` an encoded string parameter reaches the native assertion.
    #[test]
    fn an_encoded_string_parameter_reaches_the_native_assertion() {
        take();
        let inner = encoded(1, &[]);
        crate::strings_func::get_encoded_string(
            StringID(2),
            &[StringParameterData::String(&inner)],
        );
        assert_eq!(take(), [Effect::AssertEncoded(0xE001, 3)]);
    }
}
