// SPDX-License-Identifier: LGPL-2.1-or-later
#include "GhostSnapOverlay.h"
namespace SketcherGui
{
std::string ghostEdgeForSnap(const std::string& /*faceName*/, double /*cursorX*/, double /*cursorY*/, double /*maxDist*/)
{
    // Stub — actual impl calls freecad-ux via cxx in M13 next sprint.
    // For now, freecad-ux::ghost_edge_for_snap is tested in Rust (24 tests) and used via M6 eager path.
    return {};
}
} // namespace SketcherGui
