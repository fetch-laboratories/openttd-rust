/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `rail.cpp` and the `rail.h` helpers over the rail type table.
//!
//! `RailTypeInfo` stays a C++ class in `_railtypes` until its storage port. Rust resolves
//! a rail type once with [`get_rail_type_info`] and reads the members it needs through the
//! returned [`RailTypeInfoRef`]. A `RailTypeInfoRef` is valid for the call that resolved it.
use crate::bitmath_func::SetBitIterator;
use crate::company_type::CompanyID;
use crate::engine_type::VehicleAccelerationModel;
use crate::map_func::World;
use crate::rail_type::{RailType, RailTypeLabel, RailTypes};
use crate::services::Services as Shared;
use crate::tile_type::{TileIndex, TileType};
use crate::timer_game_calendar::CalendarDate;
use crate::timer_game_common::MAX_DATE;
use crate::transport_type::TransportType;
use crate::type_availability::{Services, company_available, services};
use crate::vehicle::{RailVehicleTypes, VehicleType};

/// Native `RailTypeInfo`, opaque: Rust holds a `GetRailTypeInfo` result only through a
/// [`RailTypeInfoRef`].
#[repr(C)]
pub struct RailTypeInfo {
    _private: [u8; 0],
}

/// A resolved `const RailTypeInfo *` with the shared table whose readers it calls.
#[derive(Clone, Copy)]
pub(crate) struct RailTypeInfoRef<'a> {
    shared: &'a Shared,
    rti: *const RailTypeInfo,
}

/// `GetRailTypeInfo`: returns the rail type information for a given rail type, through the
/// native helper and its `assert(railtype < RAILTYPE_END)`.
#[inline]
pub(crate) fn get_rail_type_info(shared: &Shared, railtype: RailType) -> RailTypeInfoRef<'_> {
    RailTypeInfoRef {
        shared,
        rti: (shared.rail_type_info)(railtype),
    }
}

impl RailTypeInfoRef<'_> {
    /// `RailTypeInfo::acceleration_type`: acceleration type of this rail type.
    #[inline]
    pub(crate) fn acceleration_type(self) -> VehicleAccelerationModel {
        VehicleAccelerationModel((self.shared.rail_type_acceleration_type)(self.rti))
    }
    /// `RailTypeInfo::compatible_railtypes`: bit mask of rail type numbers that are
    /// compatible with this rail type.
    #[inline]
    pub(crate) fn compatible_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_compatible_railtypes)(self.rti))
    }
    /// `RailTypeInfo::curve_speed`: multiplier for curve maximum speed advantage.
    #[inline]
    pub(crate) fn curve_speed(self) -> u8 {
        (self.shared.rail_type_curve_speed)(self.rti)
    }
    /// `RailTypeInfo::introduces_railtypes`: bitmask of which other rail types are
    /// introduced when this rail type is introduced.
    #[inline]
    pub(crate) fn introduces_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_introduces_railtypes)(self.rti))
    }
    /// `RailTypeInfo::introduction_date`: introduction date. When `INVALID_DATE` or a
    /// vehicle using this rail type gets introduced earlier, the vehicle's introduction
    /// date will be used instead for this rail type.
    #[inline]
    pub(crate) fn introduction_date(self) -> CalendarDate {
        CalendarDate((self.shared.rail_type_introduction_date)(self.rti))
    }
    /// `RailTypeInfo::introduction_required_railtypes`: bitmask of rail types that are
    /// required for this rail type to be introduced at a given `introduction_date`.
    #[inline]
    pub(crate) fn introduction_required_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_introduction_required_railtypes)(
            self.rti,
        ))
    }
    /// `RailTypeInfo::label`: unique 32 bit rail type identifier.
    #[inline]
    pub(crate) fn label(self) -> RailTypeLabel {
        RailTypeLabel((self.shared.rail_type_label)(self.rti))
    }
    /// `RailTypeInfo::max_speed`: maximum speed for vehicles travelling on this rail type.
    #[inline]
    pub(crate) fn max_speed(self) -> u16 {
        (self.shared.rail_type_max_speed)(self.rti)
    }
    /// `RailTypeInfo::powered_railtypes`: bit mask of rail type numbers that are powered
    /// on this rail type.
    #[inline]
    pub(crate) fn powered_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_powered_railtypes)(self.rti))
    }
}

/// `RailTypeSpriteGroup`: sprite groups for a rail type. Rust names only its end marker.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailTypeSpriteGroup(pub u8);

impl RailTypeSpriteGroup {
    /// End marker
    pub const RTSG_END: Self = Self(13);
}

/// `RailTypeSpriteGroup::RTSG_END`, for the files no pass has finished.
pub const RTSG_END: u8 = RailTypeSpriteGroup::RTSG_END.0;

/// `GetAllCompatibleRailTypes`: returns all compatible rail types for a set of rail types.
pub(crate) fn get_all_compatible_rail_types(shared: &Shared, railtypes: RailTypes) -> RailTypes {
    let mut compatible = RailTypes::EMPTY;
    for rt in SetBitIterator::new(railtypes.0).map(RailType) {
        compatible = compatible.with(get_rail_type_info(shared, rt).compatible_railtypes());
    }
    compatible
}

/// `GetAllPoweredRailTypes`: returns all powered rail types for a set of rail types.
pub(crate) fn get_all_powered_rail_types(shared: &Shared, railtypes: RailTypes) -> RailTypes {
    let mut powered = RailTypes::EMPTY;
    for rt in SetBitIterator::new(railtypes.0).map(RailType) {
        powered = powered.with(get_rail_type_info(shared, rt).powered_railtypes());
    }
    powered
}

/// `GetAllIntroducesRailTypes`: returns all introduced rail types for a set of rail types.
pub(crate) fn get_all_introduces_rail_types(shared: &Shared, railtypes: RailTypes) -> RailTypes {
    let mut introduces = RailTypes::EMPTY;
    for rt in SetBitIterator::new(railtypes.0).map(RailType) {
        introduces = introduces.with(get_rail_type_info(shared, rt).introduces_railtypes());
    }
    introduces
}

/// `IsCompatibleRail(RailType, RailType)`: checks if an engine of the given `RailType` can
/// drive on a tile with a given `RailType`. This would normally just be an equality check,
/// but for electric rails (which also support non-electric engines).
pub(crate) fn is_compatible_rail(
    shared: &Shared,
    enginetype: RailType,
    tiletype: RailType,
) -> bool {
    get_rail_type_info(shared, enginetype)
        .compatible_railtypes()
        .test(tiletype)
}

/// `IsCompatibleRail(RailTypes, RailType)`: checks if an engine of the given `RailTypes`
/// can drive on a tile with a given `RailType`.
pub(crate) fn is_compatible_rail_types(
    shared: &Shared,
    enginetype: RailTypes,
    tiletype: RailType,
) -> bool {
    get_all_compatible_rail_types(shared, enginetype).test(tiletype)
}

/// `HasPowerOnRail(RailType, RailType)`: checks if an engine of the given `RailType` got
/// power on a tile with a given `RailType`. This would normally just be an equality check,
/// but for electric rails (which also support non-electric engines).
pub(crate) fn has_power_on_rail(shared: &Shared, enginetype: RailType, tiletype: RailType) -> bool {
    get_rail_type_info(shared, enginetype)
        .powered_railtypes()
        .test(tiletype)
}

/// `HasPowerOnRail(RailTypes, RailType)`: checks if an engine of the given `RailTypes` got
/// power on a tile with a given `RailType`.
pub(crate) fn has_power_on_rail_types(
    shared: &Shared,
    enginetype: RailTypes,
    tiletype: RailType,
) -> bool {
    get_all_powered_rail_types(shared, enginetype).test(tiletype)
}

impl World<'_> {
    /// `GetTileRailType`: return the rail type of tile, or `INVALID_RAILTYPE` if this is no
    /// rail tile.
    pub(crate) fn get_tile_rail_type(&self, tile: TileIndex) -> RailType {
        match self.kind(tile) {
            TileType::MP_RAILWAY => return self.get_rail_type(tile),
            /* rail/road crossing */
            TileType::MP_ROAD if self.level_crossing(tile.0) => return self.get_rail_type(tile),
            TileType::MP_STATION if self.has_station_rail(tile) => {
                return self.get_rail_type(tile);
            }
            TileType::MP_TUNNELBRIDGE
                if TransportType(self.tunnel_bridge_transport(tile.0))
                    == TransportType::TRANSPORT_RAIL =>
            {
                return self.get_rail_type(tile);
            }
            _ => {}
        }
        RailType::INVALID_RAILTYPE
    }
}

/// `HasRailTypeAvail`: finds out if a company has a certain buildable rail type available.
pub(crate) fn has_rail_type_avail(s: &Services, company: CompanyID, railtype: RailType) -> bool {
    !RailTypes(crate::type_registry::rail_hidden()).test(railtype)
        && RailTypes((s.company_railtypes)(company.0)).test(railtype)
}

/// `HasAnyRailTypesAvail`: test if any buildable rail type is available for a company.
pub(crate) fn has_any_rail_types_avail(s: &Services, company: CompanyID) -> bool {
    let avail = (s.company_railtypes)(company.0) & !crate::type_registry::rail_hidden();
    RailTypes(avail).any()
}

/// `ValParamRailType`: validate functions for rail building.
pub(crate) fn val_param_rail_type(shared: &Shared, rail: RailType) -> bool {
    rail < RailType::RAILTYPE_END
        && has_rail_type_avail(
            services(shared),
            CompanyID((shared.current_company)()),
            rail,
        )
}

/// The shared table the availability table names.
#[allow(unsafe_code)]
fn shared_of(s: &Services) -> &Shared {
    // SAFETY: The availability table names the process-lifetime shared table.
    unsafe { &*s.shared }
}

/// `Engine::IterateType(VEH_TRAIN)`: each train engine in ascending ID order, with its
/// lifecycle for the company availability gate.
#[allow(unsafe_code)]
fn train_engines(s: &Services) -> impl Iterator<Item = crate::engine::EngineRef> + '_ {
    let mut from = 0;
    std::iter::from_fn(move || {
        loop {
            // SAFETY: Native pool leaves resolve one live shell; no borrow crosses calls.
            let e = unsafe { (s.next)(from) };
            if e.shell.is_null() {
                return None;
            }
            from = u32::from(e.id) + 1;
            if e.kind == VehicleType::TRAIN {
                return Some(e);
            }
        }
    })
}

/// `e->company_avail.Test(company) || TimerGameCalendar::date >= e->intro_date +
/// CalendarTime::DAYS_IN_YEAR`.
#[allow(unsafe_code)]
fn engine_available(e: crate::engine::EngineRef, company: CompanyID) -> bool {
    // SAFETY: `next` resolved this live engine shell; the reads end before any leaf.
    unsafe { company_available(e, company.0) }
}

/// `AddDateIntroducedRailTypes`: add the rail types that are to be introduced at the given
/// date.
pub(crate) fn add_date_introduced_rail_types(
    s: &Services,
    current: RailTypes,
    date: CalendarDate,
) -> RailTypes {
    let shared = shared_of(s);
    let mut rts = current;

    for rt in RailType::RAILTYPE_BEGIN.0..RailType::RAILTYPE_END.0 {
        let rti = get_rail_type_info(shared, RailType(rt));
        /* Unused rail type. */
        if rti.label() == RailTypeLabel(0) {
            continue;
        }

        /* Not date introduced. */
        let introduction_date = rti.introduction_date();
        if !(0..MAX_DATE).contains(&introduction_date.0) {
            continue;
        }

        /* Not yet introduced at this date. */
        if introduction_date > date {
            continue;
        }

        /* Have we introduced all required railtypes? */
        let required = rti.introduction_required_railtypes();
        if rts.0 & required.0 != required.0 {
            continue;
        }

        rts = rts.with(rti.introduces_railtypes());
    }

    /* When we added railtypes we need to run this method again; the added
     * railtypes might enable more rail types to become introduced. */
    if rts == current {
        rts
    } else {
        add_date_introduced_rail_types(s, rts, date)
    }
}

/// The `RailVehicleInfo::railtypes` of a train engine the caller's gates admitted, or its
/// introduced rail types, after the original's `assert(rvi->railtypes.Any())`.
fn engine_rail_types(s: &Services, railtypes: RailTypes, introduces: bool) -> RailTypes {
    crate::services_assert::assert_with(|| railtypes.any());
    if introduces {
        get_all_introduces_rail_types(shared_of(s), railtypes)
    } else {
        railtypes
    }
}

/// `GetCompanyRailTypes`: get the rail types the given company can build.
pub(crate) fn get_company_rail_types(
    s: &Services,
    company: CompanyID,
    introduces: bool,
) -> RailTypes {
    let mut rts = RailTypes::EMPTY;
    for e in train_engines(s) {
        let engine = crate::vehicle::EngineRef::of(e.shell);
        if engine.climate(shared_of(s).settings().landscape().0)
            && engine_available(e, company)
            && engine.rail_kind() != RailVehicleTypes::WAGON
        {
            rts = rts.with(engine_rail_types(
                s,
                RailTypes(engine.railtypes()),
                introduces,
            ));
        }
    }

    if introduces {
        return add_date_introduced_rail_types(s, rts, crate::timer_game_common::calendar_date());
    }
    rts
}

/// `GetRailTypes`: get list of rail types, regardless of company availability.
pub(crate) fn get_rail_types(s: &Services, introduces: bool) -> RailTypes {
    let mut rts = RailTypes::EMPTY;
    for e in train_engines(s) {
        let engine = crate::vehicle::EngineRef::of(e.shell);
        if !engine.climate(shared_of(s).settings().landscape().0) {
            continue;
        }

        if engine.rail_kind() != RailVehicleTypes::WAGON {
            rts = rts.with(engine_rail_types(
                s,
                RailTypes(engine.railtypes()),
                introduces,
            ));
        }
    }

    if introduces {
        return add_date_introduced_rail_types(s, rts, CalendarDate(MAX_DATE));
    }
    rts
}

/* Shims for `src/rail.cpp`'s facades and the files no pass has finished. */

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_rail_type(s: &Services, company: u8, rt: u8) -> u8 {
    u8::from(has_rail_type_avail(s, CompanyID(company), RailType(rt)))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_any_rail_types_abi(s: &Services, company: u8) -> u8 {
    u8::from(has_any_rail_types_avail(s, CompanyID(company)))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_val_rail_type(shared: &Shared, rt: u8) -> u8 {
    u8::from(val_param_rail_type(shared, RailType(rt)))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_add_date_rail_types(s: &Services, current: u64, date: i32) -> u64 {
    add_date_introduced_rail_types(s, RailTypes(current), CalendarDate(date)).0
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_rail_types(
    s: &Services,
    company: u8,
    introduces: bool,
) -> u64 {
    get_company_rail_types(s, CompanyID(company), introduces).0
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_types(s: &Services, introduces: bool) -> u64 {
    get_rail_types(s, introduces).0
}

/// Raw adapter: `HasRailTypeAvail`.
pub fn has_rail_type_avail_raw(s: &Services, company: u8, rt: u8) -> bool {
    has_rail_type_avail(s, CompanyID(company), RailType(rt))
}
/// Raw adapter: `ValParamRailType`.
pub fn val_param_rail_type_raw(shared: &Shared, rt: u8) -> bool {
    val_param_rail_type(shared, RailType(rt))
}
/// Raw adapter: `IsCompatibleRail(RailType, RailType)`.
pub(crate) fn is_compatible_rail_raw(shared: &Shared, engine: u8, tile: u8) -> bool {
    is_compatible_rail(shared, RailType(engine), RailType(tile))
}
/// Raw adapter: `GetAllIntroducesRailTypes`.
pub fn get_all_introduces_rail_types_raw(shared: &Shared, types: u64) -> u64 {
    get_all_introduces_rail_types(shared, RailTypes(types)).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{EngineRef, Lifecycle};
    use std::cell::RefCell;

    /// One fixture rail type's members.
    #[derive(Clone, Copy, Default)]
    struct Info {
        label: u32,
        date: i32,
        required: u64,
        introduces: u64,
        compatible: u64,
        powered: u64,
    }

    thread_local! {
        static TYPES: RefCell<Vec<Info>> = RefCell::new(vec![Info::default(); 64]);
        /// Each `(rail type, member)` read, in order; member 0 is `GetRailTypeInfo`.
        static READS: RefCell<Vec<(u8, char)>> = const { RefCell::new(Vec::new()) };
        static ENGINES: RefCell<Vec<Lifecycle>> = const { RefCell::new(Vec::new()) };
    }

    /// The fixture record of the rail type the handle encodes as its address.
    fn read(rti: *const RailTypeInfo, member: char) -> Info {
        let rt = u8::try_from(rti as usize).unwrap();
        READS.with(|r| r.borrow_mut().push((rt, member)));
        TYPES.with(|t| t.borrow()[usize::from(rt)])
    }
    extern "C" fn rail_type_info(rt: RailType) -> *const RailTypeInfo {
        READS.with(|r| r.borrow_mut().push((rt.0, 'i')));
        std::ptr::without_provenance(usize::from(rt.0))
    }
    extern "C" fn label(rti: *const RailTypeInfo) -> u32 {
        read(rti, 'l').label
    }
    extern "C" fn date(rti: *const RailTypeInfo) -> i32 {
        read(rti, 'd').date
    }
    extern "C" fn required(rti: *const RailTypeInfo) -> u64 {
        read(rti, 'r').required
    }
    extern "C" fn introduces(rti: *const RailTypeInfo) -> u64 {
        read(rti, 'n').introduces
    }
    extern "C" fn compatible(rti: *const RailTypeInfo) -> u64 {
        read(rti, 'c').compatible
    }
    extern "C" fn powered(rti: *const RailTypeInfo) -> u64 {
        read(rti, 'p').powered
    }
    /// The fixture engines in order, each its own shell and lifecycle.
    extern "C" fn next(from: u32) -> EngineRef {
        ENGINES.with(|e| {
            let engines = e.borrow();
            match engines.get(from as usize) {
                Some(record) => {
                    let record: *const Lifecycle = &raw const *record;
                    EngineRef {
                        shell: record,
                        state: record.cast_mut(),
                        id: u16::try_from(from).unwrap(),
                        kind: VehicleType::TRAIN,
                    }
                }
                None => EngineRef {
                    shell: std::ptr::null(),
                    state: std::ptr::null_mut(),
                    id: 0,
                    kind: VehicleType::TRAIN,
                },
            }
        })
    }
    extern "C" fn company_railtypes(_: u8) -> u64 {
        u64::MAX
    }

    /// The shared table over the fixture rail types, and an availability table over it and
    /// the fixture engines, in the temperate climate.
    fn tables(types: &[(u8, Info)]) -> &'static Services {
        static TEMPERATE: u8 = 0;
        TYPES.with(|t| {
            let mut t = t.borrow_mut();
            for &(rt, info) in types {
                t[usize::from(rt)] = info;
            }
        });
        let settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
            climate: &raw const TEMPERATE,
            ..crate::settings_type::SettingAddresses::NULL
        }));
        let shared = Box::leak(Box::new(Shared {
            settings,
            rail_type_compatible_railtypes: compatible,
            rail_type_info,
            rail_type_introduces_railtypes: introduces,
            rail_type_introduction_date: date,
            rail_type_introduction_required_railtypes: required,
            rail_type_label: label,
            rail_type_powered_railtypes: powered,
            ..crate::test_tables::SHARED
        }));
        Box::leak(Box::new(Services {
            assertions: true,
            company_railtypes,
            company_roadtypes: crate::test_tables::unreachable1,
            editor: crate::test_tables::unreachable0,
            next,
            road_flags: crate::test_tables::unreachable1,
            road_info: crate::test_tables::unreachable1,
            road_introduces: crate::test_tables::unreachable1,
            road_introduction_date: crate::test_tables::unreachable1,
            road_introduction_required: crate::test_tables::unreachable1,
            road_label: crate::test_tables::unreachable1,
            set_company_railtypes: crate::test_tables::unreachable2,
            set_company_roadtypes: crate::test_tables::unreachable2,
            shared,
        }))
    }
    fn reads() -> Vec<(u8, char)> {
        READS.with(|r| std::mem::take(&mut *r.borrow_mut()))
    }
    fn set(bits: &[u8]) -> RailTypes {
        RailTypes(bits.iter().fold(0, |m, &b| m | 1 << b))
    }

    /// The `rail.h` unions over a two-type set return the union of each type's member and
    /// resolve each type once; the compatibility and power tests follow them for either
    /// overload.
    #[test]
    fn unions_resolve_each_type_once_and_the_tests_follow_them() {
        let types = [
            (
                1,
                Info {
                    compatible: 0b0011,
                    powered: 0b0100,
                    introduces: 0b1_0000,
                    ..Info::default()
                },
            ),
            (
                2,
                Info {
                    compatible: 0b1000,
                    powered: 0b0110,
                    introduces: 0b10_0000,
                    ..Info::default()
                },
            ),
        ];
        let s = tables(&types);
        let shared = shared_of(s);
        let both = set(&[1, 2]);
        reads();
        assert_eq!(
            get_all_compatible_rail_types(shared, both),
            RailTypes(0b1011)
        );
        assert_eq!(reads(), [(1, 'i'), (1, 'c'), (2, 'i'), (2, 'c')]);
        assert_eq!(get_all_powered_rail_types(shared, both), RailTypes(0b0110));
        assert_eq!(reads(), [(1, 'i'), (1, 'p'), (2, 'i'), (2, 'p')]);
        assert_eq!(
            get_all_introduces_rail_types(shared, both),
            RailTypes(0b11_0000)
        );
        assert_eq!(reads(), [(1, 'i'), (1, 'n'), (2, 'i'), (2, 'n')]);

        assert!(is_compatible_rail(shared, RailType(1), RailType(0)));
        assert!(!is_compatible_rail(shared, RailType(1), RailType(3)));
        assert!(is_compatible_rail_types(shared, both, RailType(3)));
        assert!(!is_compatible_rail_types(shared, both, RailType(2)));
        assert!(!has_power_on_rail(shared, RailType(1), RailType(1)));
        assert!(has_power_on_rail(shared, RailType(2), RailType(1)));
        assert!(has_power_on_rail_types(shared, both, RailType(2)));
        assert!(!has_power_on_rail_types(shared, both, RailType(0)));
    }

    /// `AddDateIntroducedRailTypes` skips an unused type, one not date introduced, one not
    /// yet introduced and one whose required types are missing, reading no member after
    /// the test that fails, and runs again when a type it introduced enables another.
    #[test]
    fn date_introduction_skips_each_gate_and_runs_again() {
        let l = u32::from_be_bytes(*b"TEST");
        let types = [
            (
                0,
                Info {
                    label: 0,
                    introduces: set(&[5]).0,
                    ..Info::default()
                },
            ),
            (
                1,
                Info {
                    label: l,
                    date: -1,
                    introduces: set(&[6]).0,
                    ..Info::default()
                },
            ),
            (
                2,
                Info {
                    label: l,
                    date: MAX_DATE,
                    introduces: set(&[6]).0,
                    ..Info::default()
                },
            ),
            (
                3,
                Info {
                    label: l,
                    date: 200,
                    introduces: set(&[7]).0,
                    ..Info::default()
                },
            ),
            (
                4,
                Info {
                    label: l,
                    date: 50,
                    required: set(&[9]).0,
                    introduces: set(&[8]).0,
                    ..Info::default()
                },
            ),
            (
                5,
                Info {
                    label: l,
                    date: 100,
                    introduces: set(&[10]).0,
                    ..Info::default()
                },
            ),
            (
                20,
                Info {
                    label: l,
                    date: 50,
                    required: set(&[21]).0,
                    introduces: set(&[22]).0,
                    ..Info::default()
                },
            ),
            (
                30,
                Info {
                    label: l,
                    date: 0,
                    introduces: set(&[21]).0,
                    ..Info::default()
                },
            ),
        ];
        let s = tables(&types);
        reads();
        let rts = add_date_introduced_rail_types(s, RailTypes::EMPTY, CalendarDate(100));
        assert_eq!(rts, set(&[10, 21, 22]));
        let first: String = reads()
            .into_iter()
            .filter(|&(rt, m)| rt < 6 && m != 'i')
            .take(15)
            .fold(String::new(), |mut out, (rt, m)| {
                out.push_str(&rt.to_string());
                out.push(m);
                out.push(' ');
                out
            });
        assert_eq!(first, "0l 1l 1d 2l 2d 3l 3d 4l 4d 4r 5l 5d 5r 5n 0l ");
        // A fixed point is returned as it is.
        assert_eq!(
            add_date_introduced_rail_types(s, rts, CalendarDate(100)),
            rts
        );
    }

    /// `GetCompanyRailTypes` and `GetRailTypes` differ for an engine that fails only the
    /// company's availability gate, and both skip wagons.
    #[test]
    fn company_rail_types_gate_on_availability_and_skip_wagons() {
        let engine = |railtypes: u8, wagon: bool, company_avail: u16| {
            let mut record = Lifecycle::default();
            record.r#type = VehicleType::TRAIN;
            record.vehicle_info_type = VehicleType::TRAIN;
            record.company_avail = company_avail;
            // The calendar date is 0, before its first year of exclusivity ends.
            record.intro_date = 1000;
            record.info.climates = 1;
            record.vehicle_info = crate::engine::records::VehicleInfo {
                rail: crate::engine::records::RailVehicleInfo {
                    railtypes: set(&[railtypes]).0,
                    railveh_type: if wagon { RailVehicleTypes::WAGON.0 } else { 0 },
                    ..crate::engine::records::RailVehicleInfo::default()
                },
            };
            record
        };
        ENGINES.with(|e| {
            *e.borrow_mut() = vec![engine(0, false, 0), engine(2, true, 1), engine(1, false, 1)];
        });
        let types = [
            (
                0,
                Info {
                    introduces: set(&[3]).0,
                    ..Info::default()
                },
            ),
            (
                1,
                Info {
                    introduces: set(&[4]).0,
                    ..Info::default()
                },
            ),
        ];
        let s = tables(&types);
        assert_eq!(get_company_rail_types(s, CompanyID(0), false), set(&[1]));
        assert_eq!(get_rail_types(s, false), set(&[0, 1]));
        assert_eq!(get_company_rail_types(s, CompanyID(0), true), set(&[4]));
        assert_eq!(get_rail_types(s, true), set(&[3, 4]));
    }
}
