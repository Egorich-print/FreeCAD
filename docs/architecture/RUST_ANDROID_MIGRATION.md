# FreeCAD → Rust/OCCT/wgpu/Android Migration

Status: **Phase 0 audit complete, Phase 1 in progress** (2026-08)
Scope: repository `Egorich-print/FreeCAD` @ `b2da06b` (26.3.0dev).

Every claim below is tied to actual files in this repository (verified 2026-08),
not to external research documents.

---

## 1. Current architecture (verified)

| Layer | Location | Size (cpp+h) | Role |
|---|---|---|---|
| Base | `src/Base` | ~50k LOC | Utils: streams, XML tools, Console, FileInfo, Observer. Almost Qt-free |
| App | `src/App` | ~92k LOC | Document/Object/Property model, expressions, undo, FCStd I/O, Python embedding |
| Gui | `src/Gui` | ~272k LOC | Qt6 widgets + Coin3D/OpenInventor scene graph + ViewProviders |
| Mod/Part | `src/Mod/Part/App` | ~105k LOC | OCCT wrapper (`TopoShape*`, `TopoShapePy*`), meshing, booleans |
| Mod/Sketcher | `src/Mod/Sketcher/App` | ~48k LOC | Constraints + **vendored `planegcs` solver** (`src/Mod/Sketcher/App/planegcs`) |

Key verified facts:

1. **Document model**: `src/App/Document.cpp`, `DocumentObject.cpp`,
   `DocumentObjectFileIncluded.cpp`; property system = 21 `Property*.h/.cpp`
   families in `src/App`. Recompute scheduling lives in
   `Document::recompute()` (`src/App/Document.cpp:2843`), driven by the
   dependency graph built from `PropertyLink` fields (`src/App/PropertyLinks.cpp`),
   with DAG helpers (`DepEdge.h`, boost graph headers vendored at `src/App/boost_graph_*.hpp`).
2. **The "headless" core is NOT Qt-free**: `src/App` contains 56 `#include <Q...>`
   across ~20 files — including core data structures:
   `MappedName.h` (QByteArray/QHash/QVector), `StringHasher.h`,
   `Document.cpp` (QCryptographicHash), `ApplicationDirectories.cpp` (QStandardPaths).
   `src/Base` has only 2 Qt touches (`Debugger.h`, `FutureWatcherProgress.h`).
   ⇒ A Rust application layer cannot reuse App types directly; it must re-implement
   the *concepts*, not wrap the code.
3. **Coin3D usage is confined to `src/Gui`** (plus per-workbench ViewProvider code):
   monolithic include hub `src/Gui/InventorAll.h` (~40 Inventor headers),
   `View3DInventor*.cpp`, custom nodes under `src/Gui/Inventor/`.
   No `Inventor/` includes exist outside `src/Gui` (verified by grep).
   ⇒ Coin3D is a GUI-layer concern; a non-Gui Rust path needs nothing from it.
4. **OCCT access is funneled through one seam**: `Part::TopoShape`
   (`src/Mod/Part/App/TopoShape.*`, 41 headers referencing `TopoDS`).
   Tessellation already isolated in `src/Mod/Part/App/BRepMesh.{h,cpp}`;
   booleans/healing inside TopoShape methods via `BRepAlgoAPI_*` / `BRepCheck_Analyzer`.
5. **Python embedding lives in `src/App/Application.cpp`**
   (`Py_Initialize` / `PyConfig`), binding layers are threefold:
   hand-written Py objects (`*PyImp.cpp` + `.pyi` stubs), PySide/shiboken for Gui,
   and Boost.Python/pybind11 remnants. `Document::recompute()` explicitly holds
   the GIL across feature execution (comment at `src/App/Document.cpp:2850`).
6. **FCStd container**: ZIP written with `zipios++`, XML parsed with Xerces-C.
   New centralized handler `src/App/ProjectFile.{h,cpp}` (2024 refactor);
   entries: `Document.xml` (`src/App/Document.cpp:1360/:2030`), `GuiDocument.xml`,
   per-object shape files; crash recovery via `src/App/RecoverySnapshot.cpp`.
7. **Headless build already exists as a concept**: top-level CMake option
   `BUILD_GUI` (`CMakeLists.txt:108–133`) — App+Base build without Gui.
8. Fork baseline: FreeCAD 26.3.0dev (`version.json`), OCCT 7.8.x era upstream;
   local dev environment uses Homebrew **OCCT 7.9.3** for the new Rust path.

## 2. Dependency graph (simplified, verified direction of includes)

```text
MainGui/MainCmd ─► Gui ─► App ─► Base
   │                │      │       ▲
   │                │      │       └── zipios++, Xerces-C, zlib, boost(headers)
   │                │      └── Python3 (embed), QtCore(!) containers/crypto
   │                └── Qt6::Widgets/OpenGL, Coin3D(+Quarter), OpenGL desktop
   └─ Mod/* workbenches
        Part        ─► App, OCCT (TKBRep/TKBO/TKBool/TKMesh/TKSTEP…)
        PartDesign  ─► Part
        Sketcher    ─► App, Part(weak), planegcs(vendored, standalone C++)
        TechDraw    ─► Part (HLR via OCCT), Qt graphics scene
        Fem         ─► VTK, smesh/netgen (out of scope for v1 Android)
```

## 3. Identified seams (where cuts are cheap)

| # | Seam | Evidence | Rust-side analogue |
|---|---|---|---|
| S1 | OCCT behind `TopoShape` | `src/Mod/Part/App/TopoShape.*` | `freecad-kernel::GeometryKernel` trait |
| S2 | Tessellation isolated | `src/Mod/Part/App/BRepMesh.cpp` | `kernel.tessellate() -> MeshBuffer` |
| S3 | Headless build switch | `BUILD_GUI` in `CMakeLists.txt` | entire Rust stack is headless by construction |
| S4 | FCStd = ZIP+XML+BREP | `ProjectFile.cpp`, `Document.cpp:1360` | future `freecad-io::fcstd` reader (Phase 2+) |
| S5 | Solver vendored & GUI-free | `src/Mod/Sketcher/App/planegcs` | FFI reuse or port (Phase 3) |
| S6 | STEP/BREP import/export already OCCT-only | `Import`/`Part` XS controllers | first interop target (implemented now) |

## 4. Migration boundaries established (Phase 1)

```text
rust/
├── crates/freecad-core         pure Rust: MeshBuffer, camera math, ids, errors
├── crates/freecad-kernel       GeometryKernel trait (the ONLY geometry contract)
├── crates/freecad-kernel-occt  cxx bridge + minimal handwritten C++ shim
│                               └── cpp/occt_shim.{h,cpp}  ← only place that
│                                   #includes OCCT headers on the Rust side
├── crates/freecad-io           file bytes ↔ kernel (STEP/BREP today)
└── crates/freecad-render       wgpu renderer consuming freecad-core::MeshBuffer
```

Rules enforced (see mission §14): no Qt/Coin3D/Python anywhere under `rust/`;
OCCT reachable only through `freecad-kernel-occt`; renderer never sees OCCT types.

## 5. Risk register

| Risk | Impact | Mitigation |
|---|---|---|
| R1 OCCT ABI/toolchain mismatch (brew macOS vs NDK vs FreeCAD LibPack 7.8.1) | kernel-occt builds against one backend at a time | shim compiles OCCT headers only internally; version pinned per-build; Android gets its own OCCT NDK build (script provided) |
| R2 cxx compile-time blowup from heavy OCCT headers | dev velocity | bridge header uses opaque forward decls only; all `#include <OCCT>` confined to shim .cpp |
| R3 OCCT API drift 7.8→7.9 (stream APIs) | shim rot | use long-stable APIs (`BRepTools::Read(stream)` since 7.6, `STEPControl_Reader::ReadStream`); STEP export via temp file fallback documented |
| R4 triangulation normals absent/interpolated | shading artifacts | shim computes flat triangle normals deterministically |
| R5 wgpu on Android drivers (Adreno/Mali) | Phase-2 device issues | GLES backend fallback; deferred, not blocking desktop proof |
| R6 fork drift vs upstream | merge friction | everything additive under `rust/` + `docs/`; zero edits to existing files so far |

## 6. Phase 1 implementation plan (this branch)

1. `docs:` this document.
2. `build:` Cargo workspace `rust/` (edition 2024), zero-dep `freecad-core`.
3. `feat(kernel):` `GeometryKernel` trait + in-memory mock for tests (Rule 10:
   abstraction must have a concrete user before the OCCT backend lands).
4. `feat(occt):` C++ shim + cxx bridge; build.rs discovers OCCT via
   `brew --prefix opencascade` / env override `OCCT_ROOT`; links TKernel…
5. `test(kernel):` end-to-end: box+sphere → fuse/cut/common → tessellate →
   validate buffers → STEP/BREP round-trip through bytes.
6. `feat(io):` `load_step_bytes/load_brep_bytes` helpers over the trait.
7. `feat(render):` wgpu renderer (orbit camera, depth, lambert lighting,
   resize, offscreen capture proof) consuming `MeshBuffer`; `viewer` example.
8. `build(android):` cargo-ndk scaffold, per-target config, untested-but-
   reproducible OCCT-for-NDK script, honest blocker documentation.

## 7. Decisions rejected (and why)

- **Wrap `App::Document` via FFI now** — rejected: App types pull QtCore
  (see §1.2) and Python; wrapping would smuggle both into Rust. Re-implement
  concepts in `freecad-core/model` when the viewer needs them (Phase 2).
- **bindgen over full OCCT** — rejected: giant unsafe surface violates Rule 9;
  cxx + handwritten shim keeps boundary ≈15 functions.
- **Reuse `BRepMesh.cpp` sources in the shim** — rejected: it is entangled with
  `ComplexGeoData`/`App` types; calling OCCT `BRepMesh_IncrementalMesh` directly
  is smaller and dependency-free.
- **Rhai/WASM scripting now** — rejected: no consumer exists yet (Rule 10).
- **Deleting/disabling any existing FreeCAD code** — rejected by mission rules;
  the C++ app remains untouched and buildable.

---

## 8. Phase 1 outcome (2026-08)

Landed on `main` as additive commits (`rust/`, `docs/` only; zero edits to
existing FreeCAD sources):

| Commit | Contents |
|---|---|
| `docs:` audit | this document |
| `build(rust):` workspace | `freecad-core` (MeshBuffer/ShapeId/validation, zero deps) |
| `feat(kernel):` contract | `GeometryKernel` trait + `MockKernel` reference user |
| `feat(occt):` bridge | cxx + ~420-line shim, registry of `ShapeId`, typed errors |
| `feat(io):` loading | STEP/BREP bytes & paths over the trait |
| `feat(render):` wgpu | orbit camera (unit-verified math), lambert pipeline, GpuMesh with face ranges, offscreen GPU proof, winit viewer example |
| `build(android):` | ARM64 cdylib, verified 16 KB alignment, OCCT NDK recipe |
| `docs:` FCStd | container investigation → slice plan |

Verification matrix (commands actually executed on macOS/arm64):

```text
cargo test --workspace                                  16 passed / 0 failed
cargo test -p freecad-render --features gpu-tests       GPU offscreen proof ok
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo fmt --all -- --check                              clean
cargo check --target aarch64-linux-android              core/kernel/io/render clean
./rust/android/build_rust.sh arm64-v8a                  ELF aarch64 .so, LOAD align 0x4000
```

Discrepancies found while implementing (research-vs-reality notes):

- OCCT **7.9 removed** `BRepBuilderAPI_MakeBox/Sphere`; shim uses
  `BRepPrimAPI_*`. FreeCAD's own LibPack still pins 7.8.x — the shim must keep
  compiling against both when it moves into the C++ app's CI.
- OCCT 7.9 `BRepTools::Read` returns `void` (older bool overload gone).
- `TopExp_Explorer` counts duplicate topology occurrences; unique counts need
  `TopExp::MapShapes` (implemented in the shim).

## 9. What remains for Phase 2

1. OCCT static libs for Android via `rust/android/build_occt_ndk.sh`, then
   `freecad-kernel-occt` under an Android target.
2. Kotlin single-activity shell + JNI surface in `freecad-android`
   (open-bytes → mesh buffers → wgpu surface render); bundled STEP asset.
3. `freecad-io::fcstd` slice S0 (read-only geometry extraction) per
   `FCSTD_COMPATIBILITY.md`, with real-document fixtures.
4. Rust document model v0 (objects, links, placements, display state) sized to
   what the viewer needs — no historical Property zoo yet.
5. Selection/picking pass (face ranges are already carried end-to-end).
6. Wire kernel-occt into FreeCAD's own CI as an optional target so the fork
   keeps proving both worlds build.
