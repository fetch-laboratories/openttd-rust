/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Migrated game and text kernels exposed through the documented `src/rust` ABIs. Entries are
//! synchronous calls with typed `noexcept` C++ services; both profiles abort on panic, so nothing
//! unwinds across the boundary. Game-state entries hold the native mutex; worker jobs use private
//! snapshots. No borrow of owner or world state survives a service that can reenter Rust.

mod admin_conversion;
mod aircraft;
mod airport;
mod alternating;
mod animated_tile;
mod animated_tile_map;
mod auth;
mod base_consist;
mod base_station_base;
mod binaryheap;
mod bitmath_func;
mod bridge;
mod bridge_map;
mod byte_strings;
mod cargo_flow;
mod cargo_payment;
mod cargomonitor;
mod cargopacket;
mod command;
mod command_type;
mod company;
mod company_type;
mod consumer;
mod direction_func;
mod direction_type;
mod disaster;
mod economy_func;
mod economy_type;
mod engine_type;
mod follow_track;
mod game_state;
mod genland;
mod genworld;
mod geometry_type;
mod ground;
mod ground_vehicle;
mod heightmap;
mod history;
mod history_type;
mod house;
mod industry_type;
mod integer;
mod landscape;
mod landscape_generation;
mod landscape_type;
mod linkgraph;
mod linkgraph_schedule;
mod linkgraph_type;
mod map_func;
mod map_type;
mod math;
mod movement;
mod network_config;
mod newgrf;
mod newgrf_animation_base;
pub mod newgrf_bytereader;
mod newgrf_callbacks;
mod newgrf_commons;
mod newgrf_config;
mod newgrf_engine;
mod newgrf_generic;
mod newgrf_loading;
mod newgrf_properties;
mod newgrf_sound;
mod newgrf_spritegroup;
mod settings_type;

mod newgrf_station;
mod newgrf_storage;
mod node_hash;
mod object_type;
mod objects;
mod order_type;
mod overflowsafe_type;
mod packet;
mod pathfinder_func;
mod pbs;
mod rail_cmd;
pub mod rail_yapf;
mod random_func;
mod raw_field;
mod road;
mod road_cmd;
mod road_cost;
mod road_data;
mod road_func;
mod road_map;
mod road_stop;
mod road_type;
mod road_yapf;
mod roadveh;
mod script_list;
pub mod services;
mod ship_control;
mod signal;
mod signal_func;
mod signal_type;
mod slope_func;
mod sound_type;
mod sprites;
mod station;
mod station_base;
mod station_cargo;
mod station_cmd;
mod station_map;
mod station_service;
mod station_type;
mod string_validation;
mod strings_type;
mod tgp;
mod tile_cmd;
mod tile_loop;
mod tile_owner;
mod tile_type;
mod town;
mod town_map;
mod town_type;
mod townname;
mod townname_data;
mod track_func;
mod track_type;
mod transport_type;
mod tree_cmd;
mod trees;
mod tunnelbridge;
mod tunnelbridge_cmd;
mod vehicle;
mod vehicle_base;
mod vehicle_transactions;
mod vehicle_type;
mod vehiclelist;
mod viewport_func;
mod water;
mod water_cmd;
mod water_type;
mod widget_parser;
mod witness;
mod yapf_node;

pub use admin_conversion::Action as AdminAction;
pub use alternating::{AlternatingState, AlternatingStep};
pub use consumer::{
    BoundResult as ConsumerBound, ByteResult as ConsumerByte, MatchResult as ConsumerMatch,
    SeparatorResult as ConsumerSeparator,
};
pub use integer::IntegerResult;
pub use widget_parser::Action as WidgetAction;

/// Height at a coordinate within a tile, preserving `OpenTTD`'s slope rounding.
///
/// `u64::MAX` denotes only an unsupported base slope after half-tile handling, so the C++
/// adapter can reach its original `NOT_REACHED`; `u32::MAX` is a valid height. Unsigned
/// arithmetic wraps and signed results narrow as in C++; signed overflow has no equivalence guarantee.
#[allow(unsafe_code)] // Only the exported symbol attribute; the function is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_get_partial_pixel_z(x: i32, y: i32, corners: u8) -> u64 {
    landscape::partial_pixel_z(x, y, corners).map_or(u64::MAX, u64::from)
}

/// Parse borrowed arbitrary bytes using the original integer grammar.
///
/// # Safety
/// For nonzero length, `src` points to `length` readable bytes for the call; zero length
/// permits null. The span follows `ffi.h`: one live immutable allocation <= `isize::MAX`.
/// Base is 0/8/10/16 and width 1..=64; signed/clamp are 0/1.
#[allow(unsafe_code)] // Export and the one borrowed-slice construction only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_parse_integer(
    src: *const u8,
    length: usize,
    base: u8,
    width: u8,
    signed: u8,
    clamp: u8,
) -> IntegerResult {
    let bytes = if length == 0 {
        &[]
    } else {
        // SAFETY: The caller guarantees the readable allocation for this call.
        unsafe { std::slice::from_raw_parts(src, length) }
    };
    integer::parse(bytes, base, width, signed != 0, clamp != 0)
}

/// Return lexical integer skip length, independently of parse success/length.
///
/// # Safety
/// The pointer/length borrow has the same requirements as
/// `openttd_rust_parse_integer`; base must be 0/8/10/16. No pointer is retained.
/// Empty input permits null. This C ABI never unwinds.
#[allow(unsafe_code)] // Export and the one borrowed-slice construction only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_skip_integer(
    src: *const u8,
    length: usize,
    base: u8,
) -> usize {
    let bytes = if length == 0 {
        &[]
    } else {
        // SAFETY: The caller guarantees the readable allocation for this call.
        unsafe { std::slice::from_raw_parts(src, length) }
    };
    integer::skip(bytes, base)
}

/// Initialize traversal position and side from the original live boundary fact.
/// All arguments are scalar, no pointer ownership or borrowing crosses the ABI.
/// End position is zero for begin, otherwise the nonnegative range distance.
/// Both side flags are 0/1; no C++ iterator is retained and panic aborts.
#[allow(unsafe_code)] // Exported symbol attribute only; traversal is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_alternating_initialize(
    end_position: usize,
    before_at_first: u8,
) -> AlternatingState {
    alternating::initialize(end_position, before_at_first != 0)
}

/// Choose a logical advance and typed movement using the current range size.
/// Position must be less than `live_size`; state must originate from initialize
/// and completed advances. No range length is cached and panic aborts.
#[allow(unsafe_code)] // Exported symbol attribute only; traversal is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_alternating_advance(
    state: AlternatingState,
    live_size: usize,
) -> AlternatingStep {
    alternating::advance(state, live_size)
}

/// Complete a nonterminal advance using its requested live boundary fact.
/// After ++after, boundary is before == first; after --before, boundary is
/// next(after) != last. It must be 0/1. C++ retains all typed iterators; no
/// pointer or allocation crosses the non-unwinding ABI and panic aborts.
#[allow(unsafe_code)] // Exported symbol attribute only; traversal is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_alternating_complete(
    state: AlternatingState,
    boundary: u8,
) -> AlternatingState {
    alternating::complete(state, boundary != 0)
}

/// Compare positions in states whose range/middle identity C++ has checked.
/// Other state fields deliberately do not participate. No pointers or ownership
/// cross this non-unwinding ABI; panic aborts.
#[allow(unsafe_code)] // Exported symbol attribute only; traversal is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_alternating_compare(
    left: AlternatingState,
    right: AlternatingState,
) -> i8 {
    alternating::compare(left, right)
}

mod utf8;
pub use utf8::{Decoded as Utf8Decoded, Encoded as Utf8Encoded};

/// Encode a 32-bit codepoint by value without allocation or ownership transfer.
/// Surrogates are accepted; invalid values return zero length and zero bytes.
#[allow(unsafe_code)] // Export attribute only.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_encode_utf8(codepoint: u32) -> Utf8Encoded {
    utf8::encode(codepoint)
}

/// Classify an original byte, returning one for continuation bytes and zero otherwise.
#[allow(unsafe_code)] // Export attribute only.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_is_utf8_part(byte: u8) -> u8 {
    u8::from(utf8::is_part(byte))
}

/// Decode the first sequence, without examining trailing bytes or rejecting surrogates.
///
/// # Safety
/// `data` must address `length` readable initialized bytes in one allocation, valid
/// and unmodified for this call, with length at most `isize::MAX`. Zero length permits
/// null. No pointer is retained; read-only overlapping spans are allowed. Panics abort.
#[allow(unsafe_code)] // Export attribute and the single borrowed-slice construction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_decode_utf8(data: *const u8, length: usize) -> Utf8Decoded {
    // SAFETY: The caller supplies the immutable span described above.
    utf8::decode(unsafe { utf8::borrow(data, length) })
}

/// Advance one byte and skip its following continuation run, without decoding.
///
/// # Safety
/// The same input-span requirements as `openttd_rust_decode_utf8` apply, and
/// `position < length`. The original C++ assertion remains in the facade.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_utf8_next(
    data: *const u8,
    length: usize,
    position: usize,
) -> usize {
    // SAFETY: The caller supplies the immutable span described above.
    utf8::next(unsafe { utf8::borrow(data, length) }, position)
}

/// Retreat to the preceding non-continuation byte or offset zero, without decoding.
///
/// # Safety
/// The same input-span requirements as `openttd_rust_decode_utf8` apply, and
/// `0 < position <= length`. The original C++ assertion remains in the facade.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_utf8_previous(
    data: *const u8,
    length: usize,
    position: usize,
) -> usize {
    // SAFETY: The caller supplies the immutable span described above.
    utf8::previous(unsafe { utf8::borrow(data, length) }, position)
}

/// Normalize a byte offset by backward scanning; offsets at or beyond length yield end.
///
/// # Safety
/// The same input-span requirements as `openttd_rust_decode_utf8` apply.
/// The facade preserves its assertion of `offset <= length` before this call.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_utf8_at_byte(
    data: *const u8,
    length: usize,
    offset: usize,
) -> usize {
    // SAFETY: The caller supplies the immutable span described above.
    utf8::at_byte(unsafe { utf8::borrow(data, length) }, offset)
}

mod builder;
pub use builder::{FormattedInteger, LittleEndian};

/// Encode a scalar as eight little-endian bytes; the C++ facade selects its width.
/// Returns owned by-value storage without pointers, allocation, or ownership transfer.
#[allow(unsafe_code)] // Export attribute only; the implementation is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_encode_uint_le(value: u64) -> LittleEndian {
    builder::little_endian(value)
}

/// Format integer bits into the original fixed 32-byte scratch capacity.
/// Base must be 2..36; negative is one only for a signed negative input converted
/// directly to `uint64_t` modulo 2^64. Zero length suppresses the C++ sink call.
///
/// # Panics
/// Invalid bases violate the original `std::to_chars` precondition and abort.
#[allow(unsafe_code)] // Export attribute only; the implementation is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_format_integer(
    bits: u64,
    negative: u8,
    base: i32,
) -> FormattedInteger {
    builder::integer(bits, negative != 0, base)
}

/// Bound requested bytes and decide shortfall before C++ logs/commits the cursor.
/// Position must not exceed size. `usize::MAX` requests all remaining bytes.
/// No pointer, allocation, callback, or ownership crosses this non-unwinding ABI.
#[allow(unsafe_code)] // Exported symbol attribute only.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_consumer_bound(
    size: usize,
    position: usize,
    requested: usize,
) -> ConsumerBound {
    consumer::bound(size, position, requested)
}

/// Read unsigned little-endian bits without signed conversion or cursor changes.
/// # Safety
/// Data addresses one allocation of readable immutable bytes for the call only,
/// length <= `isize::MAX`; empty spans permit null. Width is 1/2/4/8 bytes.
/// No pointer is retained, no callback occurs while borrowing, and panic aborts.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consumer_little_endian(
    data: *const u8,
    length: usize,
    width: u8,
) -> ConsumerByte {
    // SAFETY: The caller guarantees the call-local read-only span above.
    consumer::little_endian(unsafe { utf8::borrow(data, length) }, width)
}

/// Match a prefix, including an empty prefix, and return conditional consumption.
/// # Safety
/// Both spans have the same borrow contract as `openttd_rust_consumer_little_endian`;
/// they may overlap read-only. Empty patterns are valid. No pointer is retained.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consumer_prefix(
    data: *const u8,
    length: usize,
    pattern: *const u8,
    pattern_length: usize,
) -> ConsumerMatch {
    // SAFETY: Both spans are call-local and immutable under the caller contract.
    consumer::prefix(unsafe { utf8::borrow(data, length) }, unsafe {
        utf8::borrow(pattern, pattern_length)
    })
}

/// Find a substring, member byte, or nonmember byte; not-found is `usize::MAX`.
/// # Safety
/// Both spans have the prefix borrow contract, including read-only overlap.
/// Pattern must be nonempty; mode is 0 substring, 1 membership, 2 nonmembership.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consumer_find(
    data: *const u8,
    length: usize,
    pattern: *const u8,
    pattern_length: usize,
    mode: u8,
) -> usize {
    // SAFETY: Both spans are call-local and immutable under the caller contract.
    consumer::find(
        unsafe { utf8::borrow(data, length) },
        unsafe { utf8::borrow(pattern, pattern_length) },
        mode,
    )
}

/// Test byte membership and return its unsigned bits only when it matches.
/// # Safety
/// Both spans have the prefix borrow contract, including read-only overlap.
/// Pattern must be nonempty; member is 0/1. No pointer or allocation is retained.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consumer_character(
    data: *const u8,
    length: usize,
    pattern: *const u8,
    pattern_length: usize,
    member: u8,
) -> ConsumerByte {
    // SAFETY: Both spans are call-local and immutable under the caller contract.
    consumer::character(
        unsafe { utf8::borrow(data, length) },
        unsafe { utf8::borrow(pattern, pattern_length) },
        member != 0,
    )
}

/// Decide returned and consumed lengths for whole, nonoverlapping separators.
/// # Safety
/// Both spans have the prefix borrow contract, including read-only overlap.
/// Separator must be nonempty. Policies 0..4 follow C++ `SeparatorUsage`; all other
/// values retain KEEP behavior. No pointer is retained and panic aborts.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consumer_separator(
    data: *const u8,
    length: usize,
    pattern: *const u8,
    pattern_length: usize,
    policy: i32,
) -> ConsumerSeparator {
    // SAFETY: Both spans are call-local and immutable under the caller contract.
    consumer::separator(
        unsafe { utf8::borrow(data, length) },
        unsafe { utf8::borrow(pattern, pattern_length) },
        policy,
    )
}

mod spiral;
pub use spiral::SpiralState;

/// Initialize a square spiral with the live map dimensions; diameter is positive.
/// All scalar arithmetic wraps at 32 bits. No pointer/ownership crosses the ABI.
#[allow(unsafe_code)] // Export attribute only; implementation is safe Rust.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_spiral_square(
    x: u32,
    y: u32,
    diameter: u32,
    size_x: u32,
    size_y: u32,
) -> SpiralState {
    spiral::square(x, y, diameter, size_x, size_y)
}

/// Initialize a rectangular-hole spiral; radius is positive, extents are unsigned.
/// Live map dimensions are supplied by the facade; no map/storage borrow occurs.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_spiral_hole(
    x: u32,
    y: u32,
    radius: u32,
    width: u32,
    height: u32,
    size_x: u32,
    size_y: u32,
) -> SpiralState {
    spiral::hole(x, y, radius, width, height, size_x, size_y)
}

/// Advance a non-ended initialized state and skip coordinates outside the live map.
/// State/results are independent copies; panic aborts without unwinding into C++.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_spiral_advance(
    state: SpiralState,
    size_x: u32,
    size_y: u32,
) -> SpiralState {
    state.advance(size_x, size_y)
}

/// End means current radius equals its limit with a non-invalid direction.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_spiral_end(state: SpiralState) -> u8 {
    u8::from(state.is_end())
}

/// Iterator equality compares coordinates only, independent of other state.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_spiral_equal(left: SpiralState, right: SpiralState) -> u8 {
    u8::from(spiral::equal(left, right))
}

mod effect;
mod effectvehicle_func;
mod encoded;
mod engine;
mod engine_loading;
mod livery;

mod engine_capabilities;
mod vehicle_effect;
pub use encoded::{
    Descriptor as EncodedDescriptor, Diagnostic as EncodedDiagnostic, View as EncodedView,
};

/// Borrow a tagged string descriptor only for the enclosing FFI operation.
///
/// # Safety
/// Tag is 0/1/2. A tag-2 nonempty byte span meets `utf8::borrow`'s contract.
/// The returned borrow never escapes the enclosing operation.
#[allow(unsafe_code)]
unsafe fn encoded_parameter<'a>(descriptor: &EncodedDescriptor) -> encoded::Parameter<'a> {
    match descriptor.kind {
        0 => encoded::Parameter::Empty,
        1 => encoded::Parameter::Integer(descriptor.value),
        2 => {
            encoded::Parameter::String(unsafe { utf8::borrow(descriptor.bytes, descriptor.length) })
        }
        _ => unreachable!("unknown encoded descriptor tag"),
    }
}

/// Encode a string ID and tagged parameters into an independently owned Rust result.
///
/// # Panics
/// Aborts if the descriptor count exceeds the documented byte-extent limit.
///
/// # Safety
/// Nonzero count addresses count initialized descriptors, and tag-2 spans stay live for the
/// call; zero count permits null. `string_assertions` is the original `WITH_ASSERT` guard
/// (0/1). The caller destroys the result once with `encoded_destroy` (see `encoded_ffi.h`).
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_encoded_serialize(
    id: u32,
    descriptors: *const EncodedDescriptor,
    count: usize,
    string_assertions: u8,
) -> *mut std::ffi::c_void {
    assert!(
        count <= usize::try_from(isize::MAX).unwrap() / std::mem::size_of::<EncodedDescriptor>()
    );
    let parameters: Vec<_> = (0..count)
        .map(|index| {
            // SAFETY: Initialized descriptor fields and the total extent are valid.
            // MSVC i686 can under-align stack arrays; never form a C++-storage
            // reference/slice. Copy unaligned fields into a Rust-owned local first.
            let descriptor = unsafe { descriptors.add(index).read_unaligned() };
            // SAFETY: Nested byte spans share the enclosing operation's lifetime.
            unsafe { encoded_parameter(&descriptor) }
        })
        .collect();
    Box::into_raw(Box::new(encoded::serialize(
        id,
        &parameters,
        string_assertions != 0,
    )))
    .cast()
}

/// Convert permissive legacy quote/colon encoding without validating numeric text.
///
/// # Safety
/// Input is immutable readable bytes in one live allocation, length <= `isize::MAX`;
/// empty permits null. `fix_code` is 0/1. No pointer is retained. The returned owner
/// has the same destroy/allocator/panic contract as `encoded_serialize`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_encoded_legacy(
    data: *const u8,
    length: usize,
    fix_code: u8,
) -> *mut std::ffi::c_void {
    // SAFETY: Input is readable and remains immutable for this call.
    let bytes = unsafe { utf8::borrow(data, length) };
    Box::into_raw(Box::new(encoded::legacy(bytes, fix_code != 0))).cast()
}

/// Canonicalize numeric records, trying unsigned hex before signed/default-zero.
///
/// # Safety
/// Same input/destroy contract as `encoded_legacy`. Diagnostics contain offsets only
/// in this complete input, not pointers; keep it alive until C++ replay finishes.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_encoded_negatives(
    data: *const u8,
    length: usize,
) -> *mut std::ffi::c_void {
    // SAFETY: Input is readable and remains immutable for this call.
    let bytes = unsafe { utf8::borrow(data, length) };
    Box::into_raw(Box::new(encoded::negatives(bytes))).cast()
}

/// Parse, replace, and serialize an internal encoded string in a single operation.
///
/// # Safety
/// Input and replacement spans follow `encoded_ffi.h` and may overlap. Numeric assertions
/// are active when !`NDEBUG` or `WITH_ASSERT`, string assertions under `WITH_ASSERT`; each
/// flag is 0/1. The result has `encoded_serialize`'s destroy contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_encoded_replace(
    data: *const u8,
    length: usize,
    index: usize,
    replacement: EncodedDescriptor,
    numeric_assertions: u8,
    string_assertions: u8,
) -> *mut std::ffi::c_void {
    // SAFETY: Both input and nested replacement bytes are immutable for this call.
    let bytes = unsafe { utf8::borrow(data, length) };
    // SAFETY: The tag and nested replacement span obey the descriptor contract.
    let replacement = unsafe { encoded_parameter(&replacement) };
    Box::into_raw(Box::new(encoded::replace(
        bytes,
        index,
        replacement,
        numeric_assertions != 0,
        string_assertions != 0,
    )))
    .cast()
}

/// Obtain a by-value immutable view without mutating/reallocating result storage.
///
/// # Safety
/// owner is a live result from exactly one encoded operation; no concurrent destroy.
/// Output bytes are borrowed from Rust until destroy, never freed by C++. Empty
/// output returns null. Diagnostics' offsets refer to the original encoded input.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_encoded_view(owner: *const std::ffi::c_void) -> EncodedView {
    // SAFETY: The pointer identifies the live Rust owner described above.
    unsafe { &*owner.cast::<encoded::Output>() }.view()
}

/// Get one ordered by-value diagnostic; getters never change owner storage.
///
/// # Safety
/// owner has `encoded_view`'s live-owner contract; index < `diagnostic_count`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_encoded_diagnostic(
    owner: *const std::ffi::c_void,
    index: usize,
) -> EncodedDiagnostic {
    // SAFETY: The owner is live and the index is bounded by its immutable view.
    unsafe { &*owner.cast::<encoded::Output>() }.diagnostic(index)
}

/// Destroy the opaque result with Rust's allocator; null is a no-op.
///
/// # Safety
/// A nonnull pointer is a unique live result returned by one encoded operation,
/// destroyed exactly once after all output/diagnostic views cease use. Never use
/// C++ delete/free for this pointer. No concurrent getters; panic cannot unwind.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_encoded_destroy(owner: *mut std::ffi::c_void) {
    if !owner.is_null() {
        // SAFETY: Ownership is returned exactly once to the allocating Rust Box.
        drop(unsafe { Box::from_raw(owner.cast::<encoded::Output>()) });
    }
}

/// Rounded square root; scalar-only ABI, no ownership or pointers. Panic aborts.
#[allow(unsafe_code)] // Only the export symbol attribute.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_int_sqrt(value: u32) -> u32 {
    math::int_sqrt(value)
}

/// Soft clamp integer bits, preserving narrow promotions and unsigned wrapping.
/// No allocations, pointers, state, or ownership; panic aborts, never unwinds.
#[allow(unsafe_code)] // Only the export symbol attribute.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_soft_clamp(
    value: u64,
    min: u64,
    max: u64,
    width: u8,
    is_signed: u8,
) -> u64 {
    math::soft_clamp(value, min, max, width, is_signed != 0)
}

pub use history::{Descriptor as HistoryDescriptor, Step as HistoryStep};

/// Allocate a scalar history engine; the contract is in `history_ffi.h`.
///
/// Month is explicit for update and rotation and the current global phase for validity;
/// query phase is supplied after C++ constructs each scratch array. Destroy the handle once.
#[allow(unsafe_code)] // Export attribute only; the descriptor token is never dereferenced.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_history_create(
    mode: u8,
    token: usize,
    mask: u64,
    age: u32,
    month: u32,
) -> *mut std::ffi::c_void {
    Box::into_raw(Box::new(history::Engine::new(
        mode, token, mask, age, month,
    )))
    .cast()
}

#[allow(unsafe_code)] // One exclusive handle borrow, never kept across an extern call.
unsafe fn history_engine<'a>(handle: *mut std::ffi::c_void) -> &'a mut history::Engine {
    // SAFETY: Each caller requires a live uniquely accessible Rust engine handle.
    unsafe { &mut *handle.cast::<history::Engine>() }
}

/// Return the next structural instruction; execute all typed work after returning.
///
/// # Safety
/// Handle is live, nonnull, created by `openttd_rust_history_create`, and exclusively accessible
/// during the call. Follow the Describe/phase/completion protocol; no C++ callback
/// occurs in Rust, and no history or scratch allocation is borrowed. No concurrent
/// handle use or descriptor mutation; panic aborts and the C ABI never unwinds.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_next(handle: *mut std::ffi::c_void) -> HistoryStep {
    // SAFETY: Caller grants this engine's exclusive borrow until return.
    unsafe { history_engine(handle) }.next()
}

/// Supply immutable scalar fields for the most recent Describe instruction.
///
/// # Safety
/// The handle contract is the same as `openttd_rust_history_next`. Descriptor identity/fields
/// come from the requested live C++ object, child token from its actual pointer;
/// no C++ layout is reinterpreted by Rust. Both descriptor objects remain live.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_describe(
    handle: *mut std::ffi::c_void,
    descriptor: HistoryDescriptor,
) {
    // SAFETY: The caller gives unique access to the waiting engine.
    unsafe { history_engine(handle) }.describe(descriptor);
}

/// Supply the live global query phase after constructing every typed scratch element.
///
/// # Safety
/// The handle contract is the same as `openttd_rust_history_next`. Call only immediately after
/// a `PrepareScratch` instruction and before requesting its children.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_phase(handle: *mut std::ffi::c_void, month: u32) {
    // SAFETY: The caller gives unique access to the waiting query engine.
    unsafe { history_engine(handle) }.phase(month);
}

/// Finish one typed leaf/reduction assignment and return its OR validity.
///
/// # Safety
/// The handle contract is the same as `openttd_rust_history_next`. Call only after successfully
/// executing Leaf or `QueryReduce`; C++ destroys its scratch scope before any later
/// typed operation or phase read. On a typed exception destroy the owner instead.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_complete(handle: *mut std::ffi::c_void) -> u8 {
    // SAFETY: The caller gives unique access after the typed assignment completes.
    u8::from(unsafe { history_engine(handle) }.complete_query())
}

/// Destroy scalar Rust storage, including on C++ typed-operation exceptions.
///
/// # Safety
/// Handle is live/non-null, created by `openttd_rust_history_create`, and destroyed exactly once
/// with no active access. C++ never deletes this allocation; no typed storage or
/// descriptor object is freed. Panic aborts and this function never unwinds.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_destroy(handle: *mut std::ffi::c_void) {
    // SAFETY: The uniquely owned handle returns its allocation to Rust exactly once.
    drop(unsafe { Box::from_raw(handle.cast::<history::Engine>()) });
}

mod abi;

/// Return size, alignment and offsets by native type name; unknown names/items return MAX.
///
/// # Safety
/// `name` is a live NUL-terminated C string for this call. No pointer is retained.
/// Metadata only; panics abort and callbacks/allocation are absent.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_abi_layout_named(
    name: *const std::ffi::c_char,
    item: usize,
) -> usize {
    // SAFETY: caller supplies the terminated name for this synchronous call.
    let name = unsafe { std::ffi::CStr::from_ptr(name) };
    name.to_str()
        .map_or(usize::MAX, |name| abi::layout(name, item))
}

/// Create independent admin conversion control; typed VM/JSON state stays in C++.
/// No C++ pointer/callback enters Rust. OOM/panic abort; the C ABI never unwinds.
#[allow(unsafe_code)] // Only the export symbol attribute.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_admin_conversion_create(
    direction: u8,
    index: i64,
    depth: i32,
) -> *mut std::ffi::c_void {
    Box::into_raw(Box::new(admin_conversion::Engine::new(
        direction, index, depth,
    )))
    .cast()
}

/// Return an action; execute it in C++ only after this function has returned.
///
/// # Safety
/// Handle is a live, nonnull, exclusively accessed owner returned by create.
/// The previous scalar response follows `admin_conversion_ffi.h`. No typed borrow
/// or callback survives return; reentrant VM actions use separate owners.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_admin_conversion_advance(
    handle: *mut std::ffi::c_void,
    response: u64,
) -> AdminAction {
    // SAFETY: The caller exclusively owns the live engine for this call.
    unsafe { &mut *handle.cast::<admin_conversion::Engine>() }.advance(response)
}

/// Destroy control allocations without executing pending typed VM cleanup.
///
/// # Safety
/// Handle is live/non-null, returned by create, and destroyed exactly once with
/// no active access. No C++ allocation is freed; exceptions never enter Rust.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_admin_conversion_destroy(handle: *mut std::ffi::c_void) {
    // SAFETY: Return unique ownership to the allocating Rust Box exactly once.
    drop(unsafe { Box::from_raw(handle.cast::<admin_conversion::Engine>()) });
}

/// Classify exact raw widget part tags; attributes and containers are distinct.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_widget_part_classify(kind: u8) -> u8 {
    u8::from(widget_parser::attribute(kind)) | (u8::from(widget_parser::container(kind)) << 1)
}

/// Create widget-only traversal state; descriptor/widget pointers stay in C++.
///
/// # Panics
/// Invalid window/trailing-check selectors violate the scalar protocol and abort.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_widget_parser_create(
    length: u64,
    window: u8,
    trailing_check: u8,
) -> *mut std::ffi::c_void {
    assert!(window <= 1 && trailing_check <= 1);
    Box::into_raw(Box::new(widget_parser::Engine::new(
        length,
        window != 0,
        trailing_check != 0,
    )))
    .cast()
}

/// Return an action for execution in C++ after this function returns.
///
/// # Safety
/// Handle is live, nonnull, exclusive and returned by create. Scalar responses
/// follow `widget_parser_ffi.h`; no callback or C++ object borrow crosses the call.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_widget_parser_advance(
    handle: *mut std::ffi::c_void,
    response: u8,
) -> WidgetAction {
    // SAFETY: The caller exclusively owns the live engine for this call.
    unsafe { &mut *handle.cast::<widget_parser::Engine>() }.advance(response)
}

/// Destroy control allocations without pending typed construction or cleanup.
///
/// # Safety
/// Handle is live/non-null, returned by create and destroyed exactly once with
/// no active access. No C++ ownership or exception enters Rust.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_widget_parser_destroy(handle: *mut std::ffi::c_void) {
    // SAFETY: Return unique ownership to the allocating Rust Box exactly once.
    drop(unsafe { Box::from_raw(handle.cast::<widget_parser::Engine>()) });
}

#[allow(unsafe_code)]
mod crypto_primitives;

/// Select cargo traversal without borrowing world state; invalid tags return 255.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_plan(mode: u8, selector: u8) -> u8 {
    station_cargo::plan(mode, selector)
}

/// Initialize caller-owned scalar collector; no allocation or retained pointer.
///
/// # Safety
/// State is writable, aligned storage of the documented C layout. Selector < 4.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_init(
    state: *mut station_cargo::Collector,
    selector: u8,
    other: u16,
) {
    // SAFETY: Caller supplies writable aligned state with no active access.
    unsafe {
        state.write(station_cargo::Collector::new(selector, other));
    }
}

/// Feed a packet after typed iteration returns to C++.
///
/// # Safety
/// State and list are live, aligned, exclusive and disjoint for this call only.
/// Neither is finalized; no callback/world/VM operation enters this Rust frame.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet(
    state: *mut station_cargo::Collector,
    list: *mut std::ffi::c_void,
    from: u16,
    via: u16,
    amount: u32,
) {
    // SAFETY: Exclusive disjoint state/list borrows expire at return.
    unsafe {
        (&mut *state).packet(&mut *list.cast::<script_list::List>(), from, via, amount);
    }
}

/// Begin an origin, resetting only the cumulative-share decoder.
///
/// # Safety
/// State is initialized, live, aligned, exclusive and not finalized.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_origin(
    state: *mut station_cargo::Collector,
    origin: u16,
) {
    // SAFETY: The caller exclusively borrows the scalar state for this call.
    unsafe {
        (&mut *state).origin(origin);
    }
}

/// Decode a visited cumulative share and feed the selected run.
///
/// # Safety
/// Same disjoint, exclusive state/list contract as `cargo_packet`; origin began.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_share(
    state: *mut station_cargo::Collector,
    list: *mut std::ffi::c_void,
    via: u16,
    cumulative: u32,
) {
    // SAFETY: The disjoint borrows end before another typed iterator operation.
    unsafe {
        (&mut *state).share(&mut *list.cast::<script_list::List>(), via, cumulative);
    }
}

/// Flush the last positive run once; RAII calls this even on C++ unwinding.
///
/// # Safety
/// State/list are live, aligned, exclusive, disjoint; destination outlives state.
/// No destructor callback crosses Rust; panic/allocation exhaustion aborts.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_finish(
    state: *mut station_cargo::Collector,
    list: *mut std::ffi::c_void,
) {
    // SAFETY: C++ keeps the destination alive through this synchronous finalizer.
    unsafe {
        (&mut *state).finish(&mut *list.cast::<script_list::List>());
    }
}

#[allow(unsafe_code)]
mod blake2b;

/// Initialize caller-owned copyable Packet state; no allocation/pointer retention.
///
/// # Safety
/// State is aligned writable storage of the documented C layout, exclusive here.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_init(state: *mut packet::State, limit: usize) {
    // SAFETY: Caller provides valid writable scalar state storage.
    unsafe {
        state.write(packet::State::new(limit));
    }
}

/// Normalize both sent bools and arbitrary received bool bytes.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_packet_boolean(value: u8) -> u8 {
    u8::from(value != 0)
}

/// Native unsigned capacity decision; no socket effect is performed here.
///
/// # Safety
/// State is initialized, aligned and borrowed shared only during this call.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_can_write(
    state: *const packet::State,
    size: usize,
    amount: usize,
) -> u8 {
    // SAFETY: Shared scalar borrow ends at return.
    u8::from(unsafe { &*state }.can_write(size, amount))
}

/// Bounds decision after C++ has checked client-quit state first.
///
/// # Safety
/// Same shared state contract as `packet_can_write`; C++ retains close policy.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_can_read(
    state: *const packet::State,
    size: usize,
    amount: usize,
) -> u8 {
    // SAFETY: Shared scalar borrow ends at return.
    u8::from(unsafe { &*state }.can_read(size, amount))
}

/// Test completion of the two-byte size header.
///
/// # Safety
/// State is initialized, live, aligned and shared for this call only.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_has_size(state: *const packet::State) -> u8 {
    // SAFETY: Shared scalar borrow ends at return.
    u8::from(unsafe { &*state }.position >= 2)
}

/// Native unsigned size-minus-cursor; no new clamping policy.
///
/// # Safety
/// State follows the live aligned shared scalar contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_remaining(
    state: *const packet::State,
    size: usize,
) -> usize {
    // SAFETY: Shared scalar borrow ends at return.
    unsafe { &*state }.remaining(size)
}

/// Plan transfer amount before the C++ callback; no byte/span borrow is retained.
///
/// # Safety
/// State is live, aligned and shared only here; original transfer-domain applies.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_transfer_amount(
    state: *const packet::State,
    size: usize,
    limit: usize,
) -> usize {
    // SAFETY: This borrow ends before C++ invokes the transfer callback.
    unsafe { &*state }.transfer_amount(size, limit)
}

/// Commit a positive result against the live post-callback cursor; narrow to u16.
///
/// # Safety
/// State is live/aligned/exclusive only here. Callback has returned normally;
/// no span/reference is retained. Original returned-count obligations remain.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_transfer_commit(
    state: *mut packet::State,
    amount: isize,
) {
    // SAFETY: The exclusive borrow starts only after the C++ callback returns.
    unsafe { &mut *state }.transfer_commit(amount);
}

/// Plan limited raw-byte insertion, preserving native unsigned subtraction.
///
/// # Safety
/// State is live/aligned/shared only here. No input bytes are borrowed.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_amount(
    state: *const packet::State,
    size: usize,
    input: usize,
) -> usize {
    // SAFETY: Shared scalar borrow ends before C++ vector insertion.
    unsafe { &*state }.send_amount(size, input)
}

/// Length-prefixed buffer capacity request uses the original native addition.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_packet_buffer_size(length: usize) -> usize {
    length.wrapping_add(2)
}

/// Narrow a length-prefix or MAC skip to the original unsigned 16-bit width.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_packet_prefix(length: usize) -> u16 {
    packet::prefix(length)
}

/// Decode width 1, 2, 4 or 8 with persistent cursor narrowing after every byte.
///
/// # Safety
/// State is initialized/aligned/exclusive. Bytes are one readable allocation of
/// length <= `ISIZE_MAX`; state/bytes are disjoint and no concurrent mutation occurs.
/// Original scalar-read bounds check succeeded; width is 1, 2, 4 or 8. Borrows end here.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_recv(
    state: *mut packet::State,
    bytes: *const u8,
    length: usize,
    width: u8,
) -> u64 {
    // SAFETY: The caller provides disjoint valid state and short-lived byte storage.
    unsafe { &mut *state }.recv(unsafe { utf8::borrow(bytes, length) }, width)
}

/// Decrement remaining and consume one byte BEFORE C++ result `push_back`.
///
/// # Safety
/// Scalar state/count/output are aligned, writable, exclusive and disjoint from
/// each other/bytes. Read storage follows `packet_recv`. Prefix/body bounds succeeded;
/// caller stops on false. No allocation or callback occurs here.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_buffer_next(
    state: *mut packet::State,
    bytes: *const u8,
    length: usize,
    remaining: *mut u16,
    output: *mut u8,
) -> u8 {
    // SAFETY: Exclusive count borrow is valid and disjoint from all other arguments.
    let count = unsafe { &mut *remaining };
    let previous = *count;
    *count = count.wrapping_sub(1);
    if previous == 0 {
        return 0;
    }
    // SAFETY: Exclusive state and valid bytes share only this call; output is disjoint.
    let byte = unsafe { &mut *state }.recv(unsafe { utf8::borrow(bytes, length) }, 1);
    unsafe {
        output.write(byte.to_le_bytes()[0]);
    }
    1
}

/// Decode/validate declared size without committing cursor before C++ resize.
///
/// # Safety
/// Live/aligned/shared state; readable disjoint storage >=2 and <= `ISIZE_MAX`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_parse_size(
    state: *const packet::State,
    bytes: *const u8,
    length: usize,
) -> u16 {
    // SAFETY: The two original required header bytes exist for this short borrow.
    unsafe { &*state }.parse_size(unsafe { utf8::borrow(bytes, length) })
}

/// Set the read cursor to 2, before external receive-handler operations or after resize.
///
/// # Safety
/// State is live, aligned and exclusive for this call only.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_read_start(state: *mut packet::State) {
    // SAFETY: Exclusive scalar borrow ends at return.
    unsafe { &mut *state }.read_start();
}

/// Set the send cursor to 0 only after Encrypt returns normally, before `shrink_to_fit`.
///
/// # Safety
/// State is live, aligned and exclusive; all C++ callbacks have returned normally.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_send_reset(state: *mut packet::State) {
    // SAFETY: This exclusive borrow begins after external encryption returns.
    unsafe { &mut *state }.send_reset();
}

/// Compute packet framing offsets with native unsigned wrap; no byte borrow.
///
/// # Safety
/// Output is live/aligned/exclusive writable Frame storage. Original span domains
/// remain caller obligations. Return value is the original strict receive-size check.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_frame(
    position: u16,
    size: usize,
    mac: usize,
    output: *mut packet::Frame,
) -> u8 {
    let frame = packet::frame(position, size, mac);
    let accepted = u8::from(size > frame.message);
    // SAFETY: Caller supplies writable output storage; no pointer survives return.
    unsafe {
        output.write(frame);
    }
    accepted
}

/// Skip narrowed MAC bytes against the live cursor only after Decrypt returns.
///
/// # Safety
/// State is initialized/live/aligned/exclusive; no external callback is active.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_skip_mac(state: *mut packet::State, mac: usize) {
    // SAFETY: Exclusive borrow begins after the normal C++ Decrypt return.
    unsafe { &mut *state }.skip_mac(mac);
}

/// Write the low 16 bits of the size header before external send-handler calls.
///
/// # Safety
/// Bytes are one exclusive writable allocation >=2 and <= `ISIZE_MAX`; caller has
/// performed the original zero-header assertion. No borrow survives this call.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_packet_write_header(bytes: *mut u8, length: usize) {
    // SAFETY: The exclusive allocation is valid for this short mutable borrow only.
    packet::header(unsafe { std::slice::from_raw_parts_mut(bytes, length) });
}

#[allow(unsafe_code)]
mod x25519;

/// Scan one historical sanitation step without retaining bytes or doing output.
///
/// # Safety
/// Input is one readable initialized allocation of length <= `ISIZE_MAX`, valid
/// and immutable only for this call; a null pointer is allowed for zero length.
/// The returned encoded bytes own their storage. No pointer escapes this call.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_validation_step(
    bytes: *const u8,
    length: usize,
    settings: u8,
) -> string_validation::Step {
    // SAFETY: The caller grants the short-lived readable byte allocation.
    string_validation::step(unsafe { utf8::borrow(bytes, length) }, settings)
}

/// Validate a fixed span, requiring its first valid NUL before its end.
///
/// # Safety
/// Same readable allocation/lifetime contract as `validation_step`; no borrow
/// escapes, no output/allocation/callback occurs, and panics abort.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_validation_valid(bytes: *const u8, length: usize) -> u8 {
    // SAFETY: The caller supplies immutable bytes for only this call.
    u8::from(string_validation::valid(unsafe {
        utf8::borrow(bytes, length)
    }))
}

/// Check live consumer capacity, copy in original forward order, then commit.
///
/// # Safety
/// The `string_validation_ffi.h` contract: with capacity accepted, `length` source bytes are
/// readable and `position..position+length` of the destination is writable, the output start
/// lies outside a nonempty source range, and zero length accesses neither pointer.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_inplace_write(
    destination: *mut u8,
    position: usize,
    consumed: usize,
    source: *const u8,
    length: usize,
) -> string_validation::Write {
    let result = string_validation::write_plan(position, consumed, length);
    if result.accepted != 0 {
        for index in 0..length {
            // SAFETY: Caller supplies live valid raw ranges and permitted overlap.
            // No shared/mutable slices are constructed; this value read ends before write.
            let byte = unsafe { source.add(index).read() };
            unsafe { destination.add(position + index).write(byte) };
        }
    }
    result
}

mod water_regions;

mod industry;
#[allow(unsafe_code)]
mod ship_yapf;

mod orders;
mod train;
mod train_state;

mod cargo_storage;
mod train_reservation;

mod fleet;

pub(crate) mod clear_map;
pub(crate) mod rail_map;
mod subsidy;
pub(crate) mod tile_map;
pub(crate) mod tree_map;
pub(crate) mod water_map;

mod cargo_registry;
mod cargo_type;
mod cargotype;
mod container_func;
mod source_type;
mod subsidy_base;
mod tilearea;
mod timer_game_common;
mod timer_game_tick;

mod subsidy_type;

pub mod landscape_cmd;

mod newgrf_vehicle;

pub mod vehicle_func;

pub mod rail_type;

pub mod base_bitset_type;

mod newgrf_airport;
mod newgrf_roadstop;

mod rail;
mod type_availability;

mod override_manager;

mod industry_map;

mod newgrf_internal;
