/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Company identity, colours, names and livery selection. Native shells retain
//! unmoved fields. Every borrowed name span ends before its next native mutation.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
use crate::company_type::{CompanyID, OpenTTDCompany};
use crate::engine_type::{EngineID, OpenTTDEngine};
use crate::livery::{
    LS_BUS, LS_DEFAULT, LS_DIESEL, LS_DMU, LS_ELECTRIC, LS_EMU, LS_END, LS_FREIGHT_SHIP,
    LS_FREIGHT_TRAM, LS_FREIGHT_WAGON, LS_HELICOPTER, LS_LARGE_PLANE, LS_MAGLEV, LS_MONORAIL,
    LS_PASSENGER_SHIP, LS_PASSENGER_TRAM, LS_PASSENGER_WAGON_DIESEL, LS_PASSENGER_WAGON_ELECTRIC,
    LS_PASSENGER_WAGON_MAGLEV, LS_PASSENGER_WAGON_MONORAIL, LS_PASSENGER_WAGON_STEAM,
    LS_SMALL_PLANE, LS_STEAM, LS_TRUCK, Livery, PRIMARY, SECONDARY,
};
use crate::raw_field::{read_field, write_field};
use crate::services::{ByteSpan, Services as Shared};
use crate::tile_type::TileIndex;
use crate::vehicle::RailVehicleTypes;
use crate::vehicle_type::{OpenTTDVehicle, VehicleID, VehicleType};
use std::mem::MaybeUninit;
type Town = crate::town_type::TownHandle;
use crate::company_manager_face::Face;
#[repr(C, align(16))]
pub struct NameStorage {
    bytes: [MaybeUninit<u8>; 128],
}
#[repr(C)]
pub struct Strings {
    pub unnamed: u32,
    pub president: u32,
    pub company: u32,
    pub president_generated: u32,
    pub andco: u32,
    pub town_begin: u32,
    pub town_end: u32,
    pub company_begin: u32,
}
#[repr(C)]
pub struct Services {
    pub shared: &'static Shared,
    pub randomizer: *mut crate::town::names::Randomizer,
    pub faces: &'static crate::company_manager_face::Services,
    pub groups: &'static crate::fleet::GroupServices,
    pub strings: Strings,
    pub company: extern "C" fn(CompanyID) -> *mut OpenTTDCompany,
    pub next_company: extern "C" fn(u32) -> *mut OpenTTDCompany,
    pub company_id: extern "C" fn(*mut OpenTTDCompany) -> CompanyID,
    pub name_id: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_name_id: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub name_parts: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_name_parts: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub president_id: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_president_id: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub president_parts: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_president_parts: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub colour: extern "C" fn(*mut OpenTTDCompany) -> u8,
    pub set_colour: extern "C" fn(*mut OpenTTDCompany, u8) -> (),
    pub last_build: extern "C" fn(*mut OpenTTDCompany) -> TileIndex,
    pub is_ai: extern "C" fn(*mut OpenTTDCompany) -> bool,
    pub name: extern "C" fn(*mut OpenTTDCompany) -> ByteSpan,
    pub set_name: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> (),
    pub president: extern "C" fn(*mut OpenTTDCompany) -> ByteSpan,
    pub set_president: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> (),
    pub livery: extern "C" fn(*mut OpenTTDCompany, u8) -> *mut Livery,
    pub face: extern "C" fn(*mut OpenTTDCompany) -> *mut Face,
    pub name_frame_init: extern "C" fn(*mut NameStorage) -> (),
    pub name_frame_drop: extern "C" fn(*mut NameStorage) -> (),
    pub render: extern "C" fn(*mut NameStorage, u32, u32) -> ByteSpan,
    pub render_other: extern "C" fn(*mut NameStorage, u32, u32) -> ByteSpan,
    pub company_renamed: extern "C" fn(*mut OpenTTDCompany, *mut NameStorage) -> (),
    pub president_renamed: extern "C" fn(*mut OpenTTDCompany, *mut NameStorage) -> (),
    pub launch_news: extern "C" fn(*mut OpenTTDCompany, *mut Town) -> (),
    pub admin_update: extern "C" fn(CompanyID) -> (),
    pub rename_transport: extern "C" fn(ByteSpan) -> (),
    pub president_windows: extern "C" fn() -> (),
    pub colour_windows: extern "C" fn() -> (),
    pub object_colours: extern "C" fn(*mut OpenTTDCompany) -> (),
    pub dirty: extern "C" fn() -> (),
    pub reset_colourmap: extern "C" fn() -> (),
    pub engine: extern "C" fn(EngineID) -> *const OpenTTDEngine,
    pub engine_type: extern "C" fn(*const OpenTTDEngine) -> VehicleType,
    pub engine_cargo: extern "C" fn(*const OpenTTDEngine) -> u8,
    pub engine_kind: extern "C" fn(*const OpenTTDEngine) -> RailVehicleTypes,
    pub engine_class: extern "C" fn(*const OpenTTDEngine) -> u8,
    pub engine_misc: extern "C" fn(*const OpenTTDEngine) -> u8,
    pub aircraft_subtype: extern "C" fn(*const OpenTTDEngine) -> u8,
    pub vehicle_cargo: extern "C" fn(*mut OpenTTDVehicle) -> u8,
    pub vehicle_first: extern "C" fn(*mut OpenTTDVehicle) -> *mut OpenTTDVehicle,
    pub articulated: extern "C" fn(*mut OpenTTDVehicle) -> bool,
    pub wagon_override: extern "C" fn(*mut OpenTTDVehicle) -> bool,
    pub cargo_freight: extern "C" fn(u8) -> bool,
    pub cargo_classes: extern "C" fn(u8) -> u16,
    pub vehicle_id: extern "C" fn(*mut OpenTTDVehicle) -> VehicleID,
    pub invalidate_vehicle: extern "C" fn(*mut OpenTTDVehicle) -> (),
    pub vehicle_owner: extern "C" fn(*mut OpenTTDVehicle) -> CompanyID,
}
impl NameStorage {
    fn new() -> Self {
        Self {
            bytes: [MaybeUninit::uninit(); 128],
        }
    }
}
struct Names<'a> {
    s: &'a Services,
    slot: *mut NameStorage,
    _lifetime: std::marker::PhantomData<&'a mut NameStorage>,
}
impl<'a> Names<'a> {
    fn new(s: &'a Services, slot: &'a mut NameStorage) -> Self {
        (s.name_frame_init)(slot);
        Self {
            s,
            slot,
            _lifetime: std::marker::PhantomData,
        }
    }
    fn render(&mut self, id: u32, param: u32) -> ByteSpan {
        (self.s.render)(self.slot, id, param)
    }
}
impl Drop for Names<'_> {
    fn drop(&mut self) {
        (self.s.name_frame_drop)(self.slot);
    }
}
fn equal(a: ByteSpan, b: ByteSpan) -> bool {
    if a.size != b.size {
        return false;
    }
    if a.size == 0 {
        return true;
    }
    unsafe {
        std::slice::from_raw_parts(a.data, a.size) == std::slice::from_raw_parts(b.data, b.size)
    }
}
fn companies(s: &Services, mut f: impl FnMut(*mut OpenTTDCompany) -> bool) -> bool {
    let mut from = 0;
    loop {
        let c = (s.next_company)(from);
        if c.is_null() {
            return true;
        }
        from = u32::from((s.company_id)(c).0) + 1;
        if !f(c) {
            return false;
        }
    }
}
static mut COLOURS: [u8; crate::company_type::MAX_COMPANIES as usize] =
    [0; crate::company_type::MAX_COMPANIES as usize];
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_colours() -> *mut u8 {
    std::ptr::addr_of_mut!(COLOURS).cast()
}
fn cache_colour(id: CompanyID, colour: u8) {
    unsafe {
        openttd_rust_company_colours()
            .add(usize::from(id.0))
            .write(colour);
    }
}
/// Original 100 draws precede weighting and pool-order colour elimination.
pub(crate) fn generate_colour(s: &Services) -> u8 {
    use crate::gfx_type::{
        COLOUR_BLUE, COLOUR_BROWN, COLOUR_CREAM, COLOUR_DARK_BLUE, COLOUR_DARK_GREEN, COLOUR_END,
        COLOUR_GREEN, COLOUR_GREY, COLOUR_LIGHT_BLUE, COLOUR_MAUVE, COLOUR_ORANGE,
        COLOUR_PALE_GREEN, COLOUR_PURPLE, COLOUR_WHITE, COLOUR_YELLOW, INVALID_COLOUR,
    };
    const WEIGHTS: [u8; COLOUR_END as usize] = [2, 2, 3, 2, 3, 2, 3, 2, 3, 2, 2, 2, 3, 1, 1, 1];
    const SIMILAR: [[u8; 2]; COLOUR_END as usize] = [
        [COLOUR_BLUE, COLOUR_LIGHT_BLUE],
        [COLOUR_GREEN, COLOUR_DARK_GREEN],
        [INVALID_COLOUR, INVALID_COLOUR],
        [COLOUR_ORANGE, INVALID_COLOUR],
        [INVALID_COLOUR, INVALID_COLOUR],
        [COLOUR_DARK_BLUE, COLOUR_BLUE],
        [COLOUR_PALE_GREEN, COLOUR_DARK_GREEN],
        [COLOUR_PALE_GREEN, COLOUR_GREEN],
        [COLOUR_DARK_BLUE, COLOUR_LIGHT_BLUE],
        [COLOUR_BROWN, COLOUR_ORANGE],
        [COLOUR_PURPLE, INVALID_COLOUR],
        [COLOUR_MAUVE, INVALID_COLOUR],
        [COLOUR_YELLOW, COLOUR_CREAM],
        [COLOUR_CREAM, INVALID_COLOUR],
        [COLOUR_WHITE, INVALID_COLOUR],
        [COLOUR_GREY, INVALID_COLOUR],
    ];
    let mut colours = std::array::from_fn::<_, { COLOUR_END as usize }, _>(|i| i as u8);
    for _ in 0..100 {
        let r = s.shared.random();
        colours.swap(
            crate::bitmath_func::gb(r, 0, 4) as usize,
            crate::bitmath_func::gb(r, 4, 4) as usize,
        );
    }
    for _ in 0..COLOUR_END {
        for j in 1..usize::from(COLOUR_END) {
            if WEIGHTS[usize::from(colours[j - 1])] < WEIGHTS[usize::from(colours[j])] {
                colours.swap(j - 1, j);
            }
        }
    }
    companies(s, |c| {
        let colour = (s.colour)(c);
        if let Some(i) = colours.iter().position(|&x| x == colour) {
            colours[i] = INVALID_COLOUR;
        }
        for similar in SIMILAR[usize::from(colour)] {
            if similar == INVALID_COLOUR {
                break;
            }
            for i in 1..usize::from(COLOUR_END) {
                if colours[i - 1] == similar {
                    colours.swap(i - 1, i);
                }
            }
        }
        true
    });
    colours
        .into_iter()
        .find(|&x| x != INVALID_COLOUR)
        .expect("NOT_REACHED")
}
pub(crate) fn generate_name(s: &Services, c: *mut OpenTTDCompany) {
    if (s.name_id)(c) != s.strings.unnamed || (s.last_build)(c).0 == 0 {
        return;
    }
    let t = crate::town::spatial::closest(&*s.shared, (s.last_build)(c).0, u32::MAX);
    let town = (s.shared.town_owner)(t);
    let simulation = unsafe { crate::town::openttd_rust_town_simulation(town) };
    let mut slot = NameStorage::new();
    let mut names = Names::new(s, &mut slot);
    let mut str = s.strings.andco;
    let mut parts;
    let town_name = unsafe { crate::town::names::openttd_rust_town_name(town).size == 0 }
        && crate::math::is_inside_mm(
            usize::from(read_field!(simulation, townnametype)),
            s.strings.town_begin as usize,
            s.strings.town_end as usize,
        );
    let mut try_town = town_name;
    loop {
        if try_town {
            str = u32::from(read_field!(simulation, townnametype)) - s.strings.town_begin
                + s.strings.company_begin;
            parts = read_field!(simulation, townnameparts);
        } else if (s.president_id)(c) == s.strings.president_generated {
            parts = (s.president_parts)(c);
            names.render(str, parts);
            break;
        } else {
            parts = s.shared.random();
        }
        let unique = companies(s, |cc| {
            (s.name_id)(cc) != str || (s.name_parts)(cc) != parts
        });
        if unique {
            let name = names.render(str, parts);
            if (s.shared.utf8_length)(name.data, name.size)
                < crate::company_type::MAX_LENGTH_COMPANY_NAME_CHARS
            {
                break;
            }
        }
        str = s.strings.andco;
        try_town = false;
    }
    (s.set_name_id)(c, str);
    (s.set_name_parts)(c, parts);
    (s.dirty)();
    (s.company_renamed)(c, names.slot);
    if (s.is_ai)(c) {
        (s.launch_news)(c, t.cast_mut());
    }
}
pub(crate) fn generate_president(s: &Services, c: *mut OpenTTDCompany) {
    let mut slot = NameStorage::new();
    let mut names = Names::new(s, &mut slot);
    loop {
        (s.set_president_parts)(c, s.shared.random());
        (s.set_president_id)(c, s.strings.president_generated);
        let name = names.render(s.strings.president, u32::from((s.company_id)(c).0));
        if (s.shared.utf8_length)(name.data, name.size)
            >= crate::company_type::MAX_LENGTH_PRESIDENT_NAME_CHARS
        {
            continue;
        }
        if companies(s, |cc| {
            cc == c
                || !equal(
                    name,
                    (s.render_other)(
                        names.slot,
                        s.strings.president,
                        u32::from((s.company_id)(cc).0),
                    ),
                )
        }) {
            return;
        }
    }
}
pub(crate) fn reset_livery(s: &Services, c: *mut OpenTTDCompany) {
    for scheme in LS_DEFAULT..LS_END {
        let l = (s.livery)(c, scheme);
        write_field!(l, in_use, 0);
        write_field!(l, colour1, (s.colour)(c));
        write_field!(l, colour2, (s.colour)(c));
    }
    let owner = (s.company_id)(c).0;
    let mut from = 0;
    loop {
        let id = unsafe { (s.groups.next_group)(from) };
        if id == u32::MAX {
            break;
        }
        from = id + 1;
        let g = unsafe { (s.groups.group)(id as u16) }.cast::<crate::fleet::GroupFields>();
        if read_field!(g, owner) == owner {
            let l = unsafe { std::ptr::addr_of_mut!((*g).livery) };
            write_field!(l, in_use, 0);
            write_field!(l, colour1, (s.colour)(c));
            write_field!(l, colour2, (s.colour)(c));
        }
    }
}
pub(crate) fn set_startup_colour(s: &Services, c: *mut OpenTTDCompany, colour: u8) {
    (s.set_colour)(c, colour);
    reset_livery(s, c);
    cache_colour((s.company_id)(c), (s.colour)(c));
}
pub(crate) fn update_liveries(s: &Services, c: *mut OpenTTDCompany) {
    let default = (s.livery)(c, LS_DEFAULT);
    for scheme in 1..LS_END {
        let l = (s.livery)(c, scheme);
        if read_field!(l, in_use) & PRIMARY == 0 {
            write_field!(l, colour1, read_field!(default, colour1));
        }
        if read_field!(l, in_use) & SECONDARY == 0 {
            write_field!(l, colour2, read_field!(default, colour2));
        }
    }
    unsafe {
        crate::fleet::company_liveries(s.groups, (s.company_id)(c).0);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_reset_livery(s: &Services, c: *mut OpenTTDCompany) {
    reset_livery(s, c);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_update_liveries(s: &Services, c: *mut OpenTTDCompany) {
    update_liveries(s, c);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_manager_face(
    s: &Services,
    style: u32,
    bits: u32,
    execute: bool,
) -> bool {
    if !crate::company_manager_face::valid(s.faces, style, bits) {
        return false;
    }
    if execute {
        let c = (s.company)(CompanyID((s.shared.current_company)()));
        let face = (s.face)(c);
        crate::company_manager_face::style(s.faces, face, style);
        unsafe {
            (s.faces.bits)(face).write(bits);
        }
        (s.dirty)();
    }
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_colour(
    s: &Services,
    scheme: u8,
    primary: bool,
    mut colour: u8,
    execute: bool,
) -> bool {
    use crate::gfx_type::{COLOUR_END, INVALID_COLOUR};
    if scheme >= LS_END || (colour >= COLOUR_END && colour != INVALID_COLOUR) {
        return false;
    }
    if scheme == LS_DEFAULT && colour == INVALID_COLOUR {
        return false;
    }
    let id = CompanyID((s.shared.current_company)());
    let c = (s.company)(id);
    if scheme == LS_DEFAULT && primary && !companies(s, |cc| cc == c || (s.colour)(cc) != colour) {
        return false;
    }
    if execute {
        let l = (s.livery)(c, scheme);
        let default = (s.livery)(c, LS_DEFAULT);
        let flag = if primary { PRIMARY } else { SECONDARY };
        if scheme != LS_DEFAULT {
            let flags = read_field!(l, in_use);
            write_field!(
                l,
                in_use,
                if colour == INVALID_COLOUR {
                    flags & !flag
                } else {
                    flags | flag
                }
            );
        }
        if primary {
            if colour == INVALID_COLOUR {
                colour = read_field!(default, colour1);
            }
            write_field!(l, colour1, colour);
            if scheme == LS_DEFAULT {
                update_liveries(s, c);
                cache_colour(id, colour);
                (s.set_colour)(c, colour);
                (s.admin_update)(id);
            }
        } else {
            if colour == INVALID_COLOUR {
                colour = read_field!(default, colour2);
            }
            write_field!(l, colour2, colour);
            if scheme == LS_DEFAULT {
                update_liveries(s, c);
            }
        }
        if read_field!(l, in_use) & (PRIMARY | SECONDARY) != 0 {
            write_field!(default, in_use, read_field!(default, in_use) | PRIMARY);
        } else {
            write_field!(
                default,
                in_use,
                read_field!(default, in_use) & !(PRIMARY | SECONDARY)
            );
            for i in LS_DEFAULT..LS_END {
                if read_field!((s.livery)(c, i), in_use) & (PRIMARY | SECONDARY) != 0 {
                    write_field!(default, in_use, read_field!(default, in_use) | PRIMARY);
                    break;
                }
            }
        }
        (s.reset_colourmap)();
        (s.dirty)();
        (s.colour_windows)();
        let mut from = 0;
        while let Some(v) = (s.shared.vehicle_next)(from) {
            let shell = v.0.as_ptr();
            from = (s.vehicle_id)(shell).0 + 1;
            if (s.vehicle_owner)(shell) == id {
                (s.invalidate_vehicle)(shell);
            }
        }
        (s.object_colours)(c);
    }
    true
}
fn rename(s: &Services, text: ByteSpan, execute: bool, president: bool) -> u8 {
    let reset = text.size == 0;
    if !reset {
        let limit = if president {
            crate::company_type::MAX_LENGTH_PRESIDENT_NAME_CHARS
        } else {
            crate::company_type::MAX_LENGTH_COMPANY_NAME_CHARS
        };
        if (s.shared.utf8_length)(text.data, text.size) >= limit {
            return 1;
        }
        if !companies(s, |c| {
            let n = if president {
                (s.president)(c)
            } else {
                (s.name)(c)
            };
            n.size == 0 || !equal(n, text)
        }) {
            return 2;
        }
    }
    if execute {
        let id = CompanyID((s.shared.current_company)());
        let c = (s.company)(id);
        if president {
            (s.set_president)(c, text);
            if !reset && (s.name_id)(c) == s.strings.unnamed && (s.name)(c).size == 0 {
                (s.rename_transport)(text);
            }
            (s.president_windows)();
        } else {
            (s.set_name)(c, text);
        }
        (s.dirty)();
        (s.admin_update)(id);
        let mut slot = NameStorage::new();
        let mut names = Names::new(s, &mut slot);
        names.render(
            if president {
                s.strings.president
            } else {
                s.strings.company
            },
            u32::from(id.0),
        );
        if president {
            (s.president_renamed)(c, names.slot);
        } else {
            (s.company_renamed)(c, names.slot);
        }
    }
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_rename(s: &Services, text: ByteSpan, execute: bool) -> u8 {
    rename(s, text, execute, false)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_president_rename(s: &Services, text: ByteSpan, execute: bool) -> u8 {
    rename(s, text, execute, true)
}
pub(crate) fn engine_livery(
    s: &Services,
    mut engine: EngineID,
    parent: EngineID,
    v: *mut OpenTTDVehicle,
) -> u8 {
    use crate::cargo_type::{INVALID_CARGO, is_valid_cargo_type};
    use crate::engine_type::{
        AIR_CTOL, AIR_FAST, AIR_HELI, EC_DIESEL, EC_ELECTRIC, EC_MAGLEV, EC_MONORAIL, EC_STEAM,
        RAIL_IS_MU, ROAD_IS_TRAM,
    };
    use crate::vehicle::RailVehicleTypes;
    let mut cargo = if v.is_null() {
        INVALID_CARGO
    } else {
        (s.vehicle_cargo)(v)
    };
    let mut e = (s.engine)(engine);
    match (s.engine_type)(e).0 {
        crate::vehicle_type::VEH_TRAIN => {
            if !v.is_null()
                && parent != EngineID::INVALID
                && ((s.wagon_override)(v)
                    || ((s.articulated)(v) && (s.engine_kind)(e) != RailVehicleTypes::WAGON))
            {
                engine = parent;
                e = (s.engine)(engine);
            }
            if !is_valid_cargo_type(cargo) {
                cargo = (s.engine_cargo)(e);
            }
            if !is_valid_cargo_type(cargo) {
                cargo =
                    crate::cargo_registry::openttd_rust_cargo_by_label(crate::cargo_type::CT_GOODS);
            }
            assert!(is_valid_cargo_type(cargo));
            if (s.engine_kind)(e) == RailVehicleTypes::WAGON {
                if (s.cargo_freight)(cargo) {
                    return LS_FREIGHT_WAGON;
                }
                if parent == EngineID::INVALID {
                    return LS_PASSENGER_WAGON_STEAM;
                }
                let p = (s.engine)(parent);
                let mu = (s.engine_misc)(p) & RAIL_IS_MU != 0;
                match (s.engine_class)(p) {
                    EC_STEAM => LS_PASSENGER_WAGON_STEAM,
                    EC_DIESEL => {
                        if mu {
                            LS_DMU
                        } else {
                            LS_PASSENGER_WAGON_DIESEL
                        }
                    }
                    EC_ELECTRIC => {
                        if mu {
                            LS_EMU
                        } else {
                            LS_PASSENGER_WAGON_ELECTRIC
                        }
                    }
                    EC_MONORAIL => LS_PASSENGER_WAGON_MONORAIL,
                    EC_MAGLEV => LS_PASSENGER_WAGON_MAGLEV,
                    _ => unreachable!("NOT_REACHED"),
                }
            } else {
                let mu = (s.engine_misc)(e) & RAIL_IS_MU != 0;
                match (s.engine_class)(e) {
                    EC_STEAM => LS_STEAM,
                    EC_DIESEL => {
                        if mu {
                            LS_DMU
                        } else {
                            LS_DIESEL
                        }
                    }
                    EC_ELECTRIC => {
                        if mu {
                            LS_EMU
                        } else {
                            LS_ELECTRIC
                        }
                    }
                    EC_MONORAIL => LS_MONORAIL,
                    EC_MAGLEV => LS_MAGLEV,
                    _ => unreachable!("NOT_REACHED"),
                }
            }
        }
        crate::vehicle_type::VEH_ROAD => {
            if !v.is_null() && parent != EngineID::INVALID {
                engine = parent;
                e = (s.engine)(engine);
                cargo = (s.vehicle_cargo)((s.vehicle_first)(v));
            }
            if !is_valid_cargo_type(cargo) {
                cargo = (s.engine_cargo)(e);
            }
            if !is_valid_cargo_type(cargo) {
                cargo =
                    crate::cargo_registry::openttd_rust_cargo_by_label(crate::cargo_type::CT_GOODS);
            }
            assert!(is_valid_cargo_type(cargo));
            if (s.engine_misc)(e) & ROAD_IS_TRAM != 0 {
                if crate::cargotype::is_cargo_in_class(
                    (s.cargo_classes)(cargo),
                    crate::cargotype::PASSENGERS,
                ) {
                    LS_PASSENGER_TRAM
                } else {
                    LS_FREIGHT_TRAM
                }
            } else if crate::cargotype::is_cargo_in_class(
                (s.cargo_classes)(cargo),
                crate::cargotype::PASSENGERS,
            ) {
                LS_BUS
            } else {
                LS_TRUCK
            }
        }
        crate::vehicle_type::VEH_SHIP => {
            if !is_valid_cargo_type(cargo) {
                cargo = (s.engine_cargo)(e);
            }
            if !is_valid_cargo_type(cargo) {
                cargo =
                    crate::cargo_registry::openttd_rust_cargo_by_label(crate::cargo_type::CT_GOODS);
            }
            assert!(is_valid_cargo_type(cargo));
            if crate::cargotype::is_cargo_in_class(
                (s.cargo_classes)(cargo),
                crate::cargotype::PASSENGERS,
            ) {
                LS_PASSENGER_SHIP
            } else {
                LS_FREIGHT_SHIP
            }
        }
        crate::vehicle_type::VEH_AIRCRAFT => match (s.aircraft_subtype)(e) {
            AIR_HELI => LS_HELICOPTER,
            AIR_CTOL => LS_SMALL_PLANE,
            x if x == AIR_CTOL | AIR_FAST => LS_LARGE_PLANE,
            _ => unreachable!("NOT_REACHED"),
        },
        _ => unreachable!("NOT_REACHED"),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_engine_livery_scheme(
    s: &Services,
    engine: EngineID,
    parent: EngineID,
    v: *mut OpenTTDVehicle,
) -> u8 {
    engine_livery(s, engine, parent, v)
}

crate::abi::layouts! {
    fn abi_layout;
 "OpenTTDCompanyNameStorage" ["company_economy"] => NameStorage { bytes, };
 "OpenTTDCompanyIdentityStrings" ["company_economy"] => Strings { unnamed, president, company, president_generated, andco, town_begin, town_end, company_begin, };
 "OpenTTDCompanyIdentityServices" ["company_economy"] => Services { shared, randomizer, faces, groups, strings, company, next_company, company_id, name_id, set_name_id, name_parts, set_name_parts, president_id, set_president_id, president_parts, set_president_parts, colour, set_colour, last_build, is_ai, name, set_name, president, set_president, livery, face, name_frame_init, name_frame_drop, render, render_other, company_renamed, president_renamed, launch_news, admin_update, rename_transport, president_windows, colour_windows, object_colours, dirty, reset_colourmap, engine, engine_type, engine_cargo, engine_kind, engine_class, engine_misc, aircraft_subtype, vehicle_cargo, vehicle_first, articulated, wagon_override, cargo_freight, cargo_classes, vehicle_id, invalidate_vehicle, vehicle_owner, };
}
