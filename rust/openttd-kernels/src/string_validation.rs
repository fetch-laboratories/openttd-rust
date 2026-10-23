/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Historical sanitation decisions and borrowed in-place write progression.

use crate::utf8;

/// One scan decision. C++ commits consumption before any output/allocation.
#[repr(C)]
pub struct Step {
    pub consumed: usize,
    pub output: utf8::Encoded,
    pub stopped: u8,
}

/// Write position is unchanged on overtake; C++ performs fatal dispatch afterward.
#[repr(C)]
pub struct Write {
    pub position: usize,
    pub accepted: u8,
}

fn printable(codepoint: u32) -> bool {
    codepoint >= 0x20 && !(0xe000..0xe200).contains(&codepoint)
}
fn sprite(codepoint: u32) -> bool {
    (0xe200..=0xe2ff).contains(&codepoint)
}
fn encoded_control(codepoint: u32) -> bool {
    codepoint == 0x1e || (0xe000..=0xe003).contains(&codepoint)
}

pub fn step(bytes: &[u8], settings: u8) -> Step {
    let decoded = utf8::decode(bytes);
    let mut result = Step {
        consumed: decoded.length,
        output: utf8::encode(0x0011_0000),
        stopped: 0,
    };
    if bytes.is_empty() {
        result.stopped = 1;
    } else if decoded.length == 0 {
        // Unlike undesirable decoded characters, malformed bytes never emit '?'.
        result.consumed = 1;
    } else if decoded.codepoint == 0 {
        result.stopped = 1;
    } else {
        let codepoint = decoded.codepoint;
        let newline = settings & 2 != 0;
        if (printable(codepoint) && !sprite(codepoint))
            || (settings & 4 != 0 && encoded_control(codepoint))
            || (newline && codepoint == 10)
        {
            result.output = utf8::encode(codepoint);
        } else if newline && codepoint == 13 && bytes[decoded.length..].starts_with(b"\n") {
            // Drop CR only; LF is consumed by the next decision.
        } else if settings & 8 != 0 && matches!(codepoint, 9 | 10 | 13) {
            result.output = utf8::encode(32);
        } else if settings & 1 != 0 {
            result.output = utf8::encode(63);
        }
    }
    result
}

pub fn valid(bytes: &[u8]) -> bool {
    let mut position = 0;
    while position < bytes.len() {
        let decoded = utf8::decode(&bytes[position..]);
        if decoded.length == 0 {
            return false;
        }
        if decoded.codepoint == 0 {
            return true;
        }
        if !printable(decoded.codepoint) || sprite(decoded.codepoint) {
            return false;
        }
        position += decoded.length;
    }
    false
}

/// Capacity uses the current external consumer, including native unsigned wrap.
pub fn write_plan(position: usize, consumed: usize, length: usize) -> Write {
    if length > consumed.wrapping_sub(position) {
        Write {
            position,
            accepted: 0,
        }
    } else {
        Write {
            position: position.wrapping_add(length),
            accepted: 1,
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDValidationStep" ["probe"] => Step {
        consumed,
        output,
        stopped,
    };
    "OpenTTDInplaceWrite" ["probe"] => Write {
        position,
        accepted,
    };
}
