/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file generation_services.hpp Shared generation-cancellation handoff. */
#ifndef RUST_GENERATION_SERVICES_HPP
#define RUST_GENERATION_SERVICES_HPP
#include "../genworld.h"
#include <exception>
/* Native-only throwing operations. These never enter a service table: each
 * caller catches inside its native handoff before returning across Rust. */
static inline void RustSetGeneratingWorldProgress(GenWorldProgress stage, uint total) { SetGeneratingWorldProgress(stage, total); }
static inline void RustIncreaseGeneratingWorldProgress(GenWorldProgress stage) { IncreaseGeneratingWorldProgress(stage); }
template <typename Frame, typename F>
inline bool RunGenerationProgress(Frame *context, F &&progress) noexcept
{
	try { progress(); }
	catch (AbortGenerateWorldSignal &) { context->cancellation = std::current_exception(); return false; }
	return true;
}
#endif /* RUST_GENERATION_SERVICES_HPP */
