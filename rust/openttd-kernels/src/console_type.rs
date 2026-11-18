/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Globally used console related types (`console_type.h`).

use crate::gfx_type::TextColour;

/* Colours of the console messages. */
/// Default colour of the console.
pub(crate) const CC_DEFAULT: TextColour = TextColour::TC_SILVER;
/// Colour for warning lines.
pub(crate) const CC_WARNING: TextColour = TextColour::TC_LIGHT_BLUE;
