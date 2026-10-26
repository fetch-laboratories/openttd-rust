/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_internal_vehicle.h NewGRF internal processing state for vehicles. */

#ifndef NEWGRF_INTERNAL_VEHICLE_H
#define NEWGRF_INTERNAL_VEHICLE_H

#include "../newgrf.h"
#include "../engine_base.h"
#include "../vehicle_func.h"
#include "newgrf_internal.h"

#ifdef WITH_RUST
struct GRFTempEngineData;
struct RustRailLabelView { RailTypeLabel *data; size_t size; };
extern "C" {
void openttd_rust_engine_temp_resize(size_t);
void openttd_rust_engine_temp_clear();
GRFTempEngineData *openttd_rust_engine_temp_fields(uint16_t);
void openttd_rust_engine_temp_refittability(GRFTempEngineData *, bool);
void openttd_rust_engine_temp_labels_clear(uint16_t);
void openttd_rust_engine_temp_labels_push(uint16_t, uint32_t);
RustRailLabelView openttd_rust_engine_temp_labels(uint16_t);
}
/** Nonowning parser view; resize and clear invalidate every field/label reference. */
struct RustEngineRailLabels {
	uint16_t engine;
	std::span<RailTypeLabel> View() const { auto v = openttd_rust_engine_temp_labels(this->engine); return {v.data, v.size}; }
	void clear() { openttd_rust_engine_temp_labels_clear(this->engine); }
	void push_back(RailTypeLabel label) { openttd_rust_engine_temp_labels_push(this->engine, label); }
	bool empty() const { return this->View().empty(); }
	RailTypeLabel &operator[](size_t index) { return this->View()[index]; }
	auto begin() const { return this->View().begin(); }
	auto end() const { return this->View().end(); }
};
#endif

/** Temporary engine data used when loading only */
struct GRFTempEngineData {
	/** Summary state of refittability properties */
	enum Refittability : uint8_t {
		UNSET    =  0,  ///< No properties assigned. Default refit masks shall be activated.
		EMPTY,          ///< GRF defined vehicle as not-refittable. The vehicle shall only carry the default cargo.
		NONEMPTY,       ///< GRF defined the vehicle as refittable. If the refitmask is empty after translation (cargotypes not available), disable the vehicle.
	};

	CargoClasses cargo_allowed;          ///< Bitmask of cargo classes that are allowed as a refit.
	CargoClasses cargo_allowed_required; ///< Bitmask of cargo classes that are required to be all present to allow a cargo as a refit.
	CargoClasses cargo_disallowed;       ///< Bitmask of cargo classes that are disallowed as a refit.
#ifdef WITH_RUST
	RustEngineRailLabels railtypelabels;
#else
	std::vector<RailTypeLabel> railtypelabels;
#endif
	uint8_t roadtramtype;
	const GRFFile *defaultcargo_grf; ///< GRF defining the cargo translation table to use if the default cargo is the 'first refittable'.
	Refittability refittability;     ///< Did the newgrf set any refittability property? If not, default refittability will be applied.
	uint8_t rv_max_speed;      ///< Temporary storage of RV prop 15, maximum speed in mph/0.8
	CargoTypes ctt_include_mask; ///< Cargo types always included in the refit mask.
	CargoTypes ctt_exclude_mask; ///< Cargo types always excluded from the refit mask.

	/**
	 * Update the summary refittability on setting a refittability property.
	 * @param non_empty true if the GRF sets the vehicle to be refittable.
	 */
	void UpdateRefittability(bool non_empty)
	{
#ifdef WITH_RUST
		openttd_rust_engine_temp_refittability(this, non_empty);
#else
		if (non_empty) {
			this->refittability = NONEMPTY;
		} else if (this->refittability == UNSET) {
			this->refittability = EMPTY;
		}
#endif
	}
};

#ifdef WITH_RUST
struct RustEngineTempView {
	GRFTempEngineData &operator[](EngineID id) { return *openttd_rust_engine_temp_fields(id.base()); }
	void resize(size_t size) { openttd_rust_engine_temp_resize(size); }
	void clear() { openttd_rust_engine_temp_clear(); }
};
extern RustEngineTempView _gted;
#else
extern TypedIndexContainer<std::vector<GRFTempEngineData>, EngineID> _gted;  ///< Temporary engine data used during NewGRF loading
#endif

Engine *GetNewEngine(const GRFFile *file, VehicleType type, uint16_t internal_id, bool static_access = false);
void ConvertTTDBasePrice(uint32_t base_pointer, std::string_view error_location, Price *index);

/**
 * Helper to check whether an image index is valid for a particular NewGRF vehicle.
 * @tparam T The type of vehicle.
 * @param image_index The image index to check.
 * @return True iff the image index is valid, or CUSTOM_VEHICLE_SPRITENUM (use new graphics).
 */
template <VehicleType T>
static inline bool IsValidNewGRFImageIndex(uint8_t image_index)
{
	return image_index == CUSTOM_VEHICLE_SPRITENUM || IsValidImageIndex<T>(image_index);
}

ChangeInfoResult CommonVehicleChangeInfo(EngineInfo *ei, int prop, ByteReader &buf);

#endif /* NEWGRF_INTERNAL_VEHICLE_H */
