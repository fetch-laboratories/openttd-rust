/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Window functions not directly related to making/drawing windows (`window_func.h`).
//!
//! Each function calls the native entry point of the same name in `window.cpp`, which owns
//! the window list until the GUI is ported. The native table is bound once for the process,
//! as `news_func.rs` binds its table; it also carries `DeleteNewGRFInspectWindow` for
//! `newgrf_debug.rs`. Every service is a synchronous `noexcept` call that holds no Rust
//! borrow. Call sites write the original's default arguments out.

use crate::window_type::{WidgetID, WindowClass, WindowNumber};
use std::sync::OnceLock;

/// Native window services.
#[repr(C)]
pub struct Services {
    /// `CloseWindowById(cls, number, force, data)`.
    pub close_window_by_id: extern "C" fn(u16, i32, bool, i32),
    /// `DeleteNewGRFInspectWindow(feature, index)`.
    pub delete_newgrf_inspect_window: extern "C" fn(u8, u32),
    /// `InvalidateWindowClassesData(cls, data, gui_scope)`.
    pub invalidate_window_classes_data: extern "C" fn(u16, i32, bool),
    /// `InvalidateWindowData(cls, number, data, gui_scope)`.
    pub invalidate_window_data: extern "C" fn(u16, i32, i32, bool),
    /// `SetWindowClassesDirty(cls)`.
    pub set_window_classes_dirty: extern "C" fn(u16),
    /// `SetWindowDirty(cls, number)`.
    pub set_window_dirty: extern "C" fn(u16, i32),
    /// `SetWindowWidgetDirty(cls, number, widget_index)`.
    pub set_window_widget_dirty: extern "C" fn(u16, i32, i32),
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDWindowServices" ["window"] => Services {
        close_window_by_id,
        delete_newgrf_inspect_window,
        invalidate_window_classes_data,
        invalidate_window_data,
        set_window_classes_dirty,
        set_window_dirty,
        set_window_widget_dirty,
    };
}

extern "C" fn unbound_close_window_by_id(_: u16, _: i32, _: bool, _: i32) {
    std::process::abort()
}
extern "C" fn unbound_delete_newgrf_inspect_window(_: u8, _: u32) {
    std::process::abort()
}
extern "C" fn unbound_invalidate_window_classes_data(_: u16, _: i32, _: bool) {
    std::process::abort()
}
extern "C" fn unbound_invalidate_window_data(_: u16, _: i32, _: i32, _: bool) {
    std::process::abort()
}
extern "C" fn unbound_set_window_classes_dirty(_: u16) {
    std::process::abort()
}
extern "C" fn unbound_set_window_dirty(_: u16, _: i32) {
    std::process::abort()
}
extern "C" fn unbound_set_window_widget_dirty(_: u16, _: i32, _: i32) {
    std::process::abort()
}
/// Services before the native table binds; each call aborts.
static UNBOUND: Services = Services {
    close_window_by_id: unbound_close_window_by_id,
    delete_newgrf_inspect_window: unbound_delete_newgrf_inspect_window,
    invalidate_window_classes_data: unbound_invalidate_window_classes_data,
    invalidate_window_data: unbound_invalidate_window_data,
    set_window_classes_dirty: unbound_set_window_classes_dirty,
    set_window_dirty: unbound_set_window_dirty,
    set_window_widget_dirty: unbound_set_window_widget_dirty,
};
static SERVICES: OnceLock<&'static Services> = OnceLock::new();

/// Bind the native table during static initialization.
///
/// # Safety
/// `services` lives for the process.
#[allow(
    unsafe_code,
    reason = "the binding export and its process-lifetime pointer contract"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_window_bind(services: *const Services) {
    // SAFETY: This function's contract.
    bind(unsafe { &*services });
}

/// Bind `services` for the process; the first binding stays.
fn bind(services: &'static Services) {
    let _ = SERVICES.set(services);
}

/// The bound table, or the aborting services before the native table binds.
pub(crate) fn services() -> &'static Services {
    SERVICES.get().copied().unwrap_or(&UNBOUND)
}

/// `SetWindowDirty`: mark the window of the given class and number as dirty.
pub(crate) fn set_window_dirty(cls: WindowClass, number: WindowNumber) {
    (services().set_window_dirty)(cls.0, number.0);
}

/// `SetWindowWidgetDirty`: mark a particular widget in a particular window as dirty.
pub(crate) fn set_window_widget_dirty(
    cls: WindowClass,
    number: WindowNumber,
    widget_index: WidgetID,
) {
    (services().set_window_widget_dirty)(cls.0, number.0, widget_index);
}

/// `SetWindowClassesDirty`: mark all windows of a particular class as dirty.
pub(crate) fn set_window_classes_dirty(cls: WindowClass) {
    (services().set_window_classes_dirty)(cls.0);
}

/// `InvalidateWindowData`: mark the window of the given class and number as invalid. The
/// window's `OnInvalidateData(data, gui_scope)` runs synchronously during the call; when
/// `gui_scope` is false it is also scheduled to run again with `gui_scope` at the next redraw.
/// The synchronous call can reenter game state, so no Rust borrow spans this call.
pub(crate) fn invalidate_window_data(
    cls: WindowClass,
    number: WindowNumber,
    data: i32,
    gui_scope: bool,
) {
    (services().invalidate_window_data)(cls.0, number.0, data, gui_scope);
}

/// `InvalidateWindowClassesData`: mark all windows of a class as invalid, with the
/// synchronous and scheduled `OnInvalidateData` calls of [`invalidate_window_data`].
pub(crate) fn invalidate_window_classes_data(cls: WindowClass, data: i32, gui_scope: bool) {
    (services().invalidate_window_classes_data)(cls.0, data, gui_scope);
}

/// `CloseWindowById`: close a window by its class and window number (if it is open).
pub(crate) fn close_window_by_id(cls: WindowClass, number: WindowNumber, force: bool, data: i32) {
    (services().close_window_by_id)(cls.0, number.0, force, data);
}

/// A recording window table shared by the tests of every caller.
#[cfg(test)]
pub(crate) mod testing {
    use super::Services;
    use crate::newgrf::GrfSpecFeature;
    use crate::window_type::{WidgetID, WindowClass, WindowNumber};
    use std::cell::RefCell;

    /// One native window call, in the order the code under test made it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Effect {
        CloseWindowById(WindowClass, WindowNumber, bool, i32),
        DeleteNewGRFInspectWindow(GrfSpecFeature, u32),
        InvalidateWindowClassesData(WindowClass, i32, bool),
        InvalidateWindowData(WindowClass, WindowNumber, i32, bool),
        SetWindowClassesDirty(WindowClass),
        SetWindowDirty(WindowClass, WindowNumber),
        SetWindowWidgetDirty(WindowClass, WindowNumber, WidgetID),
    }

    thread_local! {
        static EFFECTS: RefCell<Vec<Effect>> = const { RefCell::new(Vec::new()) };
        static OBSERVER: std::cell::Cell<Option<fn(&Effect)>> = const { std::cell::Cell::new(None) };
    }
    fn push(effect: Effect) {
        if let Some(observe) = OBSERVER.get() {
            observe(&effect);
        }
        EFFECTS.with(|e| e.borrow_mut().push(effect));
    }
    /// Call `observe` with each of this thread's window calls as it is made, so a test can
    /// read its own state at the call's observation point.
    pub(crate) fn observe(observe: Option<fn(&Effect)>) {
        OBSERVER.set(observe);
    }

    extern "C" fn close_window_by_id(cls: u16, number: i32, force: bool, data: i32) {
        push(Effect::CloseWindowById(
            WindowClass(cls),
            WindowNumber(number),
            force,
            data,
        ));
    }
    extern "C" fn delete_newgrf_inspect_window(feature: u8, index: u32) {
        push(Effect::DeleteNewGRFInspectWindow(
            GrfSpecFeature(feature),
            index,
        ));
    }
    extern "C" fn invalidate_window_classes_data(cls: u16, data: i32, gui_scope: bool) {
        push(Effect::InvalidateWindowClassesData(
            WindowClass(cls),
            data,
            gui_scope,
        ));
    }
    extern "C" fn invalidate_window_data(cls: u16, number: i32, data: i32, gui_scope: bool) {
        push(Effect::InvalidateWindowData(
            WindowClass(cls),
            WindowNumber(number),
            data,
            gui_scope,
        ));
    }
    extern "C" fn set_window_classes_dirty(cls: u16) {
        push(Effect::SetWindowClassesDirty(WindowClass(cls)));
    }
    extern "C" fn set_window_dirty(cls: u16, number: i32) {
        push(Effect::SetWindowDirty(
            WindowClass(cls),
            WindowNumber(number),
        ));
    }
    extern "C" fn set_window_widget_dirty(cls: u16, number: i32, widget_index: i32) {
        push(Effect::SetWindowWidgetDirty(
            WindowClass(cls),
            WindowNumber(number),
            widget_index,
        ));
    }
    static WINDOWS: Services = Services {
        close_window_by_id,
        delete_newgrf_inspect_window,
        invalidate_window_classes_data,
        invalidate_window_data,
        set_window_classes_dirty,
        set_window_dirty,
        set_window_widget_dirty,
    };

    /// Bind the recording table once for the process; return and clear this thread's
    /// recorded calls.
    pub(crate) fn take() -> Vec<Effect> {
        super::bind(&WINDOWS);
        EFFECTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{Effect, take};
    use crate::window_type::{WindowClass, WindowNumber};

    #[test]
    fn each_function_passes_its_arguments_to_its_entry_point() {
        take();
        super::set_window_dirty(WindowClass::WC_STATION_VIEW, WindowNumber(7));
        super::set_window_widget_dirty(WindowClass::WC_VEHICLE_VIEW, WindowNumber(-2), 11);
        super::set_window_classes_dirty(WindowClass::WC_AIRCRAFT_LIST);
        super::invalidate_window_data(WindowClass::WC_TOWN_DIRECTORY, WindowNumber(0), 1, true);
        super::invalidate_window_classes_data(WindowClass::WC_TRAINS_LIST, -3, false);
        super::close_window_by_id(WindowClass::WC_VEHICLE_DEPOT, WindowNumber(-1), true, 4);
        assert_eq!(
            take(),
            [
                Effect::SetWindowDirty(WindowClass::WC_STATION_VIEW, WindowNumber(7)),
                Effect::SetWindowWidgetDirty(WindowClass::WC_VEHICLE_VIEW, WindowNumber(-2), 11),
                Effect::SetWindowClassesDirty(WindowClass::WC_AIRCRAFT_LIST),
                Effect::InvalidateWindowData(
                    WindowClass::WC_TOWN_DIRECTORY,
                    WindowNumber(0),
                    1,
                    true
                ),
                Effect::InvalidateWindowClassesData(WindowClass::WC_TRAINS_LIST, -3, false),
                Effect::CloseWindowById(WindowClass::WC_VEHICLE_DEPOT, WindowNumber(-1), true, 4),
            ]
        );
    }
}
