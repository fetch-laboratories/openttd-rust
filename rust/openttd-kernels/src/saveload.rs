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
pub(crate) const SLA_SAVE: u8 = 1;
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
    pub action: u8,
    pub need_length: u8,
    pub block_mode: u8,
    pub expect_table_header: bool,
}

/// Native leaves the codec calls; each is a typed `noexcept` C++ function.
#[repr(C)]
pub struct OpenTTDSaveLoadServices {
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
    if !(services.read)(filter, buf, MEMORY_CHUNK_SIZE, &mut len) {
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

fn write_uint16(v: u16) {
    write_byte((v >> 8) as u8);
    write_byte(v as u8);
}

fn write_uint32(v: u32) {
    write_uint16((v >> 16) as u16);
    write_uint16(v as u16);
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
                    (services.native_assert)(i as u64 <= u64::from(u32::MAX));
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
                _ => unreachable!("NOT_REACHED in SlSetLength"),
            }
        }
        // `_sl.obj_len += (int)length`.
        NL_CALCLENGTH => {
            shared(|s| s.obj_len = s.obj_len.wrapping_add(length as i32 as isize as usize))
        }
        _ => unreachable!("NOT_REACHED in SlSetLength"),
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

/// `MemoryDumper::GetSize`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_saveload_dumper_size() -> usize {
    dumper_size()
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

/// `SlCopyBytes` when loading.
///
/// # Safety
/// `ptr` has `length` writable bytes outside the codec's buffers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_read_bytes(
    services: &OpenTTDSaveLoadServices,
    ptr: *mut u8,
    length: usize,
) -> OpenTTDRustSaveLoadStatus {
    status(read_bytes(services, ptr, length))
}

/// `SlCopyBytes` when saving.
///
/// # Safety
/// `ptr` has `length` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_saveload_write_bytes(ptr: *const u8, length: usize) {
    write_bytes(ptr, length);
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
        action,
        need_length,
        block_mode,
        expect_table_header,
    };
    "OpenTTDSaveLoadServices" ["probe"] => OpenTTDSaveLoadServices {
        native_assert,
        debug_iterate_error,
        read,
        write,
        finish,
    };
}
