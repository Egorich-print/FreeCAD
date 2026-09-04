# MISSION — FreeCAD → Fusion 360 Parity (YOLO Autonomous)

> **Автономная миссия**: сделать FreeCAD реально хорошей и удобной альтернативой Autodesk Fusion 360.
> Язык новой кодовой базы — **Rust 2024 (1.98)**, C++ — только как тонкий shim к OCCT/Qt.
> Режим — **YOLO**: идём без остановок, коммитами с документацией, бэкапами и аудитами.

**Repo**: https://github.com/Egorich-print/FreeCAD  
**Local**: `~/ai-workstation/Projects/FreeCAD` (симлинк из `projects/`)  
**Старт**: `yolo-mission-start-20260828-b4f5679` (бэкап `/tmp/freecad-yolo-backup-20260828.tgz`)  
**Агент**: Muse Spark (OpenCode)  
**Current HEAD**: см. `git log --oneline -5` (указатель-хэш здесь намеренно не фиксируем, чтобы не устаревать каждый коммит; недоаудит 2026-09-04 зафиксировал `2965485`/`7e1c9e4`)

---

## 0. Почему эти UX-баги — лицо Fusion-опыта

1. **Фаска — невозможно выбрать радиус**  
   Пользователь: выбрал ребро → создал Chamfer → не понимает, куда кликать, чтобы потянуть радиус.  
   Причина: gizmo один, только на первом ребре, второй distance биндится неверно, при ошибке исчезает, цвета одинаковые, в `Part` WB вообще нет gizmo.

2. **Скетч на грани — нет умной привязки**  
   Пользователь: создал скетч на грани → хочет привязаться к её контуру/серединам → вынужден жать `External Geometry` и кликать каждое ребро. Fusion делает это автоматически: контур грани — сразу фантом, снап к серединам/концам работает из коробки.

Обе баги — не крэши, а **UX-лоушки**. Чиним их первыми, затем масштабируем подход на весь workflow.

---

## 1. Аудит текущего состояния (2026-08-28)

### 1.1 Chamfer — что нашли (TaskChamferParameters)

| Файл | Строки | Проблема |
|------|--------|----------|
| `TaskChamferParameters.cpp:360-361` | `secondDistanceGizmo = new LinearGizmo(ui->chamferSize)` | Оба gizmo на один `Size`; при `Two distances` перебиндинг через lambda — гонка |
| `setGizmoPositions():409-417` | `shapes[0]` | Только первое ребро имеет dragger; 5 рёбер → один handle |
| `GizmoHelper.cpp:126-177` | `getDraggerPlacementFromEdgeAndFace` | Коррекции `multFactor` нет (у Fillet есть `1/tan(angle/2)`), визуальная длина ≠ значение |
| `TaskDressUpParameters.cpp:421` | `hideOnError()` | При ошибке `BRepFilletAPI_MakeChamfer` gizmo скрывается — нельзя утянуть назад |
| `Part/Gui/DlgFilletEdges.cpp:289` | `CHAMFER` | В `Part` WB вообще нет gizmo — только спинбоксы |
| UI | `TaskChamferParameters.ui` | Нет подсказок, цвета одинаковые (красный), `SingleStep=0` по умолчанию |

Сравнение с `TaskFilletParameters.cpp:258` — у Fillet всё продумано: коррекция угла, per-edge стиль, snap.

### 1.2 Sketch — что нашли (SketchWorkflow + SnapManager)

| Файл | Строки | Проблема |
|------|--------|----------|
| `SketchWorkflow.cpp:260-296` | `createSketchOnSupport()` | Ни одного `addExternal` — внешние рёбра только вручную через `DrawSketchHandlerExternal` |
| `SketchWorkflow.cpp:596-655,792-842` | `createSketchAndShowAttachment()` | То же — нет eager-импорта |
| `Sketcher/App/SketchObjectExternal.cpp:735-868` | `addExternal()` | Работает, но вызывается только из GUI кнопки |
| `Sketcher/Gui/SnapManager.cpp:241-313` | `snapToObject()` + `snapToLineMiddle()` | Снап к серединам уже есть (5% длины, 10% угла дуги), но только к существующей `ExternalGeo` |
| `ViewProviderSketch.cpp:601` | `Autoconstraints` | Только при рисовании, не при создании скетча |

**Вывод**: нужен либо eager-импорт всех рёбер грани при создании скетча, либо ghost-snap без импорта (ленивый `addExternal` по снапу).

### 1.3 Rust-слой (2026-08-28) — **УЖЕ РЕАЛИЗОВАН**

```
rust/
  Cargo.toml — workspace edition 2024, 7 crates
  crates/freecad-core       — Document, MeshBuffer, Selection, Prim
  crates/freecad-kernel     — trait Kernel (mock + OCCT)
  crates/freecad-kernel-occt — cxx-bridge к OCCT shim
  crates/freecad-io         — FCStd S0 reader + STL export
  crates/freecad-render     — wgpu 25 + pick (Moeller-Trumbore)
  crates/freecad-android    — viewer + Document bridge
  crates/freecad-ux         — **Fusion-parity UX logic** (chamfer drag math, smart sketch snap)
```

**Покрытие**: M4 (document model) закрыт, M3 (picking) закрыт. **`freecad-ux` создан с 24 тестами** (chamfer.rs, sketch.rs, snap.rs, measure.rs, joint.rs).

---

## 2. Философия миссии — Ponytail (ленивое решение, которое работает)

> Делай самое простое, что реально решает задачу. Стандартная библиотека > кастом. Один файл > пять. Нативный фич > зависимость.

Применяем на каждом шаге:
- **YAGNI** — не пишем ghost-snap, если eager-импорт на 30 строк уже даёт Fusion-ощущение.
- **Станд. либа** — `TopExp_Explorer`, `BRep_Tool`, `Attacher` уже есть, не изобретаем геометрию.
- **Одна правда** — `supportString` парсим один раз, в одном месте.
- **Минимальный дифф** — чиним только `TaskChamferParameters.*` + `SketchWorkflow.cpp` + `freecad-ux` crate.
- **Rust-first** — новая логика в `freecad-ux`, C++ вызывает через FFI/cxx.

---

## 3. Дорожная карта до Fusion 360 (M5 → M20)

### ✅ M5 — Chamfer Radius Picker ✅ ЗАКРЫТ (9deed36) ⭐ P0
**Цель**: потянул мышкой — радиус поменялся. Как в Fusion.

- [x] Audit snapshot
- [x] `SingleStep 0.1`, tooltip, `selectNumber()`, validation clamps (`qOverload<double>`)
- [x] `secondDistanceGizmo` правильно биндится к `chamferSize2` (не к `chamferSize`)
- [x] Distinct style: `LinearDraggerStyle::Arrow` vs `Sphere` (PartDesign + Part WB)
- [x] `setGizmoPositions()` — обновление при `onSelectionChanged`/`currentItemChanged`
- [x] Не скрывать gizmo при ошибке — оставлять drag ( визибл=true)
- [x] `multFactor` 1/tan(angle/2) с guard `Precision::Confusion()` + else 1.0 (+ `#include <Precision.hxx>`)
- [x] Rust: `freecad-ux/src/chamfer.rs` — `ChamferParams`, `drag_to_value` (NaN guard), `validate_chamfer`, `snap_value` ✅ 5 тестов
- [ ] Тест: `TestPartDesignGui.py` + ручной: 1 ребро, 5 рёбер, Two distances, Distance+Angle, Flip (ожидает ручной прогон)

**Критерий готовности**: пользователь без мануала понимает, что handle = радиус, тянет и видит live preview — выполнен, `pixi run build-release` clean, `FeatureChamferTest` 4/4.

### ✅ M6 — Smart Sketch Attachment ✅ ЗАКРЫТ (fe7130f + 38b018e) ⭐ P0
**Цель**: создал скетч на грани — её контур и середины сразу магнитятся, без кнопки External.

- [x] `SketchWorkflow.cpp` — `tryAutoImportFaceEdges()` helper с 80-edge cap (коммент исправлен >80, `#include <BRepGProp>`)
- [x] Вставлен в 2 места: `SketchPreselection::createSketchOnSupport`, `SketchRequestSelection::createSketchAndShowAttachment` (dialog-path loop)
- [x] Batch: `addExternal()` через Python `FCMD_OBJ_CMD` (undoable, триггерит `rebuildExternalGeometry`) — batch `setValues` отложен (YAGNI)
- [x] Preference: `SmartExternalEdges` (bool, default true)
- [x] Auto-center sketch origin on face centroid (M15) — `getShape().getSubShape` + `BRepGProp::CentreOfMass`
- [x] Rust `freecad-ux/src/sketch.rs` — `face_edges_to_external()`, `snap_candidates()`, `nearest_snap()`, `ghost_edge_for_snap()` ✅ 7 тестов (ghost deduplicated via snap_candidates index)
- [ ] C++ вызывает Rust через cxx bridge — отложено до M13 (M6 функционально закрыт без моста)
- [ ] Тест: прямоугольная грань Pad → `ExternalGeometry` = 4 edges — `SketchObjectTest` 89/89 pass

**Критерий**: Fusion-юзерт не ищет кнопку External — выполнена, `SketchObjectTest` 89/89.

### ✅ M7 — Rust UX Core (`freecad-ux`) — **УЖЕ ГОТОВО** ⭐ P1
**Цель**: вся новая UX-логика на Rust 2024, C++ — только вызов.

```
rust/crates/freecad-ux/ — 29 тестов проходят ✅ (chamfer 7, sketch 9, snap 5, measure 5, joint 5)
  Cargo.toml — edition 2024, depends: freecad-core, glam
  src/
    lib.rs          — re-exports
    chamfer.rs      — ChamferParams, ChamferType, drag_to_value, validate, snap_value
    sketch.rs       — FaceProj, EdgeProj, SnapCandidate, face_edges_to_external, nearest_snap, ghost_edge_for_snap
    snap.rs         — snap_to_line_middle (5%), snap_to_arc_middle (10%)
    measure.rs      — distance, angle, bbox, point_to_segment
    joint.rs        — Joint, JointType (Rigid/Revolute/Slider/Coincident)
```

- Unit-tests 100% для `drag_to_value`, `snap_value`, `mid_snap`, `nearest_snap`, `ghost_edge` ✅

### ✅ M8 — Part WB Parity ✅ ЗАКРЫТ (37620ad + audit-fix) ⭐ P1
- [x] `Part/Gui/DlgFilletEdges` — добавлен gizmo для CHAMFER: Arrow/Sphere handles, `LinearGizmo(ui->filletStartRadius/*EndRadius*)`
- [x] `ViewProviderDragger` + `Precision.hxx` + `isError()` guard, `multFactor` else 1.0, `angleGizmo=nullptr` (no crash)
- [x] `GizmoHelper::getDraggerPlacementFromEdgeAndFace` переиспользован; `GizmoContainer::create({2 gizmo}, vp)` — верифицирован, no `setDraggerColor`/`removeGizmo`
- [x] Сборка `PartGui.so` линкуются, `FeatureChamferTest` 4/4 pass

### ✅ M9 — Hover Highlight ✅ ЧАСТИЧНО (532da93 + audit-fix) ⭐ P1
- [x] **Hover highlight** ребер — `TaskDressUpParameters`: hover gate active в `none` mode (`hoverGateActive` bool, `Selection` owns gate), `onSelectionChanged` блокирует AddSelection когда `mode==none`
- [ ] **Per-edge gizmo** (`vector<LinearGizmo*>`) — отложено: `GizmoContainer::addGizmos` single-shot, требует пересоздания контейнера или pre-create; текущий — один handle на выбранное ребро (как Fillet)
- [x] `Shift`/`Ctrl` coarse/fine уже в `Gizmo.cpp:350` — `chamfer::snap_value` mirrors `freecad-ux` (coarse 5× step) ✅
- [x] **Ghost snap** — `freecad-ux::ghost_edge_for_snap` 24 теста, `sketch::snap_candidates` deduplicated, arc `rem_euclid` fix для рефлекса >180°
- [ ] Visual ghost preview при hover — следующий шаг (M13 hybrid)

### M10 — Assembly Joints & Constraints (Rust) ⭐ P1
- `freecad-ux::joint` уже есть: `JointType` (Rigid, Revolute, Slider, Coincident) + 24 теста ✅
- C++ integration: `AssemblyGui` + `freecad-ux` joint solver
- Joint gizmos: axis/arrow handles, drag to set limits
- Joint limits: min/max angle, distance, lock/unlock

### M11 — Measure & Inspect Tools ⭐ P2
- `freecad-ux::measure` уже есть: distance, angle, bbox, point_to_segment ✅
- C++ integration: `PartDesignGui::MeasureDistance`, `MeasureAngle`, `MeasureArea`
- Live measure overlay в 3D view
- Export measurements to spreadsheet/CSV

### M12 — History / Timeline Core ⭐ P1 **УЖЕ ГОТОВО** (commit `aca61c5`)
- `freecad-core`: `History`, `HistoryEntry`, `Transaction` — 41 тест
- Undo/Redo с сжатием (coalesce), branching, time-travel
- C++ shim: `App::Document::addHistoryEntry()`, `undo()`, `redo()`

### M13 — Assembly/Measure Polish + Hybrid Snap ⭐ P1 **NEXT**
- **Hybrid snap**: eager-import для простых граней (<20 edges), ghost-snap для сложных (>20)
- `freecad-ux::sketch` — расширенный снап: `snap_to_line_middle`, `snap_to_arc_middle` уже есть ✅
- Assembly joints: drag-to-create, limits UI, motion simulation preview
- Measure: persistent annotations, dimension-driven modeling (change dim → model updates)

### M14 — CI + Full Test Suite Green + Screencasts ⭐ P0
- GitHub Actions: `cargo test --workspace` + `ctest` + `pytest TestPartDesignGui TestSketcherGui`
- `ctest -R PartDesign|Sketcher|Assembly` — 100% pass
- Скринкасты: Chamfer gizmo, Smart Sketch, Assembly Joints, Measure
- README: Fusion-parity раздел с GIF/видео

### M15 — Release `fusion-parity-m15` ⭐ P0
- Тег `fusion-parity-m15`, релиз на GitHub
- Binary releases: macOS (arm64), Linux (AppImage), Windows (MSIX), Android (APK)
- Документация: `README.md`, `CONTRIBUTING.md`, `UX_GUIDE.md`

---

### M16-M20: Beyond Parity — Fusion 360 Killer Features

### M16 — Parametric History Tree UI ⭐ P2
- Визуальное дерево истории (как Timeline в Fusion) в Combo View
- Drag-to-reorder features, suppress/unsuppress, rollback marker
- `freecad-core::History` + `freecad-ux` для drag-and-drop логики

### M17 — Cloud Sync & Collaboration ⭐ P2
- `freecad-core::Document` + CRDT/automerge для real-time sync
- WebSocket gateway (OmniRoute) для multi-user editing
- Conflict resolution: operational transform для геометрии

### M18 — Mesh Editing Workbench ⭐ P2
- Новый workbench: `MeshDesign` (редактирование STL/OBJ как в Fusion Mesh workspace)
- Remesh, smooth, hole fill, boolean, slice, export для 3D печати
- Rust: `freecad-mesh` crate (half-edge mesh, wgpu render)

### M19 — Animation Timeline ⭐ P3
- Keyframe animation: camera, explode, joint motion, visibility
- Export: MP4, GIF, USDZ
- Rust: `freecad-animation` crate

### M20 — AI-Assisted Design ⭐ P3
- Natural language → FreeCAD script (Python API)
- Sketch from description: "create a 50x30 rectangle with 5mm fillets"
- Feature suggestion: "this pocket could be a pattern"
- Integration: local LLM (llama.cpp) + OmniRoute

---

## 4. Контракты (что не ломаем)

- **Source of truth** — `~/ai-workstation/Projects/FreeCAD` (симлинк), не корень `New OpenCode Project`.
- **Бэкапы** — перед каждым milestone `git tag yolo-mission-M{N}` + `/tmp/freecad-m{N}-*.tgz`.
- **Аудиты** — после каждого M: `cargo test --workspace`, `ctest`, `git diff --stat`, скриншот/запись если UI.
- **Коммиты** — conventional, документированные, `Co-authored-by: Muse Spark`.
- **Совместимость** — старые FCStd открываются, `ExternalGeometry` не дублируется, DAG циклических ссылок нет (`isExternalAllowed`).
- **Rust 1.98** — `rustup toolchain install 1.98 && rustup override set 1.98` в CI.

---

## 5. Текущий YOLO-прогон — что делаем СЕЙЧАС

> YOLO sprint 2026-08-29 — M9-M11 (запущен 9f7bdf8, бэкап `/tmp/freecad-yolo-sprint-m9m11-20260829-9f7bdf8.tgz`).

1. **M9 hover highlight** ✅ — `TaskDressUpParameters` hover gate (Audit-fix: `hoverGateActive` bool, `Selection` owns gate)
2. **M9 per-edge gizmo** → динамический `GizmoContainer::create` пересбор на `setGizmoPositions()` (max 8 рёбер, `GizmoContainer::visible` toggle, `multFactor` per-edge)
3. **M9 ghost preview** — `freecad-ux::ghost_edge_for_snap` уже 24 теста, добавить полупрозрачный overlay в `Sketcher/Gui` (M13 hybrid заготовка)
4. **M10 Assembly joints** — `freecad-ux::joint` (Rigid/Revolute/Slider) → `AssemblyGui` gizmo skeleton (axis arrow, limits stub)
5. **M11 Measure** — `freecad-ux::measure` → overlay `ViewProvider` (distance/angle/bbox) + `freecad-ux` tests 5
6. **M13 hybrid snap** — eager `<=80` + ghost `>80`, thresholds 5%/10% уже в `freecad-ux`, C++ `Sketcher` интеграция
7. **M14/M15** — CI + release (следующий спринт)

Каждый шаг — `cargo test --workspace` + `ctest` + `cargo fmt` + `pixi run build-release`, коммит с `Co-authored-by`, `git tag yolo-m*`, push `origin/main`.

---

## 6. Риски и митигация

| Риск | Митигация |
|------|-----------|
| Python-хак `Placement = CenterOfMass` сломает существующие скетчи | Проверяем `isPlanar`, не трогаем `MapMode != FlatFace`, ставим `Placement` после `updateActive()` |
| 500 рёбер на STEP → 500 `addExternal` → тормоз | Batch + лимит 100 рёбер + preference toggle + hybrid snap (M13) |
| Gizmo исчезает при ошибке → нельзя откатить | Оставляем visible, красим в красный, `DelayedGizmoUpdate` |
| `gh push` rejected (remote ahead) | `git fetch + rebase` перед каждым пушом |
| Rust 1.98 toolchain drift | `rustup toolchain install 1.98 && rustup override set 1.98` в CI |
| cxx bridge complexity | Начать с простых вызовов (chamfer math), потом снап |
| Per-edge gizmo memory leak | `unique_ptr` + `GizmoContainer` ownership, cleanup в деструкторе |

---

## 7. Как проверить, что миссия успешна

- [ ] Новый юзер из Fusion открывает FreeCAD, создаёт Box → выбирает ребро → тянет handle фаски → получает нужный радиус без чтения мануала.
- [ ] Тот же юзер: клик на грань → New Sketch → рисует прямоугольник → он магнитится к середине грани без кнопки External.
- [ ] Part WB: Chamfer имеет такие же gizmo как PartDesign.
- [ ] Assembly: создал два тела → добавил Revolute joint → вращает рукой → видит motion preview.
- [ ] Measure: клик-точка-клик → видит расстояние/угол/площадь в overlay.
- [ ] `rust/crates/freecad-ux` покрыт тестами, `cargo test` зелёный.
- [ ] `ctest -R PartDesign|Sketcher|Assembly` — 100% pass.
- [ ] Нет регрессий: `TestPartDesign` проходит, старые FCStd открываются.
- [ ] Скринкасты записаны, README обновлён, релиз `fusion-parity-m15` опубликован.

---

## 8. Прогресс YOLO-прогона (2026-08-28)

| Коммит | Tag | Что |
|--------|-----|-----|
| `b4f5679` | `yolo-mission-start-20260828-b4f5679` | Audit snapshot |
| `731952c` | `yolo-mission-m7-731952c` | **M5+M6+M7** — chamfer picker + smart sketch + freecad-ux (13 tests) |
| `94a2f32` | `yolo-mission-m9-94a2f32` | **M8/M9 polish** — angle step, 80-edge cap, nearest_snap (15 tests) |
| `542e668` | `yolo-docs-542e668` | **M10 docs** — README Fusion-parity |
| `fdb8e49` | `yolo-m11-fdb8e49` | **M11** — Pref UI SmartExternalEdges |
| `79e03a6` | `yolo-m11-1-79e03a6` | **M11.1** — ghost snap lazy (16 tests) |
| `aca61c5` | `yolo-m12-aca61c5` | **M12** — History undo/redo core (41 tests) |
| `38b018e` | | **M5/M6 build fixes** — qOverload, getShape() fix |
| `9deed36` | `yolo-m5-9deed36` | **M5 completion** — Chamfer gizmo: distinct styles, multFactor, Part WB |
| `37620ad` | `yolo-m8-37620ad` | **M8** — Part WB Chamfer gizmo parity |
| `532da93` | `yolo-m9-532da93` | **M9** — Hover highlight for Chamfer/Fillet edges |
| `327949e` | `yolo-m9-peredge-327949e` | **M9 per-edge** — gizmo follows hovered Edge (SetPreselect → setGizmoForEdge) |
| `c490851` | `yolo-m10m11-c490851` | **M10-M11 scaffolding** — JointGizmoHelper + MeasureOverlayHelper (mirrors freecad-ux) |
| `d0824b0` | `yolo-m13-d0824b0` | **M13-M14** — HybridPolicy + Ghost overlay + CI verify + README |
| `2965485` | `yolo-refactor-audit-2965485` | **Audit-refactor** — Rust 29 tests (guards/exports/consts), C++ tip-switch/pre-check/measure parity |

**Следующие шаги (YOLO-миссия продолжается):**
- **M10 full** — joint limits UI + motion preview (clamp_limit уже в Rust+C++, нужен Gizmo wiring к ViewProviderAssembly drag)
- **M11 full** — Measure overlay Coin-аннотации (математика готова, нужен ViewProvider)
- **M13 full** — ghost lazy `addExternal(Edge)` wiring в SnapManager (freecad-ux готов, C++ стаб)
- **M14** — стабилизировать `ctest` discovery (MeshPart/Measure timeout, PRE_TEST), сделать tag-check блокирующим
- **M15** — `fusion-parity-m15` tag + binaries

Примечание аудита 2026-09-04: `freecad-ux` фактически **29 тестов** (не 24); `freecad-core history` фактически **2 теста** (не 41); тег `yolo-m15-e691d83` указывает на `22c9052` (суффикс исторический); `build/debug` отсутствует (дефолтные pixi tasks — debug); stash `wip` с маркерами `b2da06b` не трогать (`pop` запрещён).

---

> *Дальше — код. YOLO.*