/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file session_native.hpp Stack lifetimes for simulation-session calls. */
#ifndef RUST_SESSION_NATIVE_HPP
#define RUST_SESSION_NATIVE_HPP
#include "session_ffi.h"
#include "landscape_generation_frame.hpp"
#include "company_adapter.h"
#include "company_services.h"
#include "environment_services.h"
#include "presentation_services.h"
#include "settings_services.h"
#include "newgrf_storage_ffi.h"
#include <exception>
static_assert(GM_MENU == 0 && GM_NORMAL == 1 && GM_EDITOR == 2 && GM_BOOTSTRAP == 3);
static_assert(SM_NONE == 0 && SM_NEWGAME == 1 && SM_RESTARTGAME == 2 && SM_RELOADGAME == 3 && SM_EDITOR == 4 && SM_LOAD_GAME == 5 && SM_MENU == 6 && SM_SAVE_GAME == 7 && SM_SAVE_HEIGHTMAP == 8 && SM_GENRANDLAND == 9 && SM_LOAD_SCENARIO == 10 && SM_START_HEIGHTMAP == 11 && SM_LOAD_HEIGHTMAP == 12 && SM_RESTART_HEIGHTMAP == 13 && SM_JOIN_GAME == 14);
static_assert(PauseMode::Normal == PauseMode(0) && PauseMode::SaveLoad == PauseMode(1) && PauseMode::Join == PauseMode(2) && PauseMode::Error == PauseMode(3) && PauseMode::ActiveClients == PauseMode(4) && PauseMode::GameScript == PauseMode(5) && PauseMode::LinkGraph == PauseMode(6) && PauseMode::CommandDuringPause == PauseMode(7));
static_assert(FT_SAVEGAME == 1 && FT_SCENARIO == 2 && FT_HEIGHTMAP == 3 && SL_OK == 0 && SL_REINIT == 2 && COLOUR_END == 16);
struct OpenTTDWorldSetupFrame { std::exception_ptr cancellation; };
struct RustSessionVehicleCosts { CommandCost replacement, refit, temporary; };
struct OpenTTDSessionFrame {
	std::exception_ptr exception;
	std::optional<PerformanceMeasurer> framerate;
	std::optional<Backup<CompanyID>> company;
	std::optional<PerformanceMeasurer> scripts;
	std::optional<RustGenerationLoop> loop;
	std::optional<RustSessionVehicleCosts> vehicle;
	std::optional<OpenTTDWorldSetupFrame> setup;
	std::optional<RustRoadCommandContext> road;
	std::optional<PerformanceAccumulator> landscape;
	std::optional<PerformanceMeasurer> schedule;
	void Rethrow() const { if (this->exception) std::rethrow_exception(this->exception); }
};
static_assert(sizeof(OpenTTDSessionFrame) <= sizeof(OpenTTDSessionFrameStorage));
static_assert(alignof(OpenTTDSessionFrame) <= alignof(OpenTTDSessionFrameStorage));
struct OpenTTDSessionLoad {
	const std::string &filename;
	SaveLoadOperation operation;
	DetailedFileType type;
	Subdirectory subdir;
	std::shared_ptr<LoadFilter> filter;
};
/* Script VMs, scheduled console commands, load/generation and networking can
 * unwind in ordinary play. Each named service records that exception here. */
template <typename F>
inline bool SessionException(OpenTTDSessionFrame *frame, F &&operation) noexcept
{
	try { operation(); }
	catch (...) { frame->exception = std::current_exception(); return false; }
	return true;
}
#endif
