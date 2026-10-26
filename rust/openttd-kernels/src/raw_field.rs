/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Raw field access for records that C++ also views. A using module states once that its
//! record pointers are live and serialized for the access; each access copies one field
//! and creates no reference.

/// Copy the field at `path` (such as `old[i].income`) of the record `$ptr` points to.
macro_rules! read_field {
    ($ptr:expr, $($field:ident $([$index:expr])*).+) => {
        // SAFETY: The using module's contract keeps the record live and serialized; the
        // read copies one field and creates no reference.
        unsafe { ::std::ptr::addr_of!((*$ptr).$($field $([$index])*).+).read() }
    };
}
/// Evaluate `$value`, then store it in the field at `path`, in assignment order. The old
/// value is overwritten, not dropped.
macro_rules! write_field {
    ($ptr:expr, $($field:ident $([$index:expr])*).+, $value:expr) => {{
        let value = $value;
        // SAFETY: The using module's contract keeps the record live, serialized and
        // writable; the store creates no reference.
        unsafe { ::std::ptr::addr_of_mut!((*$ptr).$($field $([$index])*).+).write(value) }
    }};
}
pub(crate) use {read_field, write_field};
