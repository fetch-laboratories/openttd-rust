/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NewGRF` buffer reader (`newgrf/newgrf_bytereader.h`, `.cpp`), twice.
//!
//! [`ByteReader`] reads the pseudo-sprite buffer of an action Rust interprets.
//! [`ByteReaderRef`] reads through a C++ `ByteReader` ([`NativeByteReader`]), which an
//! unported handler builds from the bytes [`ByteReader::tail`] hands it and which the
//! Action 0 handlers pass back to the Rust property parsers. A failed read is
//! [`OTTDByteReaderSignal`]: Rust returns it to the native caller, which rethrows the
//! C++ signal in its own exception frame. The C++ view and `tail` leave when #669's
//! "`NewGRF` configuration/common publication/actions delivery" line ports those handlers.
//!
//! Both readers meet one contract, which their constructors take: the buffer, or the
//! native reader and the table of its operations, stay valid, readable and unchanged,
//! and the native reader exclusively borrowed, for the reader's lifetime.

use crate::consumer::{self, SeparatorUsage};

/// `OTTDByteReaderSignal`: a read past the end of the buffer.
#[derive(Clone, Copy, Debug)]
pub struct OTTDByteReaderSignal;

/// Class to read from a `NewGRF` file
pub struct ByteReader {
    data: *const u8,
    len: usize,
    pos: usize,
}
impl ByteReader {
    /// `ByteReader(data, len)`.
    ///
    /// # Safety
    /// `len` bytes at `data` stay readable and unchanged while the reader lives.
    #[allow(unsafe_code, reason = "the contract the reader's methods rely on")]
    pub(crate) const unsafe fn new(data: *const u8, len: usize) -> Self {
        Self { data, len, pos: 0 }
    }

    /// `StringConsumer`'s view of the bytes not yet read.
    #[allow(
        unsafe_code,
        reason = "a view of the buffer the constructor's contract keeps"
    )]
    fn left(&self) -> &[u8] {
        if self.pos == self.len {
            return &[];
        }
        // SAFETY: the constructor's contract keeps `len` bytes at `data` readable and
        // unchanged, and `pos < len`.
        unsafe { std::slice::from_raw_parts(self.data.add(self.pos), self.len - self.pos) }
    }

    /// `TryReadUint8`, `TryReadUint16LE` or `TryReadUint32LE`, by `width`: the
    /// position moves only on success.
    fn try_read_uint(&mut self, width: u8) -> Result<u64, OTTDByteReaderSignal> {
        let result = consumer::little_endian(self.left(), width);
        if result.length == 0 {
            return Err(OTTDByteReaderSignal);
        }
        self.pos += result.length;
        Ok(result.value_bits)
    }

    /// Read a NUL-terminated string.
    /// @returns String read from the buffer.
    pub(crate) fn read_string(&mut self) -> (*const u8, usize) {
        let left = self.left();
        let data = left.as_ptr();
        /* Terminating NUL may be missing at the end of sprite. */
        let result = consumer::until(left, &[0], SeparatorUsage::SKIP_ONE_SEPARATOR);
        self.pos += result.consumed_length;
        (data, result.result_length)
    }

    /// `GetBytesLeft`.
    pub(crate) const fn remaining(&self) -> usize {
        self.len - self.pos
    }

    /// The bytes not yet read, which an unported handler reads with its own C++
    /// reader. It has no original: the original's handlers take the reader itself.
    pub(crate) fn tail(&self) -> (*const u8, usize) {
        let left = self.left();
        (left.as_ptr(), left.len())
    }
}

/// C++ `ByteReader`, which unported handlers build and own.
#[repr(C)]
pub struct NativeByteReader {
    _opaque: [u8; 0],
}

/// C++ `std::vector<BadgeID>`, a spec's `badges`.
#[repr(C)]
pub struct BadgeList {
    _opaque: [u8; 0],
}

/// The operations of a C++ `ByteReader`, and `ReadBadgeList` and `SkipBadgeList`
/// (`newgrf/newgrf_internal.h`) over it. Each returns false where the C++ reader
/// throws, with the reader's position and partial effects left as they are.
#[repr(C)]
pub struct Services {
    /// `ByteReader::HasData`.
    pub has_data: unsafe extern "C" fn(*mut NativeByteReader, usize) -> bool,
    /// `ByteReader::PeekDWord`.
    pub peek_dword: unsafe extern "C" fn(*mut NativeByteReader, *mut u32) -> bool,
    /// `ByteReader::ReadBytes`.
    pub read_bytes: unsafe extern "C" fn(*mut NativeByteReader, usize, *mut *const u8) -> bool,
    /// `ByteReader::ReadByte`.
    pub read_byte: unsafe extern "C" fn(*mut NativeByteReader, *mut u8) -> bool,
    /// `ByteReader::ReadWord`.
    pub read_word: unsafe extern "C" fn(*mut NativeByteReader, *mut u16) -> bool,
    /// `ByteReader::ReadDWord`.
    pub read_dword: unsafe extern "C" fn(*mut NativeByteReader, *mut u32) -> bool,
    /// `ByteReader::Skip`.
    pub skip: unsafe extern "C" fn(*mut NativeByteReader, usize) -> bool,
    /// `spec->badges = ...`, the assignment of a list `ReadBadgeList` returns.
    pub assign_badges: unsafe extern "C" fn(*mut BadgeList, *const u32, usize),
    /// `ReadBadgeList` after its count: the IDs and their number.
    pub read_badge_ids:
        unsafe extern "C" fn(*mut NativeByteReader, u8, u16, *mut u32, *mut usize) -> bool,
    /// `SkipBadgeList`.
    pub skip_badges: unsafe extern "C" fn(*mut NativeByteReader) -> bool,
}

/// A view of a C++ `ByteReader` through its operations.
pub struct ByteReaderRef<'a> {
    native: *mut NativeByteReader,
    services: &'a Services,
}
#[allow(
    unsafe_code,
    reason = "calls of the native reader's operations under the view's contract"
)]
impl<'a> ByteReaderRef<'a> {
    /// # Safety
    /// The native reader stays valid and exclusively borrowed for this lifetime, and
    /// every operation of `services` accepts it and its declared output storage.
    pub(crate) unsafe fn new(native: *mut NativeByteReader, services: &'a Services) -> Self {
        Self { native, services }
    }

    /// `HasData(count)`.
    #[must_use]
    pub(crate) fn has_data(&self, count: usize) -> bool {
        // SAFETY: the constructor's contract.
        unsafe { (self.services.has_data)(self.native, count) }
    }

    /// Read a single `DWord` (32 bits).
    /// @note The buffer is NOT advanced.
    /// @returns Value read from buffer.
    pub(crate) fn peek_dword(&mut self) -> Result<u32, OTTDByteReaderSignal> {
        let mut value = 0;
        // SAFETY: the constructor's contract.
        if unsafe { (self.services.peek_dword)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(OTTDByteReaderSignal)
        }
    }

    /// `ReadBytes(size)`: the address of the next `size` bytes, which stay readable
    /// while the native reader's buffer does.
    pub(crate) fn read_bytes(&mut self, size: usize) -> Result<*const u8, OTTDByteReaderSignal> {
        let mut value = std::ptr::null();
        // SAFETY: the constructor's contract.
        if unsafe { (self.services.read_bytes)(self.native, size, &raw mut value) } {
            Ok(value)
        } else {
            Err(OTTDByteReaderSignal)
        }
    }

    /// `Skip(len)`.
    pub(crate) fn skip(&mut self, len: usize) -> Result<(), OTTDByteReaderSignal> {
        // SAFETY: the constructor's contract.
        if unsafe { (self.services.skip)(self.native, len) } {
            Ok(())
        } else {
            Err(OTTDByteReaderSignal)
        }
    }

    /// `spec->badges = ReadBadgeList(buf, feature)`.
    ///
    /// # Safety
    /// `badges` is a valid native badge vector, exclusively borrowed for the call.
    pub(crate) unsafe fn assign_badge_list(
        &mut self,
        feature: u8,
        badges: *mut BadgeList,
    ) -> Result<(), OTTDByteReaderSignal> {
        let ids = self.read_badge_list(feature)?;
        // SAFETY: the caller's contract for `badges`; `ids` holds `len` IDs.
        unsafe { (self.services.assign_badges)(badges, ids.as_ptr(), ids.len()) };
        Ok(())
    }

    /// `ReadBadgeList`: reserve from the count, then let the native leaf fill the
    /// allocation. On failure the destination stays untouched and native badge effects remain.
    pub(crate) fn read_badge_list(
        &mut self,
        feature: u8,
    ) -> Result<Vec<u32>, OTTDByteReaderSignal> {
        let count = self.read_word()?;
        let mut ids = Vec::with_capacity(usize::from(count));
        let mut len = 0;
        // SAFETY: the constructor's contract; `ids` has room for the `count` IDs the
        // leaf writes at most.
        if unsafe {
            (self.services.read_badge_ids)(
                self.native,
                feature,
                count,
                ids.as_mut_ptr(),
                &raw mut len,
            )
        } {
            // SAFETY: the native leaf wrote at most `count` IDs and initialized `len`.
            unsafe { ids.set_len(len) };
            Ok(ids)
        } else {
            Err(OTTDByteReaderSignal)
        }
    }

    /// `SkipBadgeList`.
    pub(crate) fn skip_badge_list(&mut self) -> Result<(), OTTDByteReaderSignal> {
        // SAFETY: the constructor's contract.
        if unsafe { (self.services.skip_badges)(self.native) } {
            Ok(())
        } else {
            Err(OTTDByteReaderSignal)
        }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDByteReaderServices" ["services"] => Services { has_data, peek_dword, read_bytes, read_byte, read_word, read_dword, skip, assign_badges, read_badge_ids, skip_badges };
}

/// The reads both readers share, with `ReadExtendedByte` and `ReadVarSize` written
/// once over them.
pub(crate) trait Read {
    /// Read a single byte (8 bits).
    /// @return Value read from buffer.
    fn read_byte(&mut self) -> Result<u8, OTTDByteReaderSignal>;

    /// Read a single Word (16 bits).
    /// @return Value read from buffer.
    fn read_word(&mut self) -> Result<u16, OTTDByteReaderSignal>;

    /// Read a single `DWord` (32 bits).
    /// @return Value read from buffer.
    fn read_dword(&mut self) -> Result<u32, OTTDByteReaderSignal>;

    /// Read a single Extended Byte (8 or 16 bits).
    /// @return Value read from buffer.
    fn read_extended_byte(&mut self) -> Result<u16, OTTDByteReaderSignal> {
        let val = u16::from(self.read_byte()?);
        if val == 0xFF {
            self.read_word()
        } else {
            Ok(val)
        }
    }

    /// Read a value of the given number of bytes.
    /// @returns Value read from buffer.
    fn read_var_size(&mut self, size: u8) -> Result<u32, OTTDByteReaderSignal> {
        match size {
            1 => self.read_byte().map(u32::from),
            2 => self.read_word().map(u32::from),
            4 => self.read_dword(),
            _ => crate::services_assert::not_reached(),
        }
    }
}

impl Read for ByteReader {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "TryReadUint8's value, which has its width"
    )]
    fn read_byte(&mut self) -> Result<u8, OTTDByteReaderSignal> {
        self.try_read_uint(1).map(|value| value as u8)
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "TryReadUint16LE's value, which has its width"
    )]
    fn read_word(&mut self) -> Result<u16, OTTDByteReaderSignal> {
        self.try_read_uint(2).map(|value| value as u16)
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "TryReadUint32LE's value, which has its width"
    )]
    fn read_dword(&mut self) -> Result<u32, OTTDByteReaderSignal> {
        self.try_read_uint(4).map(|value| value as u32)
    }
}

#[allow(
    unsafe_code,
    reason = "calls of the native reader's operations under the view's contract"
)]
impl Read for ByteReaderRef<'_> {
    fn read_byte(&mut self) -> Result<u8, OTTDByteReaderSignal> {
        let mut value = 0;
        // SAFETY: the constructor's contract.
        if unsafe { (self.services.read_byte)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(OTTDByteReaderSignal)
        }
    }

    fn read_word(&mut self) -> Result<u16, OTTDByteReaderSignal> {
        let mut value = 0;
        // SAFETY: the constructor's contract.
        if unsafe { (self.services.read_word)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(OTTDByteReaderSignal)
        }
    }

    fn read_dword(&mut self) -> Result<u32, OTTDByteReaderSignal> {
        let mut value = 0;
        // SAFETY: the constructor's contract.
        if unsafe { (self.services.read_dword)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(OTTDByteReaderSignal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteReader, Read};

    /// `ByteReader` over each length of `{0x81, 0x23, 0x45, 0x67}`, as
    /// `src/tests/newgrf_bytereader.cpp` holds the C++ reader: each read's value or
    /// failure, and the bytes left after it.
    #[test]
    #[allow(unsafe_code, reason = "readers over a local array that outlives them")]
    fn reads_fail_without_moving() {
        let data = [0x81, 0x23, 0x45, 0x67];
        for size in 0..=data.len() {
            // SAFETY: `data` outlives each reader and is not written.
            let new = || unsafe { ByteReader::new(data.as_ptr(), size) };
            let mut byte = new();
            assert_eq!(byte.read_byte().ok(), (size >= 1).then_some(0x81));
            assert_eq!(byte.remaining(), if size >= 1 { size - 1 } else { size });
            let mut word = new();
            assert_eq!(word.read_word().ok(), (size >= 2).then_some(0x2381));
            assert_eq!(word.remaining(), if size >= 2 { size - 2 } else { size });
            let mut dword = new();
            assert_eq!(dword.read_dword().ok(), (size >= 4).then_some(0x6745_2381));
            assert_eq!(dword.remaining(), if size >= 4 { size - 4 } else { size });
            let mut extended = new();
            assert_eq!(
                extended.read_extended_byte().ok(),
                (size >= 1).then_some(0x81)
            );
            assert_eq!(extended.remaining(), size.saturating_sub(1));
        }
        let data = [0xFF, 0x34, 0x12, 0xFF, 0x34];
        // SAFETY: `data` outlives each reader and is not written.
        let mut reader = unsafe { ByteReader::new(data.as_ptr(), data.len()) };
        assert_eq!(reader.read_extended_byte().ok(), Some(0x1234));
        assert!(reader.read_extended_byte().is_err());
        // The byte `0xFF` was read; the word that should follow it was not.
        assert_eq!(reader.remaining(), 1);
        for size in [1, 2, 4] {
            // SAFETY: `data` outlives each reader and is not written.
            let mut reader = unsafe { ByteReader::new(data.as_ptr(), data.len()) };
            let expected = [0xFF, 0x34FF, 0, 0xFF12_34FF][usize::from(size) - 1];
            assert_eq!(reader.read_var_size(size).ok(), Some(expected));
            assert_eq!(reader.remaining(), data.len() - usize::from(size));
        }
    }

    /// `ReadString` returns the bytes before the NUL and skips it; at the end of the
    /// sprite it returns the rest without one.
    #[test]
    #[allow(unsafe_code, reason = "readers over a local array that outlives them")]
    fn read_string_skips_its_terminator() {
        let data = *b"ab\0cd";
        // SAFETY: `data` outlives the reader and is not written.
        let mut reader = unsafe { ByteReader::new(data.as_ptr(), data.len()) };
        let (text, len) = reader.read_string();
        assert_eq!((text, len), (data.as_ptr(), 2));
        assert_eq!(reader.remaining(), 2);
        let (text, len) = reader.read_string();
        assert_eq!((text, len), (data[3..].as_ptr(), 2));
        assert_eq!(reader.remaining(), 0);
        assert_eq!(reader.read_string().1, 0);
        assert_eq!(reader.tail().1, 0);
    }

    /// `ReadVarSize`'s `default: NOT_REACHED()`: a size of 3 ends at the native
    /// unreachable path, in a child whose bound `NOT_REACHED` writes a marker and exits.
    #[test]
    #[allow(
        unsafe_code,
        reason = "binds the child's policy and reads a local array"
    )]
    fn read_var_size_three_is_not_reached() {
        const NAME: &str = "newgrf_bytereader::tests::read_var_size_three_is_not_reached";
        const EXIT: i32 = 87;
        const MARKER: &str = "newgrf_bytereader test: ReadVarSize reached NOT_REACHED";
        extern "C" fn not_reached() {
            eprintln!("{MARKER}");
            std::process::exit(EXIT);
        }
        extern "C" fn abort_on_failure(condition: bool) {
            if !condition {
                std::process::abort();
            }
        }
        if crate::services_assert::isolate_test_ends(NAME, EXIT, MARKER) {
            return;
        }
        // `assert_with` reads only `assertions` of the bound map statics.
        let map = Box::leak(Box::new(crate::map_func::MapStatics {
            base: std::ptr::null(),
            extended: std::ptr::null(),
            size: std::ptr::null(),
            log_x: std::ptr::null(),
            log_y: std::ptr::null(),
            assertions: true,
        }));
        let shared = Box::leak(Box::new(crate::services::Services {
            map,
            not_reached,
            ..crate::test_tables::SHARED
        }));
        // SAFETY: The leaked map statics live through the child's last assertion.
        unsafe { crate::services_assert::bind_test_policy(abort_on_failure, shared) };
        let data = [1, 2, 3];
        // SAFETY: `data` outlives the reader and is not written.
        let mut reader = unsafe { ByteReader::new(data.as_ptr(), data.len()) };
        let _ = reader.read_var_size(3);
        // Reaching this line would mean the unreachable path returned.
        std::process::exit(0);
    }
}
