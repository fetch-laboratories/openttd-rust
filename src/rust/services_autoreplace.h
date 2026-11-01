/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_autoreplace.h Shared native engine-renewal observations. */
#ifndef RUST_SERVICES_AUTOREPLACE_H
#define RUST_SERVICES_AUTOREPLACE_H

#include "../autoreplace_base.h"

template <class Handle>
inline uint32_t RustEngineRenewIndex(Handle shell) noexcept { return static_cast<const EngineRenew *>(static_cast<const void *>(shell))->index.base(); }

#endif /* RUST_SERVICES_AUTOREPLACE_H */
