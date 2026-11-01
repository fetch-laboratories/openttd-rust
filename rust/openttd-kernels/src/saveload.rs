/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Save/load codec owner from `saveload.cpp`: codec state, read buffer and memory dumper
//! cursors, and gamma/array-length encoding, plus native save/load completion results.
//!
//! One process-wide [`Codec`] holds the state the original keeps in `_sl`, `_next_offs`,
//! `ReadBuffer` and `MemoryDumper`. Its [`OpenTTDRustSaveLoadCodec`] head has a C layout
//! that native `SlReadByte`/`SlWriteByte` and unported codec code access in place.
//! Saving to memory and loading run under the game-state mutex; the `ottd:savegame`
//! thread flushes and closes the dumper while the main thread holds `saveinprogress`
//! and touches no codec state, as in the original.
//!
//! Errors never unwind Rust frames. A Rust-detected corruption stores its message and
//! returns [`SL_STATUS_CORRUPT`]; the native facade then calls `SlErrorCorrupt`. A native
//! filter exception is caught by its named native frame, which keeps the exception and
//! returns failure; Rust returns [`SL_STATUS_NATIVE`] and the facade rethrows it.
//! `SlError` may reenter the codec through `SlNullPointers` from either point, so no
//! borrow of the codec is held across a native call.

#![allow(unsafe_code)]
// Casts reproduce the narrowing and sign conversions of `saveload.cpp`.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use crate::game_state::GameState;
use std::ffi::c_void;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SaveOrLoadResult(pub u8);
pub(crate) const SL_OK: SaveOrLoadResult = SaveOrLoadResult(0);
pub(crate) const SL_REINIT: SaveOrLoadResult = SaveOrLoadResult(2);

/// `MEMORY_CHUNK_SIZE` in `saveload.cpp`: read and dump in chunks of 128 KiB.
const MEMORY_CHUNK_SIZE: usize = 128 * 1024;

/// `SaveLoadAction` in `saveload.cpp`.
pub(crate) const SLA_LOAD: u8 = 0;
pub(crate) const SLA_SAVE: u8 = 1;
pub(crate) const SLA_PTRS: u8 = 2;
pub(crate) const SLA_NULL: u8 = 3;
pub(crate) const SLA_LOAD_CHECK: u8 = 4;
/// `NeedLength` in `saveload.cpp`.
pub(crate) const NL_NONE: u8 = 0;
pub(crate) const NL_WANTLENGTH: u8 = 1;
pub(crate) const NL_CALCLENGTH: u8 = 2;
/// `ChunkType` in `saveload.h`.
pub(crate) const CH_RIFF: u8 = 0;
pub(crate) const CH_ARRAY: u8 = 1;
pub(crate) const CH_SPARSE_ARRAY: u8 = 2;
pub(crate) const CH_TABLE: u8 = 3;
pub(crate) const CH_SPARSE_TABLE: u8 = 4;

/// `SaveLoadVersion` values in `saveload.h` that the codec tests.
pub(crate) const SLV_4: u16 = 4;
pub(crate) const SLV_69: u16 = 69;
pub(crate) const SLV_169: u16 = 169;
pub(crate) const SLV_SAVELOAD_LIST_LENGTH: u16 = 293;
pub(crate) const SLV_ENCODED_STRING_FORMAT: u16 = 350;
pub(crate) const SLV_FIX_SCC_ENCODED_NEGATIVE: u16 = 353;

/// `SLRefType` in `saveload.h`.
pub(crate) const REF_VEHICLE: u32 = 1;
pub(crate) const REF_STATION: u32 = 2;
pub(crate) const REF_TOWN: u32 = 3;
pub(crate) const REF_VEHICLE_OLD: u32 = 4;
pub(crate) const REF_ROADSTOPS: u32 = 5;
pub(crate) const REF_ENGINE_RENEWS: u32 = 6;
pub(crate) const REF_CARGO_PACKET: u32 = 7;
pub(crate) const REF_ORDERLIST: u32 = 8;
pub(crate) const REF_STORAGE: u32 = 9;
pub(crate) const REF_LINK_GRAPH: u32 = 10;
pub(crate) const REF_LINK_GRAPH_JOB: u32 = 11;

/// `VarTypes` in `saveload.h`.
pub(crate) const SLE_FILE_END: u32 = 0;
pub(crate) const SLE_FILE_I8: u32 = 1;
pub(crate) const SLE_FILE_U8: u32 = 2;
pub(crate) const SLE_FILE_I16: u32 = 3;
pub(crate) const SLE_FILE_U16: u32 = 4;
pub(crate) const SLE_FILE_I32: u32 = 5;
pub(crate) const SLE_FILE_U32: u32 = 6;
pub(crate) const SLE_FILE_I64: u32 = 7;
pub(crate) const SLE_FILE_U64: u32 = 8;
pub(crate) const SLE_FILE_STRINGID: u32 = 9;
pub(crate) const SLE_FILE_STRING: u32 = 10;
pub(crate) const SLE_VAR_BL: u32 = 0 << 4;
pub(crate) const SLE_VAR_I8: u32 = 1 << 4;
pub(crate) const SLE_VAR_U8: u32 = 2 << 4;
pub(crate) const SLE_VAR_I16: u32 = 3 << 4;
pub(crate) const SLE_VAR_U16: u32 = 4 << 4;
pub(crate) const SLE_VAR_I32: u32 = 5 << 4;
pub(crate) const SLE_VAR_U32: u32 = 6 << 4;
pub(crate) const SLE_VAR_I64: u32 = 7 << 4;
pub(crate) const SLE_VAR_U64: u32 = 8 << 4;
pub(crate) const SLE_VAR_NULL: u32 = 9 << 4;
pub(crate) const SLE_VAR_STR: u32 = 12 << 4;
pub(crate) const SLE_VAR_STRQ: u32 = 13 << 4;
pub(crate) const SLE_VAR_NAME: u32 = 14 << 4;
pub(crate) const SLE_INT8: u32 = SLE_FILE_I8 | SLE_VAR_I8;
pub(crate) const SLE_UINT8: u32 = SLE_FILE_U8 | SLE_VAR_U8;
pub(crate) const SLE_INT16: u32 = SLE_FILE_I16 | SLE_VAR_I16;
pub(crate) const SLE_UINT16: u32 = SLE_FILE_U16 | SLE_VAR_U16;
pub(crate) const SLE_INT32: u32 = SLE_FILE_I32 | SLE_VAR_I32;
pub(crate) const SLE_UINT32: u32 = SLE_FILE_U32 | SLE_VAR_U32;
pub(crate) const SLE_STRINGID: u32 = SLE_FILE_STRINGID | SLE_VAR_U32;
pub(crate) const SLF_ALLOW_CONTROL: u32 = 1 << 8;
pub(crate) const SLF_ALLOW_NEWLINE: u32 = 1 << 9;
pub(crate) const SLF_REPLACE_TABCRLF: u32 = 1 << 10;

/// `GetVarMemType`.
const fn mem_type(conv: u32) -> u32 {
    conv & 0xF0
}

/// `GetVarFileType`.
const fn file_type(conv: u32) -> u32 {
    conv & 0xF
}

/// Result of a codec entry that can fail.
pub type OpenTTDRustSaveLoadStatus = u8;
pub(crate) const SL_STATUS_OK: OpenTTDRustSaveLoadStatus = 0;
/// The codec found a corrupt savegame; the message is in the codec's error slot.
pub(crate) const SL_STATUS_CORRUPT: OpenTTDRustSaveLoadStatus = 1;
/// A native frame caught an exception and keeps it for the facade to rethrow.
pub(crate) const SL_STATUS_NATIVE: OpenTTDRustSaveLoadStatus = 2;

type Status = Result<(), OpenTTDRustSaveLoadStatus>;

/// Codec state shared in place with native code.
#[repr(C)]
pub struct OpenTTDRustSaveLoadCodec {
    /// `ReadBuffer::bufp`.
    pub read_cursor: *mut u8,
    /// `ReadBuffer::bufe`.
    pub read_end: *mut u8,
    /// `MemoryDumper::buf`.
    pub write_cursor: *mut u8,
    /// `MemoryDumper::bufe`.
    pub write_end: *mut u8,
    pub obj_len: usize,
    /// `_next_offs`.
    pub next_offs: usize,
    pub array_index: i32,
    pub last_array_index: i32,
    /// `&_sl_version`, set once by native code.
    pub version: *const u16,
    /// `&_sl_minor_version`, set once by native code.
    pub minor_version: *const u8,
    pub action: u8,
    pub need_length: u8,
    pub block_mode: u8,
    pub expect_table_header: bool,
}

/// Native `NOT_REACHED`: prints the native fatal-error message and exits.
fn not_reached(services: &OpenTTDSaveLoadServices) -> ! {
    // SAFETY: `shared` is the process-wide shared table, live for the whole program.
    unsafe { ((*services.shared).not_reached)() };
    std::process::abort()
}

/// Native leaves the codec calls; each is a typed `noexcept` C++ function.
#[repr(C)]
pub struct OpenTTDSaveLoadServices {
    /// Shared native services; `not_reached` is the native `NOT_REACHED`.
    pub shared: *const crate::services::Services,
    /// Native `assert`, keeping the native assertion policy.
    pub native_assert: extern "C" fn(bool),
    /// `Debug(sl, 0, "SlIterateArray error")`.
    pub debug_iterate_error: extern "C" fn(),
    /// `LoadFilter::Read` on the reader's filter; false when it threw.
    pub read:
        extern "C" fn(filter: *mut c_void, buf: *mut u8, len: usize, read: *mut usize) -> bool,
    /// `SaveFilter::Write`; false when it threw.
    pub write: extern "C" fn(filter: *mut c_void, buf: *const u8, len: usize) -> bool,
    /// `SaveFilter::Finish`; false when it threw.
    pub finish: extern "C" fn(filter: *mut c_void) -> bool,
    /// `RemapOldStringID`.
    pub remap_old_string: extern "C" fn(id: u32) -> u32,
    /// `*reinterpret_cast<std::string *>(ptr) = CopyFromOldName(id)`.
    pub copy_from_old_name: extern "C" fn(ptr: *mut c_void, id: u32),
    /// Pool references: `index` returns `obj->index`; `get` returns `T::Get(index)` when
    /// `T::IsValidID(index)`, otherwise null.
    pub cargo_packet_get: extern "C" fn(index: usize) -> *mut c_void,
    pub cargo_packet_index: extern "C" fn(obj: *const c_void) -> u32,
    pub engine_renew_get: extern "C" fn(index: usize) -> *mut c_void,
    pub engine_renew_index: extern "C" fn(obj: *const c_void) -> u32,
    pub link_graph_get: extern "C" fn(index: usize) -> *mut c_void,
    pub link_graph_index: extern "C" fn(obj: *const c_void) -> u32,
    pub link_graph_job_get: extern "C" fn(index: usize) -> *mut c_void,
    pub link_graph_job_index: extern "C" fn(obj: *const c_void) -> u32,
    pub order_list_get: extern "C" fn(index: usize) -> *mut c_void,
    pub order_list_index: extern "C" fn(obj: *const c_void) -> u32,
    pub persistent_storage_get: extern "C" fn(index: usize) -> *mut c_void,
    pub persistent_storage_index: extern "C" fn(obj: *const c_void) -> u32,
    pub road_stop_get: extern "C" fn(index: usize) -> *mut c_void,
    pub road_stop_index: extern "C" fn(obj: *const c_void) -> u32,
    pub station_get: extern "C" fn(index: usize) -> *mut c_void,
    pub station_index: extern "C" fn(obj: *const c_void) -> u32,
    pub town_get: extern "C" fn(index: usize) -> *mut c_void,
    pub town_index: extern "C" fn(obj: *const c_void) -> u32,
    pub vehicle_get: extern "C" fn(index: usize) -> *mut c_void,
    pub vehicle_index: extern "C" fn(obj: *const c_void) -> u32,
    /// `std::string` at `ptr`: `data()`, with `size()` in `length`.
    pub string_data: extern "C" fn(ptr: *const c_void, length: *mut usize) -> *const u8,
    /// `std::string` at `ptr`: `resize(length)`, then `data()`.
    pub string_resize: extern "C" fn(ptr: *mut c_void, length: usize) -> *mut u8,
    /// Replay native diagnostics/assertions and copy the borrowed encoded output into `*str`.
    pub apply_encoded: extern "C" fn(
        ptr: *mut c_void,
        view: *const crate::encoded::View,
        diagnostics: *const crate::encoded::Diagnostic,
    ),
    /// `proc(arg)` for `SlAutolength`, where `proc` is an `AutolengthProc *`; false when it threw.
    pub autolength: extern "C" fn(proc_: *const c_void, arg: i32) -> bool,
}

struct Codec {
    shared: OpenTTDRustSaveLoadCodec,
    /// `ReadBuffer::buf`, present while a reader is open.
    read_buffer: Option<Box<[u8]>>,
    /// `ReadBuffer::reader`: the native `LoadFilter`, kept alive by native code.
    read_filter: *mut c_void,
    /// `ReadBuffer::read`.
    read: usize,
    /// `MemoryDumper::blocks`.
    blocks: Vec<Box<[u8]>>,
    /// Message of the last corruption the codec detected.
    error: String,
}

static CODEC: GameState<Codec> = GameState::new(Codec {
    shared: OpenTTDRustSaveLoadCodec {
        read_cursor: std::ptr::null_mut(),
        read_end: std::ptr::null_mut(),
        write_cursor: std::ptr::null_mut(),
        write_end: std::ptr::null_mut(),
        obj_len: 0,
        next_offs: 0,
        array_index: 0,
        last_array_index: 0,
        version: std::ptr::null(),
        minor_version: std::ptr::null(),
        action: 0,
        need_length: NL_NONE,
        block_mode: 0,
        expect_table_header: false,
    },
    read_buffer: None,
    read_filter: std::ptr::null_mut(),
    read: 0,
    blocks: Vec::new(),
    error: String::new(),
});

/// Run `f` on the codec. The borrow ends before `f` returns; never call native code in `f`.
fn codec<R>(f: impl FnOnce(&mut Codec) -> R) -> R {
    // SAFETY: Serialized as described in the module documentation; the borrow is local to
    // `f`, which calls no native code that could reenter the codec.
    f(unsafe { &mut *CODEC.get() })
}

fn shared<R>(f: impl FnOnce(&mut OpenTTDRustSaveLoadCodec) -> R) -> R {
    codec(|c| f(&mut c.shared))
}

fn corrupt(message: String) -> OpenTTDRustSaveLoadStatus {
    codec(|c| c.error = message);
    SL_STATUS_CORRUPT
}

/// `ReadBuffer::ReadByte`'s refill: read the next chunk from the native filter.
fn fill(services: &OpenTTDSaveLoadServices) -> Status {
    let (filter, buf) = codec(|c| {
        let buf = c
            .read_buffer
            .as_mut()
            .map_or(std::ptr::null_mut(), |b| b.as_mut_ptr());
        (c.read_filter, buf)
    });
    let mut len = 0;
    if !(services.read)(filter, buf, MEMORY_CHUNK_SIZE, &raw mut len) {
        return Err(SL_STATUS_NATIVE);
    }
    if len == 0 {
        return Err(corrupt("Unexpected end of chunk".to_owned()));
    }
    codec(|c| {
        c.read += len;
        c.shared.read_cursor = buf;
        // SAFETY: The filter filled `len <= MEMORY_CHUNK_SIZE` bytes of the buffer.
        c.shared.read_end = unsafe { buf.add(len) };
    });
    Ok(())
}

/// `ReadBuffer::ReadByte`.
fn read_byte(services: &OpenTTDSaveLoadServices) -> Result<u8, OpenTTDRustSaveLoadStatus> {
    if shared(|s| s.read_cursor == s.read_end) {
        fill(services)?;
    }
    Ok(shared(|s| {
        // SAFETY: The cursor is inside the filled part of the read buffer.
        unsafe {
            let b = *s.read_cursor;
            s.read_cursor = s.read_cursor.add(1);
            b
        }
    }))
}

/// `ReadBuffer::GetSize`.
fn read_size() -> usize {
    codec(|c| c.read - (c.shared.read_end as usize - c.shared.read_cursor as usize))
}

/// `MemoryDumper::WriteByte`'s new block.
fn grow() {
    codec(|c| {
        let mut block = vec![0u8; MEMORY_CHUNK_SIZE].into_boxed_slice();
        let buf = block.as_mut_ptr();
        c.blocks.push(block);
        c.shared.write_cursor = buf;
        // SAFETY: One past the end of the new block.
        c.shared.write_end = unsafe { buf.add(MEMORY_CHUNK_SIZE) };
    });
}

/// `MemoryDumper::WriteByte`.
fn write_byte(b: u8) {
    if shared(|s| s.write_cursor == s.write_end) {
        grow();
    }
    shared(|s| {
        // SAFETY: The cursor is inside the current block.
        unsafe {
            *s.write_cursor = b;
            s.write_cursor = s.write_cursor.add(1);
        }
    });
}

/// `MemoryDumper::GetSize`.
fn dumper_size() -> usize {
    codec(|c| {
        c.blocks.len() * MEMORY_CHUNK_SIZE
            - (c.shared.write_end as usize - c.shared.write_cursor as usize)
    })
}

fn read_uint16(services: &OpenTTDSaveLoadServices) -> Result<u16, OpenTTDRustSaveLoadStatus> {
    let x = u16::from(read_byte(services)?) << 8;
    Ok(x | u16::from(read_byte(services)?))
}

fn read_uint32(services: &OpenTTDSaveLoadServices) -> Result<u32, OpenTTDRustSaveLoadStatus> {
    let x = u32::from(read_uint16(services)?) << 16;
    Ok(x | u32::from(read_uint16(services)?))
}

fn read_uint64(services: &OpenTTDSaveLoadServices) -> Result<u64, OpenTTDRustSaveLoadStatus> {
    let x = read_uint32(services)?;
    let y = read_uint32(services)?;
    Ok(u64::from(x) << 32 | u64::from(y))
}

fn write_uint16(v: u16) {
    write_byte((v >> 8) as u8);
    write_byte(v as u8);
}

fn write_uint32(v: u32) {
    write_uint16((v >> 16) as u16);
    write_uint16(v as u16);
}

fn write_uint64(x: u64) {
    write_uint32((x >> 32) as u32);
    write_uint32(x as u32);
}

/// `IsSavegameVersionBefore(major)`.
fn version_before(major: u16) -> bool {
    // SAFETY: Native code points the codec at `_sl_version` before any save or load.
    unsafe { *shared(|s| s.version) < major }
}

/// `IsSavegameVersionBefore(major, minor)`.
fn version_before_minor(major: u16, minor: u8) -> bool {
    let (version, minor_version) = shared(|s| (s.version, s.minor_version));
    // SAFETY: As in `version_before`, for both version globals.
    unsafe { *version < major || (minor > 0 && *version == major && *minor_version < minor) }
}

/// `_sl_version`.
fn version() -> u16 {
    // SAFETY: As in `version_before`.
    unsafe { *shared(|s| s.version) }
}

/// `SlReadSimpleGamma`.
fn read_simple_gamma(services: &OpenTTDSaveLoadServices) -> Result<u32, OpenTTDRustSaveLoadStatus> {
    let mut i = u32::from(read_byte(services)?);
    if i & 0x80 != 0 {
        i &= !0x80;
        if i & 0x40 != 0 {
            i &= !0x40;
            if i & 0x20 != 0 {
                i &= !0x20;
                if i & 0x10 != 0 {
                    i &= !0x10;
                    if i & 0x08 != 0 {
                        return Err(corrupt("Unsupported gamma".to_owned()));
                    }
                    i = u32::from(read_byte(services)?); // 32 bits only.
                }
                i = (i << 8) | u32::from(read_byte(services)?);
            }
            i = (i << 8) | u32::from(read_byte(services)?);
        }
        i = (i << 8) | u32::from(read_byte(services)?);
    }
    Ok(i)
}

/// `SlWriteSimpleGamma`.
fn write_simple_gamma(services: &OpenTTDSaveLoadServices, i: usize) {
    if i >= 1 << 7 {
        if i >= 1 << 14 {
            if i >= 1 << 21 {
                if i >= 1 << 28 {
                    (services.native_assert)(u32::try_from(i).is_ok());
                    write_byte(0xF0);
                    write_byte((i >> 24) as u8);
                } else {
                    write_byte((0xE0 | (i >> 24)) as u8);
                }
                write_byte((i >> 16) as u8);
            } else {
                write_byte((0xC0 | (i >> 16)) as u8);
            }
            write_byte((i >> 8) as u8);
        } else {
            write_byte((0x80 | (i >> 8)) as u8);
        }
    }
    write_byte(i as u8);
}

/// `SlGetGammaLength`.
fn gamma_length(i: usize) -> u32 {
    1 + u32::from(i >= 1 << 7)
        + u32::from(i >= 1 << 14)
        + u32::from(i >= 1 << 21)
        + u32::from(i >= 1 << 28)
}

/// `SlIterateArray`.
fn iterate_array(services: &OpenTTDSaveLoadServices) -> Result<i32, OpenTTDRustSaveLoadStatus> {
    let next_offs = shared(|s| s.next_offs);
    if next_offs != 0 && read_size() != next_offs {
        return Err(corrupt(format!(
            "Invalid chunk size iterating array - expected to be at position {}, actually at {}",
            next_offs,
            read_size()
        )));
    }

    loop {
        let mut length = read_simple_gamma(services)?;
        if length == 0 {
            (services.native_assert)(!shared(|s| s.expect_table_header));
            shared(|s| s.next_offs = 0);
            return Ok(-1);
        }

        length -= 1;
        let size = read_size();
        let header = shared(|s| {
            s.obj_len = length as usize;
            s.next_offs = size.wrapping_add(length as usize);
            std::mem::replace(&mut s.expect_table_header, false)
        });
        if header {
            return Ok(i32::MAX);
        }

        let index = match shared(|s| s.block_mode) {
            CH_SPARSE_TABLE | CH_SPARSE_ARRAY => read_simple_gamma(services)? as i32,
            CH_TABLE | CH_ARRAY => shared(|s| {
                let index = s.array_index;
                s.array_index = index.wrapping_add(1);
                index
            }),
            _ => {
                (services.debug_iterate_error)();
                return Ok(-1);
            }
        };

        if length != 0 {
            return Ok(index);
        }
    }
}

/// `SlSkipBytes`, reading whole buffered runs at a time with the original refills.
fn skip_bytes(services: &OpenTTDSaveLoadServices, mut length: usize) -> Status {
    while length != 0 {
        if shared(|s| s.read_cursor == s.read_end) {
            fill(services)?;
        }
        shared(|s| {
            let run = (s.read_end as usize - s.read_cursor as usize).min(length);
            // SAFETY: `run` bytes remain in the filled part of the read buffer.
            s.read_cursor = unsafe { s.read_cursor.add(run) };
            length -= run;
        });
    }
    Ok(())
}

/// `SlSkipArray`.
fn skip_array(services: &OpenTTDSaveLoadServices) -> Status {
    while iterate_array(services)? != -1 {
        let skip = shared(|s| s.next_offs).wrapping_sub(read_size());
        skip_bytes(services, skip)?;
    }
    Ok(())
}

/// `SlSetLength`.
fn set_length(services: &OpenTTDSaveLoadServices, length: usize) {
    let (action, need_length) = shared(|s| (s.action, s.need_length));
    (services.native_assert)(action == SLA_SAVE);

    match need_length {
        NL_WANTLENGTH => {
            let (block_mode, header) = shared(|s| {
                s.need_length = NL_NONE;
                (s.block_mode, s.expect_table_header)
            });
            if (block_mode == CH_TABLE || block_mode == CH_SPARSE_TABLE) && header {
                shared(|s| s.expect_table_header = false);
                write_simple_gamma(services, length.wrapping_add(1));
                return;
            }

            match block_mode {
                CH_RIFF => {
                    // Lower 24 bits are normal; bits 24:27 go in the uppermost 4 bits.
                    (services.native_assert)(length < 1 << 28);
                    write_uint32(((length & 0xFF_FFFF) | ((length >> 24) << 28)) as u32);
                }
                CH_TABLE | CH_ARRAY => {
                    let (last, index) = shared(|s| (s.last_array_index, s.array_index));
                    (services.native_assert)(last <= index);
                    loop {
                        let last = shared(|s| {
                            s.last_array_index = s.last_array_index.wrapping_add(1);
                            s.last_array_index
                        });
                        if last > index {
                            break;
                        }
                        write_simple_gamma(services, 1);
                    }
                    write_simple_gamma(services, length.wrapping_add(1));
                }
                CH_SPARSE_TABLE | CH_SPARSE_ARRAY => {
                    let index = shared(|s| s.array_index);
                    // SlGetArrayLength takes size_t from int; SlWriteSparseIndex takes uint.
                    let sparse = gamma_length(index as isize as usize) as usize;
                    write_simple_gamma(services, length.wrapping_add(1).wrapping_add(sparse));
                    write_simple_gamma(services, index as u32 as usize);
                }
                _ => not_reached(services),
            }
        }
        // `_sl.obj_len += (int)length`.
        NL_CALCLENGTH => {
            shared(|s| s.obj_len = s.obj_len.wrapping_add(length as i32 as isize as usize));
        }
        _ => not_reached(services),
    }
}

/// `SlCopyBytes` when loading, copying whole buffered runs with the original refills.
fn read_bytes(services: &OpenTTDSaveLoadServices, ptr: *mut u8, mut length: usize) -> Status {
    let mut p = ptr;
    while length != 0 {
        if shared(|s| s.read_cursor == s.read_end) {
            fill(services)?;
        }
        shared(|s| {
            let run = (s.read_end as usize - s.read_cursor as usize).min(length);
            // SAFETY: `run` bytes remain in the read buffer; the caller provides `length`
            // writable bytes at `ptr`, which never alias the codec's buffer.
            unsafe {
                std::ptr::copy_nonoverlapping(s.read_cursor, p, run);
                s.read_cursor = s.read_cursor.add(run);
                p = p.add(run);
            }
            length -= run;
        });
    }
    Ok(())
}

/// `SlCopyBytes` when saving.
fn write_bytes(ptr: *const u8, length: usize) {
    for i in 0..length {
        // SAFETY: The caller provides `length` readable bytes at `ptr`.
        write_byte(unsafe { *ptr.add(i) });
    }
}

/// `MemoryDumper::Flush`.
fn flush(services: &OpenTTDSaveLoadServices, writer: *mut c_void) -> Status {
    let mut t = dumper_size();
    let mut i = 0;
    while t > 0 {
        let to_write = MEMORY_CHUNK_SIZE.min(t);
        let block = codec(|c| c.blocks[i].as_ptr());
        i += 1;
        if !(services.write)(writer, block, to_write) {
            return Err(SL_STATUS_NATIVE);
        }
        t -= to_write;
    }
    if !(services.finish)(writer) {
        return Err(SL_STATUS_NATIVE);
    }
    Ok(())
}

/// `SlCopyBytes`.
fn copy_bytes(services: &OpenTTDSaveLoadServices, ptr: *mut u8, length: usize) -> Status {
    match shared(|s| s.action) {
        SLA_LOAD_CHECK | SLA_LOAD => read_bytes(services, ptr, length),
        SLA_SAVE => {
            write_bytes(ptr, length);
            Ok(())
        }
        _ => not_reached(services),
    }
}

/// `ReadValue`.
///
/// # Safety
/// `ptr` points to a value of the memory type of `conv`.
unsafe fn read_value(services: &OpenTTDSaveLoadServices, ptr: *const c_void, conv: u32) -> i64 {
    // SAFETY: The caller's contract.
    unsafe {
        match mem_type(conv) {
            SLE_VAR_BL => i64::from(ptr.cast::<u8>().read_unaligned() != 0),
            SLE_VAR_I8 => i64::from(ptr.cast::<i8>().read_unaligned()),
            SLE_VAR_U8 => i64::from(ptr.cast::<u8>().read_unaligned()),
            SLE_VAR_I16 => i64::from(ptr.cast::<i16>().read_unaligned()),
            SLE_VAR_U16 => i64::from(ptr.cast::<u16>().read_unaligned()),
            SLE_VAR_I32 => i64::from(ptr.cast::<i32>().read_unaligned()),
            SLE_VAR_U32 => i64::from(ptr.cast::<u32>().read_unaligned()),
            SLE_VAR_I64 => ptr.cast::<i64>().read_unaligned(),
            SLE_VAR_U64 => ptr.cast::<u64>().read_unaligned() as i64,
            SLE_VAR_NULL => 0,
            _ => not_reached(services),
        }
    }
}

/// `WriteValue`.
///
/// # Safety
/// `ptr` points to a value of the memory type of `conv`.
unsafe fn write_value(services: &OpenTTDSaveLoadServices, ptr: *mut c_void, conv: u32, val: i64) {
    // SAFETY: The caller's contract.
    unsafe {
        match mem_type(conv) {
            SLE_VAR_BL => ptr.cast::<bool>().write_unaligned(val != 0),
            SLE_VAR_I8 => ptr.cast::<i8>().write_unaligned(val as i8),
            SLE_VAR_U8 => ptr.cast::<u8>().write_unaligned(val as u8),
            SLE_VAR_I16 => ptr.cast::<i16>().write_unaligned(val as i16),
            SLE_VAR_U16 => ptr.cast::<u16>().write_unaligned(val as u16),
            SLE_VAR_I32 => ptr.cast::<i32>().write_unaligned(val as i32),
            SLE_VAR_U32 => ptr.cast::<u32>().write_unaligned(val as u32),
            SLE_VAR_I64 => ptr.cast::<i64>().write_unaligned(val),
            SLE_VAR_U64 => ptr.cast::<u64>().write_unaligned(val as u64),
            SLE_VAR_NAME => (services.copy_from_old_name)(ptr, val as u32),
            SLE_VAR_NULL => {}
            _ => not_reached(services),
        }
    }
}

/// `SlSaveLoadConv`.
///
/// # Safety
/// `ptr` points to a value of the memory type of `conv`.
unsafe fn save_load_conv(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut c_void,
    conv: u32,
) -> Status {
    let check = services.native_assert;
    match shared(|s| s.action) {
        SLA_SAVE => {
            // SAFETY: The caller's contract.
            let x = unsafe { read_value(services, ptr, conv) };
            match file_type(conv) {
                SLE_FILE_I8 => {
                    check((-128..=127).contains(&x));
                    write_byte(x as u8);
                }
                SLE_FILE_U8 => {
                    check((0..=255).contains(&x));
                    write_byte(x as u8);
                }
                SLE_FILE_I16 => {
                    check((-32768..=32767).contains(&x));
                    write_uint16(x as u16);
                }
                SLE_FILE_STRINGID | SLE_FILE_U16 => {
                    check((0..=65535).contains(&x));
                    write_uint16(x as u16);
                }
                SLE_FILE_I32 | SLE_FILE_U32 => write_uint32(x as u32),
                SLE_FILE_I64 | SLE_FILE_U64 => write_uint64(x as u64),
                _ => not_reached(services),
            }
        }
        SLA_LOAD_CHECK | SLA_LOAD => {
            let x = match file_type(conv) {
                SLE_FILE_I8 => i64::from(read_byte(services)? as i8),
                SLE_FILE_U8 => i64::from(read_byte(services)?),
                SLE_FILE_I16 => i64::from(read_uint16(services)? as i16),
                SLE_FILE_U16 => i64::from(read_uint16(services)?),
                SLE_FILE_I32 => i64::from(read_uint32(services)? as i32),
                SLE_FILE_U32 => i64::from(read_uint32(services)?),
                SLE_FILE_I64 | SLE_FILE_U64 => read_uint64(services)? as i64,
                SLE_FILE_STRINGID => i64::from((services.remap_old_string)(u32::from(
                    read_uint16(services)?,
                ))),
                _ => not_reached(services),
            };
            // SAFETY: The caller's contract.
            unsafe { write_value(services, ptr, conv, x) };
        }
        SLA_PTRS | SLA_NULL => {}
        _ => not_reached(services),
    }
    Ok(())
}

/// `SlCalcConvMemLen`; string types read their length from the savegame.
fn calc_conv_mem_len(
    services: &OpenTTDSaveLoadServices,
    conv: u32,
) -> Result<u32, OpenTTDRustSaveLoadStatus> {
    Ok(match mem_type(conv) {
        SLE_VAR_BL | SLE_VAR_I8 | SLE_VAR_U8 => 1,
        SLE_VAR_I16 | SLE_VAR_U16 => 2,
        SLE_VAR_I32 | SLE_VAR_U32 => 4,
        SLE_VAR_I64 | SLE_VAR_U64 => 8,
        SLE_VAR_NULL => 0,
        SLE_VAR_STR | SLE_VAR_STRQ => read_simple_gamma(services)?,
        _ => not_reached(services),
    })
}

/// `SlCalcConvFileLen`; `SLE_FILE_STRING` reads its length, narrowed to `uint8_t`.
fn calc_conv_file_len(
    services: &OpenTTDSaveLoadServices,
    conv: u32,
) -> Result<u8, OpenTTDRustSaveLoadStatus> {
    Ok(match file_type(conv) {
        SLE_FILE_END => 0,
        SLE_FILE_I8 | SLE_FILE_U8 => 1,
        SLE_FILE_I16 | SLE_FILE_U16 | SLE_FILE_STRINGID => 2,
        SLE_FILE_I32 | SLE_FILE_U32 => 4,
        SLE_FILE_I64 | SLE_FILE_U64 => 8,
        SLE_FILE_STRING => read_simple_gamma(services)? as u8,
        _ => not_reached(services),
    })
}

/// `SlCopyInternal`.
///
/// # Safety
/// `object` holds `length` values of the memory type of `conv`.
unsafe fn copy_internal(
    services: &OpenTTDSaveLoadServices,
    object: *mut c_void,
    length: usize,
    conv: u32,
) -> Status {
    let action = shared(|s| s.action);
    if mem_type(conv) == SLE_VAR_NULL {
        (services.native_assert)(action != SLA_SAVE);
        let size = length.wrapping_mul(usize::from(calc_conv_file_len(services, conv)?));
        return skip_bytes(services, size);
    }

    // Really old versions saved everything as bytes.
    if action != SLA_SAVE && version() == 0 {
        if matches!(
            conv,
            SLE_INT16 | SLE_UINT16 | SLE_STRINGID | SLE_INT32 | SLE_UINT32
        ) {
            let size = length.wrapping_mul(usize::from(calc_conv_file_len(services, conv)?));
            return copy_bytes(services, object.cast(), size);
        }
        // Money from 32 to 64 bits.
        if conv == (SLE_FILE_I32 | SLE_VAR_I64) {
            for i in 0..length {
                let x = i64::from(read_uint32(services)?.swap_bytes() as i32);
                // SAFETY: The caller's contract.
                unsafe { object.cast::<i64>().add(i).write_unaligned(x) };
            }
            return Ok(());
        }
    }

    if conv == SLE_INT8 || conv == SLE_UINT8 {
        copy_bytes(services, object.cast(), length)
    } else {
        let mut a = object.cast::<u8>();
        let mem_size = calc_conv_mem_len(services, conv)? as u8;
        for _ in 0..length {
            // SAFETY: The caller's contract.
            unsafe {
                save_load_conv(services, a.cast(), conv)?;
                a = a.add(usize::from(mem_size));
            }
        }
        Ok(())
    }
}

/// `SlCopy`.
///
/// # Safety
/// As [`copy_internal`].
unsafe fn copy(
    services: &OpenTTDSaveLoadServices,
    object: *mut c_void,
    length: usize,
    conv: u32,
) -> Status {
    if matches!(shared(|s| s.action), SLA_PTRS | SLA_NULL) {
        return Ok(());
    }

    if shared(|s| s.need_length) != NL_NONE {
        let size = length.wrapping_mul(usize::from(calc_conv_file_len(services, conv)?));
        set_length(services, size);
        if shared(|s| s.need_length) == NL_CALCLENGTH {
            return Ok(());
        }
    }

    // SAFETY: The caller's contract.
    unsafe { copy_internal(services, object, length, conv) }
}

/// `SlCalcArrayLen`.
fn calc_array_len(
    services: &OpenTTDSaveLoadServices,
    length: usize,
    conv: u32,
) -> Result<usize, OpenTTDRustSaveLoadStatus> {
    let file = usize::from(calc_conv_file_len(services, conv)?);
    Ok(file
        .wrapping_mul(length)
        .wrapping_add(gamma_length(length) as usize))
}

/// `SlArray`.
///
/// # Safety
/// As [`copy_internal`].
unsafe fn array(
    services: &OpenTTDSaveLoadServices,
    object: *mut c_void,
    mut length: usize,
    conv: u32,
) -> Status {
    match shared(|s| s.action) {
        SLA_SAVE => {
            write_simple_gamma(services, length);
            // SAFETY: The caller's contract.
            unsafe { copy_internal(services, object, length, conv) }
        }
        SLA_LOAD_CHECK | SLA_LOAD => {
            if !version_before(SLV_SAVELOAD_LIST_LENGTH) {
                let sv_length = read_simple_gamma(services)? as usize;
                if mem_type(conv) == SLE_VAR_NULL {
                    // An unknown field; trust the savegame's length.
                    length = sv_length;
                } else if sv_length != length {
                    return Err(corrupt("Fixed-length array is of wrong length".to_owned()));
                }
            }
            // SAFETY: The caller's contract.
            unsafe { copy_internal(services, object, length, conv) }
        }
        SLA_PTRS | SLA_NULL => Ok(()),
        _ => not_reached(services),
    }
}

/// `ReferenceToInt`: index + 1, or 0 for null.
fn reference_to_int(services: &OpenTTDSaveLoadServices, obj: *const c_void, rt: u32) -> usize {
    (services.native_assert)(shared(|s| s.action) == SLA_SAVE);

    if obj.is_null() {
        return 0;
    }

    // Old vehicles are saved as new ones.
    let index = match rt {
        REF_VEHICLE_OLD | REF_VEHICLE => services.vehicle_index,
        REF_STATION => services.station_index,
        REF_TOWN => services.town_index,
        REF_ROADSTOPS => services.road_stop_index,
        REF_ENGINE_RENEWS => services.engine_renew_index,
        REF_CARGO_PACKET => services.cargo_packet_index,
        REF_ORDERLIST => services.order_list_index,
        REF_STORAGE => services.persistent_storage_index,
        REF_LINK_GRAPH => services.link_graph_index,
        REF_LINK_GRAPH_JOB => services.link_graph_job_index,
        _ => not_reached(services),
    };
    index(obj) as usize + 1
}

/// `IntToReference`: index 0 is null, other indices are one more than the pool index.
fn int_to_reference(
    services: &OpenTTDSaveLoadServices,
    mut index: usize,
    mut rt: u32,
) -> Result<*mut c_void, OpenTTDRustSaveLoadStatus> {
    (services.native_assert)(shared(|s| s.action) == SLA_PTRS);

    // From 4.4, REF_VEHICLE_OLD is saved as REF_VEHICLE.
    if rt == REF_VEHICLE_OLD && !version_before_minor(SLV_4, 4) {
        rt = REF_VEHICLE;
    }

    if index == if rt == REF_VEHICLE_OLD { 0xFFFF } else { 0 } {
        return Ok(std::ptr::null_mut());
    }

    // Old vehicles were saved differently: invalid was 0xFFFF.
    if rt != REF_VEHICLE_OLD {
        index = index.wrapping_sub(1);
    }

    let (get, kind) = match rt {
        REF_ORDERLIST => (services.order_list_get, "OrderList"),
        REF_VEHICLE_OLD | REF_VEHICLE => (services.vehicle_get, "Vehicle"),
        REF_STATION => (services.station_get, "Station"),
        REF_TOWN => (services.town_get, "Town"),
        REF_ROADSTOPS => (services.road_stop_get, "RoadStop"),
        REF_ENGINE_RENEWS => (services.engine_renew_get, "EngineRenew"),
        REF_CARGO_PACKET => (services.cargo_packet_get, "CargoPacket"),
        REF_STORAGE => (services.persistent_storage_get, "PersistentStorage"),
        REF_LINK_GRAPH => (services.link_graph_get, "LinkGraph"),
        REF_LINK_GRAPH_JOB => (services.link_graph_job_get, "LinkGraphJob"),
        _ => not_reached(services),
    };
    let obj = get(index);
    if obj.is_null() {
        return Err(corrupt(format!("Referencing invalid {kind}")));
    }
    Ok(obj)
}

/// `SlSaveLoadRef`.
///
/// # Safety
/// `ptr` points to a pointer field that holds an index between load and pointer fixing.
unsafe fn save_load_ref(services: &OpenTTDSaveLoadServices, ptr: *mut c_void, conv: u32) -> Status {
    let ptr = ptr.cast::<*mut c_void>();
    // `(SLRefType)conv` narrows to the enum's byte.
    let rt = conv & 0xFF;
    match shared(|s| s.action) {
        SLA_SAVE => {
            // SAFETY: The caller's contract.
            let obj = unsafe { ptr.read_unaligned() };
            write_uint32(reference_to_int(services, obj, rt) as u32);
        }
        SLA_LOAD_CHECK | SLA_LOAD => {
            let index = if version_before(SLV_69) {
                usize::from(read_uint16(services)?)
            } else {
                read_uint32(services)? as usize
            };
            // SAFETY: The caller's contract; size_t and pointers have the same size.
            unsafe { ptr.cast::<usize>().write_unaligned(index) };
        }
        SLA_PTRS => {
            // SAFETY: The caller's contract.
            let index = unsafe { ptr.cast::<usize>().read_unaligned() };
            let obj = int_to_reference(services, index, rt)?;
            // SAFETY: The caller's contract.
            unsafe { ptr.write_unaligned(obj) };
        }
        // SAFETY: The caller's contract.
        SLA_NULL => unsafe { ptr.write_unaligned(std::ptr::null_mut()) },
        _ => not_reached(services),
    }
    Ok(())
}

/// `SlCalcStdStringLen`: the string's length plus its gamma length.
fn calc_std_string_len(services: &OpenTTDSaveLoadServices, ptr: *const c_void) -> usize {
    let mut len = 0;
    (services.string_data)(ptr, &raw mut len);
    len + gamma_length(len) as usize
}

/// `SlReadString`.
fn read_string(services: &OpenTTDSaveLoadServices, ptr: *mut c_void, length: usize) -> Status {
    let data = (services.string_resize)(ptr, length);
    copy_bytes(services, data, length)
}

/// `FixSCCEncoded` and `FixSCCEncodedNegative` on the `std::string` at `ptr`: the string is
/// replaced, with the native diagnostic logs and assertions, only when the fix applies.
fn fix_encoded(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut c_void,
    fix: impl FnOnce(&[u8]) -> crate::encoded::Output,
) {
    let mut len = 0;
    let data = (services.string_data)(ptr, &raw mut len);
    // SAFETY: `string_data` returns the string's live bytes, unchanged until `apply_encoded`.
    let output = fix(unsafe { crate::utf8::borrow(data, len) });
    let view = output.view();
    if view.apply != 0 {
        (services.apply_encoded)(ptr, &raw const view, output.diagnostics().as_ptr());
    }
}

/// `StrMakeValidInPlace(std::string &)`: compact the existing allocation in place.
fn make_valid(services: &OpenTTDSaveLoadServices, ptr: *mut c_void, conv: u32) {
    let mut length = 0;
    let data = (services.string_data)(ptr, &raw mut length).cast_mut();
    let settings = 1
        | (u8::from(conv & SLF_ALLOW_NEWLINE != 0) << 1)
        | (u8::from(conv & SLF_ALLOW_CONTROL != 0) << 2)
        | (u8::from(conv & SLF_REPLACE_TABCRLF != 0) << 3);
    let mut consumed = 0;
    let mut written = 0;
    while consumed < length {
        // SAFETY: Each decision borrows the unread suffix only until the in-place write.
        let step = crate::string_validation::step(
            unsafe { crate::utf8::borrow(data.add(consumed), length - consumed) },
            settings,
        );
        consumed += step.consumed;
        if step.stopped != 0 {
            break;
        }
        // SAFETY: Validation emits no more bytes than it consumes, as required by
        // `InPlaceReplacement`; the output array is independent of the string.
        unsafe {
            std::ptr::copy_nonoverlapping(
                step.output.bytes.as_ptr(),
                data.add(written),
                step.output.length,
            );
        }
        written += step.output.length;
    }
    // Every string borrow has ended before the native resize.
    (services.string_resize)(ptr, written);
}

/// `SlStdString`.
fn std_string(services: &OpenTTDSaveLoadServices, ptr: *mut c_void, conv: u32) -> Status {
    match shared(|s| s.action) {
        SLA_SAVE => {
            let mut len = 0;
            let data = (services.string_data)(ptr, &raw mut len);
            write_simple_gamma(services, len);
            copy_bytes(services, data.cast_mut(), len)?;
        }
        SLA_LOAD_CHECK | SLA_LOAD => {
            let len = read_simple_gamma(services)? as usize;
            if mem_type(conv) == SLE_VAR_NULL {
                return skip_bytes(services, len);
            }

            read_string(services, ptr, len)?;

            let control = conv & SLF_ALLOW_CONTROL != 0;
            if control {
                if version_before(SLV_ENCODED_STRING_FORMAT) {
                    let fix_code = version_before(SLV_169);
                    fix_encoded(services, ptr, |src| crate::encoded::legacy(src, fix_code));
                }
                if version_before(SLV_FIX_SCC_ENCODED_NEGATIVE) {
                    fix_encoded(services, ptr, crate::encoded::negatives);
                }
            }
            make_valid(services, ptr, conv);
        }
        SLA_PTRS | SLA_NULL => {}
        _ => not_reached(services),
    }
    Ok(())
}

/// `SlSetStructListLength`.
fn set_struct_list_length(services: &OpenTTDSaveLoadServices, length: usize) {
    if shared(|s| s.need_length) != NL_NONE {
        set_length(services, gamma_length(length) as usize);
        if shared(|s| s.need_length) == NL_CALCLENGTH {
            return;
        }
    }
    write_simple_gamma(services, length);
}

/// `SlGetStructListLength`.
fn get_struct_list_length(
    services: &OpenTTDSaveLoadServices,
    limit: usize,
) -> Result<usize, OpenTTDRustSaveLoadStatus> {
    let length = read_simple_gamma(services)? as usize;
    if length > limit {
        return Err(corrupt("List exceeds storage size".to_owned()));
    }
    Ok(length)
}

/// `SlAutolength`.
fn autolength(services: &OpenTTDSaveLoadServices, proc_: *const c_void, arg: i32) -> Status {
    (services.native_assert)(shared(|s| s.action) == SLA_SAVE);

    // Calculate the length.
    shared(|s| {
        s.need_length = NL_CALCLENGTH;
        s.obj_len = 0;
    });
    if !(services.autolength)(proc_, arg) {
        return Err(SL_STATUS_NATIVE);
    }

    // Write the length, then the data.
    let obj_len = shared(|s| {
        s.need_length = NL_WANTLENGTH;
        s.obj_len
    });
    set_length(services, obj_len);

    let start_pos = dumper_size();
    let expected_offs = start_pos.wrapping_add(shared(|s| s.obj_len));

    if !(services.autolength)(proc_, arg) {
        return Err(SL_STATUS_NATIVE);
    }

    if expected_offs != dumper_size() {
        return Err(corrupt(format!(
            "Invalid chunk size when writing autolength block, expected {}, got {}",
            shared(|s| s.obj_len),
            dumper_size().wrapping_sub(start_pos)
        )));
    }
    Ok(())
}

fn status(result: Status) -> OpenTTDRustSaveLoadStatus {
    result.err().unwrap_or(SL_STATUS_OK)
}

/// Address of the shared codec state; it lives for the whole process.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_codec() -> *mut OpenTTDRustSaveLoadCodec {
    // SAFETY: Only the address of the static's field is taken; no borrow is created.
    unsafe { &raw mut (*CODEC.get()).shared }
}

/// Message of the last corruption the codec detected, valid until the next one.
///
/// # Safety
/// `length` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_error_message(length: *mut usize) -> *const u8 {
    codec(|c| {
        // SAFETY: The caller's contract.
        unsafe { *length = c.error.len() };
        c.error.as_ptr()
    })
}

/// `std::make_unique<ReadBuffer>(filter)`; native code keeps `filter` alive until
/// [`openttd_rust_saveload_reader_close`].
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_reader_open(filter: *mut c_void) {
    codec(|c| {
        c.read_buffer = Some(vec![0u8; MEMORY_CHUNK_SIZE].into_boxed_slice());
        c.read_filter = filter;
        c.read = 0;
        c.shared.read_cursor = std::ptr::null_mut();
        c.shared.read_end = std::ptr::null_mut();
    });
}

/// `_sl.reader = nullptr`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_reader_close() {
    codec(|c| {
        c.read_buffer = None;
        c.read_filter = std::ptr::null_mut();
        c.read = 0;
        c.shared.read_cursor = std::ptr::null_mut();
        c.shared.read_end = std::ptr::null_mut();
    });
}

/// `ReadBuffer::ReadByte`'s refill, for native `SlReadByte` at an empty buffer.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_fill(
    services: &OpenTTDSaveLoadServices,
) -> OpenTTDRustSaveLoadStatus {
    status(fill(services))
}

/// `ReadBuffer::GetSize`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_read_size() -> usize {
    read_size()
}

/// `std::make_unique<MemoryDumper>()`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_dumper_open() {
    openttd_rust_saveload_dumper_close();
}

/// `_sl.dumper = nullptr`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_dumper_close() {
    codec(|c| {
        c.blocks = Vec::new();
        c.shared.write_cursor = std::ptr::null_mut();
        c.shared.write_end = std::ptr::null_mut();
    });
}

/// `MemoryDumper::WriteByte`'s new block, for native `SlWriteByte` at a full block.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_grow() {
    grow();
}

/// `MemoryDumper::Flush` into the native `SaveFilter` `writer`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_flush(
    services: &OpenTTDSaveLoadServices,
    writer: *mut c_void,
) -> OpenTTDRustSaveLoadStatus {
    status(flush(services, writer))
}

/// `SlReadSimpleGamma`, also `SlReadSparseIndex` and `SlReadArrayLength`.
///
/// # Safety
/// `value` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_read_gamma(
    services: &OpenTTDSaveLoadServices,
    value: *mut u32,
) -> OpenTTDRustSaveLoadStatus {
    status(read_simple_gamma(services).map(|v| {
        // SAFETY: The caller's contract.
        unsafe { *value = v }
    }))
}

/// `SlWriteSimpleGamma`, also `SlWriteSparseIndex` and `SlWriteArrayLength`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_write_gamma(services: &OpenTTDSaveLoadServices, i: usize) {
    write_simple_gamma(services, i);
}

/// `SlGetGammaLength`, also `SlGetArrayLength`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_gamma_length(i: usize) -> u32 {
    gamma_length(i)
}

/// `SlIterateArray`.
///
/// # Safety
/// `index` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_iterate_array(
    services: &OpenTTDSaveLoadServices,
    index: *mut i32,
) -> OpenTTDRustSaveLoadStatus {
    status(iterate_array(services).map(|v| {
        // SAFETY: The caller's contract.
        unsafe { *index = v }
    }))
}

/// `SlSkipArray`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_skip_array(
    services: &OpenTTDSaveLoadServices,
) -> OpenTTDRustSaveLoadStatus {
    status(skip_array(services))
}

/// `SlSetLength`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_set_length(
    services: &OpenTTDSaveLoadServices,
    length: usize,
) {
    set_length(services, length);
}

/// `SlReadUint16`.
///
/// # Safety
/// `value` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_read_uint16(
    services: &OpenTTDSaveLoadServices,
    value: *mut u16,
) -> OpenTTDRustSaveLoadStatus {
    status(read_uint16(services).map(|v| {
        // SAFETY: The caller's contract.
        unsafe { *value = v }
    }))
}

/// `SlReadUint32`.
///
/// # Safety
/// `value` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_read_uint32(
    services: &OpenTTDSaveLoadServices,
    value: *mut u32,
) -> OpenTTDRustSaveLoadStatus {
    status(read_uint32(services).map(|v| {
        // SAFETY: The caller's contract.
        unsafe { *value = v }
    }))
}

/// `SlWriteUint32`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_write_uint32(v: u32) {
    write_uint32(v);
}

/// `ReadValue`.
///
/// # Safety
/// `ptr` points to a value of the memory type of `conv`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_read_value(
    services: &OpenTTDSaveLoadServices,
    ptr: *const c_void,
    conv: u32,
) -> i64 {
    // SAFETY: The caller's contract.
    unsafe { read_value(services, ptr, conv) }
}

/// `WriteValue`.
///
/// # Safety
/// `ptr` points to a value of the memory type of `conv`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_write_value(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut c_void,
    conv: u32,
    val: i64,
) {
    // SAFETY: The caller's contract.
    unsafe { write_value(services, ptr, conv, val) }
}

/// `SlSaveLoadConv`.
///
/// # Safety
/// `ptr` points to a value of the memory type of `conv`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_conv(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut c_void,
    conv: u32,
) -> OpenTTDRustSaveLoadStatus {
    // SAFETY: The caller's contract.
    status(unsafe { save_load_conv(services, ptr, conv) })
}

/// `SlCalcConvFileLen`.
///
/// # Safety
/// `length` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_conv_file_len(
    services: &OpenTTDSaveLoadServices,
    conv: u32,
    length: *mut u8,
) -> OpenTTDRustSaveLoadStatus {
    status(calc_conv_file_len(services, conv).map(|v| {
        // SAFETY: The caller's contract.
        unsafe { *length = v }
    }))
}

/// `SlCopy`.
///
/// # Safety
/// `object` holds `length` values of the memory type of `conv`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_copy(
    services: &OpenTTDSaveLoadServices,
    object: *mut c_void,
    length: usize,
    conv: u32,
) -> OpenTTDRustSaveLoadStatus {
    // SAFETY: The caller's contract.
    status(unsafe { copy(services, object, length, conv) })
}

/// `SlArray`.
///
/// # Safety
/// `object` holds `length` values of the memory type of `conv`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_array(
    services: &OpenTTDSaveLoadServices,
    object: *mut c_void,
    length: usize,
    conv: u32,
) -> OpenTTDRustSaveLoadStatus {
    // SAFETY: The caller's contract.
    status(unsafe { array(services, object, length, conv) })
}

/// `SlCalcArrayLen`.
///
/// # Safety
/// `length` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_array_len(
    services: &OpenTTDSaveLoadServices,
    elements: usize,
    conv: u32,
    length: *mut usize,
) -> OpenTTDRustSaveLoadStatus {
    status(calc_array_len(services, elements, conv).map(|v| {
        // SAFETY: The caller's contract.
        unsafe { *length = v }
    }))
}

/// `SlSaveLoadRef`.
///
/// # Safety
/// `ptr` points to a reference field.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_ref(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut c_void,
    conv: u32,
) -> OpenTTDRustSaveLoadStatus {
    // SAFETY: The caller's contract.
    status(unsafe { save_load_ref(services, ptr, conv) })
}

/// `SlCalcRefLen`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_ref_len() -> usize {
    if version_before(SLV_69) { 2 } else { 4 }
}

/// `SlStdString`.
///
/// # Safety
/// `ptr` points to a `std::string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_std_string(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut c_void,
    conv: u32,
) -> OpenTTDRustSaveLoadStatus {
    status(std_string(services, ptr, conv))
}

/// `SlCalcStdStringLen`.
///
/// # Safety
/// `ptr` points to a `std::string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_std_string_len(
    services: &OpenTTDSaveLoadServices,
    ptr: *const c_void,
) -> usize {
    calc_std_string_len(services, ptr)
}

/// `SlReadString`.
///
/// # Safety
/// `ptr` points to a `std::string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_read_string(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut c_void,
    length: usize,
) -> OpenTTDRustSaveLoadStatus {
    status(read_string(services, ptr, length))
}

/// `SlSetStructListLength`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_set_struct_list_length(
    services: &OpenTTDSaveLoadServices,
    length: usize,
) {
    set_struct_list_length(services, length);
}

/// `SlGetStructListLength`.
///
/// # Safety
/// `length` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_get_struct_list_length(
    services: &OpenTTDSaveLoadServices,
    limit: usize,
    length: *mut usize,
) -> OpenTTDRustSaveLoadStatus {
    status(get_struct_list_length(services, limit).map(|v| {
        // SAFETY: The caller's contract.
        unsafe { *length = v }
    }))
}

/// `SlSetArrayIndex`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_set_array_index(index: u32) {
    shared(|s| {
        s.need_length = NL_WANTLENGTH;
        s.array_index = index as i32;
    });
}

/// `SlAutolength`; `proc_` is an `AutolengthProc *` that `services.autolength` calls.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_autolength(
    services: &OpenTTDSaveLoadServices,
    proc_: *const c_void,
    arg: i32,
) -> OpenTTDRustSaveLoadStatus {
    status(autolength(services, proc_, arg))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustSaveLoadCodec" ["probe"] => OpenTTDRustSaveLoadCodec {
        read_cursor,
        read_end,
        write_cursor,
        write_end,
        obj_len,
        next_offs,
        array_index,
        last_array_index,
        version,
        minor_version,
        action,
        need_length,
        block_mode,
        expect_table_header,
    };
    "OpenTTDSaveLoadServices" ["probe"] => OpenTTDSaveLoadServices {
        shared,
        native_assert,
        debug_iterate_error,
        read,
        write,
        finish,
        remap_old_string,
        copy_from_old_name,
        cargo_packet_get,
        cargo_packet_index,
        engine_renew_get,
        engine_renew_index,
        link_graph_get,
        link_graph_index,
        link_graph_job_get,
        link_graph_job_index,
        order_list_get,
        order_list_index,
        persistent_storage_get,
        persistent_storage_index,
        road_stop_get,
        road_stop_index,
        station_get,
        station_index,
        town_get,
        town_index,
        vehicle_get,
        vehicle_index,
        string_data,
        string_resize,
        apply_encoded,
        autolength,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn assert_ok(ok: bool) {
        assert!(ok, "native assertion");
    }
    extern "C" fn unused() {
        unreachable!("unused service");
    }
    extern "C" fn unused_read(_: *mut c_void, _: *mut u8, _: usize, _: *mut usize) -> bool {
        unreachable!("the test buffer holds every byte");
    }
    extern "C" fn unused_write(_: *mut c_void, _: *const u8, _: usize) -> bool {
        unreachable!("unused service")
    }
    extern "C" fn unused_filter(_: *mut c_void) -> bool {
        unreachable!("unused service")
    }
    extern "C" fn unused_u32(_: u32) -> u32 {
        unreachable!("unused service")
    }
    extern "C" fn unused_ptr_u32(_: *mut c_void, _: u32) {
        unreachable!("unused service")
    }
    extern "C" fn unused_get(_: usize) -> *mut c_void {
        unreachable!("unused service")
    }
    extern "C" fn unused_index(_: *const c_void) -> u32 {
        unreachable!("unused service")
    }
    extern "C" fn unused_data(_: *const c_void, _: *mut usize) -> *const u8 {
        unreachable!("unused service")
    }
    extern "C" fn unused_resize(_: *mut c_void, _: usize) -> *mut u8 {
        unreachable!("unused service")
    }
    extern "C" fn unused_apply(
        _: *mut c_void,
        _: *const crate::encoded::View,
        _: *const crate::encoded::Diagnostic,
    ) {
        unreachable!("unused service")
    }
    extern "C" fn unused_autolength(_: *const c_void, _: i32) -> bool {
        unreachable!("unused service")
    }

    const SERVICES: OpenTTDSaveLoadServices = OpenTTDSaveLoadServices {
        shared: std::ptr::null(),
        native_assert: assert_ok,
        debug_iterate_error: unused,
        read: unused_read,
        write: unused_write,
        finish: unused_filter,
        remap_old_string: unused_u32,
        copy_from_old_name: unused_ptr_u32,
        cargo_packet_get: unused_get,
        cargo_packet_index: unused_index,
        engine_renew_get: unused_get,
        engine_renew_index: unused_index,
        link_graph_get: unused_get,
        link_graph_index: unused_index,
        link_graph_job_get: unused_get,
        link_graph_job_index: unused_index,
        order_list_get: unused_get,
        order_list_index: unused_index,
        persistent_storage_get: unused_get,
        persistent_storage_index: unused_index,
        road_stop_get: unused_get,
        road_stop_index: unused_index,
        station_get: unused_get,
        station_index: unused_index,
        town_get: unused_get,
        town_index: unused_index,
        vehicle_get: unused_get,
        vehicle_index: unused_index,
        string_data: unused_data,
        string_resize: unused_resize,
        apply_encoded: unused_apply,
        autolength: unused_autolength,
    };

    /// Decode `bytes` from an in-place read buffer; returns the value and the bytes read.
    fn read_gamma(bytes: &mut [u8]) -> (Result<u32, OpenTTDRustSaveLoadStatus>, usize) {
        let start = bytes.as_mut_ptr();
        shared(|s| {
            s.read_cursor = start;
            // SAFETY: One past the end of `bytes`.
            s.read_end = unsafe { start.add(bytes.len()) };
        });
        let value = read_simple_gamma(&SERVICES);
        let used = shared(|s| s.read_cursor as usize - start as usize);
        shared(|s| {
            (s.read_cursor, s.read_end) = (std::ptr::null_mut(), std::ptr::null_mut());
        });
        (value, used)
    }

    /// The gamma encoding of `saveload.cpp`, including the 4- and 5-byte forms that need a
    /// length of 2 MiB or more and that no regression save reaches.
    #[test]
    fn gamma_follows_the_documented_encoding() {
        let table: &[(usize, &[u8])] = &[
            (0, &[0x00]),
            (0x7F, &[0x7F]),
            (0x80, &[0x80, 0x80]),
            (0x3FFF, &[0xBF, 0xFF]),
            (0x4000, &[0xC0, 0x40, 0x00]),
            (0x1F_FFFF, &[0xDF, 0xFF, 0xFF]),
            (0x20_0000, &[0xE0, 0x20, 0x00, 0x00]),
            (0x0FFF_FFFF, &[0xEF, 0xFF, 0xFF, 0xFF]),
            (0x1000_0000, &[0xF0, 0x10, 0x00, 0x00, 0x00]),
            (0x1234_5678, &[0xF0, 0x12, 0x34, 0x56, 0x78]),
            (0xFFFF_FFFF, &[0xF0, 0xFF, 0xFF, 0xFF, 0xFF]),
        ];
        for &(value, bytes) in table {
            assert_eq!(
                gamma_length(value) as usize,
                bytes.len(),
                "length of {value:#x}"
            );

            let mut out = [0u8; 8];
            let start = out.as_mut_ptr();
            shared(|s| {
                s.write_cursor = start;
                // SAFETY: One past the end of `out`.
                s.write_end = unsafe { start.add(out.len()) };
            });
            write_simple_gamma(&SERVICES, value);
            let written = shared(|s| s.write_cursor as usize - start as usize);
            shared(|s| {
                (s.write_cursor, s.write_end) = (std::ptr::null_mut(), std::ptr::null_mut());
            });
            assert_eq!(&out[..written], bytes, "encoding of {value:#x}");

            let mut input = bytes.to_vec();
            let (decoded, used) = read_gamma(&mut input);
            assert_eq!(decoded.ok(), Some(value as u32), "decoding of {value:#x}");
            assert_eq!(used, bytes.len(), "bytes read for {value:#x}");
        }

        // The reader accepts non-minimal forms and ignores the 5-byte prefix's low bits.
        for (mut bytes, value) in [
            (vec![0x80, 0x05], 5),
            (vec![0xE0, 0x00, 0x00, 0x7F], 0x7F),
            (vec![0xF7, 0x12, 0x34, 0x56, 0x78], 0x1234_5678),
        ] {
            let len = bytes.len();
            assert_eq!(read_gamma(&mut bytes), (Ok(value), len));
        }

        // A sixth form is corrupt after reading only its prefix.
        for prefix in [0xF8, 0xFF] {
            let (decoded, used) = read_gamma(&mut [prefix, 0, 0, 0, 0]);
            assert_eq!((decoded, used), (Err(SL_STATUS_CORRUPT), 1));
            assert_eq!(codec(|c| std::mem::take(&mut c.error)), "Unsupported gamma");
        }
    }
}
