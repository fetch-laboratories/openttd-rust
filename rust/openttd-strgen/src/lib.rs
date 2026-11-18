/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Ports of the string generator (`src/strgen`). For now this holds only the master file's
//! string-ID assignment, which `openttd-kernels`' build script runs to define `table::strings`
//! while C++ strgen still writes `table/strings.h`; the binary holds the two to the same values.

pub mod strgen_base;
