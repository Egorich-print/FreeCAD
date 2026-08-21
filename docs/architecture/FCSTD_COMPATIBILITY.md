# FCStd Compatibility Investigation

Status: **investigation only — no Rust codec implemented yet** (deliberate,
per mission rule: a bad FCStd implementation is worse than explicit STEP/BREP).
All facts below were read from this repository's sources on 2026-08.

## 1. Container format (verified)

A `.FCStd` file is a plain ZIP archive:

| Aspect | Implementation | Evidence |
|---|---|---|
| Zip read/write | `zipios++` (`ZipFile`, `ZipOutputStream`) | `src/App/ProjectFile.cpp` includes `zipios++/*`; new centralized handler since 2024 |
| Main document XML | entry **`Document.xml`**, Xerces-C DOM parse | `src/App/Document.cpp:1360/:2030` (`writer.putNextEntry("Document.xml")`); `src/App/ProjectFile.cpp:331/:369` |
| GUI state | optional entry **`GuiDocument.xml`** | `src/Gui/Document.cpp:1919/:1950/:1968` |
| Thumbnails | optional PNG under the zip when "SaveThumbnail" enabled | `src/Gui/Document.cpp:1924`, `Thumbnail.h` |
| Geometry payloads | per-object extra files added via `Base::Writer::addFile` | `src/App/Document.cpp:1129` comment re `Writer.addFile`; Part features store OCCT BRep streams |
| Compression | standard deflate through zipios++ | `zipios++` usage in ProjectFile/Writer |

`.FCStd1` is the same content unpacked into a directory tree (documented user
feature; the code paths write through the same `Base::Writer` abstraction).

## 2. Document.xml structure (verified shape, simplified)

```xml
<Document SchemaVersion="…" ProgramVersion="…">   <!-- versioning lives HERE -->
  <Properties>            <!-- document-level props: Meta, License, Uid … -->
    <Property name="Comment" type="App::PropertyString">…</Property>
  </Properties>
  <Object type="PartDesign::Pad" name="Pad" id="NN" />   <!-- registry -->
  <ObjectData DurableName="…">
    <Object name="Pad">
      <Properties>
        <Property name="Label"     type="App::PropertyString">…</Property>
        <Property name="Base"      type="App::PropertyLink">…</Property>
        <Property name="Placement" type="App::PropertyPlacement">
          <PropertyPlacement Px="…" … Q0="…"/>          <!-- custom sub-XML -->
        </Property>
      </Properties>
    </Object>
  </ObjectData>
</Document>
```

Key semantics extracted from `src/App/Document.cpp` + `src/App/*.cpp`:

1. **Two passes**: objects are declared (`<Object>`), then filled
   (`<ObjectData>`). This allows forward references between objects while
   reading sequentially.
2. **Identity**: `name` (unique per document, ASCII-safe internal name),
   numeric `id`, and optional durable name. Links store target by name/id —
   a Rust reader must reproduce this resolution to rebuild the dependency graph.
3. **Property polymorphism**: every property serializes its concrete type as an
   attribute plus type-specific child XML (`PropertyPlacement`,
   `PropertyFloatList`, link lists with sub-elements…). There are ~21 property
   families in `src/App` and workbenches add more (e.g. Sketcher constraints,
   TechDraw views) — **the schema is open-ended by design**.
4. **Unknown-property tolerance**: readers skip unknown properties/types
   (forward compatibility used by FreeCAD itself); a Rust codec must do the same
   to survive documents from newer FreeCAD versions.
5. **Binary-ish payloads**: `Base::Reader` exposes additional files to objects;
   `TopoShape::RestoreDocFile` feeds the stream into `BRepTools::Read`
   (`src/Mod/Part/App/TopoShape.cpp:802/:817`) — i.e. shapes are stored in
   **standard OCCT ASCII BRep format** (`CASCADE Topology V…`), not a private
   encoding. Filenames are generated (`*.brp`).
6. **Expression bindings**: expression engine stores bound expressions inside
   property XML (`ExpressionEngine` property on objects).
7. **Recovery**: crash-recovery snapshots use the same writer machinery
   (`src/App/RecoverySnapshot.cpp`) — a Rust reader that handles normal files
   gets most snapshot support indirectly.

## 3. What a Rust codec must implement (ordered slices)

| Slice | Contents | Risk |
|---|---|---|
| S0 (recommended first) | open zip, parse `Document.xml` shallowly: object list (type/name/id), links, placements; extract `*.brp` entries via OCCT bridge (`read_brep`) | **low** — read-only, no semantic recompute; yields exact frozen geometry of any Part/PartDesign document |
| S1 | property value extraction for display (labels, visibility, colors from GuiDocument) | low-medium |
| S2 | sketch reconstruction (constraints XML → solver input) | high — constraint schema is large |
| S3 | full parametric replay (recompute Pad/Pocket/… through kernel booleans) | highest — must mirror feature semantics incl. toponaming-era quirks |

S0 alone already satisfies the mission's interop goal:
`FreeCAD .FCStd → [zip+XML+brp] → Rust kernel → mesh → wgpu`, without claiming
semantic compatibility. Full `.FCStd` *write* support should only come after S2/S3.

## 4. Explicit non-goals until demonstrated

- Writing `.FCStd` from Rust (risk of corrupting user data > benefit).
- Claiming "FreeCAD compatible": compatibility is per-slice and must be proven
  against a corpus of real documents (the repo has none bundled; tests must ship
  representative `.FCStd` fixtures generated by the C++ app).
- Re-implementing Xerces tolerance edge cases (recovery from truncated zips is
  FreeCAD-specific behaviour in `RecoverySnapshot`/`Document.cpp` error paths).

## 5. Recommendation

Implement S0 behind `freecad-io::fcstd` with its own error taxonomy, gated
behind a feature flag, using only: `zip` crate (or `rc-zip`), a streaming XML
reader (`quick-xml`), and `GeometryKernel::read_brep`. Acceptance test: open a
real `.FCStd` produced by this very fork, list objects, tessellate every shape
payload, render it in the viewer example.
