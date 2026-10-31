/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file town_services.h Canonical native town lookup services. */
#ifndef OPENTTD_RUST_TOWN_SERVICES_H
#define OPENTTD_RUST_TOWN_SERVICES_H
#include "../town.h"
#include "town_ffi.h"
#include "environment_services.h"

/** One definition for const resolver scans and mutable town destruction tables. */
template <typename TTown>
inline size_t RustTownStorageCount(TTown *town) noexcept { return town->psa_list.size(); }
#endif /* OPENTTD_RUST_TOWN_SERVICES_H */
