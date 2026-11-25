/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The cargo registry: the globals of `cargotype.cpp` and the bodies over `CargoSpec::array`
//! from `cargotype.cpp`, `newgrf_cargo.cpp`, `newgrf.cpp`, `newgrf/newgrf_act0_cargo.cpp`
//! and `town_cmd.cpp`.
//!
//! The specs stay in C++'s `CargoSpec::array` and the files in `_grf_files`; both are read
//! through their handles ([`CargoSpecRef`], [`GRFFileRef`]). The registry's state is
//! process-wide. It is reset and rebuilt only by `NewGRF` loading and language loading, which
//! run outside the game loop and command execution. C++ reads the two masks, the sort order
//! and the three label tables in place, and the sorted lists through spans this module
//! publishes after each change. No borrow of the state lasts across a service call.
use crate::bitmath_func::{clr_bit64, set_bit64};
use crate::cargo_type::{CargoLabel, CargoType, CargoTypes, NUM_CARGO};
use crate::cargotype::{
    CargoClass, CargoSpec, CargoSpecRef, TownAcceptanceEffect, TownProductionEffect,
    cargo_spec_get, iterate,
};
use crate::game_state::GameState;
use crate::gfx_type::SpriteID;
use crate::landscape_type::{LandscapeType, NUM_LANDSCAPE};
use crate::newgrf::{GRFFile, GRFFileRef, GrfSpecFeature};
use crate::newgrf_callbacks::CallbackID;
use crate::newgrf_spritegroup::{self, State};
use crate::services_assert::assert_with;
use crate::table::strings::{
    STR_NEWGRF_INVALID_CARGO, STR_NEWGRF_INVALID_CARGO_ABBREV, STR_NEWGRF_INVALID_CARGO_QUANTITY,
};
use std::{collections::BTreeMap, ffi::c_void, ptr};

type Handle = *const c_void;

/// `std::span<const CargoLabel>`: a cargo translation table, live while its owner is
/// unchanged (a `GRFFile`'s `cargo_list`, or one of the registry's two label tables).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Labels {
    pub(crate) data: *const CargoLabel,
    pub(crate) size: usize,
}

impl Labels {
    /// The labels, borrowed for a search with no call in between.
    #[allow(
        unsafe_code,
        reason = "the span's live labels; an empty span may carry a null pointer"
    )]
    pub(crate) fn as_slice(&self) -> &[CargoLabel] {
        if self.size == 0 {
            &[]
        } else {
            // SAFETY: A span holds `size` live labels until its owner changes, and the
            // returned slice borrows the span for a search that makes no call.
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
}

/// The cargo registry table: the `CargoSpec` members Rust reads and writes, the C++ bodies
/// over the default cargo table, the sorted-list spans GUI code reads, and the resolver
/// frame of `CargoResolverObject`.
#[repr(C)]
pub struct Services {
    /// The common sprite-group table.
    common: *const newgrf_spritegroup::Services,
    /// The shared table.
    shared: *const crate::services::Services,

    /// `CargoSpec::Get`, with its native index assertion.
    pub(crate) spec: extern "C" fn(u8) -> *const CargoSpec,
    /// `*insert = std::visit(visitor{}, _default_climate_cargo[l][slot])` (`SetupCargoForClimate`),
    /// with the visitor's `NOT_REACHED`.
    copy_default_cargo: extern "C" fn(u8, u8),
    /// `std::fill(insert, std::end(CargoSpec::array), CargoSpec{})` from slot `first`.
    fill_default: extern "C" fn(u8),

    /// `CargoSpec::bitnum`.
    pub(crate) bitnum: extern "C" fn(*const CargoSpec) -> u8,
    /// `CargoSpec::classes`.
    pub(crate) classes: extern "C" fn(*const CargoSpec) -> u16,
    /// `CargoSpec::Index`.
    pub(crate) index: extern "C" fn(*const CargoSpec) -> u8,
    /// `CargoSpec::label`.
    pub(crate) label: extern "C" fn(*const CargoSpec) -> u32,
    /// `CargoSpec::town_acceptance_effect`.
    pub(crate) town_acceptance_effect: extern "C" fn(*const CargoSpec) -> u8,
    /// `CargoSpec::town_production_effect`.
    pub(crate) town_production_effect: extern "C" fn(*const CargoSpec) -> u8,
    /// `CargoSpec::town_production_multiplier`.
    pub(crate) town_production_multiplier: extern "C" fn(*const CargoSpec) -> u16,

    /// Store `CargoSpec::abbrev`.
    pub(crate) set_abbrev: extern "C" fn(*mut CargoSpec, u32),
    /// Store `CargoSpec::grffile`.
    pub(crate) set_grffile: extern "C" fn(*mut CargoSpec, *const GRFFile),
    /// Store `CargoSpec::name`.
    pub(crate) set_name: extern "C" fn(*mut CargoSpec, u32),
    /// Store `CargoSpec::name_single`.
    pub(crate) set_name_single: extern "C" fn(*mut CargoSpec, u32),
    /// Store `CargoSpec::quantifier`.
    pub(crate) set_quantifier: extern "C" fn(*mut CargoSpec, u32),
    /// Store `CargoSpec::town_production_effect`.
    pub(crate) set_town_production_effect: extern "C" fn(*mut CargoSpec, u8),
    /// Store `CargoSpec::units_volume`.
    pub(crate) set_units_volume: extern "C" fn(*mut CargoSpec, u32),

    /// Store the span `_sorted_cargo_specs`.
    set_sorted_cargo_specs: extern "C" fn(*mut *const CargoSpec, usize),
    /// Store the span `_sorted_standard_cargo_specs`.
    set_sorted_standard_cargo_specs: extern "C" fn(*mut *const CargoSpec, usize),
    /// Store the span `CargoSpec::town_production_cargoes[tpe]`.
    set_town_production_cargoes: extern "C" fn(u8, *mut *const CargoSpec, usize),
    /// `std::sort(..., &CargoSpecClassSorter)`, whose names are compared by `GetString`.
    sort: extern "C" fn(*mut *const CargoSpec, usize),

    /// Builds `CargoResolverObject object(cs, callback, callback_param1, callback_param2)`
    /// and calls `openttd_rust_cargo_evaluate_callback` with it.
    pub(crate) callback_frame:
        extern "C" fn(*const CargoSpec, u16, u32, u32, *mut i32, usize) -> u16,
    /// `ResultSpriteGroup::num_sprites`.
    num_sprites: extern "C" fn(Handle) -> u16,
    /// `ResultSpriteGroup::sprite`.
    sprite: extern "C" fn(Handle) -> u32,
    /// Builds `CargoResolverObject object(cs)` and calls `openttd_rust_cargo_evaluate_sprite`
    /// with it.
    sprite_frame: extern "C" fn(*const CargoSpec) -> u32,
}

/// Test-only table for `crate::test_tables`; its private fields keep it in this module.
#[cfg(test)]
pub(crate) const TEST_TABLE: Services = Services {
    common: std::ptr::null(),
    shared: std::ptr::null(),

    spec: crate::test_tables::unreachable1,
    copy_default_cargo: crate::test_tables::unreachable2,
    fill_default: crate::test_tables::unreachable1,

    bitnum: crate::test_tables::unreachable1,
    classes: crate::test_tables::unreachable1,
    index: crate::test_tables::unreachable1,
    label: crate::test_tables::unreachable1,
    town_acceptance_effect: crate::test_tables::unreachable1,
    town_production_effect: crate::test_tables::unreachable1,
    town_production_multiplier: crate::test_tables::unreachable1,

    set_abbrev: crate::test_tables::unreachable2,
    set_grffile: crate::test_tables::unreachable2,
    set_name: crate::test_tables::unreachable2,
    set_name_single: crate::test_tables::unreachable2,
    set_quantifier: crate::test_tables::unreachable2,
    set_town_production_effect: crate::test_tables::unreachable2,
    set_units_volume: crate::test_tables::unreachable2,

    set_sorted_cargo_specs: crate::test_tables::unreachable2,
    set_sorted_standard_cargo_specs: crate::test_tables::unreachable2,
    set_town_production_cargoes: crate::test_tables::unreachable3,
    sort: crate::test_tables::unreachable2,

    callback_frame: crate::test_tables::unreachable6,
    num_sprites: crate::test_tables::unreachable1,
    sprite: crate::test_tables::unreachable1,
    sprite_frame: crate::test_tables::unreachable1,
};

/// The globals of `cargotype.cpp`, under their names, with the table the last setup or
/// sort installed.
struct Registry {
    services: *const Services,
    /// Bitmask of cargo types available. This includes phony cargoes like regearing cargoes.
    cargo_mask: CargoTypes,
    /// Bitmask of real cargo types available. Phony cargoes like regearing cargoes are
    /// excluded.
    standard_cargo_mask: CargoTypes,
    /// List of default cargo labels, used when setting up cargo types for default vehicles.
    /// This is done by label so that a cargo label can be redefined in a different slot.
    default_cargo_labels: Vec<CargoLabel>,
    /// Default cargo translation for up to version 7 `NewGRF`s.
    /// This maps the original 12 cargo slots to their original label. If a climate dependent
    /// cargo is not present it will map to `CT_INVALID`. For default cargoes this ends up as a
    /// 1:1 mapping via climate slot -> label -> cargo type.
    climate_dependent_cargo_labels: [CargoLabel; 12],
    /// Default cargo translation for version 8+ `NewGRF`s.
    /// This maps the 32 "bitnum" cargo slots to their original label. If a bitnum is not
    /// present it will map to `CT_INVALID`.
    climate_independent_cargo_labels: [CargoLabel; 32],
    /// `CargoSpec::label_map`.
    label_map: BTreeMap<CargoLabel, CargoType>,
    /// Cargo specifications sorted alphabetically by name.
    sorted_cargo_specs: Vec<*const CargoSpec>,
    /// Sort order of cargoes by cargo type.
    sorted_cargo_types: [u8; NUM_CARGO as usize],
    /// List of cargo specs for each Town Product Effect.
    town_production_cargoes: [Vec<*const CargoSpec>; TownProductionEffect::NUM_TPE.0 as usize],
}

static REGISTRY: GameState<Registry> = GameState::new(Registry {
    services: ptr::null(),
    cargo_mask: CargoTypes(0),
    standard_cargo_mask: CargoTypes(0),
    default_cargo_labels: Vec::new(),
    climate_dependent_cargo_labels: [CargoLabel(0); 12],
    climate_independent_cargo_labels: [CargoLabel(0); 32],
    label_map: BTreeMap::new(),
    sorted_cargo_specs: Vec::new(),
    sorted_cargo_types: [0; NUM_CARGO as usize],
    town_production_cargoes: [const { Vec::new() }; TownProductionEffect::NUM_TPE.0 as usize],
});

/// Run `f` on the registry state. `f` makes no service call, so no other borrow of the
/// state can begin while it runs.
#[allow(
    unsafe_code,
    reason = "the process-wide registry, borrowed for one closure that makes no call"
)]
fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    // SAFETY: The state is reset only by serialized loading, and the borrow ends before any
    // service call (the module's contract).
    f(unsafe { &mut *REGISTRY.get() })
}

/// The table the last setup or sort installed.
#[allow(unsafe_code, reason = "the installed process-lifetime table")]
pub(crate) fn services() -> &'static Services {
    let s = with_registry(|r| r.services);
    // SAFETY: Setup and sort install the process-lifetime table before any reader runs.
    unsafe { &*s }
}

impl Services {
    /// The shared table.
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: Both tables are process-lifetime.
        unsafe { &*self.shared }
    }

    /// The common sprite-group table.
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    fn common(&self) -> &newgrf_spritegroup::Services {
        // SAFETY: Both tables are process-lifetime.
        unsafe { &*self.common }
    }
}

/// The file the original dereferences: `_cur_gps.grffile`, or a spec's `grffile`.
#[allow(unsafe_code, reason = "the caller's live file")]
fn grf_file(file: Handle) -> GRFFileRef<'static> {
    // SAFETY: Each caller passes a live `GRFFile` the original dereferences unchecked.
    unsafe { GRFFileRef::of(services().shared(), file.cast()) }
}

#[allow(unsafe_code, reason = "export; C++ binds a reference to the global")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_mask() -> *mut u64 {
    with_registry(|r| (&raw mut r.cargo_mask).cast())
}

#[allow(unsafe_code, reason = "export; C++ binds a reference to the global")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_standard_cargo_mask() -> *mut u64 {
    with_registry(|r| (&raw mut r.standard_cargo_mask).cast())
}

#[allow(unsafe_code, reason = "export; C++ binds a span to the global")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_reverse() -> *mut u8 {
    with_registry(|r| (&raw mut r.sorted_cargo_types).cast())
}

/// `SetupCargoForClimate`: set up the default cargo types for the given landscape type.
pub(crate) fn setup_cargo_for_climate(s: &'static Services, l: LandscapeType) {
    assert_with(|| l.0 < NUM_LANDSCAPE);

    with_registry(|r| {
        r.services = s;
        r.cargo_mask = CargoTypes(0);
        r.default_cargo_labels.clear();
        r.climate_dependent_cargo_labels
            .fill(CargoLabel::CT_INVALID);
        r.climate_independent_cargo_labels
            .fill(CargoLabel::CT_INVALID);
    });

    // Copy from default cargo by label or index.
    let mut insert = 0;
    while insert < CargoType::NUM_ORIGINAL_CARGO.0 {
        // `*insert = std::visit(visitor{}, cl)`, whose visitor the table's body holds:
        // Copy the default cargo by index.
        // Search for label in default cargo types and copy if found.
        // Index or label is invalid, this should not happen.
        (s.copy_default_cargo)(l.0, insert);
        let cs = cargo_spec_get(s, CargoType(insert));

        if cs.is_valid() {
            let index = cs.index();
            let label = cs.label();
            let bitnum = cs.bitnum();
            with_registry(|r| {
                set_bit64(&mut r.cargo_mask.0, index.0);
                r.default_cargo_labels.push(label);
                r.climate_dependent_cargo_labels[usize::from(index.0)] = label;
                r.climate_independent_cargo_labels[usize::from(bitnum)] = label;
            });
        }
        insert += 1;
    }

    // Reset and disable remaining cargo types.
    (s.fill_default)(insert);

    build_cargo_label_map();
}

#[allow(unsafe_code, reason = "export and the caller's process-lifetime table")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_setup(s: *const Services, climate: u8) {
    // SAFETY: The caller passes the process-lifetime table.
    setup_cargo_for_climate(unsafe { &*s }, LandscapeType(climate));
}

/// `GetClimateDependentCargoTranslationTable`: get default climate-dependent cargo
/// translation table for a `NewGRF`, used if the `NewGRF` does not provide its own.
pub(crate) fn get_climate_dependent_cargo_translation_table() -> Labels {
    with_registry(|r| Labels {
        data: r.climate_dependent_cargo_labels.as_ptr(),
        size: r.climate_dependent_cargo_labels.len(),
    })
}

#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_climate_labels() -> Labels {
    get_climate_dependent_cargo_translation_table()
}

/// `GetClimateIndependentCargoTranslationTable`: get default climate-independent cargo
/// translation table for a `NewGRF`, used if the `NewGRF` does not provide its own.
pub(crate) fn get_climate_independent_cargo_translation_table() -> Labels {
    with_registry(|r| Labels {
        data: r.climate_independent_cargo_labels.as_ptr(),
        size: r.climate_independent_cargo_labels.len(),
    })
}

/// `BuildCargoLabelMap`: build cargo label map.
/// This is called multiple times during `NewGRF` initialization as cargos are defined, so
/// that `TranslateRefitMask()` and `GetCargoTranslation()`, also used during initialization,
/// get the correct information.
pub(crate) fn build_cargo_label_map() {
    with_registry(|r| r.label_map.clear());
    let s = services();
    for index in 0..CargoType::NUM_CARGO.0 {
        let cs = cargo_spec_get(s, CargoType(index));
        // During initialization, CargoSpec can be marked valid before the label has been set.
        if !cs.is_valid() {
            continue;
        }
        let label = cs.label();
        // `CargoLabel{}`.
        if label == CargoLabel(0) || label == CargoLabel::CT_INVALID {
            continue;
        }
        // Label already exists, don't add again.
        if with_registry(|r| r.label_map.contains_key(&label)) {
            continue;
        }

        let index = cs.index();
        with_registry(|r| r.label_map.insert(label, index));
    }
}

#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_labels_rebuild() {
    build_cargo_label_map();
}

/// `CargoReserveInfo`'s property `0x08` (bit number of cargo), after
/// `cs->bitnum = buf.ReadByte()` (`newgrf/newgrf_act0_cargo.cpp`).
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_bitnum_changed(
    spec: *const CargoSpec,
    id: u8,
    grffile: *const GRFFile,
) {
    let cs = CargoSpecRef::of(services(), spec);
    if cs.is_valid() {
        cs.set_grffile(grffile);
        with_registry(|r| set_bit64(&mut r.cargo_mask.0, id));
    } else {
        with_registry(|r| clr_bit64(&mut r.cargo_mask.0, id));
    }
    build_cargo_label_map();
}

/// `GetCargoTypeByLabel`: get the cargo type of `label`, or `INVALID_CARGO`.
pub(crate) fn get_cargo_type_by_label(label: CargoLabel) -> CargoType {
    with_registry(|r| {
        r.label_map
            .get(&label)
            .copied()
            .unwrap_or(CargoType::INVALID_CARGO)
    })
}

#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_by_label(label: u32) -> u8 {
    get_cargo_type_by_label(CargoLabel(label)).0
}

/// `IsDefaultCargo`: test if a cargo is a default cargo type.
pub(crate) fn is_default_cargo(cargo_type: CargoType) -> bool {
    let cs = cargo_spec_get(services(), cargo_type);
    if !cs.is_valid() {
        return false;
    }

    let label = cs.label();
    with_registry(|r| r.default_cargo_labels.contains(&label))
}

#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_is_default_abi(cargo: u8) -> u8 {
    u8::from(is_default_cargo(CargoType(cargo)))
}

/// `CargoSpec::town_production_cargoes[tpe]`: the cargo specs of one town production effect.
/// Each call reads the list again, so no borrow of the registry lasts across a caller's
/// random draw or callback.
#[derive(Clone, Copy)]
pub(crate) struct TownProductionCargoes {
    tpe: TownProductionEffect,
}

/// `CargoSpec::town_production_cargoes[tpe]`.
pub(crate) fn town_production_cargoes(tpe: TownProductionEffect) -> TownProductionCargoes {
    TownProductionCargoes { tpe }
}

impl TownProductionCargoes {
    /// `size()`.
    pub(crate) fn len(self) -> usize {
        with_registry(|r| r.town_production_cargoes[usize::from(self.tpe.0)].len())
    }

    /// `operator[]`: the spec at `index`, which is below [`Self::len`].
    pub(crate) fn get(self, index: usize) -> CargoSpecRef<'static> {
        let cs = with_registry(|r| r.town_production_cargoes[usize::from(self.tpe.0)][index]);
        CargoSpecRef::of(services(), cs)
    }
}

/// `InitializeSortedCargoSpecs`: initialize the list of sorted cargo specifications.
pub(crate) fn initialize_sorted_cargo_specs(s: &'static Services) {
    // Language loading sorts the initially empty specification array before climate setup.
    with_registry(|r| r.services = s);
    for tpe in 0..TownProductionEffect::NUM_TPE.0 {
        let data = with_registry(|r| {
            let tpc = &mut r.town_production_cargoes[usize::from(tpe)];
            tpc.clear();
            tpc.as_mut_ptr()
        });
        (s.set_town_production_cargoes)(tpe, data, 0);
    }
    let data = with_registry(|r| {
        r.sorted_cargo_specs.clear();
        r.sorted_cargo_specs.as_mut_ptr()
    });
    (s.set_sorted_cargo_specs)(data, 0);
    // Add each cargo spec to the list, and determine the largest cargo icon size.
    for cargo in iterate(s) {
        with_registry(|r| r.sorted_cargo_specs.push(cargo.as_ptr()));
    }
    let (data, count) = with_registry(|r| {
        (
            r.sorted_cargo_specs.as_mut_ptr(),
            r.sorted_cargo_specs.len(),
        )
    });
    (s.set_sorted_cargo_specs)(data, count);

    // Sort cargo specifications by cargo class and name.
    (s.sort)(data, count);

    // Populate
    let sorted = |index: usize| CargoSpecRef::of(s, with_registry(|r| r.sorted_cargo_specs[index]));
    for it in 0..count {
        let cargo = sorted(it).index();
        #[allow(
            clippy::cast_possible_truncation,
            reason = "static_cast<uint8_t>(it - std::begin(_sorted_cargo_specs))"
        )]
        with_registry(|r| r.sorted_cargo_types[usize::from(cargo.0)] = it as u8);
    }

    // Count the number of standard cargos and fill the mask.
    with_registry(|r| r.standard_cargo_mask = CargoTypes(0));
    let mut nb_standard_cargo: u8 = 0;
    for it in 0..count {
        let cargo = sorted(it);
        let town_production_effect = cargo.town_production_effect();
        assert_with(|| town_production_effect != TownProductionEffect::INVALID_TPE);
        let (town_data, town_count) = with_registry(|r| {
            let tpc = &mut r.town_production_cargoes[usize::from(town_production_effect.0)];
            tpc.push(cargo.as_ptr());
            (tpc.as_mut_ptr(), tpc.len())
        });
        (s.set_town_production_cargoes)(town_production_effect.0, town_data, town_count);
        if cargo.classes().test(CargoClass::Special) {
            break;
        }
        nb_standard_cargo += 1;
        let index = cargo.index();
        with_registry(|r| set_bit64(&mut r.standard_cargo_mask.0, index.0));
    }

    // _sorted_standard_cargo_specs is a subset of _sorted_cargo_specs.
    (s.set_sorted_standard_cargo_specs)(data, usize::from(nb_standard_cargo));
}

#[allow(unsafe_code, reason = "export and the caller's process-lifetime table")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_sort(s: *const Services) {
    // SAFETY: The caller passes the process-lifetime table.
    initialize_sorted_cargo_specs(unsafe { &*s });
}

/// `FinaliseCargoArray`: check for invalid cargoes.
pub(crate) fn finalise_cargo_array(s: &Services) {
    for index in 0..CargoType::NUM_CARGO.0 {
        let cs = cargo_spec_get(s, CargoType(index));
        if cs.town_production_effect() == TownProductionEffect::INVALID_TPE {
            // Set default town production effect by cargo label.
            cs.set_town_production_effect(match cs.label() {
                CargoLabel::CT_PASSENGERS => TownProductionEffect::TPE_PASSENGERS,
                CargoLabel::CT_MAIL => TownProductionEffect::TPE_MAIL,
                _ => TownProductionEffect::TPE_NONE,
            });
        }
        if !cs.is_valid() {
            // `cs.name = cs.name_single = cs.units_volume = STR_NEWGRF_INVALID_CARGO`.
            cs.set_units_volume(STR_NEWGRF_INVALID_CARGO);
            cs.set_name_single(STR_NEWGRF_INVALID_CARGO);
            cs.set_name(STR_NEWGRF_INVALID_CARGO);
            cs.set_quantifier(STR_NEWGRF_INVALID_CARGO_QUANTITY);
            cs.set_abbrev(STR_NEWGRF_INVALID_CARGO_ABBREV);
        }
    }
}

/// Kept for the cargo registry console fixture (#634).
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_finalise() {
    finalise_cargo_array(services());
}

/// `GetCargoTranslationTable`: get the cargo translation table to use for the given GRF file.
pub(crate) fn get_cargo_translation_table(file: Handle) -> Labels {
    let grffile = grf_file(file);
    // Always use the translation table if it's installed.
    let cargo_list = grffile.cargo_list();
    if cargo_list.size != 0 {
        return cargo_list;
    }

    // Pre-v7 use climate-dependent "slot" table.
    if grffile.grf_version() < 7 {
        return get_climate_dependent_cargo_translation_table();
    }

    // Otherwise use climate-independent "bitnum" table.
    get_climate_independent_cargo_translation_table()
}

/// `GetCargoTranslation`: translate a GRF-local cargo slot/bitnum into a `CargoType`.
/// `usebit` defines the meaning of `cargo` for GRF version < 7: if true, then `cargo` is a
/// bitnum; if false, then `cargo` is a cargoslot. For GRF version >= 7 `cargo` is always a
/// translated cargo bit. Returns `CargoType` or `INVALID_CARGO` if the cargo is not available.
pub(crate) fn get_cargo_translation(cargo: u8, file: Handle, usebit: bool) -> CargoType {
    let grffile = grf_file(file);
    // We can't use GetCargoTranslationTable here as the usebit flag changes behaviour.
    // Pre-version 7 uses the bitnum lookup from (standard in v8) instead of climate dependent in some places..
    let cargo_list = if grffile.grf_version() < 7 && !usebit && !grffile.cargo_list_is_fallback() {
        get_climate_dependent_cargo_translation_table()
    } else {
        let cargo_list = grffile.cargo_list();
        if cargo_list.size != 0 {
            cargo_list
        } else {
            get_climate_independent_cargo_translation_table()
        }
    };

    match cargo_list.as_slice().get(usize::from(cargo)) {
        Some(&label) => get_cargo_type_by_label(label),
        None => CargoType::INVALID_CARGO,
    }
}

#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_translate(cargo: u8, file: Handle, usebit: bool) -> u8 {
    get_cargo_translation(cargo, file, usebit).0
}

/// `BuildCargoTranslationMap`: construct the cargo mapping, the reverse of a cargo
/// translation table, for the file being loaded.
pub(crate) fn build_cargo_translation_map(file: Handle) {
    let grffile = grf_file(file);
    grffile.cargo_map_fill(u8::MAX);

    let cargo_list = get_cargo_translation_table(file);

    for cs in iterate(services()) {
        // Check the translation table for this cargo's label
        let idx = crate::container_func::find_index(cargo_list.as_slice(), &cs.label());
        if idx >= 0 {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "cargo_map[cs->Index()] = idx stores the int to a uint8_t"
            )]
            grffile.set_cargo_map(cs.index(), idx as u8);
        }
    }
}

/// Kept for the cargo registry console fixture (#634).
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_build_inverse(file: Handle) {
    build_cargo_translation_map(file);
}

/// `MaybeInstallFallbackCargoLabel`: install cargo label in a fallback cargo list, if a
/// `NewGRF`-defined cargo list is not present. This allows this `NewGRF` to use its
/// self-defined cargo types without needing an explicit translation table.
pub(crate) fn maybe_install_fallback_cargo_label(
    file: Handle,
    id: u32,
    last: u32,
    label: CargoLabel,
) {
    let grffile = grf_file(file);
    if grffile.cargo_list().size == 0 {
        // Cargo translation table isn't configured yet, assume it won't be and configure the cargo list as a fallback list.
        let default_cargo_list = get_cargo_translation_table(file);
        grffile.cargo_list_assign(default_cargo_list);
        grffile.set_cargo_list_is_fallback(true);
    }

    if grffile.cargo_list_is_fallback() {
        // Automatically fill fallback cargo list with the defined label, resizing as needed.
        let last = last as usize;
        if grffile.cargo_list().size < last {
            grffile.cargo_list_resize(last, CargoLabel::CT_INVALID);
        }
        grffile.set_cargo_list(id, label);
    }
}

#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_fallback(file: Handle, id: u32, last: u32, label: u32) {
    maybe_install_fallback_cargo_label(file, id, last, CargoLabel(label));
}

/// `GetCustomCargoSprite`: get the custom sprite for the given cargo type, or 0 if not
/// available. The frame builds `CargoResolverObject object(cs)` and resolves in
/// [`openttd_rust_cargo_evaluate_sprite`].
pub(crate) fn get_custom_cargo_sprite(cs: CargoSpecRef<'_>) -> SpriteID {
    (cs.services().sprite_frame)(cs.as_ptr())
}

#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_custom_sprite(spec: *const CargoSpec) -> u32 {
    get_custom_cargo_sprite(CargoSpecRef::of(services(), spec))
}

/// `GetCargoCallback`. The frame builds
/// `CargoResolverObject object(cs, callback, param1, param2)` and resolves in
/// [`openttd_rust_cargo_evaluate_callback`].
pub(crate) fn get_cargo_callback(
    callback: CallbackID,
    param1: u32,
    param2: u32,
    cs: CargoSpecRef<'_>,
    regs100: &mut [i32],
) -> u16 {
    (cs.services().callback_frame)(
        cs.as_ptr(),
        callback.0,
        param1,
        param2,
        regs100.as_mut_ptr(),
        regs100.len(),
    )
}

/// # Safety
/// `regs` holds `count` writable values.
#[allow(unsafe_code, reason = "export and the caller's register span")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_callback(
    callback: u16,
    param1: u32,
    param2: u32,
    spec: *const CargoSpec,
    regs: *mut i32,
    count: usize,
) -> u16 {
    let regs100 = if count == 0 {
        &mut []
    } else {
        // SAFETY: The caller's span holds `count` writable values.
        unsafe { std::slice::from_raw_parts_mut(regs, count) }
    };
    get_cargo_callback(
        CallbackID(callback),
        param1,
        param2,
        CargoSpecRef::of(services(), spec),
        regs100,
    )
}

/// `object.ResolveCallback(regs100)` in `GetCargoCallback`'s live frame.
///
/// # Safety
/// The resolver frame and its state are live, and `regs` holds `count` writable values.
#[allow(unsafe_code, reason = "export and the live resolver frame")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_evaluate_callback(
    root: Handle,
    resolver: *mut c_void,
    state: *mut State,
    file: Handle,
    regs: *mut i32,
    count: usize,
) -> u16 {
    // SAFETY: The caller's live frame and register span.
    unsafe {
        newgrf_spritegroup::resolve_callback(
            services().common(),
            root,
            resolver,
            state,
            file,
            regs,
            count,
        )
    }
}

/// The rest of `GetCustomCargoSprite` in its live frame, from
/// `object.Resolve<ResultSpriteGroup>()`.
///
/// # Safety
/// The resolver frame and its state are live.
#[allow(unsafe_code, reason = "export and the live resolver frame")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_evaluate_sprite(
    root: Handle,
    resolver: *mut c_void,
    state: *mut State,
    file: Handle,
) -> u32 {
    let s = services();
    // SAFETY: The caller's live frame.
    let result = unsafe { newgrf_spritegroup::do_resolve(s.common(), root, resolver, state, file) };
    // ResolverResult alternative 2 is const ResultSpriteGroup*; another is a null group.
    if result.kind != 2 || result.group.is_null() || (s.num_sprites)(result.group) == 0 {
        return 0;
    }

    (s.sprite)(result.group)
}

/// `CargoResolverObject::GetFeature`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_feature() -> u8 {
    GrfSpecFeature::GSF_CARGOES.0
}

/// `CargoResolverObject::GetDebugID`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_debug_id(cargospec: *const CargoSpec) -> u32 {
    CargoSpecRef::of(services(), cargospec).label().0
}

/// `FindFirstCargoWithTownAcceptanceEffect`: determines the first cargo with a certain town
/// effect. Returns the first active cargo slot with that effect.
pub(crate) fn find_first_cargo_with_town_acceptance_effect(
    effect: TownAcceptanceEffect,
) -> Option<CargoSpecRef<'static>> {
    iterate(services()).find(|cs| cs.town_acceptance_effect() == effect)
}

/// `cargo_list.size()` of the given GRF file.
pub(crate) fn file_cargo_count(file: Handle) -> usize {
    grf_file(file).cargo_list().size
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoLabels" ["probe"] => Labels {data,size};
    "OpenTTDCargoRegistryServices" ["probe"] => Services {common,shared,spec,copy_default_cargo,fill_default,bitnum,classes,index,label,town_acceptance_effect,town_production_effect,town_production_multiplier,set_abbrev,set_grffile,set_name,set_name_single,set_quantifier,set_town_production_effect,set_units_volume,set_sorted_cargo_specs,set_sorted_standard_cargo_specs,set_town_production_cargoes,sort,callback_frame,num_sprites,sprite,sprite_frame};
}

/// Test-only: the current file reports `labels` as its cargo list at version 8 with no
/// fallback, and each label maps to its cargo. Forget it with [`clear_test_labels`].
#[cfg(test)]
pub(crate) fn install_test_labels(labels: &'static [u32], cargos: &[u8]) {
    use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
    static DATA: AtomicPtr<u32> = AtomicPtr::new(ptr::null_mut());
    static SIZE: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn cargo_list(_: Handle) -> Labels {
        Labels {
            data: DATA.load(Ordering::Relaxed).cast_const().cast(),
            size: SIZE.load(Ordering::Relaxed),
        }
    }
    extern "C" fn cargo_list_is_fallback(_: Handle) -> bool {
        false
    }
    extern "C" fn grf_file_version(_: Handle) -> u8 {
        8
    }
    DATA.store(labels.as_ptr().cast_mut(), Ordering::Relaxed);
    SIZE.store(labels.len(), Ordering::Relaxed);
    let mut shared = crate::test_tables::SHARED;
    shared.grf_file_cargo_list = cargo_list;
    shared.grf_file_cargo_list_is_fallback = cargo_list_is_fallback;
    shared.grf_file_version = grf_file_version;
    let mut table = TEST_TABLE;
    table.shared = Box::leak(Box::new(shared));
    let table: &'static Services = Box::leak(Box::new(table));
    with_registry(|r| {
        r.services = table;
        r.label_map.clear();
        for (label, cargo) in labels.iter().zip(cargos) {
            r.label_map.insert(CargoLabel(*label), CargoType(*cargo));
        }
    });
}

/// Test-only: the registry holds no valid cargo spec, so [`iterate`] visits none. Forget it
/// with [`clear_test_labels`].
#[cfg(test)]
pub(crate) fn install_test_no_cargo() {
    extern "C" fn spec(_: u8) -> *const CargoSpec {
        ptr::null()
    }
    extern "C" fn bitnum(_: *const CargoSpec) -> u8 {
        crate::cargotype::INVALID_CARGO_BITNUM
    }
    let mut table = TEST_TABLE;
    table.spec = spec;
    table.bitnum = bitnum;
    let table: &'static Services = Box::leak(Box::new(table));
    with_registry(|r| r.services = table);
}

/// Test-only: forget [`install_test_labels`] and [`install_test_no_cargo`].
#[cfg(test)]
pub(crate) fn clear_test_labels() {
    with_registry(|r| {
        r.services = ptr::null();
        r.label_map.clear();
    });
}

#[cfg(test)]
mod tests {
    use super::{TEST_TABLE, finalise_cargo_array};
    use crate::cargo_type::{CargoLabel, CargoType};
    use crate::cargotype::{CargoSpec, INVALID_CARGO_BITNUM, TownProductionEffect};
    use crate::table::strings::{
        STR_NEWGRF_INVALID_CARGO, STR_NEWGRF_INVALID_CARGO_ABBREV,
        STR_NEWGRF_INVALID_CARGO_QUANTITY,
    };
    use std::cell::RefCell;

    /// One spec per cargo slot, as `FinaliseCargoArray` reads and writes it.
    #[derive(Clone, Copy)]
    struct Spec {
        label: CargoLabel,
        bitnum: u8,
        town_production_effect: TownProductionEffect,
    }

    thread_local! {
        static SPECS: RefCell<Vec<Spec>> = const { RefCell::new(Vec::new()) };
        static STORES: RefCell<Vec<(u8, &'static str, u32)>> = const { RefCell::new(Vec::new()) };
    }

    // Each spec pointer is its slot number, offset by one so that slot 0 is not null; no
    // stub reads through it.
    fn slot(cs: *const CargoSpec) -> usize {
        cs.addr() - 1
    }
    extern "C" fn spec(cargo: u8) -> *const CargoSpec {
        std::ptr::without_provenance(usize::from(cargo) + 1)
    }
    extern "C" fn index(cs: *const CargoSpec) -> u8 {
        u8::try_from(slot(cs)).unwrap()
    }
    extern "C" fn bitnum(cs: *const CargoSpec) -> u8 {
        SPECS.with_borrow(|specs| specs[slot(cs)].bitnum)
    }
    extern "C" fn label(cs: *const CargoSpec) -> u32 {
        SPECS.with_borrow(|specs| specs[slot(cs)].label.0)
    }
    extern "C" fn town_production_effect(cs: *const CargoSpec) -> u8 {
        SPECS.with_borrow(|specs| specs[slot(cs)].town_production_effect.0)
    }
    extern "C" fn set_town_production_effect(cs: *mut CargoSpec, effect: u8) {
        SPECS.with_borrow_mut(|specs| {
            specs[slot(cs)].town_production_effect = TownProductionEffect(effect);
        });
        record(cs, "town_production_effect", u32::from(effect));
    }
    fn record(cs: *mut CargoSpec, member: &'static str, value: u32) {
        STORES.with_borrow_mut(|stores| {
            stores.push((u8::try_from(slot(cs)).unwrap(), member, value));
        });
    }
    extern "C" fn set_name(cs: *mut CargoSpec, value: u32) {
        record(cs, "name", value);
    }
    extern "C" fn set_name_single(cs: *mut CargoSpec, value: u32) {
        record(cs, "name_single", value);
    }
    extern "C" fn set_units_volume(cs: *mut CargoSpec, value: u32) {
        record(cs, "units_volume", value);
    }
    extern "C" fn set_quantifier(cs: *mut CargoSpec, value: u32) {
        record(cs, "quantifier", value);
    }
    extern "C" fn set_abbrev(cs: *mut CargoSpec, value: u32) {
        record(cs, "abbrev", value);
    }

    /// Runs `FinaliseCargoArray` over `specs` (the rest of the array is valid passengers with
    /// an explicit effect) and returns the stores it made, in order.
    fn finalise(specs: &[Spec]) -> Vec<(u8, &'static str, u32)> {
        let mut all = vec![
            Spec {
                label: CargoLabel::CT_PASSENGERS,
                bitnum: 0,
                town_production_effect: TownProductionEffect::TPE_NONE,
            };
            usize::from(CargoType::NUM_CARGO.0)
        ];
        all[..specs.len()].copy_from_slice(specs);
        SPECS.set(all);
        STORES.with_borrow_mut(Vec::clear);
        let mut table = TEST_TABLE;
        table.spec = spec;
        table.index = index;
        table.bitnum = bitnum;
        table.label = label;
        table.town_production_effect = town_production_effect;
        table.set_town_production_effect = set_town_production_effect;
        table.set_name = set_name;
        table.set_name_single = set_name_single;
        table.set_units_volume = set_units_volume;
        table.set_quantifier = set_quantifier;
        table.set_abbrev = set_abbrev;
        finalise_cargo_array(&table);
        STORES.take()
    }

    /// `FinaliseCargoArray` stores the five strings of an invalid spec with their constants,
    /// in the chained assignment's order, and none for a valid spec. The console observes
    /// only `name`.
    #[test]
    fn finalise_cargo_array_invalidates_each_string_of_an_invalid_spec() {
        let valid = Spec {
            label: CargoLabel::CT_COAL,
            bitnum: 1,
            town_production_effect: TownProductionEffect::TPE_NONE,
        };
        let invalid = Spec {
            bitnum: INVALID_CARGO_BITNUM,
            ..valid
        };
        assert_eq!(
            finalise(&[valid, invalid]),
            [
                (1, "units_volume", STR_NEWGRF_INVALID_CARGO.0),
                (1, "name_single", STR_NEWGRF_INVALID_CARGO.0),
                (1, "name", STR_NEWGRF_INVALID_CARGO.0),
                (1, "quantifier", STR_NEWGRF_INVALID_CARGO_QUANTITY.0),
                (1, "abbrev", STR_NEWGRF_INVALID_CARGO_ABBREV.0),
            ]
        );
    }

    /// `FinaliseCargoArray` gives a spec labelled `CT_MAIL` with no effect `TPE_MAIL`. The
    /// console observes the passenger and default arms and an explicit effect being kept.
    #[test]
    fn finalise_cargo_array_defaults_mail_to_tpe_mail() {
        let mail = Spec {
            label: CargoLabel::CT_MAIL,
            bitnum: 2,
            town_production_effect: TownProductionEffect::INVALID_TPE,
        };
        assert_eq!(
            finalise(&[mail]),
            [(
                0,
                "town_production_effect",
                u32::from(TownProductionEffect::TPE_MAIL.0)
            )]
        );
    }
}
