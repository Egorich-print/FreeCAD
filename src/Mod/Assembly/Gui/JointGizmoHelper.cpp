// SPDX-License-Identifier: LGPL-2.1-or-later
#include "JointGizmoHelper.h"

#include <cmath>
#include <utility>
#include <Gui/Inventor/Draggers/Gizmo.h>

namespace AssemblyGui
{

Base::Vector3d jointTranslation(const Base::Vector3d& a, const Base::Vector3d& b)
{
    return a - b;
}

double jointAxisAngle(const Base::Vector3d& axisA, const Base::Vector3d& axisB)
{
    double la = axisA.Length();
    double lb = axisB.Length();
    if (la < 1e-12 || lb < 1e-12) {
        return 0.0;
    }
    double dot = axisA.Dot(axisB);
    double c = dot / (la * lb);
    c = std::clamp(c, -1.0, 1.0);
    return std::acos(c);
}

bool jointIsCoincident(const Base::Vector3d& a, const Base::Vector3d& b, double tol)
{
    if (!std::isfinite(tol) || tol < 0.0) {
        return false;
    }
    return (a - b).Length() <= tol;
}

double jointClampLimit(double value, double min, double max, bool enabledMin, bool enabledMax)
{
    if (!std::isfinite(value)) {
        return 0.0;
    }
    double lo = min, hi = max;
    if (std::isfinite(lo) && std::isfinite(hi) && lo > hi) {
        std::swap(lo, hi);
    }
    double v = value;
    if (enabledMin && std::isfinite(lo) && v < lo) {
        v = lo;
    }
    if (enabledMax && std::isfinite(hi) && v > hi) {
        v = hi;
    }
    return v;
}

} // namespace AssemblyGui
