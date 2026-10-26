/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Checked native `ByteReader` operations. Native leaves catch only `OTTDByteReaderSignal`.
//! Failure ends the Rust interpreter; its native caller rethrows after Rust drops scratch.
//! A reader and badge destination are exclusively borrowed for a synchronous loading entry.
#![allow(unsafe_code)]
#[repr(C)]
pub struct ByteReader {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct BadgeList {
    _opaque: [u8; 0],
}
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
    pub unsafe fn new(native: *mut ByteReader, services: &'a Services) -> Self {
        Self { native, services }
    }
    pub fn byte(&mut self) -> Result<u8, ()> {
        let mut value = 0;
        if unsafe { (self.services.read_byte)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(())
        }
    }
    pub fn word(&mut self) -> Result<u16, ()> {
        let mut value = 0;
        if unsafe { (self.services.read_word)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(())
        }
    }
    pub fn dword(&mut self) -> Result<u32, ()> {
        let mut value = 0;
        if unsafe { (self.services.read_dword)(self.native, &raw mut value) } {
            Ok(value)
        } else {
            Err(())
        }
    }
    pub fn skip(&mut self, count: usize) -> Result<(), ()> {
        if unsafe { (self.services.skip)(self.native, count) } {
            Ok(())
        } else {
            Err(())
        }
    }
    pub unsafe fn badges(&mut self, feature: u8, destination: *mut BadgeList) -> Result<(), ()> {
        if unsafe { (self.services.read_badges)(self.native, feature, destination) } {
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn skip_badges(&mut self) -> Result<(), ()> {
        if unsafe { (self.services.skip_badges)(self.native) } {
            Ok(())
        } else {
            Err(())
        }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDByteReaderServices" ["industry"] => Services { read_byte, read_word, read_dword, skip, read_badges, skip_badges };
}
