/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_cargo.h Shared cargo leaves for synchronous Rust services. */
#ifndef RUST_SERVICES_CARGO_H
#define RUST_SERVICES_CARGO_H
#include "../cargotype.h"
#include "../economy_func.h"
#include "../newgrf_cargo.h"
#include "../newgrf_spritegroup.h"
#include "../newgrf.h"
#include "../industrytype.h"
#include "services_newgrf.h"
struct OpenTTDIndustrySpecHandle;

/* Keep native types and source observation points. Scaling may draw Random;
 * delivery may reenter owners, so callers end their owner access first.
 * Environmental failures terminate inside these noexcept leaves. */
inline uint16_t RustCargoSpecClasses(const CargoSpec *cs) noexcept { return cs->classes.base(); }
inline uint8_t RustCargoBitnum(CargoType cargo) noexcept { return CargoSpec::Get(cargo)->bitnum; }
inline uint16_t RustCargoMultiplier(CargoType cargo) noexcept { return CargoSpec::Get(cargo)->multiplier; }
inline uint16_t RustCargoClasses(CargoType cargo) noexcept { return RustCargoSpecClasses(CargoSpec::Get(cargo)); }
inline uint OpenTTDScaleCargo(uint amount, bool town) noexcept { return ScaleByCargoScale(amount, town); }
inline CargoType OpenTTDCargoTranslation(uint8_t cargo, const GRFFile *grf) noexcept { return GetCargoTranslation(cargo, grf); }
inline CargoType RustHouseCargoTranslation(uint16_t house, uint8_t cargo) noexcept { return OpenTTDCargoTranslation(cargo, RustHouseGrf(house)); }
inline CargoType RustResolverCargoTranslation(const ResolverObject *resolver, uint8_t cargo) noexcept { return OpenTTDCargoTranslation(cargo, RustResolverGRFFile(resolver)); }
/* Raw inverse-map reads retain the native file and cargo-index preconditions;
 * they do not apply GetCargoTranslation's version or nullable-file policy. */
inline uint8_t RustGRFCargoMap(const GRFFile *file, CargoType cargo) noexcept { return file->cargo_map[cargo]; }
template <typename Spec>
inline uint8_t RustSpecCargoMap(const Spec *spec, CargoType cargo) noexcept { return RustGRFCargoMap(RustSpecGRFFile(spec), cargo); }
inline CargoType OpenTTDCargoByLabel(CargoLabel label) noexcept { return GetCargoTypeByLabel(label); }
inline uint OpenTTDMoveGoodsToStation(CargoType cargo, uint amount, Source source, const StationList &stations, Owner exclusivity = INVALID_OWNER) noexcept
{
	return MoveGoodsToStation(cargo, amount, source, stations, exclusivity);
}
/* Legacy tables specialize the shared meanings without another native copy. */
inline uint OpenTTDScaleTownCargo(uint amount) noexcept { return OpenTTDScaleCargo(amount, true); }
inline uint OpenTTDScaleIndustryCargo(uint amount) noexcept { return OpenTTDScaleCargo(amount, false); }
inline bool OpenTTDIsValidCargo(CargoType cargo) noexcept { return IsValidCargoType(cargo); }
inline CargoType OpenTTDFoodCargo() noexcept { return OpenTTDCargoByLabel(CT_FOOD); }
inline CargoType OpenTTDMailCargo() noexcept { return OpenTTDCargoByLabel(CT_MAIL); }
inline CargoType OpenTTDPassengerCargo() noexcept { return OpenTTDCargoByLabel(CT_PASSENGERS); }
inline CargoType OpenTTDIndustryCargoTranslation(const OpenTTDIndustrySpecHandle *spec, uint8_t cargo) noexcept { return OpenTTDCargoTranslation(cargo, RustSpecGRFFile(reinterpret_cast<const IndustrySpec *>(spec))); }
inline CargoType OpenTTDIndustryTileCargoTranslation(const IndustryTileSpec *spec, uint8_t cargo) noexcept { return OpenTTDCargoTranslation(cargo, RustSpecGRFFile(spec)); }
inline bool OpenTTDCargoSpecValid(CargoType cargo) noexcept { return CargoSpec::Get(cargo)->IsValid(); }
#endif /* RUST_SERVICES_CARGO_H */
