// SPDX-License-Identifier: LGPL-2.1-or-later
// MeasureOverlayHelper — M11 measure/inspect stub, mirrors freecad-ux::measure (distance/angle/bbox)
#pragma once

#include <Base/Vector3D.h>
#include <array>

namespace PartDesignGui
{
struct MeasureOverlay
{
    double distance = 0.0;
    double angleDeg = 0.0;
    Base::Vector3d bboxDiag{0,0,0};
};

// Pure math mirrors rust/freecad-ux/src/measure.rs
double measureDistance(const Base::Vector3d& a, const Base::Vector3d& b);
double measureAngleDeg(const Base::Vector3d& a, const Base::Vector3d& b, const Base::Vector3d& c); // angle ABC
Base::Vector3d measureBboxDiag(const Base::Vector3d& minPt, const Base::Vector3d& maxPt);
}
