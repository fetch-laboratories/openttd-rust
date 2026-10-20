/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry_shell_services.h Canonical native industry shell fields. */
#ifndef RUST_INDUSTRY_SHELL_SERVICES_H
#define RUST_INDUSTRY_SHELL_SERVICES_H
#include "../industry.h"
inline Station *RustIndustryNeutralStation(const Industry *industry) noexcept { return industry->neutral_station; }
#endif /* RUST_INDUSTRY_SHELL_SERVICES_H */
