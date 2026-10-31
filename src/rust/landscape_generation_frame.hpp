/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file landscape_generation_frame.hpp Call-local generation resources. */
#ifndef RUST_LANDSCAPE_GENERATION_FRAME_HPP
#define RUST_LANDSCAPE_GENERATION_FRAME_HPP
#include "landscape_generation_ffi.h"
#include "ground_services.h"
#include "road_command_services.h"
#include "generation_sea_set.hpp"
#include "heightmap_services.hpp"
#include "../framerate_type.h"
#include <exception>
#include <optional>
OpenTTDLandscapeLoopServices RustLandscapeLoopServices(OpenTTDGroundCall *, const OpenTTDRoadCommandContext *) noexcept;
struct RustGenerationLoop {
	PerformanceAccumulator framerate{PFE_GL_LANDSCAPE};
	OpenTTDGroundCall ground;
	RustRoadCommandContext road;
	OpenTTDLandscapeLoopServices services{RustLandscapeLoopServices(&ground, &road.view)};
	OpenTTDGenerationLoop view{GetRustTreeLoopSettings(), &services};
};
struct OpenTTDGenerationFrame {
	std::exception_ptr cancellation;
	RustGenerationSeaSet sea;
	std::optional<RustGenerationLoop> loop;
};

struct OpenTTDLandscapeGenerationCall {
	OpenTTDGenerationFrame frame;
	RustHeightmapInput heightmap;
	OpenTTDLandscapeGenerationSettings settings;
	OpenTTDLandscapeGenerationCall();
};
#endif /* RUST_LANDSCAPE_GENERATION_FRAME_HPP */
