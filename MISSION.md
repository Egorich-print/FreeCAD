# MISSION — FreeCAD → Fusion 360 Parity (YOLO Autonomous)

> **Автономная миссия**: сделать FreeCAD реально хорошей и удобной альтернативой Autodesk Fusion 360.
> Язык новой кодовой базы — **Rust 2024 (1.98)**, C++ — только как тонкий shim к OCCT/Qt.
> Режим — **YOLO**: идём без остановок, коммитами с документацией, бэкапами и аудитами.

**Repo**: https://github.com/Egorich-print/FreeCAD  
**Local**: `~/ai-workstation/Projects/FreeCAD` (симлинк из `projects/`)  
**Старт**: `yolo-mission-start-20260828-b4f5679` (бэкап `/tmp/freecad-yolo-backup-20260828.tgz`)  
**Агент**: Muse Spark (OpenCode)  
**Current HEAD**: `38b018e` (M5/M6 build fixes applied)

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

### ✅ M5 — Chamfer Radius Picker (частично сделано, нужен допил) ⭐ P0
**Цель**: потянул мышкой — радиус поменялся. Как в Fusion.

- [x] Audit snapshot
- [x] `SingleStep 0.1`, tooltip, `selectNumber()`, validation clamps
- [ ] **Fix**: `secondDistanceGizmo` правильно биндится к `chamferSize2` (не к `chamferSize`)
- [ ] **Fix**: Distinct style: `LinearDraggerStyle::Arrow` (distance1, красный) vs `Sphere` (distance2, оранжевый)
- [ ] **Fix**: `setGizmoPositions()` — per-edge gizmo positions, обновление при `onSelectionChanged`/`currentItemChanged`
- [ ] **Fix**: Не скрывать gizmo при ошибке — красить в красный + оставлять drag (уже частично)
- [ ] **Fix**: `GizmoHelper::getDraggerPlacementFromEdgeAndFace` — добавить `multFactor` коррекцию как у Fillet
- [ ] **Part WB**: Добавить gizmo в `Part/Gui/DlgFilletEdges.cpp` для CHAMFER
- [ ] Rust: `freecad-ux/src/chamfer.rs` — уже есть `ChamferParams`, `drag_to_value`, `validate_chamfer`, `snap_value` ✅
- [ ] Тест: `TestPartDesignGui.py` + ручной: 1 ребро, 5 рёбер, Two distances, Distance+Angle, Flip

**Критерий готовности**: пользователь без мануала понимает, что handle = радиус, тянет и видит live preview.

### ✅ M6 — Smart Sketch Attachment (частично сделано, нужен допил) ⭐ P0
**Цель**: создал скетч на грани — её контур и середины сразу магнитятся, без кнопки External.

- [x] `SketchWorkflow.cpp` — `tryAutoImportFaceEdges()` helper с 80-edge cap
- [x] Вставлен в 3 места: `SketchPreselection::createSketchOnSupport`, `SketchRequestSelection::createSketchAndShowAttachment`
- [x] Batch: `addExternal()` через Python command (undoable, триггерит `rebuildExternalGeometry`)
- [x] Preference: `User parameter:BaseApp/Preferences/Mod/Sketcher/General → SmartExternalEdges (bool, default true)`
- [x] Auto-center sketch origin on face centroid (M15 bonus)
- [ ] **Fix**: `tryAutoImportFaceEdges` — использовать `ExternalGeometry.setValues()` batch вместо Python-хак для производительности
- [ ] **Fix**: Rust `freecad-ux/src/sketch.rs` — `face_edges_to_external()`, `snap_candidates()`, `nearest_snap()`, `ghost_edge_for_snap()` ✅ (уже есть)
- [ ] **Fix**: C++ вызывает Rust через cxx bridge для снап-кандидатов
- [ ] Тест: прямоугольная грань Pad → `ExternalGeometry` = 4 edges, снап к серединам подсвечивается жёлтым

**Критерий**: Fusion-юзерт не ищет кнопку External — она ему не нужна.

### ✅ M7 — Rust UX Core (`freecad-ux`) — **УЖЕ ГОТОВО** ⭐ P1
**Цель**: вся новая UX-логика на Rust 2024, C++ — только вызов.

```
rust/crates/freecad-ux/ — 24 тестов проходят ✅
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

### M8 — Part WB Parity + Fillet/Chamfer Unify ⭐ P1
- Унифицировать `Part/Gui/DlgFilletEdges` → добавить gizmo или проксировать через `ViewProviderPartExt`
- Общий `GizmoHelper::getChamferOffsetProps` для консистентности PartDesign/Part
- Переиспользовать `freecad-ux::chamfer` логику в Part WB

### M9 — Polish как в Fusion ⭐ P1
- **Per-edge gizmo** (`vector<LinearGizmo*>`), клик по ребру → прыжок gizmo
- **Hover highlight** ребер при наведении
- `Shift`/`Ctrl` coarse/fine (уже в `Gizmo.cpp:350`), подсказки `showDraggerHints()`
- **Ghost snap** (ленивый `addExternal` при снапе) — если M6 eager окажется тяжёлым на STEP с 500 рёбрами
- Visual feedback: ghost geometry preview при hover над ребром грани

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

> Идём от M5/M6 допиля до M15 включительно в этом прогоне (без остановок).

1. **M5 completion** — допилить `TaskChamferParameters.cpp` до Fusion-качества (per-edge gizmo, distinct styles, multFactor, Part WB)
2. **M6 completion** — заменить Python-хак на batch `setValues()` в `tryAutoImportFaceEdges`, C++→Rust cxx bridge для снапов
3. **M8** — Part WB chamfer gizmo parity
4. **M9** — Per-edge gizmo, hover highlight, Shift/Ctrl coarse/fine, ghost snap
5. **M10** — Assembly Joints C++ integration + gizmos
6. **M11** — Measure tools C++ integration
7. **M13** — Hybrid snap (eager + ghost), Assembly motion preview
8. **M14** — CI pipeline, full test suite, screencasts
9. **M15** — Release tag, binaries, docs

Каждый шаг — с проверкой, бэкапом и откатом если что-то пошло не так.

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

**Следующие шаги автономно (YOLO без пауз):**
- **M9 remaining** — Per-edge gizmo, Shift/Ctrl coarse/fine, ghost snap
- **M10** — Assembly Joints C++ integration + gizmos (axis/arrow handles, limits UI, motion preview)
- **M11** — Measure tools C++ integration (distance/angle/area overlay)
- **M13** — Hybrid snap (eager + ghost), Assembly motion preview
- **M14** — CI pipeline, full test suite, screencasts
- **M15** — Release `fusion-parity-m15`

---

> *Дальше — код. YOLO.*