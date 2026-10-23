/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Layout declarations drive Rust metadata and generated native probes/visitors.
//! Each component invokes `layouts!` in its own module; no file lists them.

macro_rules! layouts {
    (fn $function:ident; $( $cpp:literal [$group:literal] => $type:ty {
        $($field:tt $(=> $alias:ident)?),* $(,)?
    } $(width $width:ident : $width_type:ty)?; )*) => {
        pub(crate) fn $function(name: &str, item: usize) -> usize {
            match name {
                $($cpp => [
                    std::mem::size_of::<$type>(),
                    std::mem::align_of::<$type>(),
                    $(std::mem::offset_of!($type, $field),)*
                    $({
                        let _: fn(&$type) -> &$width_type = |value| &value.$width;
                        std::mem::size_of::<$width_type>()
                    },)?
                ].get(item).copied().unwrap_or(usize::MAX),)*
                _ => usize::MAX,
            }
        }
    };
}
pub(crate) use layouts;

/// Look up `name` in every module that invokes `layouts!`; `build.rs` lists them.
pub fn layout(name: &str, item: usize) -> usize {
    for layout in include!(concat!(env!("OUT_DIR"), "/abi_layouts.rs")) {
        let value = layout(name, item);
        if value != usize::MAX {
            return value;
        }
    }
    usize::MAX
}
