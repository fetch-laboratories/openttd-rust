/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `industrytype.h`: the industry and industry-tile specifications, their canonical storage
//! and the spec methods. Native readers view the records in place through the generated
//! declarations. Every access holds the game-state mutex; no reference into the storage
//! survives a native callback.
#![allow(unsafe_code)]
use crate::game_state::GameState;
use crate::industry_type::{
    INDUSTRY_NUM_INPUTS, INDUSTRY_NUM_OUTPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    INDUSTRY_ORIGINAL_NUM_OUTPUTS, NUM_INDUSTRYTILES, NUM_INDUSTRYTYPES,
};
use crate::newgrf::CargoLabel;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ptr::NonNull;

/// `NUM_LANDSCAPE`.
pub const NUM_LANDSCAPE: usize = 4;
/// `AnimationStatus::NoAnimation`.
const NO_ANIMATION: u8 = 0xff;

/// A Rust-owned vector that native readers view in place as `RustVector<T>`. Native
/// tables also build borrowed views with zero capacity, which Rust only clones.
#[repr(C)]
pub struct NativeVec<T> {
    items: *mut T,
    count: usize,
    capacity: usize,
}
impl<T> NativeVec<T> {
    pub const fn new() -> Self {
        Self {
            items: NonNull::dangling().as_ptr(),
            count: 0,
            capacity: 0,
        }
    }
    pub fn from_vec(vec: Vec<T>) -> Self {
        let mut vec = std::mem::ManuallyDrop::new(vec);
        Self {
            items: vec.as_mut_ptr(),
            count: vec.len(),
            capacity: vec.capacity(),
        }
    }
    pub(crate) const fn as_mut_ptr(&self) -> *mut T {
        self.items
    }
    pub(crate) fn resize(&mut self, count: usize, value: T)
    where
        T: Clone,
    {
        self.edit(|vec| vec.resize(count, value));
    }
    pub fn as_slice(&self) -> &[T] {
        if self.count == 0 {
            &[]
        } else {
            // SAFETY: owned storage or a process-lifetime native table of `count` elements.
            unsafe { std::slice::from_raw_parts(self.items, self.count) }
        }
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub(crate) fn into_vec(mut self) -> Vec<T> {
        let vec = if self.capacity == 0 {
            Vec::new()
        } else {
            // SAFETY: nonzero capacity marks an allocation from `from_vec`.
            unsafe { Vec::from_raw_parts(self.items, self.count, self.capacity) }
        };
        self.capacity = 0;
        self.count = 0;
        vec
    }
    // The canonical owner is empty while editing; unwind drops the detached vector.
    // Callers finish edits before callbacks can observe native aliases.
    pub(crate) fn edit<R>(&mut self, edit: impl FnOnce(&mut Vec<T>) -> R) -> R {
        let mut vec = std::mem::take(self).into_vec();
        let result = edit(&mut vec);
        *self = Self::from_vec(vec);
        result
    }
    pub(crate) fn pop(&mut self) -> Option<T> {
        self.edit(Vec::pop)
    }
    pub(crate) fn insert(&mut self, index: usize, value: T) {
        self.edit(|vec| vec.insert(index, value));
    }
    pub(crate) fn resize_with(&mut self, size: usize, make: impl FnMut() -> T) {
        self.edit(|vec| vec.resize_with(size, make));
    }
    pub fn push(&mut self, value: T) {
        self.edit(|vec| vec.push(value));
    }
}
impl<T> std::ops::Deref for NativeVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T> std::ops::DerefMut for NativeVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: mutable access is only used for Rust-owned vectors.
        unsafe { std::slice::from_raw_parts_mut(self.items, self.count) }
    }
}
impl<'a, T> IntoIterator for &'a NativeVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut NativeVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
impl NativeVec<u32> {
    pub(crate) fn badge_view(&self) -> crate::newgrf_specs::BadgeSpan {
        crate::newgrf_specs::BadgeSpan {
            data: self.items,
            len: self.count,
        }
    }
}
impl<T> Default for NativeVec<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Drop for NativeVec<T> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            // SAFETY: nonzero capacity marks an allocation from `from_vec`.
            drop(unsafe { Vec::from_raw_parts(self.items, self.count, self.capacity) });
        }
    }
}
impl<T: Clone> Clone for NativeVec<T> {
    fn clone(&self) -> Self {
        Self::from_vec(self.as_slice().to_vec())
    }
    fn clone_from(&mut self, source: &Self) {
        self.edit(|vec| {
            vec.truncate(source.len());
            let existing = vec.len();
            vec.clone_from_slice(&source.as_slice()[..existing]);
            vec.extend_from_slice(&source.as_slice()[existing..]);
        });
    }
}

/// `IndustryTileLayoutTile`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LayoutTile {
    pub ti: crate::map_type::TileIndexDiffC,
    pub gfx: u16,
}
pub type Layout = NativeVec<LayoutTile>;

/// `SubstituteGRFFileProps`, viewed in place.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SubstituteProps {
    pub local_id: u16,
    pub grfid: u32,
    pub grffile: *const c_void,
    pub spritegroups: [*const c_void; 2],
    pub subst_id: u16,
    pub override_id: u16,
}
impl SubstituteProps {
    /// `SubstituteGRFFileProps(subst_id)`.
    pub const fn new(subst_id: u16) -> Self {
        Self {
            local_id: 0,
            grfid: 0,
            grffile: std::ptr::null(),
            spritegroups: [std::ptr::null(); 2],
            subst_id,
            override_id: subst_id,
        }
    }
    /// `GRFFilePropsBase::HasGrfFile`.
    pub fn has_grf_file(&self) -> bool {
        !self.grffile.is_null()
    }
    /// `StandardGRFFileProps::HasSpriteGroups`.
    pub fn has_sprite_groups(&self) -> bool {
        !self.spritegroups[0].is_null()
    }
    /// `StandardGRFFileProps::GetSpriteGroup(entity_exists)`.
    pub fn sprite_group(&self, entity_exists: bool) -> *const c_void {
        let purchase = if entity_exists {
            std::ptr::null()
        } else {
            self.spritegroups[1]
        };
        if purchase.is_null() {
            self.spritegroups[0]
        } else {
            purchase
        }
    }
}
/// `AnimationInfo<IndustryAnimationTriggers>`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AnimationInfo {
    pub frames: u8,
    pub status: u8,
    pub speed: u8,
    pub triggers: u8,
}

crate::abi::shared_records! {
    fn abi_record;
    /// `IndustrySpec`; `layouts` indices are those saved in `Industry::selected_layout`.
    #[derive(Clone)]
    "IndustrySpec" ["industry_spec"] => pub struct IndustrySpec {
        pub layouts: NativeVec<Layout> => "RustVector<RustVector<IndustryTileLayoutTile>>",
        pub cost_multiplier: u8,
        pub removal_cost_multiplier: u32,
        pub prospecting_chance: u32,
        pub conflicting: [u8; 3] => "IndustryType[3]",
        pub check_proc: u8,
        pub produced_cargo: [u8; INDUSTRY_NUM_OUTPUTS] => "std::array<CargoType, INDUSTRY_NUM_OUTPUTS>",
        pub production_rate: [u8; INDUSTRY_NUM_OUTPUTS],
        pub minimal_cargo: u8,
        pub accepts_cargo: [u8; INDUSTRY_NUM_INPUTS] => "std::array<CargoType, INDUSTRY_NUM_INPUTS>",
        pub input_cargo_multiplier: [[u16; INDUSTRY_NUM_OUTPUTS]; INDUSTRY_NUM_INPUTS],
        pub life_type: u8 => "IndustryLifeTypes",
        pub climate_availability: u8 => "LandscapeTypes",
        pub behaviour: u32 => "IndustryBehaviours",
        pub map_colour: u8 => "PixelColour",
        pub name: u32 => "StringID",
        pub new_industry_text: u32 => "StringID",
        pub closure_text: u32 => "StringID",
        pub production_up_text: u32 => "StringID",
        pub production_down_text: u32 => "StringID",
        pub station_name: u32 => "StringID",
        pub appear_ingame: [u8; NUM_LANDSCAPE],
        pub appear_creation: [u8; NUM_LANDSCAPE],
        pub callback_mask: u16 => "IndustryCallbackMasks",
        pub enabled: bool,
        pub grf_prop: SubstituteProps => "SubstituteGRFFileProps",
        pub random_sounds: NativeVec<u8> => "RustVector<uint8_t>",
        pub badges: NativeVec<u32> => "RustVector<BadgeID>",
        pub produced_cargo_label: [CargoLabel; INDUSTRY_ORIGINAL_NUM_OUTPUTS] => "OpenTTDNewGRFCargoLabel[INDUSTRY_ORIGINAL_NUM_OUTPUTS]",
        pub accepts_cargo_label: [CargoLabel; INDUSTRY_ORIGINAL_NUM_INPUTS] => "OpenTTDNewGRFCargoLabel[INDUSTRY_ORIGINAL_NUM_INPUTS]",
    }
    /// `IndustryTileSpec`.
    #[derive(Clone)]
    "IndustryTileSpec" ["industry_spec"] => pub struct IndustryTileSpec {
        pub accepts_cargo: [u8; INDUSTRY_NUM_INPUTS] => "std::array<CargoType, INDUSTRY_NUM_INPUTS>",
        pub acceptance: [i8; INDUSTRY_NUM_INPUTS] => "std::array<int8_t, INDUSTRY_NUM_INPUTS>",
        pub slopes_refused: u8 => "Slope",
        pub anim_production: u8,
        pub anim_next: u8,
        pub anim_state: bool,
        pub callback_mask: u8 => "IndustryTileCallbackMasks",
        pub animation: AnimationInfo => "AnimationInfo<IndustryAnimationTriggers>",
        pub special_flags: u8 => "IndustryTileSpecialFlags",
        pub enabled: bool,
        pub grf_prop: SubstituteProps => "SubstituteGRFFileProps",
        pub badges: NativeVec<u32> => "RustVector<BadgeID>",
        pub accepts_cargo_label: [CargoLabel; INDUSTRY_ORIGINAL_NUM_INPUTS] => "OpenTTDNewGRFCargoLabel[INDUSTRY_ORIGINAL_NUM_INPUTS]",
    }
}

/// The default label of a value-initialized `std::variant<CargoLabel, MixedCargoType>`.
const NO_LABEL: CargoLabel = CargoLabel {
    value: 0,
    mixed: false,
};
/// `IndustrySpec{}`.
pub const EMPTY_INDUSTRY: IndustrySpec = IndustrySpec {
    layouts: NativeVec::new(),
    cost_multiplier: 0,
    removal_cost_multiplier: 0,
    prospecting_chance: 0,
    conflicting: [0; 3],
    check_proc: 0,
    produced_cargo: [0; INDUSTRY_NUM_OUTPUTS],
    production_rate: [0; INDUSTRY_NUM_OUTPUTS],
    minimal_cargo: 0,
    accepts_cargo: [0; INDUSTRY_NUM_INPUTS],
    input_cargo_multiplier: [[0; INDUSTRY_NUM_OUTPUTS]; INDUSTRY_NUM_INPUTS],
    life_type: 0,
    climate_availability: 0,
    behaviour: 0,
    map_colour: 0,
    name: 0,
    new_industry_text: 0,
    closure_text: 0,
    production_up_text: 0,
    production_down_text: 0,
    station_name: 0,
    appear_ingame: [0; NUM_LANDSCAPE],
    appear_creation: [0; NUM_LANDSCAPE],
    callback_mask: 0,
    enabled: false,
    grf_prop: SubstituteProps::new(0),
    random_sounds: NativeVec::new(),
    badges: NativeVec::new(),
    produced_cargo_label: [NO_LABEL; INDUSTRY_ORIGINAL_NUM_OUTPUTS],
    accepts_cargo_label: [NO_LABEL; INDUSTRY_ORIGINAL_NUM_INPUTS],
};
/// `IndustryTileSpec{}`; `AnimationInfo` defaults its speed to 2.
pub const EMPTY_TILE: IndustryTileSpec = IndustryTileSpec {
    accepts_cargo: [0; INDUSTRY_NUM_INPUTS],
    acceptance: [0; INDUSTRY_NUM_INPUTS],
    slopes_refused: 0,
    anim_production: 0,
    anim_next: 0,
    anim_state: false,
    callback_mask: 0,
    animation: AnimationInfo {
        frames: 0,
        status: NO_ANIMATION,
        speed: 2,
        triggers: 0,
    },
    special_flags: 0,
    enabled: false,
    grf_prop: SubstituteProps::new(0),
    badges: NativeVec::new(),
    accepts_cargo_label: [NO_LABEL; INDUSTRY_ORIGINAL_NUM_INPUTS],
};

/// `_industry_specs`.
static SPECS: GameState<[IndustrySpec; NUM_INDUSTRYTYPES as usize]> =
    GameState::new([EMPTY_INDUSTRY; NUM_INDUSTRYTYPES as usize]);
/// `_industry_tile_specs`.
static TILES: GameState<[IndustryTileSpec; NUM_INDUSTRYTILES as usize]> =
    GameState::new([EMPTY_TILE; NUM_INDUSTRYTILES as usize]);

/// `GRFFile::industryspec` and `GRFFile::indtspec`, keyed by file. Boxes keep the
/// string-mapping addresses stable until publication.
#[derive(Default)]
pub(crate) struct FileSpecs {
    pub(crate) industries: Vec<Option<Box<IndustrySpec>>>,
    pub(crate) tiles: Vec<Option<Box<IndustryTileSpec>>>,
}
static FILES: GameState<BTreeMap<usize, FileSpecs>> = GameState::new(BTreeMap::new());

/// The native assertion policy used by the original spec accessors, installed at reset.
static SHARED: GameState<*const crate::services::Services> = GameState::new(std::ptr::null());

fn spec_index<T>(base: *mut T, id: u16, count: u16, assert: impl FnOnce(bool)) -> *mut T {
    if id >= count {
        assert(false);
    }
    // With native assertions disabled, the original accessor adds no bounds rejection.
    base.wrapping_add(usize::from(id))
}
fn spec_assert(ok: bool) {
    // SAFETY: production resets the specs with the process-lifetime shared table before use.
    crate::movement::native_assert(unsafe { SHARED.table() }, ok);
}
/// `GetIndustrySpec`, including its native assertion policy.
pub(crate) fn industry(id: u16) -> *mut IndustrySpec {
    spec_index(
        SPECS.get().cast(),
        id,
        u16::from(NUM_INDUSTRYTYPES),
        spec_assert,
    )
}
/// `GetIndustryTileSpec`, including its native assertion policy.
pub(crate) fn tile(gfx: u16) -> *mut IndustryTileSpec {
    spec_index(TILES.get().cast(), gfx, NUM_INDUSTRYTILES, spec_assert)
}
/// The per-file specs of `file`, created empty on first use.
///
/// # Safety
/// The returned borrow ends before any call that can reach this storage again.
pub(crate) unsafe fn file_specs<'a>(file: *const c_void) -> &'a mut FileSpecs {
    unsafe { (*FILES.get()).entry(file as usize).or_default() }
}
/// Drop the per-file specs of `file`, as clearing `industryspec` and `indtspec` does.
pub(crate) fn clear_file(file: *const c_void) {
    // SAFETY: serialized storage; no borrow is live.
    unsafe {
        (*FILES.get()).remove(&(file as usize));
    }
}

/// `ResetIndustries`: copy the original tables and clear the `NewGRF` slots. Every
/// `GRFFile` is destroyed with the same reset, so its specs go too.
///
/// # Safety
/// `origin` and `origin_tiles` address the native tables of `NEW_INDUSTRYOFFSET` and
/// `NEW_INDUSTRYTILEOFFSET` records.
pub(crate) unsafe fn reset(
    shared: *const crate::services::Services,
    climate: u8,
    origin: *const IndustrySpec,
    origin_tiles: *const IndustryTileSpec,
) {
    use crate::industry_type::{NEW_INDUSTRYOFFSET, NEW_INDUSTRYTILEOFFSET};
    SHARED.exchange(shared);
    let specs = unsafe { &mut *SPECS.get() };
    for (id, spec) in specs.iter_mut().enumerate() {
        *spec = if id < usize::from(NEW_INDUSTRYOFFSET) {
            unsafe { (*origin.add(id)).clone() }
        } else {
            EMPTY_INDUSTRY
        };
    }
    for spec in &mut specs[..usize::from(NEW_INDUSTRYOFFSET)] {
        spec.enabled = spec.climate_availability & (1 << climate) != 0;
    }
    let tiles = unsafe { &mut *TILES.get() };
    for (gfx, spec) in tiles.iter_mut().enumerate() {
        *spec = if gfx < usize::from(NEW_INDUSTRYTILEOFFSET) {
            unsafe { (*origin_tiles.add(gfx)).clone() }
        } else {
            EMPTY_TILE
        };
    }
    unsafe { (*FILES.get()).clear() };
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_specs() -> *mut IndustrySpec {
    industry(0)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_tile_specs() -> *mut IndustryTileSpec {
    tile(0)
}
/// `AddBadgeToSpecs` over the file's industry or industry-tile specs; whether any exists.
///
/// # Safety
/// Serialized by the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_file_badge_abi(
    file: *const c_void,
    tiles: bool,
    badge: u32,
) -> u8 {
    let Some(specs) = (unsafe { (*FILES.get()).get_mut(&(file as usize)) }) else {
        return 0;
    };
    let mut any = false;
    if tiles {
        for spec in specs.tiles.iter_mut().flatten() {
            spec.badges.push(badge);
            any = true;
        }
    } else {
        for spec in specs.industries.iter_mut().flatten() {
            spec.badges.push(badge);
            any = true;
        }
    }
    u8::from(any)
}
/// The animation facts of an industry tile spec, for the shared animation owner.
///
/// # Safety
/// `spec` is a live record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_animation_info(
    spec: *const IndustryTileSpec,
) -> crate::newgrf_animation_base::Info {
    let spec = unsafe { &*spec };
    crate::newgrf_animation_base::Info {
        speed: spec.animation.speed,
        frames: spec.animation.frames,
        // AnimationStatus::Looping.
        looping: spec.animation.status == 1,
        // IndustryTileCallbackMask::AnimationSpeed and AnimationNextFrame.
        speed_callback: spec.callback_mask & (1 << 1) != 0,
        next_callback: spec.callback_mask & 1 != 0,
        // IndustryTileSpecialFlag::NextFrameRandomBits.
        random: spec.special_flags & 1 != 0,
        enabled: true,
    }
}
/// `IndustrySpec::IsRawIndustry`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_spec_raw_abi(spec: *const IndustrySpec) -> u8 {
    u8::from(crate::industry::spec_raw(unsafe { (*spec).life_type }))
}
/// `IndustrySpec::IsProcessingIndustry`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_spec_processing_abi(
    spec: *const IndustrySpec,
) -> u8 {
    let (life, behaviour) = unsafe { ((*spec).life_type, (*spec).behaviour) };
    u8::from(crate::industry::spec_processing(life, behaviour))
}
/// `IndustrySpec::UsesOriginalEconomy` under the economy type setting.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_original_economy_abi(
    spec: *const IndustrySpec,
    economy: u8,
) -> u8 {
    u8::from(crate::industry::original_economy(
        unsafe { (*spec).callback_mask },
        economy,
    ))
}
/// `IndustrySpec::GetConstructionCost` under the raw-industry construction setting.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_construction_cost(
    spec: *const IndustrySpec,
    raw_setting: u8,
) -> i64 {
    let (multiplier, life) = unsafe { ((*spec).cost_multiplier, (*spec).life_type) };
    crate::industry::construction_cost(multiplier, life, raw_setting)
}

crate::abi::layouts! {
    fn abi_layout;
    "IndustryTileLayoutTile" ["industry_spec"] => LayoutTile { ti, gfx };
    "RustVector<uint8_t>" ["industry_spec"] => NativeVec<u8> { items, count, capacity };
    "SubstituteGRFFileProps" ["industry_spec"] => SubstituteProps { local_id, grfid, grffile, spritegroups, subst_id, override_id };
    "AnimationInfo<IndustryAnimationTriggers>" ["industry_spec"] => AnimationInfo { frames, status, speed, triggers };
}

#[cfg(test)]
mod tests {
    #[test]
    fn spec_indices_follow_native_assertion_policy() {
        let mut values = [0_u8; 2];
        let base = values.as_mut_ptr();
        let mut assertions = 0;
        assert_eq!(
            super::spec_index(base, 1, 2, |_| assertions += 1),
            base.wrapping_add(1)
        );
        assert_eq!(assertions, 0);
        // A returning native handler models WITH_ASSERT off; do not dereference the invalid pointer.
        assert_eq!(
            super::spec_index(base, 2, 2, |ok| {
                assert!(!ok);
                assertions += 1;
            }),
            base.wrapping_add(2)
        );
        assert_eq!(assertions, 1);
        assert_eq!(
            super::spec_index(base, u16::MAX, 2, |_| {}),
            base.wrapping_add(usize::from(u16::MAX))
        );
    }
}
