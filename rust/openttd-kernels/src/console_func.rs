/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Console functions used outside of the console code (`console_func.h`).
//!
//! `IConsolePrint` runs natively (`console.cpp`) until the console is ported, through the news
//! table of `news_func.rs`, which is bound once for the process.

use crate::gfx_type::TextColour;
use crate::services::ByteSpan;

/// `IConsolePrint(colour_code, string)`: handle the printing of text entered into the console
/// or redirected there by any other means. Text can be redirected to other clients in a
/// network game as well as to a logfile. If the network server is a dedicated server, all
/// activities are also logged. All lines to print are added to a temporary buffer which can be
/// used as a history to print them onscreen.
pub(crate) fn iconsole_print(colour_code: TextColour, string: &[u8]) {
    (crate::news_func::services().iconsole_print)(
        colour_code.0,
        ByteSpan {
            data: string.as_ptr(),
            size: string.len(),
        },
    );
}

/// `IConsolePrint(colour_code, format, args...)`: the formatting overload, which formats its
/// arguments as `fmt::format` does and prints the result. Native strings are formatted through
/// [`crate::debug::Bytes`].
macro_rules! iconsole_print_format {
    ($colour_code:expr, $($arg:tt)+) => {
        $crate::console_func::iconsole_print(
            $colour_code,
            &$crate::debug::format(format_args!($($arg)+)),
        )
    };
}
pub(crate) use iconsole_print_format as iconsole_print;

#[cfg(test)]
mod tests {
    use crate::console_type::{CC_DEFAULT, CC_WARNING};
    use crate::debug::Bytes;
    use crate::news_func::testing::{Effect, take};

    /// The formatting overload prints the bytes `fmt::format` produces, a native string's
    /// bytes unchanged, and passes the colour through; the plain overload passes its bytes.
    #[test]
    fn prints_formatted_bytes_in_the_given_colour() {
        take();
        let ip = b"h\xF6st";
        super::iconsole_print!(
            CC_WARNING,
            "Client #{} (IP: {}) is dropped because it fails to send valid acks.",
            7_u32,
            Bytes(ip)
        );
        super::iconsole_print(CC_DEFAULT, b"plain \xFF");
        assert_eq!(
            take(),
            [
                Effect::IConsolePrint(
                    0x0D,
                    b"Client #7 (IP: h\xF6st) is dropped because it fails to send valid acks."
                        .to_vec()
                ),
                Effect::IConsolePrint(0x01, b"plain \xFF".to_vec()),
            ]
        );
    }
}
