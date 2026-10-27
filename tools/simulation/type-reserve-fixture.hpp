/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** Allocate fixture labels through the production Action0 reservation owner. */
#include "newgrf/newgrf_internal.h"
#include "core/backup_type.hpp"
template <GrfSpecFeature Feature>
static uint8_t ReserveFixtureType(uint32_t label)
{
#ifdef WITH_RUST
	GRFFile file;
	Backup<GRFFile *> current(_cur_gps.grffile, &file);
	const std::array<uint8_t, 4> bytes{uint8_t(label >> 24), uint8_t(label >> 16), uint8_t(label >> 8), uint8_t(label)};
	ByteReader reader(bytes.data(), bytes.size());
	if (GrfChangeInfoHandler<Feature>::Reserve(0, 1, 8, reader) != CIR_SUCCESS) std::abort();
	current.Restore();
	if constexpr (Feature == GSF_RAILTYPES) return file.railtype_map[0];
	if constexpr (Feature == GSF_ROADTYPES) return file.roadtype_map[0];
	if constexpr (Feature == GSF_TRAMTYPES) return file.tramtype_map[0];
#else
	if constexpr (Feature == GSF_RAILTYPES) return AllocateRailType(label);
	if constexpr (Feature == GSF_ROADTYPES) return AllocateRoadType(label, RTT_ROAD);
	if constexpr (Feature == GSF_TRAMTYPES) return AllocateRoadType(label, RTT_TRAM);
#endif
}
