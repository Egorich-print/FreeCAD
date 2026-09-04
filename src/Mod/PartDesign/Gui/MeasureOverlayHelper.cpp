// SPDX-License-Identifier: LGPL-2.1-or-later
#include "MeasureOverlayHelper.h"
#include <cmath>
#include <limits>

namespace PartDesignGui
{
double measureDistance(const Base::Vector3d& a, const Base::Vector3d& b)
{
    return (b - a).Length();
}
double measureAngleDeg(const Base::Vector3d& a, const Base::Vector3d& b, const Base::Vector3d& c)
{
    // Angle at A between AB and AC — matches freecad-ux::angle_at (radians) converted to degrees.
    Base::Vector3d ab = b - a;
    Base::Vector3d ac = c - a;
    double la = ab.Length(), lc = ac.Length();
    if (la < 1e-12 || lc < 1e-12) return 0.0;
    double dot = ab.Dot(ac) / (la * lc);
    dot = std::clamp(dot, -1.0, 1.0);
    return std::acos(dot) * 180.0 / M_PI;
}
double measureBboxDiagLen(const Base::Vector3d& minPt, const Base::Vector3d& maxPt)
{
    return (maxPt - minPt).Length();
}
Base::Vector3d measureBboxDiagVec(const Base::Vector3d& minPt, const Base::Vector3d& maxPt)
{
    return maxPt - minPt;
}
double measurePointToSegmentDist(
    const Base::Vector3d& p,
    const Base::Vector3d& a,
    const Base::Vector3d& b)
{
    Base::Vector3d ab = b - a;
    Base::Vector3d ap = p - a;
    double ab_len2 = ab.Dot(ab);
    if (ab_len2 < 1e-18) {
        return (p - a).Length();
    }
    double t = ap.Dot(ab) / ab_len2;
    if (!std::isfinite(t)) {
        return std::numeric_limits<double>::quiet_NaN();
    }
    t = std::clamp(t, 0.0, 1.0);
    Base::Vector3d proj = a + ab * t;
    return (p - proj).Length();
}
}
