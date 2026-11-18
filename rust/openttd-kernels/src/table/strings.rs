/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `table/strings.h`: every string ID of the master language file and `STR_LAST_STRINGID`.
//!
//! The build script derives them from `src/lang/english.txt` with `openttd-strgen`, as strgen
//! assigns them, and every `CMake` build checks them against the `table/strings.h` its strgen wrote.
//! Import each constant by name.

use crate::strings_type::StringID;

include!(concat!(env!("OUT_DIR"), "/strings.rs"));
