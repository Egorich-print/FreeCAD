// SPDX-License-Identifier: LGPL-2.1-or-later
// JointGizmoHelper — M10 Fusion-parity Assembly joints (Ponytail stub, mirrors freecad-ux::joint)
// Provides Gizmo placement for Rigid/Revolute/Slider joints; C++ shim will call freecad-ux via cxx later.

#pragma once

#include <Base/Vector3D.h>
#include <Gui/Inventor/Draggers/Gizmo.h>

namespace AssemblyGui
{

enum class JointGizmoType
{
    Rigid = 0,
    Revolute = 1,
    Slider = 2,
    Coincident = 3,
};

struct JointGizmoPlacement
{
    Base::Vector3d origin;
    Base::Vector3d axis;
    double angleRad = 0.0;
    Base::Vector3d translation{};
};

// Pure math — mirrors freecad-ux/src/joint.rs:translation/axis_angle/is_coincident
// Note: freecad-ux has 4 types (Rigid/Revolute/Slider/Coincident); the full
// Assembly solver has 13 (AssemblyUtils.h) — this helper covers the common subset.
Base::Vector3d jointTranslation(const Base::Vector3d& a, const Base::Vector3d& b);
double jointAxisAngle(const Base::Vector3d& axisA, const Base::Vector3d& axisB);
bool jointIsCoincident(const Base::Vector3d& a, const Base::Vector3d& b, double tol);
// Mirrors freecad-ux::Joint::clamp_limit (AssemblyObject.cpp limit logic, auto-swap min/max).
double jointClampLimit(double value, double min, double max, bool enabledMin, bool enabledMax);

// Gizmo wiring (stub — creates LinearGizmo for translation, RotationGizmo for revolute)
struct JointGizmos
{
    Gui::LinearGizmo* translation = nullptr;
    Gui::RotationGizmo* rotation = nullptr;
};

} // namespace AssemblyGui
