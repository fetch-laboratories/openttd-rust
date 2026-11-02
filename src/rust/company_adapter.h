/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file company_adapter.h Company service tables; portable bodies remain at callers. */
#ifndef RUST_COMPANY_ADAPTER_H
#define RUST_COMPANY_ADAPTER_H
#include "company_ffi.h"
#include "../command_type.h"
const OpenTTDCompanyFinanceServices &GetRustCompanyFinanceServices() noexcept;
const OpenTTDCompanyServices &GetRustCompanyServices() noexcept;
CommandCost RustCompanyCost(const OpenTTDCompanyCost &);
#endif
