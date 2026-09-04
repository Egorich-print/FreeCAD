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

// Pure math mirrors rust/freecad-ux/src/measure.rs (angle_at at A, bbox diagonal length)
double measureDistance(const Base::Vector3d& a, const Base::Vector3d& b);
/// Angle at A between AB and AC, in degrees (matches freecad-ux::angle_at which returns radians).
double measureAngleDeg(const Base::Vector3d& a, const Base::Vector3d& b, const Base::Vector3d& c);
/// BBox diagonal length (matches freecad-ux::bbox_diagonal scalar).
double measureBboxDiagLen(const Base::Vector3d& minPt, const Base::Vector3d& maxPt);
/// BBox diagonal vector (convenience, not in Rust).
Base::Vector3d measureBboxDiagVec(const Base::Vector3d& minPt, const Base::Vector3d& maxPt);
/// Closest distance point → segment ab (matches freecad-ux::point_to_segment_dist).
double measurePointToSegmentDist(
    const Base::Vector3d& p,
    const Base::Vector3d& a,
    const Base::Vector3d& b);
}
