/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Checked native `ByteReader` operations. Native leaves catch only `OTTDByteReaderSignal`.
//! Failure ends the Rust interpreter; its native caller rethrows after Rust drops scratch.
//! A native reader and badge destination are exclusively borrowed for one loading entry.
//! `Cursor` reads the interpreter's borrowed pseudo-sprite buffer directly.
#![allow(unsafe_code, missing_docs, clippy::missing_errors_doc)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ReadBounds;
pub(crate) struct Cursor {
    data: *const u8,
    len: usize,
    pos: usize,
}
impl Cursor {
    pub(crate) const fn new(data: *const u8, len: usize) -> Self {
        Self { data, len, pos: 0 }
    }
    pub(crate) const fn remaining(&self) -> usize {
        self.len - self.pos
    }
    pub(crate) unsafe fn byte(&mut self) -> Result<u8, ReadBounds> {
        if self.remaining() < 1 {
            return Err(ReadBounds);
        }
        let result = unsafe { self.data.add(self.pos).read() };
        self.pos += 1;
        Ok(result)
    }
    pub(crate) unsafe fn word(&mut self) -> Result<u16, ReadBounds> {
        if self.remaining() < 2 {
            return Err(ReadBounds);
        }
        let result = unsafe { self.data.add(self.pos).cast::<u16>().read_unaligned() };
        self.pos += 2;
        Ok(u16::from_le(result))
    }
    pub(crate) unsafe fn dword(&mut self) -> Result<u32, ReadBounds> {
        if self.remaining() < 4 {
            return Err(ReadBounds);
        }
        let result = unsafe { self.data.add(self.pos).cast::<u32>().read_unaligned() };
        self.pos += 4;
        Ok(u32::from_le(result))
    }
    pub(crate) unsafe fn extended(&mut self) -> Result<u16, ReadBounds> {
        let v = unsafe { self.byte()? };
        if v == 255 {
            unsafe { self.word() }
        } else {
            Ok(u16::from(v))
        }
    }
    pub(crate) unsafe fn string(&mut self) -> (*const u8, usize) {
        let data = unsafe { self.data.add(self.pos) };
        let mut len = 0;
        while self.pos < self.len {
            let v = unsafe { self.data.add(self.pos).read() };
            self.pos += 1;
            if v == 0 {
                break;
            }
            len += 1;
        }
        (data, len)
    }
    pub(crate) unsafe fn tail(&self) -> (*const u8, usize) {
        (unsafe { self.data.add(self.pos) }, self.remaining())
    }
}

#[repr(C)]
pub struct ByteReader {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct BadgeList {
    _opaque: [u8; 0],
}
#[derive(Debug)]
pub struct ReadError;

#[repr(C)]
pub struct Services {
    pub has_data: unsafe extern "C" fn(*mut ByteReader, usize) -> bool,
    pub peek_dword: unsafe extern "C" fn(*mut ByteReader, *mut u32) -> bool,
    pub read_bytes: unsafe extern "C" fn(*mut ByteReader, usize, *mut *const u8) -> bool,
    pub read_byte: unsafe extern "C" fn(*mut ByteReader, *mut u8) -> bool,
    pub read_word: unsafe extern "C" fn(*mut ByteReader, *mut u16) -> bool,
    pub read_dword: unsafe extern "C" fn(*mut ByteReader, *mut u32) -> bool,
    pub skip: unsafe extern "C" fn(*mut ByteReader, usize) -> bool,
    pub read_badges: unsafe extern "C" fn(*mut ByteReader, u8, *mut BadgeList) -> bool,
    pub skip_badges: unsafe extern "C" fn(*mut ByteReader) -> bool,
}
pub struct Reader<'a> {
    native: *mut ByteReader,
    services: &'a Services,
}
impl<'a> Reader<'a> {
    /// # Safety
    /// The reader stays valid and exclusively borrowed for this lifetime; every
    /// service accepts that reader and its declared native output storage.
    pub unsafe fn new(native: *mut ByteReader, services: &'a Services) -> Self {
        Self { native, services }
    }
    pub fn extended(&mut self) -> Result<u16, ReadError> {
        let value = self.byte()?;
        if value == 255 {
            self.word()
        } else {
            Ok(u16::from(value))
        }
    }
    #[must_use]
    pub fn has_data(&self, count: usize) -> bool {
        unsafe { (self.services.has_data)(self.native, count) }
    }
    pub fn peek_dword(&mut self) -> Result<u32, ReadError> {
        let mut value = 0;
        if unsafe { (self.services.peek_dword)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(ReadError)
        }
    }
    pub fn bytes(&mut self, count: usize) -> Result<*const u8, ReadError> {
        let mut value = std::ptr::null();
        if unsafe { (self.services.read_bytes)(self.native, count, &raw mut value) } {
            Ok(value)
        } else {
            Err(ReadError)
        }
    }
    pub fn native(&mut self) -> *mut ByteReader {
        self.native
    }
    pub fn byte(&mut self) -> Result<u8, ReadError> {
        let mut value = 0;
        if unsafe { (self.services.read_byte)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(ReadError)
        }
    }
    pub fn word(&mut self) -> Result<u16, ReadError> {
        let mut value = 0;
        if unsafe { (self.services.read_word)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(ReadError)
        }
    }
    pub fn dword(&mut self) -> Result<u32, ReadError> {
        let mut value = 0;
        if unsafe { (self.services.read_dword)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(ReadError)
        }
    }
    pub fn skip(&mut self, count: usize) -> Result<(), ReadError> {
        if unsafe { (self.services.skip)(self.native, count) } {
            Ok(())
        } else {
            Err(ReadError)
        }
    }
    /// # Safety
    /// The destination is a valid, exclusively borrowed native badge vector for the call.
    pub unsafe fn badges(
        &mut self,
        feature: u8,
        destination: *mut BadgeList,
    ) -> Result<(), ReadError> {
        if unsafe { (self.services.read_badges)(self.native, feature, destination) } {
            Ok(())
        } else {
            Err(ReadError)
        }
    }
    pub fn skip_badges(&mut self) -> Result<(), ReadError> {
        if unsafe { (self.services.skip_badges)(self.native) } {
            Ok(())
        } else {
            Err(ReadError)
        }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDByteReaderServices" ["services"] => Services { has_data, peek_dword, read_bytes, read_byte, read_word, read_dword, skip, read_badges, skip_badges };
}
