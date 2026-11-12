/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `SQObjectType` from `3rdparty/squirrel/include/squirrel.h`.

/// The type of a Squirrel object, as `sq_gettype` returns it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SQObjectType(pub u32);

impl SQObjectType {
    /// `_RT_NULL | SQOBJECT_CANBEFALSE`.
    pub const OT_NULL: Self = Self(0x0100_0001);
    /// `_RT_INTEGER | SQOBJECT_NUMERIC | SQOBJECT_CANBEFALSE`.
    pub const OT_INTEGER: Self = Self(0x0500_0002);
    /// `_RT_BOOL | SQOBJECT_CANBEFALSE`.
    pub const OT_BOOL: Self = Self(0x0100_0008);
    /// `_RT_STRING | SQOBJECT_REF_COUNTED`.
    pub const OT_STRING: Self = Self(0x0800_0010);
    /// `_RT_TABLE | SQOBJECT_REF_COUNTED | SQOBJECT_DELEGABLE`.
    pub const OT_TABLE: Self = Self(0x0a00_0020);
    /// `_RT_ARRAY | SQOBJECT_REF_COUNTED`.
    pub const OT_ARRAY: Self = Self(0x0800_0040);
}
