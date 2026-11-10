/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `industrytype.h`: the industry and industry-tile specifications, their canonical storage
//! and the spec methods. Native readers view the records in place through the generated
//! declarations. Every access holds the game-state mutex; no reference into the storage
//! survives a native callback.
#![allow(unsafe_code)]
use crate::game_state::GameState;
use crate::industry_type::{
    INDUSTRY_NUM_INPUTS, INDUSTRY_NUM_OUTPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    INDUSTRY_ORIGINAL_NUM_OUTPUTS, IndustryGfx, IndustryType,
};
use crate::newgrf::CargoLabel;
use crate::newgrf_animation_type::AnimationStatus;
use crate::newgrf_badge_type::BadgeID;
use crate::newgrf_callbacks::{IndustryTileCallbackMask, IndustryTileCallbackMasks};
use crate::newgrf_commons::StandardSpriteGroup;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ptr::NonNull;

/// `NUM_LANDSCAPE`.
pub const NUM_LANDSCAPE: usize = crate::landscape_type::NUM_LANDSCAPE as usize;

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
#[allow(
    clippy::cast_sign_loss,
    reason = "StandardGRFFileProps' static_cast<size_t>(StandardSpriteGroup::End)"
)]
pub struct SubstituteProps {
    pub local_id: u16,
    pub grfid: u32,
    pub grffile: *const c_void,
    pub spritegroups: [*const c_void; StandardSpriteGroup::End.0 as usize],
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
            spritegroups: [std::ptr::null(); StandardSpriteGroup::End.0 as usize],
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
        !self.spritegroups[StandardSpriteGroup::Default.0 as usize].is_null()
    }
    /// `StandardGRFFileProps::GetSpriteGroup(entity_exists)`.
    pub fn sprite_group(&self, entity_exists: bool) -> *const c_void {
        let purchase = if entity_exists {
            std::ptr::null()
        } else {
            self.spritegroups[StandardSpriteGroup::Purchase.0 as usize]
        };
        if purchase.is_null() {
            self.spritegroups[StandardSpriteGroup::Default.0 as usize]
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
        /// List of possible tile layouts for the industry
        pub layouts: NativeVec<Layout> => "RustVector<RustVector<IndustryTileLayoutTile>>",
        /// Base construction cost multiplier.
        pub cost_multiplier: u8,
        /// Base removal cost multiplier.
        pub removal_cost_multiplier: u32,
        /// Chance prospecting succeeds
        pub prospecting_chance: u32,
        /// Industries this industry cannot be close to
        pub conflicting: [u8; 3] => "IndustryType[3]",
        /// Index to a procedure to check for conflicting circumstances
        pub check_proc: u8,
        pub produced_cargo: [u8; INDUSTRY_NUM_OUTPUTS] => "std::array<CargoType, INDUSTRY_NUM_OUTPUTS>",
        pub production_rate: [u8; INDUSTRY_NUM_OUTPUTS],
        /// minimum amount of cargo transported to the stations.
        /// If the waiting cargo is less than this number, no cargo is moved to it.
        pub minimal_cargo: u8,
        /// 16 accepted cargoes.
        pub accepts_cargo: [u8; INDUSTRY_NUM_INPUTS] => "std::array<CargoType, INDUSTRY_NUM_INPUTS>",
        /// Input cargo multipliers (multiply amount of incoming cargo for the produced cargoes)
        pub input_cargo_multiplier: [[u16; INDUSTRY_NUM_OUTPUTS]; INDUSTRY_NUM_INPUTS],
        /// This is also known as Industry production flag, in newgrf specs
        pub life_type: u8 => "IndustryLifeTypes",
        /// Bitmask, giving landscape enums as bit position
        pub climate_availability: u8 => "LandscapeTypes",
        /// How this industry will behave, and how others entities can use it
        pub behaviour: u32 => "IndustryBehaviours",
        /// colour used for the small map
        pub map_colour: u8 => "PixelColour",
        /// Displayed name of the industry
        pub name: u32 => "StringID",
        /// Message appearing when the industry is built
        pub new_industry_text: u32 => "StringID",
        /// Message appearing when the industry closes
        pub closure_text: u32 => "StringID",
        /// Message appearing when the industry's production is increasing
        pub production_up_text: u32 => "StringID",
        /// Message appearing when the industry's production is decreasing
        pub production_down_text: u32 => "StringID",
        /// Default name for nearby station
        pub station_name: u32 => "StringID",
        /// Probability of appearance in game
        pub appear_ingame: [u8; NUM_LANDSCAPE],
        /// Probability of appearance during map creation
        pub appear_creation: [u8; NUM_LANDSCAPE],
        /// Bitmask of industry callbacks that have to be called
        pub callback_mask: u16 => "IndustryCallbackMasks",
        /// entity still available (by default true).newgrf can disable it, though
        pub enabled: bool,
        /// properties related to the grf file
        pub grf_prop: SubstituteProps => "SubstituteGRFFileProps",
        /// Random sounds;
        pub random_sounds: NativeVec<u8> => "RustVector<uint8_t>",
        pub badges: NativeVec<u32> => "RustVector<BadgeID>",
        /// Cargo labels of produced cargo for default industries.
        pub produced_cargo_label: [CargoLabel; INDUSTRY_ORIGINAL_NUM_OUTPUTS] => "OpenTTDNewGRFCargoLabel[INDUSTRY_ORIGINAL_NUM_OUTPUTS]",
        /// Cargo labels of accepted cargo for default industries.
        pub accepts_cargo_label: [CargoLabel; INDUSTRY_ORIGINAL_NUM_INPUTS] => "OpenTTDNewGRFCargoLabel[INDUSTRY_ORIGINAL_NUM_INPUTS]",
    }
    /// `IndustryTileSpec`: defines the data structure of each individual tile of an industry.
    /// A tile can at most accept 3 types of cargo, even if an industry as a whole can accept
    /// more types.
    #[derive(Clone)]
    "IndustryTileSpec" ["industry_spec"] => pub struct IndustryTileSpec {
        /// Cargo accepted by this tile
        pub accepts_cargo: [u8; INDUSTRY_NUM_INPUTS] => "std::array<CargoType, INDUSTRY_NUM_INPUTS>",
        /// Level of acceptance per cargo type (signed, may be negative!)
        pub acceptance: [i8; INDUSTRY_NUM_INPUTS] => "std::array<int8_t, INDUSTRY_NUM_INPUTS>",
        /// slope pattern on which this tile cannot be built
        pub slopes_refused: u8 => "Slope",
        /// Animation frame to start when goods are produced
        pub anim_production: u8,
        /// Next frame in an animation
        pub anim_next: u8,
        /// When true, the tile has to be drawn using the animation
        /// state instead of the construction stage
        pub anim_state: bool,
        /// Bitmask of industry tile callbacks that have to be called
        pub callback_mask: u8 => "IndustryTileCallbackMasks",
        /// Information about the animation (is it looping, how many loops etc)
        pub animation: AnimationInfo => "AnimationInfo<IndustryAnimationTriggers>",
        /// Bitmask of extra flags used by the tile
        pub special_flags: u8 => "IndustryTileSpecialFlags",
        /// entity still available (by default true).newgrf can disable it, though
        pub enabled: bool,
        /// properties related to the grf file
        pub grf_prop: SubstituteProps => "SubstituteGRFFileProps",
        pub badges: NativeVec<u32> => "RustVector<BadgeID>",
        /// Cargo labels of accepted cargo for default industry tiles.
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
        status: AnimationStatus::NoAnimation.0,
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
static SPECS: GameState<[IndustrySpec; IndustryType::NUM_INDUSTRYTYPES.0 as usize]> =
    GameState::new([EMPTY_INDUSTRY; IndustryType::NUM_INDUSTRYTYPES.0 as usize]);
/// `_industry_tile_specs`.
static TILES: GameState<[IndustryTileSpec; IndustryGfx::NUM_INDUSTRYTILES.0 as usize]> =
    GameState::new([EMPTY_TILE; IndustryGfx::NUM_INDUSTRYTILES.0 as usize]);

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
/// `GetIndustrySpec`: accessor for array `_industry_specs`, including its native assertion
/// policy.
pub(crate) fn get_industry_spec(thistype: IndustryType) -> *mut IndustrySpec {
    spec_index(
        SPECS.get().cast(),
        u16::from(thistype.0),
        u16::from(IndustryType::NUM_INDUSTRYTYPES.0),
        spec_assert,
    )
}
/// `GetIndustryTileSpec`: accessor for array `_industry_tile_specs`, including its native
/// assertion policy.
pub(crate) fn get_industry_tile_spec(gfx: IndustryGfx) -> *mut IndustryTileSpec {
    spec_index(
        TILES.get().cast(),
        gfx.0,
        IndustryGfx::NUM_INDUSTRYTILES.0,
        spec_assert,
    )
}
/// `IndustrySpec::behaviour`, as its `IndustryBehaviours` wrapper.
pub(crate) fn spec_behaviour(spec: *const IndustrySpec) -> IndustryBehaviours {
    // SAFETY: Callers pass a spec from `GetIndustrySpec`, live for the process.
    IndustryBehaviours(unsafe { (*spec).behaviour })
}
/// `IndustrySpec::callback_mask`, as its `IndustryCallbackMasks` wrapper.
pub(crate) fn spec_callback_mask(
    spec: *const IndustrySpec,
) -> crate::newgrf_callbacks::IndustryCallbackMasks {
    // SAFETY: Callers pass a spec from `GetIndustrySpec`, live for the process.
    crate::newgrf_callbacks::IndustryCallbackMasks(unsafe { (*spec).callback_mask })
}
/// Temporary raw adapter for [`get_industry_spec`]; every caller widens an `IndustryType`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "GetIndustrySpec takes IndustryType (uint8_t); every raw caller widens one"
)]
pub(crate) fn industry(id: u16) -> *mut IndustrySpec {
    get_industry_spec(IndustryType(id as u8))
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
    climate: crate::landscape_type::LandscapeType,
    origin: *const IndustrySpec,
    origin_tiles: *const IndustryTileSpec,
) {
    SHARED.exchange(shared);
    let specs = unsafe { &mut *SPECS.get() };
    for (id, spec) in specs.iter_mut().enumerate() {
        *spec = if id < usize::from(IndustryType::NEW_INDUSTRYOFFSET.0) {
            unsafe { (*origin.add(id)).clone() }
        } else {
            EMPTY_INDUSTRY
        };
    }
    // Enable only the current climate industries
    for spec in &mut specs[..usize::from(IndustryType::NEW_INDUSTRYOFFSET.0)] {
        spec.enabled =
            crate::landscape_type::LandscapeTypes(spec.climate_availability).test(climate);
    }
    let tiles = unsafe { &mut *TILES.get() };
    for (gfx, spec) in tiles.iter_mut().enumerate() {
        *spec = if gfx < usize::from(IndustryGfx::NEW_INDUSTRYTILEOFFSET.0) {
            unsafe { (*origin_tiles.add(gfx)).clone() }
        } else {
            EMPTY_TILE
        };
    }
    // Every `GRFFile`, and with it its override managers' file specs, is reset here.
    unsafe { (*FILES.get()).clear() };
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_specs() -> *mut IndustrySpec {
    get_industry_spec(IndustryType(0))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_tile_specs() -> *mut IndustryTileSpec {
    get_industry_tile_spec(IndustryGfx(0))
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
    u8::from(unsafe { add_badge_to_specs(file, tiles, BadgeID(badge)) })
}
/// `AddBadgeToSpecs` over the file's industry or industry-tile specs; whether any exists.
///
/// # Safety
/// Serialized by the game-state mutex.
unsafe fn add_badge_to_specs(file: *const c_void, tiles: bool, badge: BadgeID) -> bool {
    let Some(specs) = (unsafe { (*FILES.get()).get_mut(&(file as usize)) }) else {
        return false;
    };
    let mut any = false;
    if tiles {
        for spec in specs.tiles.iter_mut().flatten() {
            spec.badges.push(badge.0);
            any = true;
        }
    } else {
        for spec in specs.industries.iter_mut().flatten() {
            spec.badges.push(badge.0);
            any = true;
        }
    }
    any
}
/// The animation facts of an industry tile spec, for the shared animation owner.
///
/// # Safety
/// `spec` is a live record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_animation_info(
    spec: *const IndustryTileSpec,
) -> crate::newgrf_animation_base::Info {
    tile_animation_info(unsafe { &*spec })
}
/// The animation facts of an industry tile spec, for the shared animation owner.
fn tile_animation_info(spec: &IndustryTileSpec) -> crate::newgrf_animation_base::Info {
    let callback_mask = IndustryTileCallbackMasks(spec.callback_mask);
    crate::newgrf_animation_base::Info {
        speed: spec.animation.speed,
        frames: spec.animation.frames,
        looping: AnimationStatus(spec.animation.status) == AnimationStatus::Looping,
        speed_callback: callback_mask.test(IndustryTileCallbackMask::AnimationSpeed),
        next_callback: callback_mask.test(IndustryTileCallbackMask::AnimationNextFrame),
        random: IndustryTileSpecialFlags(spec.special_flags)
            .test(IndustryTileSpecialFlag::NextFrameRandomBits),
        enabled: true,
    }
}
/// `IndustrySpec::IsRawIndustry`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_spec_raw_abi(spec: *const IndustrySpec) -> u8 {
    u8::from(crate::industry::is_raw_industry(IndustryLifeTypes(
        unsafe { (*spec).life_type },
    )))
}
/// `IndustrySpec::IsProcessingIndustry`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_spec_processing_abi(
    spec: *const IndustrySpec,
) -> u8 {
    let (life, behaviour) = unsafe { ((*spec).life_type, (*spec).behaviour) };
    u8::from(crate::industry::is_processing_industry(
        IndustryLifeTypes(life),
        IndustryBehaviours(behaviour),
    ))
}
/// `IndustrySpec::UsesOriginalEconomy` under the economy type setting.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_original_economy_abi(
    spec: *const IndustrySpec,
    economy: u8,
) -> u8 {
    u8::from(crate::industry::uses_original_economy(
        crate::newgrf_callbacks::IndustryCallbackMasks(unsafe { (*spec).callback_mask }),
        crate::economy_type::EconomyType(economy),
    ))
}
/// `IndustrySpec::GetConstructionCost` under the raw-industry construction setting.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_construction_cost(
    spec: *const IndustrySpec,
    raw_setting: u8,
) -> i64 {
    let (cost_multiplier, life_type) = unsafe { ((*spec).cost_multiplier, (*spec).life_type) };
    crate::industry::get_construction_cost(
        cost_multiplier,
        IndustryLifeTypes(life_type),
        raw_setting,
    )
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

/// Native `industrytype.h::IndustryLifeType`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryLifeType(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryLifeType {
    /// Like mines
    pub const Extractive: Self = Self(0);
    /// Like forests
    pub const Organic: Self = Self(1);
    /// Like factories
    pub const Processing: Self = Self(2);
}

/// Native `IndustryLifeTypes`; elements are `IndustryLifeType` bit positions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryLifeTypes(pub u8);
impl IndustryLifeTypes {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native bit set containing one element.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: IndustryLifeType) -> Self {
        Self((1_u64 << element.0) as u8)
    }
    /// Native `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: IndustryLifeType) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native `BaseBitSet::Any`: whether any of the given values are set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Native `industrytype.h::CheckProc`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckProc(pub u8);
impl CheckProc {
    /// Always succeeds.
    pub const CHECK_NOTHING: Self = Self(0);
    /// %Industry should be build above snow-line in arctic climate.
    pub const CHECK_FOREST: Self = Self(1);
    /// %Industry should be positioned near edge of the map.
    pub const CHECK_REFINERY: Self = Self(2);
    /// %Industry should be below snow-line in arctic.
    pub const CHECK_FARM: Self = Self(3);
    /// %Industry should NOT be in the desert.
    pub const CHECK_PLANTATION: Self = Self(4);
    /// %Industry should be in the desert.
    pub const CHECK_WATER: Self = Self(5);
    /// %Industry should be in the rainforest.
    pub const CHECK_LUMBERMILL: Self = Self(6);
    /// %Industry should be in low land.
    pub const CHECK_BUBBLEGEN: Self = Self(7);
    /// Industries at sea should be positioned near edge of the map.
    pub const CHECK_OIL_RIG: Self = Self(8);
    /// End marker of the industry check procedures.
    pub const CHECK_END: Self = Self(9);
}

/// Native `industrytype.h::IndustryConstructionType`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryConstructionType(pub u8);
impl IndustryConstructionType {
    /// in previous game version or without newindustries activated
    pub const ICT_UNKNOWN: Self = Self(0);
    /// either by user or random creation process
    pub const ICT_NORMAL_GAMEPLAY: Self = Self(1);
    /// during random map creation
    pub const ICT_MAP_GENERATION: Self = Self(2);
    /// while editing a scenario
    pub const ICT_SCENARIO_EDITOR: Self = Self(3);
}

/// Native `industrytype.h::IndustryBehaviour`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryBehaviour(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryBehaviour {
    /// periodically plants fields around itself (temp and arctic farms)
    pub const PlantFields: Self = Self(0);
    /// cuts trees and produce first output cargo from them (lumber mill)
    pub const CutTrees: Self = Self(1);
    /// is built on water (oil rig)
    pub const BuiltOnWater: Self = Self(2);
    /// can only be built in towns larger than 1200 inhabitants (temperate bank)
    pub const Town1200More: Self = Self(3);
    /// can only be built in towns (arctic/tropic banks, water tower)
    pub const OnlyInTown: Self = Self(4);
    /// is always built near towns (toy shop)
    pub const OnlyNearTown: Self = Self(5);
    /// Fields are planted around when built (all farms)
    pub const PlantOnBuild: Self = Self(6);
    /// do not increase production (oil wells) in the temperate climate
    pub const DontIncrProd: Self = Self(7);
    /// can only be built before 1950 (oil wells)
    pub const Before1950: Self = Self(8);
    /// can only be built after 1960 (oil rigs)
    pub const After1960: Self = Self(9);
    /// ai will attempt to establish air/ship routes to this industry (oil rig)
    pub const AIAirShipRoutes: Self = Self(10);
    /// can be exploded by a military airplane (oil refinery)
    pub const AirplaneAttacks: Self = Self(11);
    /// can be exploded by a military helicopter (factory)
    pub const ChopperAttacks: Self = Self(12);
    /// can cause a subsidence (coal mine, shaft that collapses)
    pub const CanSubsidence: Self = Self(13);
    /// The following behaviours are only for newindustries, not normal TTD behaviour.
    /// Automatic production multiplier handling
    pub const ProdMultiHandling: Self = Self(14);
    /// Production callback needs random bits in var 10
    pub const ProdCallbackRandom: Self = Self(15);
    /// Do not force one instance of this type to appear on map generation
    pub const NoBuildMapCreation: Self = Self(16);
    /// Allow closing down the last instance of this type
    pub const CanCloseLastInstance: Self = Self(17);
    /// Allow produced/accepted cargoes callbacks to supply more than 2 and 3 types
    pub const CargoTypesUnlimited: Self = Self(18);
    /// Do not clamp production of passengers. (smooth economy only)
    pub const NoPaxProdClamp: Self = Self(19);
}

/// Native `IndustryBehaviours`; elements are `IndustryBehaviour` bit positions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryBehaviours(pub u32);
impl IndustryBehaviours {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native bit set containing one element.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: IndustryBehaviour) -> Self {
        Self((1_u64 << element.0) as u32)
    }
    /// Native `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: IndustryBehaviour) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native `BaseBitSet::Any`: whether any of the given values are set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Native `industrytype.h::IndustryTileSpecialFlag`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryTileSpecialFlag(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryTileSpecialFlag {
    /// Callback 0x26 needs random bits
    pub const NextFrameRandomBits: Self = Self(0);
    /// Tile always accepts all cargoes the associated industry accepts
    pub const AcceptsAllCargo: Self = Self(1);
}

/// Native `IndustryTileSpecialFlags`; elements are `IndustryTileSpecialFlag` bit positions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryTileSpecialFlags(pub u8);
impl IndustryTileSpecialFlags {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: IndustryTileSpecialFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
}

/// Industries like power plants and banks.
pub const INDUSTRYLIFE_BLACK_HOLE: IndustryLifeTypes = IndustryLifeTypes::EMPTY;

/// Raw compatibility mask for `IndustryBehaviour::AirplaneAttacks`.
pub const AIRPLANE_ATTACKS: u32 =
    IndustryBehaviours::from_element(IndustryBehaviour::AirplaneAttacks).0;

/// Raw compatibility mask for `IndustryBehaviour::ChopperAttacks`.
pub const CHOPPER_ATTACKS: u32 =
    IndustryBehaviours::from_element(IndustryBehaviour::ChopperAttacks).0;

/// Raw compatibility mask for `IndustryBehaviour::CanSubsidence`.
pub const CAN_SUBSIDENCE: u32 =
    IndustryBehaviours::from_element(IndustryBehaviour::CanSubsidence).0;
