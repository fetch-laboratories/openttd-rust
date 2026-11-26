/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_class.h Header file for classes to be used by e.g. NewGRF stations and airports */

#ifndef NEWGRF_CLASS_H
#define NEWGRF_CLASS_H

#include "strings_type.h"
#include "rust/newgrf_class_ffi.h"

/* Base for each type of NewGRF spec to be used with NewGRFClass. */
template <typename Tindex>
struct NewGRFSpecBase {
	Tindex class_index; ///< Class index of this spec, invalid until class is allocated.
	uint16_t index; ///< Index within class of this spec, invalid until inserted into class.
};

/**
 * Struct containing information relating to NewGRF classes for stations and airports.
 */
template <typename Tspec, typename Tindex, Tindex Tmax>
class NewGRFClass {
public:
	uint32_t global_id; ///< Global ID for class, e.g. 'DFLT', 'WAYP', etc.
	StringID name;    ///< Name of this class.
	uint32_t index;   ///< Index of class within the list of classes, a Tindex held as the shared record's uint32_t.
private:
	using Ops = RustGRFClassOps<Tspec>;
public:
	using spec_type = Tspec;
	using index_type = Tindex;
	std::span<Tspec * const> Specs() const { return {Ops::specs(this->Index()), this->GetSpecCount()}; }
	static std::span<NewGRFClass const> Classes() { return {static_cast<const NewGRFClass *>(Ops::classes()), Ops::count()}; }
	Tindex Index() const { return static_cast<Tindex>(this->index); }
	uint GetSpecCount() const { return Ops::spec_count(this->Index()); }
	uint GetUISpecCount() const requires requires { Ops::ui_spec_count; } { return Ops::ui_spec_count(this->Index()); }
	const Tspec *GetSpec(uint index) const { return Ops::spec(this->Index(), index); }
	static const OpenTTDGRFClassServices<Tspec> &GetServices();
	static void Reset();
	static Tindex Allocate(uint32_t global_id);
	static void Assign(Tspec *spec) requires requires { Ops::assign; } { Ops::assign(spec); }
	static uint GetClassCount() { return Ops::count(); }
	static uint GetUIClassCount() { return Ops::ui_count(); }
	/* The inline accessor over the shared record, as the Rust NewGRFClass::get. */
	static NewGRFClass *Get(Tindex class_index) { assert(static_cast<size_t>(class_index) < Ops::count()); return static_cast<NewGRFClass *>(Ops::classes()) + class_index; }
	static const Tspec *GetByGrf(uint32_t grfid, uint16_t local_id) requires requires { Ops::by_grf; } { return Ops::by_grf(grfid, local_id); }
};

#endif /* NEWGRF_CLASS_H */
