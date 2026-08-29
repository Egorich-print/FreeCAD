// SPDX-License-Identifier: LGPL-2.1-or-later
#include "JointGizmoHelper.h"

#include <cmath>
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
    return (a - b).Length() <= tol;
}

} // namespace AssemblyGui
