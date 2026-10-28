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
struct RustSessionVehicleCosts { CommandCost replacement, refit, temporary; };
struct OpenTTDSessionFrame {
	std::exception_ptr exception;
	std::optional<PerformanceMeasurer> framerate;
	std::optional<Backup<CompanyID>> company;
	std::optional<PerformanceMeasurer> scripts;
	std::optional<RustGenerationLoop> loop;
	std::optional<RustSessionVehicleCosts> vehicle;
	std::optional<RustRoadCommandContext> road;
	std::optional<PerformanceAccumulator> landscape;
	std::optional<PerformanceMeasurer> schedule;
	void Rethrow() const { if (this->exception) std::rethrow_exception(this->exception); }
};
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
