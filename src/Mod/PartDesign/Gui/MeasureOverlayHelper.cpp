// SPDX-License-Identifier: LGPL-2.1-or-later
#include "MeasureOverlayHelper.h"
#include <cmath>

namespace PartDesignGui
{
double measureDistance(const Base::Vector3d& a, const Base::Vector3d& b)
{
    return (b - a).Length();
}
double measureAngleDeg(const Base::Vector3d& a, const Base::Vector3d& b, const Base::Vector3d& c)
{
    Base::Vector3d ba = a - b;
    Base::Vector3d bc = c - b;
    double la = ba.Length(), lb = bc.Length();
    if (la < 1e-12 || lb < 1e-12) return 0.0;
    double dot = ba.Dot(bc) / (la * lb);
    dot = std::clamp(dot, -1.0, 1.0);
    return std::acos(dot) * 180.0 / M_PI;
}
Base::Vector3d measureBboxDiag(const Base::Vector3d& minPt, const Base::Vector3d& maxPt)
{
    return maxPt - minPt;
}
}
