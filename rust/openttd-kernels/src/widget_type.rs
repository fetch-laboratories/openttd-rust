/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared widget vocabulary from `widget_type.h`.

/// `WidgetType`, retaining all underlying byte values and push-button bits.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct WidgetType(pub u8);
impl WidgetType {
    /// Simple depressed panel
    pub const WWT_PANEL: Self = Self(1);
    /// Pressed (inset) panel, most commonly used as combo box _text_ area
    pub const WWT_INSET: Self = Self(2);
    /// Frame
    pub const WWT_FRAME: Self = Self(13);
    /// Horizontal container.
    pub const NWID_HORIZONTAL: Self = Self(24);
    /// Horizontal container that doesn't change the order of the widgets for `RTL` languages.
    pub const NWID_HORIZONTAL_LTR: Self = Self(25);
    /// Vertical container.
    pub const NWID_VERTICAL: Self = Self(26);
    /// Matrix container.
    pub const NWID_MATRIX: Self = Self(27);
    /// Stacked widgets, only one visible at a time (eg in a panel with tabs).
    pub const NWID_SELECTION: Self = Self(29);
    /// Layered widgets, all visible together.
    pub const NWID_LAYER: Self = Self(30);
    /// Begin marker for attribute `NWidgetPart` types.
    pub const WPT_ATTRIBUTE_BEGIN: Self = Self(36);
    /// End marker for attribute `NWidgetPart` types.
    pub const WPT_ATTRIBUTE_END: Self = Self(50);
    /// Widget part to denote end of a container.
    pub const WPT_ENDCONTAINER: Self = Self(52);
}
