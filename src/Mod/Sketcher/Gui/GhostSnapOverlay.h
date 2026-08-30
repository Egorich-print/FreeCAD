// SPDX-License-Identifier: LGPL-2.1-or-later
// M13 GhostSnapOverlay — transparent preview of face edge when cursor snaps without ExternalGeometry.
// Ponytail: 30-line stub, mirrors freecad-ux::ghost_edge_for_snap; full Coin overlay in next sprint.

#pragma once
#include <string>
namespace SketcherGui
{
// Returns edge name to ghost-highlight for cursor; empty if no snap. Mirrors rust freecad-ux.
std::string ghostEdgeForSnap(const std::string& faceName, double cursorX, double cursorY, double maxDist);
} // namespace SketcherGui
