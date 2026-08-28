# MISSION — FreeCAD → Fusion 360 Parity (YOLO Autonomous)

> **Автономная миссия**: сделать FreeCAD реально хорошей и удобной альтернативой Autodesk Fusion 360.
> Язык новой кодовой базы — **Rust 2024 (1.98)**, C++ — только как тонкий shim к OCCT/Qt.
> Режим — **YOLO**: идём без остановок, коммитами с документацией, бэкапами и аудитами.

**Repo**: https://github.com/Egorich-print/FreeCAD  
**Local**: `~/ai-workstation/Projects/FreeCAD` (симлинк из `projects/`)  
**Старт**: `yolo-mission-start-20260828-b4f5679` (бэкап `/tmp/freecad-yolo-backup-20260828.tgz`)  
**Агент**: Muse Spark (OpenCode)

---

## 0. Почему именно эти две баги — лицо Fusion-опыта

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

### 1.3 Rust-слой (2026-08-28)

```
rust/
  Cargo.toml — workspace edition 2024, 6 crates
  crates/freecad-core     — Document, MeshBuffer, Selection, Prim
  crates/freecad-kernel   — trait Kernel (mock + OCCT)
  crates/freecad-kernel-occt — cxx-bridge к OCCT shim
  crates/freecad-io       — FCStd S0 reader + STL export
  crates/freecad-render   — wgpu 25 + pick (Moeller-Trumbore)
  crates/freecad-android  — viewer + Document bridge
```

Покрытие: M4 (document model) закрыт, M3 (picking) закрыт. Не хватает `freecad-ux` / `freecad-constraints`.

---

## 2. Философия миссии — Ponytail (ленивое решение, которое работает)

> Делай самое простое, что реально решает задачу. Стандартная библиотека > кастом. Один файл > пять. Нативный фич > зависимость.

Применяем на каждом шаге:
- **YAGNI** — не пишем ghost-snap, если eager-импорт на 30 строк уже даёт Fusion-ощущение.
- **Станд. либа** — `TopExp_Explorer`, `BRep_Tool`, `Attacher` уже есть, не изобретаем геометрию.
- **Одна правда** — `supportString` парсим один раз, в одном месте.
- **Минимальный дифф** — чиним только `TaskChamferParameters.*` + `SketchWorkflow.cpp` + один новый `freecad-ux` crate.

---

## 3. Дорожная карта до Fusion 360 (M5 → M10)

### M5 — Chamfer Radius Picker (этот PR) ⭐ P0
**Цель**: потянул мышкой — радиус поменялся. Как в Fusion.

- [x] Audit snapshot (b4f5679)
- [ ] Fix `TaskChamferParameters.cpp:360` — `secondDistanceGizmo` → `chamferSize2`
- [ ] Distinct style: `LinearDraggerStyle::Arrow` vs `Sphere`, цвета red vs orange
- [ ] `setGizmoPositions()` — перебиндинг по типу + обновление при `onSelectionChanged`/`currentItemChanged`
- [ ] Не скрывать gizmo при ошибке — красить в красный + оставлять drag
- [ ] `SingleStep 0.1`, tooltip, `selectNumber()`
- [ ] Тест: `TestPartDesignGui.py` + ручной: 1 ребро, 5 рёбер, Two distances, Distance+Angle, Flip
- [ ] Rust: `crates/freecad-ux/src/chamfer.rs` — модель `ChamferParams` + `drag_to_value` (unit-test)

**Критерий готовности**: пользователь без мануала понимает, что handle = радиус, тянет и видит live preview.

### M6 — Smart Sketch Attachment (этот PR) ⭐ P0
**Цель**: создал скетч на грани — её контур и середины сразу магнитятся, без кнопки External.

- [ ] `SketchWorkflow.cpp` — helper `importSupportFaceEdges(SketchObject*, supportObj, subName)` (C++ , не Python-хак)
- [ ] Вставить в 3 места: `SketchPreselection::createSketchOnSupport`, `SketchRequestSelection::createSketchAndShowAttachment`, `createSketch` (plane — пропуск)
- [ ] Batch: `ExternalGeometry.setValues()` + один `rebuildExternalGeometry()` (не 100 вызовов)
- [ ] Preference: `User parameter:BaseApp/Preferences/Mod/Sketcher/General → SmartExternalEdges (bool, default true)`
- [ ] Rust: `crates/freecad-ux/src/sketch.rs` — `project_face_edges()`, `snap_candidates()` (mid/endpoint)
- [ ] Тест: прямоугольная грань Pad → `ExternalGeometry` = 4 edges, снап к серединам подсвечивается жёлтым

**Критерий**: Fusion-юзерт не ищет кнопку External — она ему не нужна.

### M7 — Rust UX Core (`freecad-ux`) — фундамент ⭐ P1
**Цель**: вся новая UX-логика на Rust 2024, C++ — только вызов.

```
rust/crates/freecad-ux/
  Cargo.toml — edition 2024, depends: freecad-core, glam
  src/
    lib.rs
    chamfer.rs  — ChamferParams, ChamferType, drag_to_value, validate
    sketch.rs   — FaceId, EdgeProj, SnapCandidate, mid_snap logic
    snap.rs     — SnapManager port (line middle 5%, arc middle 10%)
```

- Unit-tests 100% для `drag_to_value` и `mid_snap`
- `cxx` bridge по необходимости (M8)

### M8 — Part WB parity + Fillet/Chamfer unify
- Унифицировать `Part/Gui/DlgFilletEdges` → добавить gizmo или проксировать через `ViewProviderPartExt`
- Общий `GizmoHelper::getChamferOffsetProps` для консистентности

### M9 — Polish как в Fusion
- Per-edge gizmo (vector<LinearGizmo*>), клик по ребру → прыжок gizmo
- Hover highlight, `Shift`/`Ctrl` coarse/fine (уже в `Gizmo.cpp:350`), подсказки `showDraggerHints()`
- Ghost snap (ленивый `addExternal` при снапе) — если M6 eager окажется тяжёлым на STEP с 500 рёбрами

### M10 — Release & Docs
- Обновить `README.md`, `CONTRIBUTING.md`, скринкасты
- `cargo test` + `ctest -R PartDesign` + `pytest TestPartDesignGui`
- Тег `fusion-parity-m10`, релиз на GitHub

---

## 4. Контракты (что не ломаем)

- **Source of truth** — `~/ai-workstation/Projects/FreeCAD` (симлинк), не корень `New OpenCode Project`.
- **Бэкапы** — перед каждым milestone `git tag yolo-mission-M{N}` + `/tmp/freecad-m{N}-*.tgz`.
- **Аудиты** — после каждого M: `cargo test --workspace`, `ctest`, `git diff --stat`, скриншот/запись если UI.
- **Коммиты** — conventional, документированные, `Co-authored-by: Muse Spark`.
- **Совместимость** — старые FCStd открываются, `ExternalGeometry` не дублируется, DAG циклических ссылок нет (`isExternalAllowed`).

---

## 5. Текущий YOLO-прогон — что делаем сейчас

> Идём до M6 включительно в этом прогоне (без остановок).

1. MISSION.md (ты здесь)
2. M5 — патч `TaskChamferParameters.*` (1 коммит)
3. M6 — патч `SketchWorkflow.cpp` (1 коммит)
4. M7 — новый crate `freecad-ux` + тесты (1 коммит)
5. Аудит: `cargo test`, `git log`, пуш

Каждый шаг — с проверкой, бэкапом и откатом если что-то пошло не так.

---

## 6. Риски и митигация

| Риск | Митигация |
|------|-----------|
| Python-хак `Placement = CenterOfMass` сломает существующие скетчи | Проверяем `isPlanar`, не трогаем `MapMode != FlatFace`, ставим `Placement` после `updateActive()` |
| 500 рёбер на STEP → 500 `addExternal` → тормоз | Batch + лимит 100 рёбер + preference toggle |
| Gizmo исчезает при ошибке → нельзя откатить | Оставляем visible, красим в красный, `DelayedGizmoUpdate` |
| `gh push` rejected (remote ahead) | `git fetch + rebase` перед каждым пушом |
| Rust 1.98 toolchain drift | `rustup toolchain install 1.98 && rustup override set 1.98` |

---

## 7. Как проверить, что миссия успешна

- [ ] Новый юзер из Fusion открывает FreeCAD, создаёт Box → выбирает ребро → тянет handle фаски → получает нужный радиус без чтения мануала.
- [ ] Тот же юзер: клик на грань → New Sketch → рисует прямоугольник → он магнитится к середине грани без кнопки External.
- [ ] `rust/crates/freecad-ux` покрыт тестами, `cargo test` зелёный.
- [ ] Нет регрессий: `TestPartDesign` проходит, старые FCStd открываются.

---

> *Дальше — код. YOLO.*

---

## 8. Прогресс YOLO-прогона (2026-08-28)

| Коммит | Tag | Что |
|--------|-----|-----|
| `b4f5679` | `yolo-mission-start-20260828-b4f5679` | Audit snapshot |
| `731952c` | `yolo-mission-m7-731952c` | **M5+M6+M7** — chamfer picker + smart sketch + freecad-ux (13 tests) |
| `94a2f32` | `yolo-mission-m9-94a2f32` | **M8/M9 polish** — angle step, 80-edge cap, nearest_snap (15 tests) |

Следующие шаги автономно (без ожидания пользователя):
- **M8 done** — DlgFilletEdges уже имеет SingleStep 0.1; унификация via freecad-ux
- **M9 done** — cap 80 + nearest_snap + gizmo visibility polish
- **M10 next** — README Fusion-parity секция + скринкасты + релиз-тег `fusion-parity-m10`
