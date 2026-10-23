/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Copyable scalar packet state; buffers/allocations/external effects stay in C++.

/// Native-width limit with the original persistently narrowed 16-bit cursor.
#[repr(C)]
pub struct State {
    pub limit: usize,
    pub position: u16,
}

/// Packet-specific framing offsets, with native unsigned arithmetic.
#[repr(C)]
pub struct Frame {
    pub message: usize,
    pub payload: usize,
}

impl State {
    pub(crate) fn new(limit: usize) -> Self {
        Self { limit, position: 0 }
    }
    pub(crate) fn can_write(&self, size: usize, amount: usize) -> bool {
        size.wrapping_add(amount) <= self.limit
    }
    pub(crate) fn can_read(&self, size: usize, amount: usize) -> bool {
        usize::from(self.position).wrapping_add(amount) <= size
    }
    pub(crate) fn remaining(&self, size: usize) -> usize {
        size.wrapping_sub(usize::from(self.position))
    }
    pub(crate) fn transfer_amount(&self, size: usize, limit: usize) -> usize {
        self.remaining(size).min(limit)
    }
    pub(crate) fn transfer_commit(&mut self, amount: isize) {
        if amount > 0 {
            // Read the live post-callback cursor, not a captured planning cursor.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let narrowed = amount as u16;
            self.position = self.position.wrapping_add(narrowed);
        }
    }
    pub(crate) fn send_amount(&self, size: usize, input: usize) -> usize {
        input.min(self.limit.wrapping_sub(size))
    }
    pub(crate) fn read_start(&mut self) {
        self.position = 2;
    }
    pub(crate) fn send_reset(&mut self) {
        self.position = 0;
    }
    pub(crate) fn skip_mac(&mut self, size: usize) {
        self.position = self.position.wrapping_add(prefix(size));
    }
    pub(crate) fn recv(&mut self, bytes: &[u8], width: u8) -> u64 {
        assert!(matches!(width, 1 | 2 | 4 | 8));
        let mut value = 0;
        for byte in 0..width {
            value |= u64::from(bytes[usize::from(self.position)]) << (byte * 8);
            self.position = self.position.wrapping_add(1);
        }
        value
    }
    pub(crate) fn parse_size(&self, bytes: &[u8]) -> u16 {
        let size = u16::from_le_bytes([bytes[0], bytes[1]]);
        if size < 3 || usize::from(size) > self.limit {
            0
        } else {
            size
        }
    }
}

pub(crate) fn frame(position: u16, size: usize, mac: usize) -> Frame {
    let message = usize::from(position).wrapping_add(mac);
    Frame {
        message,
        payload: size.wrapping_sub(message),
    }
}

pub(crate) fn prefix(size: usize) -> u16 {
    // The public constructor has no additional u16-sized limit; preserve narrowing.
    #[allow(clippy::cast_possible_truncation)]
    {
        size as u16
    }
}

pub(crate) fn header(bytes: &mut [u8]) {
    let encoded = crate::builder::little_endian(bytes.len() as u64);
    bytes[..2].copy_from_slice(&encoded.bytes[..2]);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDPacketState" ["probe"] => State {
        limit,
        position,
    };
    "OpenTTDPacketFrame" ["probe"] => Frame {
        message,
        payload,
    };
}
