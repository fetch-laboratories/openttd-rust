/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file linkgraph.cpp Definition of link graph classes used for cargo distribution. */

#include "../stdafx.h"
#include "../core/pool_func.hpp"
#include "linkgraph.h"

#include "../safeguards.h"

/* Initialize the link-graph-pool */
LinkGraphPool _link_graph_pool("LinkGraph");
INSTANTIATE_POOL_METHODS(LinkGraph)
