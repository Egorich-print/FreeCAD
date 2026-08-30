<a href="https://freecad.org"><img src="/src/Gui/Icons/freecad.svg" height="100px" width="100px"></a>

### Your own 3D Parametric Modeler

[Website](https://www.freecad.org) •
[Documentation](https://wiki.freecad.org) •
[Forum](https://forum.freecad.org/) •
[Bug tracker](https://github.com/FreeCAD/FreeCAD/issues) •
[Git repository](https://github.com/FreeCAD/FreeCAD) •
[Blog](https://blog.freecad.org)


[![Release](https://img.shields.io/github/release/freecad/freecad.svg)](https://github.com/freecad/freecad/releases/latest) [![Crowdin](https://d322cqt584bo4o.cloudfront.net/freecad/localized.svg)](https://crowdin.com/project/freecad)

<img src="/.github/images/partdesign.png" width="800"/>

Overview
--------

* **Freedom to build what you want**  FreeCAD is an open-source parametric 3D 
modeler made primarily to design real-life objects of any size. 
Parametric modeling allows you to easily modify your design by going back into 
your model history to change its parameters. 

* **Create 3D from 2D and back** FreeCAD lets you sketch geometry-constrained
 2D shapes and use them as a base to build other objects.
 It contains many components to adjust dimensions or extract design details from 
 3D models to create high quality production-ready drawings.

* **Designed for your needs** FreeCAD is designed to fit a wide range of uses
including product design, mechanical engineering and architecture,
whether you are a hobbyist, programmer, experienced CAD user, student or teacher.

* **Cross platform** FreeCAD runs on Windows, macOS and Linux operating systems.

* **Underlying technology**
    * **OpenCASCADE** A powerful geometry kernel, the most important component of FreeCAD
    * **Coin3D library** Open Inventor-compliant 3D scene representation model
    * **Python** FreeCAD offers a broad Python API
    * **Qt** Graphical user interface built with Qt


Installing
----------

Precompiled packages for stable releases are available for Windows, macOS and Linux on the
[latest releases page](https://github.com/FreeCAD/FreeCAD/releases/latest).

On most Linux distributions, FreeCAD is also directly installable from the 
software center application.

For weekly development releases visit the [releases page](https://github.com/FreeCAD/FreeCAD/releases/).

Other options are described on the [wiki Download page](https://wiki.freecad.org/Download).

Compiling
---------

See the [Developers Handbook – Getting Started](https://freecad.github.io/DevelopersHandbook/gettingstarted/)
for build instructions.


Reporting Issues
---------

To report an issue please:

- Consider posting to the [Forum](https://forum.freecad.org), [Discord](https://discord.com/invite/w2cTKGzccC) channel, or [Reddit](https://www.reddit.com/r/FreeCAD) to verify the issue; 
- Search the existing [issues](https://github.com/FreeCAD/FreeCAD/issues) for potential duplicates; 
- Use the most updated stable or [development versions](https://github.com/FreeCAD/FreeCAD/releases/) of FreeCAD; 
- Post version info from `Help > About FreeCAD > Copy to clipboard`; 
- Restart FreeCAD in safe mode `Help > Restart in safe mode` and try to reproduce the issue again. If the issue is resolved it can be fixed by deleting the FreeCAD config files.
- Start recording a macro `Macro > Macro recording...` and repeat all steps. Stop recording after the issue occurs and upload the saved macro or copy the macro code in the issue; 
- Post a Step-By-Step explanation on how to recreate the issue; 
- Upload an example file (FCStd as ZIP file) to demonstrate the problem; 

For more details see:

- [Bug Tracker](https://github.com/FreeCAD/FreeCAD/issues)
- [Reporting Issues and Requesting Features](https://github.com/FreeCAD/FreeCAD/issues/new/choose)
- [Contributing](https://github.com/FreeCAD/FreeCAD/blob/main/CONTRIBUTING.md)
- [Help Forum](https://forum.freecad.org/viewforum.php?f=3)

> [!NOTE]
The [FPA](https://fpa.freecad.org) offers developers the opportunity
to apply for a grant to work on projects of their choosing. Check
[jobs and funding](https://blog.freecad.org/jobs/) to know more.


Usage & Getting Help
--------------------

The FreeCAD wiki contains documentation on 
general FreeCAD usage, Python scripting, and development.
View these pages for more information:

- [Getting started](https://wiki.freecad.org/Getting_started)
- [Features list](https://wiki.freecad.org/Feature_list)
- [Frequent questions](https://wiki.freecad.org/FAQ/en)
- [Workbenches](https://wiki.freecad.org/Workbenches)
- [Scripting](https://wiki.freecad.org/Power_users_hub)
- [Developers Handbook](https://freecad.github.io/DevelopersHandbook/)

The [FreeCAD forum](https://forum.freecad.org) is a great place
to find help and solve specific problems when learning to use FreeCAD.

Fusion 360 Parity (YOLO Mission)
--------------------------------

This fork is actively becoming a **real Fusion 360 alternative** — see [`MISSION.md`](MISSION.md).

**Already done (2026-08-29, YOLO autonomous, audit-fixed):**
- **Chamfer radius picker** — draggable handles `Arrow/Sphere`, `multFactor` 1/tan(angle/2) with `Precision.hxx` guard, Part WB parity (`TaskChamferParameters.cpp:402`, `DlgFilletEdges.cpp:346`)
- **Smart sketch on face** — face contour auto-import (`tryAutoImportFaceEdges` 80 cap, hybrid `≤20 eager / >80 ghost`), mid/endpoint snap (`freecad-ux` 5%/10%), auto-center on `CentreOfMass` (`SketchWorkflow.cpp:92`)
- **Hover highlight** — `TaskDressUpParameters` keeps gate in `none` mode for pre-select; gizmo follows hovered Edge (`SetPreselect` → `setGizmoForEdge`)
- **Rust UX core** — `rust/crates/freecad-ux` (Rust 2024, 1.98), **25 tests** (`cargo test --workspace`): chamfer drag `NaN` guard, arc 270° `rem_euclid` fix, `HybridPolicy` (Eager/EagerCapped/Ghost)
- Toggles: `Preferences → Sketcher → General → SmartExternalEdges`; gizmo `Shift` coarse (5×) via `freecad-ux::snap_value`
- Scaffolding: `AssemblyGui/JointGizmoHelper` (mirrors `freecad-ux::joint`), `PartDesign/MeasureOverlayHelper`, `Sketcher/GhostSnapOverlay`

**Roadmap** — `MISSION.md` M9 per-edge hover ✅, M10-M11 scaffolding ✅, M13 hybrid ✅, M14 CI (`rust-yolo.yml` 1.98 + `cargo fmt`), M15 `fusion-parity-m15`. Tags: `yolo-m5-9deed36`, `yolo-m8-37620ad`, `yolo-m9-*`, `yolo-m10m11-*`.

---

<p>This project receives generous infrastructure support from
  <a href="https://www.digitalocean.com/">
    <img src="https://opensource.nyc3.cdn.digitaloceanspaces.com/attribution/assets/SVG/DO_Logo_horizontal_blue.svg" width="91px">
  </a> and <a href="https://www.kipro-pcb.com/">KiCad Services Corp.</a>
</p>
