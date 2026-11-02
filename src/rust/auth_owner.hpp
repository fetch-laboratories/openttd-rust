/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file auth_owner.hpp RAII for Rust authentication owners. */
#ifndef OPENTTD_RUST_AUTH_OWNER_HPP
#define OPENTTD_RUST_AUTH_OWNER_HPP

#include "auth_ffi.h"
#include <memory>

template <typename T, void (*Destroy)(T *)>
class RustAuthOwner {
	struct Deleter {
		void operator()(T *value) const noexcept { Destroy(value); }
	};
	std::unique_ptr<T, Deleter> value;
public:
	explicit RustAuthOwner(T *value) : value(value) {}
	RustAuthOwner(const RustAuthOwner &) = delete;
	RustAuthOwner &operator=(const RustAuthOwner &) = delete;
	T *get() const noexcept { return this->value.get(); }
};

using RustAuthHandler = RustAuthOwner<OpenTTDAuthHandler, openttd_rust_auth_handler_destroy>;
using RustAuthStream = RustAuthOwner<OpenTTDAuthStream, openttd_rust_auth_stream_destroy>;

#endif /* OPENTTD_RUST_AUTH_OWNER_HPP */
