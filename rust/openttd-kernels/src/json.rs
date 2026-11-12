/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `nlohmann::json::value_t` from `3rdparty/nlohmann/json.hpp`, as its underlying byte.

/// The JSON value types, pinned by `admin_conversion.cpp`'s static assertions.
pub mod value_t {
    /// Null value.
    pub const NULL: u8 = 0;
    /// Object (unordered set of name/value pairs).
    pub const OBJECT: u8 = 1;
    /// Array (ordered collection of values).
    pub const ARRAY: u8 = 2;
    /// String value.
    pub const STRING: u8 = 3;
    /// Boolean value.
    pub const BOOLEAN: u8 = 4;
    /// Number value (signed integer).
    pub const NUMBER_INTEGER: u8 = 5;
    /// Number value (unsigned integer).
    pub const NUMBER_UNSIGNED: u8 = 6;
}
