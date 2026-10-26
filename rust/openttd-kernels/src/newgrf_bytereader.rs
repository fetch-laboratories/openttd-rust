/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Checked native `ByteReader` operations. Native leaves catch only `OTTDByteReaderSignal`.
//! Failure ends the Rust interpreter; its native caller rethrows after Rust drops scratch.
//! A reader and badge destination are exclusively borrowed for a synchronous loading entry.
#![allow(unsafe_code, missing_docs, clippy::missing_errors_doc)]
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
    "OpenTTDByteReaderServices" ["services"] => Services { read_byte, read_word, read_dword, skip, read_badges, skip_badges };
}
