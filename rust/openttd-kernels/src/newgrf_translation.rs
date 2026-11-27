/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `LoadTranslationTable` (`newgrf/newgrf_act0_globalvar.cpp`) for the cargo, rail
//! type, road type and tram type tables. Each table is a `std::vector` member of a
//! C++ `GRFFile`, which the table's leaves reach through its address; no call that
//! could move a vector lies between a lookup and its last use. A failed read returns
//! to the native caller, which rethrows it in its own exception frame, with the labels
//! read before it already in the table.

use crate::debug::{Bytes, grf_msg};
use crate::newgrf::{GRFFile, GRFFileRef};
use crate::newgrf_bytereader::{
    ByteReaderRef, NativeByteReader, OTTDByteReaderSignal, Read, Services as ReaderServices,
};
use crate::newgrf_internal::ChangeInfoResult;
use std::ffi::c_char;

/// A `std::vector<T>` translation table of a `GRFFile`.
#[repr(C)]
pub struct TranslationTable {
    _opaque: [u8; 0],
}

/// The leaves of one translation table, which C++ fills for each table type.
#[repr(C)]
pub struct Services {
    /// The byte reader's operations.
    reader: *const ReaderServices,
    shared: *const crate::services::Services,
    /// `gettable(grf)`: the table of a file.
    gettable: extern "C" fn(*mut GRFFile) -> *mut TranslationTable,
    /// `translation_table.clear()`.
    clear: extern "C" fn(*mut TranslationTable),
    /// `translation_table.reserve(last)`.
    reserve: extern "C" fn(*mut TranslationTable, u32),
    /// `translation_table.push_back(T(label))`.
    push_back: extern "C" fn(*mut TranslationTable, u32),
    /// `override_table = translation_table`.
    assign: extern "C" fn(*mut TranslationTable, *const TranslationTable),
    /// `GetCurrentGRFOverride()`.
    get_current_grf_override: extern "C" fn() -> *mut GRFFile,
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTranslationServices" ["engine"] => Services { reader, shared, gettable, clear, reserve, push_back, assign, get_current_grf_override };
}

/// Load a cargo- or railtype-translation table.
/// @param first ID of the first translation table entry.
/// @param last ID of the last translation table entry.
/// @param buf The property value.
/// @param name Name of the table for debug output.
/// @return ChangeInfoResult.
///
/// `file` is `_cur_gps.grffile`, and `s` holds `gettable`, the function to get storage
/// for the translation table.
fn load_translation_table(
    s: &Services,
    buf: &mut ByteReaderRef<'_>,
    file: GRFFileRef<'_>,
    first: u32,
    last: u32,
    name: &[u8],
) -> Result<ChangeInfoResult, OTTDByteReaderSignal> {
    if first != 0 {
        grf_msg!(
            1,
            "LoadTranslationTable: {} translation table must start at zero",
            Bytes(name)
        );
        return Ok(ChangeInfoResult::CIR_INVALID_ID);
    }

    let translation_table = (s.gettable)(file.as_ptr().cast_mut());
    (s.clear)(translation_table);
    (s.reserve)(translation_table, last);
    for _id in first..last {
        (s.push_back)(translation_table, buf.read_dword()?.swap_bytes());
    }

    let grf_override = s.get_current_grf_override();
    if let Some(grf_override) = grf_override {
        /* GRF override is present, copy the translation table to the overridden GRF as well. */
        grf_msg!(
            1,
            "LoadTranslationTable: Copying {} translation table to override GRFID {:08X}",
            Bytes(name),
            grf_override.grfid().swap_bytes()
        );
        let override_table = (s.gettable)(grf_override.as_ptr().cast_mut());
        (s.assign)(override_table, translation_table);
    }

    Ok(ChangeInfoResult::CIR_SUCCESS)
}

#[allow(
    unsafe_code,
    reason = "the shared table and the override file the leaves return"
)]
impl Services {
    /// The shared table.
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: the translation table names the process-lifetime shared table.
        unsafe { &*self.shared }
    }

    /// `GetCurrentGRFOverride()`, or `None` without an override.
    fn get_current_grf_override(&self) -> Option<GRFFileRef<'_>> {
        // SAFETY: `GetCurrentGRFOverride` returns a live file or null.
        unsafe { GRFFileRef::new(self.shared(), (self.get_current_grf_override)()) }
    }
}

/// `LoadTranslationTable`, for the C++ Action 0 handler: the result, or a negative
/// value for a failed read, which the caller rethrows.
///
/// # Safety
/// `s` and its tables live for the process, `file` is the loader's live file, `native`
/// a live C++ reader borrowed for the call, and `name` holds `len` bytes.
#[allow(
    unsafe_code,
    reason = "export; the C++ handler's reader, file and name"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_load_translation(
    s: *const Services,
    file: *mut GRFFile,
    first: u32,
    last: u32,
    native: *mut NativeByteReader,
    name: *const c_char,
    len: usize,
) -> i32 {
    // SAFETY: the caller's contract.
    let (s, name) = unsafe { (&*s, std::slice::from_raw_parts(name.cast::<u8>(), len)) };
    // SAFETY: the caller's contract.
    let mut buf = unsafe { ByteReaderRef::new(native, &*s.reader) };
    // SAFETY: the caller's contract.
    let file = unsafe { GRFFileRef::of(s.shared(), file) };
    load_translation_table(s, &mut buf, file, first, last, name)
        .map_or(-1, |result| i32::from(result.0))
}
