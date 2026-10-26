/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `ByteReader` cursor over the current pseudo-sprite, including unterminated strings.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
#[derive(Clone, Copy, Debug)]
pub struct ReadBounds;
pub struct ByteReader {
    data: *const u8,
    len: usize,
    pos: usize,
}
impl ByteReader {
    pub const fn new(data: *const u8, len: usize) -> Self {
        Self { data, len, pos: 0 }
    }
    pub const fn remaining(&self) -> usize {
        self.len - self.pos
    }
    pub unsafe fn byte(&mut self) -> Result<u8, ReadBounds> {
        if self.remaining() < 1 {
            return Err(ReadBounds);
        }
        let result = unsafe { self.data.add(self.pos).read() };
        self.pos += 1;
        Ok(result)
    }
    pub unsafe fn word(&mut self) -> Result<u16, ReadBounds> {
        if self.remaining() < 2 {
            return Err(ReadBounds);
        }
        let result = unsafe { self.data.add(self.pos).cast::<u16>().read_unaligned() };
        self.pos += 2;
        Ok(u16::from_le(result))
    }
    pub unsafe fn dword(&mut self) -> Result<u32, ReadBounds> {
        if self.remaining() < 4 {
            return Err(ReadBounds);
        }
        let result = unsafe { self.data.add(self.pos).cast::<u32>().read_unaligned() };
        self.pos += 4;
        Ok(u32::from_le(result))
    }
    pub unsafe fn extended(&mut self) -> Result<u16, ReadBounds> {
        let v = unsafe { self.byte()? };
        if v == 255 {
            unsafe { self.word() }
        } else {
            Ok(u16::from(v))
        }
    }
    pub unsafe fn string(&mut self) -> (*const u8, usize) {
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
    pub unsafe fn tail(&self) -> (*const u8, usize) {
        (unsafe { self.data.add(self.pos) }, self.remaining())
    }
}
