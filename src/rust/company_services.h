/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file company_services.h Shared current-company context for synchronous Rust callbacks. */
#ifndef RUST_COMPANY_SERVICES_H
#define RUST_COMPANY_SERVICES_H
#include "../company_type.h"
#include "../company_func.h"
#include "../company_base.h"
#include "company_ffi.h"

inline bool OpenTTDCompanyIsValid(uint8_t company) noexcept { return Company::IsValidID(CompanyID(company)); }
inline CompanyID OpenTTDCurrentCompany() noexcept { return _current_company; }
inline void OpenTTDSetCurrentCompany(CompanyID company) noexcept { _current_company = company; }
inline uint32_t RustCompanyRenewReserve(const Company *company) noexcept { return company->settings.engine_renew_money; }
inline uint16_t RustCompanyAircraftServiceInterval(CompanyID owner) noexcept { return Company::Get(owner)->settings.vehicle.servint_aircraft; }
inline uint8_t RustCurrentCompanyID() noexcept { return OpenTTDCurrentCompany().base(); }
inline void RustSetCurrentCompanyID(uint8_t company) noexcept { OpenTTDSetCurrentCompany(CompanyID(company)); }
inline uint8_t RustLocalCompanyID() noexcept { return _local_company.base(); }
/** Shared pool traversal preserves the native ascending ID order. */
inline Company *RustCompanyNext(uint32_t from) noexcept
{
	for (Company *c : Company::Iterate(from)) return c;
	return nullptr;
}
inline OpenTTDCompanyRef RustCompanyRefNext(uint32_t from) noexcept
{
	Company *c = RustCompanyNext(from);
	return c == nullptr ? OpenTTDCompanyRef{nullptr, UINT32_MAX} : OpenTTDCompanyRef{&c->Finances(), c->index.base()};
}
inline uint32_t RustNextCompanyID(uint32_t from) noexcept
{
	Company *c = RustCompanyNext(from);
	return c == nullptr ? UINT32_MAX : c->index.base();
}
inline uint64_t RustCompanyRailTypes(uint8_t id) noexcept { return Company::Get(CompanyID{id})->avail_railtypes.base(); }
inline uint64_t RustCompanyRoadTypes(uint8_t id) noexcept { return Company::Get(CompanyID{id})->avail_roadtypes.base(); }
inline bool RustIsInteractiveCompany(uint8_t id) noexcept { return IsInteractiveCompany(CompanyID{id}); }
#endif /* RUST_COMPANY_SERVICES_H */
