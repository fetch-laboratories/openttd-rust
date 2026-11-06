/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file signs_base.h Base class for signs. */

#ifndef SIGNS_BASE_H
#define SIGNS_BASE_H

#include "signs_type.h"
#include "viewport_type.h"
#include "rust/pool.hpp"
#include "rust/owned_bytes.h"
#include "company_type.h"

#include "abi-record-signs_type.h"
extern "C" {
void openttd_rust_sign_pool_bind(RustPoolCore *);
OpenTTDSignRecord *openttd_rust_sign_record_new(Sign *, uint16_t);
void openttd_rust_sign_record_delete(OpenTTDSignRecord *);
}
using SignPool = RustPool<Sign, SignID, 16>;
extern SignPool _sign_pool;

struct Sign : SignPool::PoolItem<&_sign_pool> {
	struct RecordDeleter {
		void operator()(OpenTTDSignRecord *record) const
		{
			_sign_pool.RetireRecord(record->index.base(), record->shell, record);
			openttd_rust_sign_record_delete(record);
		}
	};
	std::unique_ptr<OpenTTDSignRecord, RecordDeleter> record{openttd_rust_sign_record_new(this, this->index.base())};
	TrackedViewportSign sign{};
	OpenTTDSignRecord &Record() { return *this->record; }
	const OpenTTDSignRecord &Record() const { return *this->record; }
	std::string_view Name() const { return RustBytesView(this->record->name); }
	void SetName(std::string_view name) { RustBytesAssign(this->record->name, name); }
	Sign() { _sign_pool.BindRecord(this->index.base(), this, this->record.get()); }
	Sign(Owner owner, int32_t x, int32_t y, int32_t z, std::string_view name) : Sign()
	{
		this->SetName(name);
		this->Record().x = x; this->Record().y = y; this->Record().z = z; this->Record().owner = owner;
	}
	~Sign();
	void UpdateVirtCoord();
};

#endif /* SIGNS_BASE_H */
