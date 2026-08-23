//! FCStd slice S0 (read-only): open the ZIP container, parse `Document.xml`
//! shallowly (object registry, links, placements), and expose per-object
//! shape payloads (`*.brp`, standard OCCT ASCII BRep) for the kernel.
//!
//! Scope guard (docs/architecture/FCSTD_COMPATIBILITY.md): this is NOT a
//! semantic FreeCAD replacement — no expressions, no recompute, no writing.

use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Placement {
    /// Quaternion components Q0..Q3 (x, y, z, w as stored by FreeCAD).
    pub q: [f64; 4],
    /// Position Px, Py, Pz.
    pub pos: [f64; 3],
}

impl Placement {
    pub fn identity() -> Self {
        Self {
            q: [0.0, 0.0, 0.0, 1.0],
            pos: [0.0; 3],
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FcStdObject {
    /// Internal unique name (e.g. "Pad", "Sketch").
    pub name: String,
    /// Type id (e.g. "PartDesign::Pad", "Part::Feature").
    pub type_id: String,
    pub placement: Option<Placement>,
    /// Shape payload entry name inside the zip (e.g. "Frame.Shape.brp").
    pub shape_file: Option<String>,
    /// Link targets declared as App::PropertyLink properties.
    pub links: Vec<String>,
}

impl FcStdObject {
    pub fn is_shape_bearing(&self) -> bool {
        self.shape_file.is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub struct FcStdDocument {
    pub program_version: String,
    pub objects: Vec<FcStdObject>,
}

impl FcStdDocument {
    pub fn find(&self, name: &str) -> Option<&FcStdObject> {
        self.objects.iter().find(|o| o.name == name)
    }

    pub fn shape_objects(&self) -> impl Iterator<Item = &FcStdObject> {
        self.objects.iter().filter(|o| o.is_shape_bearing())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FcStdError {
    NotAZip,
    NoDocumentXml,
    NoShapePayload,
    Xml(String),
    Io,
}

impl std::fmt::Display for FcStdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FcStdError::NotAZip => f.write_str("payload is not a zip archive"),
            FcStdError::NoDocumentXml => f.write_str("zip lacks Document.xml entry"),
            FcStdError::NoShapePayload => f.write_str("no shape payloads found in document"),
            FcStdError::Xml(m) => write!(f, "Document.xml parse error: {m}"),
            FcStdError::Io => f.write_str("i/o failure"),
        }
    }
}

impl std::error::Error for FcStdError {}

/// Parse `Document.xml` bytes into a shallow document description:
/// object declarations (type/name), then ObjectData pass with placements,
/// Shape payload references and links.
pub fn parse_document_xml(xml: &[u8]) -> Result<FcStdDocument, FcStdError> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut doc = FcStdDocument::default();

    #[derive(Default)]
    struct Ctx {
        in_object_data: bool,
        in_shape_prop: bool,
        cur_obj: Option<FcStdObject>,
        cur_placement: Option<Placement>,
    }
    let mut decl_types: BTreeMap<String, String> = BTreeMap::new();
    let mut ctx = Ctx::default();

    loop {
        let event = reader.read_event_into(&mut buf);
        let event = event.map_err(|e| FcStdError::Xml(e.to_string()))?;
        match event {
            quick_xml::events::Event::Start(e) => {
                let local = String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                if std::env::var("FCXML_DEBUG").is_ok() {
                    println!("START {}", local);
                }
                match local.as_str() {
                    "Document" => {
                        for a in e.attributes().flatten() {
                            if a.key.local_name().as_ref() == b"ProgramVersion" {
                                doc.program_version =
                                    String::from_utf8_lossy(&a.value).into_owned();
                            }
                        }
                    }
                    "Object" if !ctx.in_object_data => {
                        let (mut name, mut type_id) = (String::new(), String::new());
                        for a in e.attributes().flatten() {
                            match a.key.local_name().as_ref() {
                                b"type" => type_id = String::from_utf8_lossy(&a.value).into_owned(),
                                b"name" => name = String::from_utf8_lossy(&a.value).into_owned(),
                                _ => {}
                            }
                        }
                        decl_types.insert(name, type_id);
                    }
                    "ObjectData" => ctx.in_object_data = true,
                    "Object" if ctx.in_object_data => {
                        let mut obj = FcStdObject::default();
                        for a in e.attributes().flatten() {
                            if a.key.local_name().as_ref() == b"name" {
                                obj.name = String::from_utf8_lossy(&a.value).into_owned();
                            }
                        }
                        if obj.type_id.is_empty() {
                            obj.type_id = decl_types.get(&obj.name).cloned().unwrap_or_default();
                        }
                        ctx.cur_obj = Some(obj);
                    }
                    "Property" if ctx.in_object_data => {
                        let mut pname = String::new();
                        let mut ptype = String::new();
                        for a in e.attributes().flatten() {
                            match a.key.local_name().as_ref() {
                                b"name" => pname = String::from_utf8_lossy(&a.value).into_owned(),
                                b"type" => ptype = String::from_utf8_lossy(&a.value).into_owned(),
                                _ => {}
                            }
                        }
                        if ptype == "App::PropertyPlacement" && pname == "Placement" {
                            ctx.cur_placement = Some(Placement::identity());
                        }
                        if pname == "Shape" && ptype == "Part::PropertyPartShape" {
                            ctx.in_shape_prop = true;
                        }
                    }
                    "PropertyPlacement" if ctx.in_object_data => {
                        let mut pl = Placement::identity();
                        for a in e.attributes().flatten() {
                            let key = a.key.local_name();
                            let val = std::str::from_utf8(&a.value).unwrap_or("0");
                            let num: f64 = val.parse().unwrap_or(0.0);
                            match key.as_ref() {
                                b"Px" => pl.pos[0] = num,
                                b"Py" => pl.pos[1] = num,
                                b"Pz" => pl.pos[2] = num,
                                b"Q0" => pl.q[0] = num,
                                b"Q1" => pl.q[1] = num,
                                b"Q2" => pl.q[2] = num,
                                b"Q3" => pl.q[3] = num,
                                _ => {}
                            }
                        }
                        ctx.cur_placement = Some(pl);
                    }
                    "Part" if ctx.in_object_data && ctx.in_shape_prop => {
                        for a in e.attributes().flatten() {
                            if a.key.local_name().as_ref() == b"file"
                                && let Some(obj) = ctx.cur_obj.as_mut()
                            {
                                obj.shape_file =
                                    Some(String::from_utf8_lossy(&a.value).into_owned());
                            }
                        }
                    }
                    _ => {}
                }
            }
            quick_xml::events::Event::End(e) => {
                let local = String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                match local.as_str() {
                    "ObjectData" => ctx.in_object_data = false,
                    "Object" if ctx.in_object_data => {
                        if let Some(mut obj) = ctx.cur_obj.take() {
                            obj.placement = ctx.cur_placement.take();
                            doc.objects.push(obj);
                        }
                    }
                    "Property" if ctx.in_object_data => {
                        // NOTE: keep cur_placement — it belongs to the object
                        // and is consumed at End(Object).
                        ctx.in_shape_prop = false;
                    }
                    _ => {}
                }
            }
            quick_xml::events::Event::Empty(e) => {
                let local = String::from_utf8_lossy(e.name().local_name().as_ref()).into_owned();
                println!(
                    "EMPTY {} in_data={} shape_prop={}",
                    local, ctx.in_object_data, ctx.in_shape_prop
                );
                if local == "PropertyPlacement" && ctx.in_object_data {
                    let mut pl = Placement::identity();
                    for a in e.attributes().flatten() {
                        let key = a.key.local_name();
                        let val = std::str::from_utf8(&a.value).unwrap_or("0");
                        let num: f64 = val.parse().unwrap_or(0.0);
                        match key.as_ref() {
                            b"Px" => pl.pos[0] = num,
                            b"Py" => pl.pos[1] = num,
                            b"Pz" => pl.pos[2] = num,
                            b"Q0" => pl.q[0] = num,
                            b"Q1" => pl.q[1] = num,
                            b"Q2" => pl.q[2] = num,
                            b"Q3" => pl.q[3] = num,
                            _ => {}
                        }
                    }
                    ctx.cur_placement = Some(pl);
                } else if local == "Part" && ctx.in_object_data && ctx.in_shape_prop {
                    for a in e.attributes().flatten() {
                        if a.key.local_name().as_ref() == b"file"
                            && let Some(obj) = ctx.cur_obj.as_mut()
                        {
                            obj.shape_file = Some(String::from_utf8_lossy(&a.value).into_owned());
                        }
                    }
                }
                if local == "Object" && !ctx.in_object_data {
                    let (mut name, mut type_id) = (String::new(), String::new());
                    for a in e.attributes().flatten() {
                        match a.key.local_name().as_ref() {
                            b"type" => type_id = String::from_utf8_lossy(&a.value).into_owned(),
                            b"name" => name = String::from_utf8_lossy(&a.value).into_owned(),
                            _ => {}
                        }
                    }
                    decl_types.insert(name, type_id);
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(doc)
}

/// Open a `.FCStd` payload (zip bytes): parse Document.xml and collect every
/// `*.brp` shape payload keyed by its zip entry name.
pub struct FcStdArchive {
    pub document: FcStdDocument,
    pub shapes: BTreeMap<String, Vec<u8>>,
}

pub fn open_archive(data: &[u8]) -> Result<FcStdArchive, FcStdError> {
    let cursor = std::io::Cursor::new(data);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|_| FcStdError::NotAZip)?;

    let mut xml_bytes = Vec::new();
    {
        let mut xml_entry = zip
            .by_name("Document.xml")
            .map_err(|_| FcStdError::NoDocumentXml)?;
        xml_entry
            .read_to_end(&mut xml_bytes)
            .map_err(|_| FcStdError::Io)?;
    }
    let document = parse_document_xml(&xml_bytes)?;

    let names: Vec<String> = zip.file_names().map(str::to_owned).collect();
    let mut shapes = BTreeMap::new();
    for name in names {
        if name.ends_with(".brp") {
            let mut payload = Vec::new();
            if zip
                .by_name(&name)
                .map_err(|_| FcStdError::Io)?
                .read_to_end(&mut payload)
                .is_ok()
            {
                shapes.insert(name, payload);
            }
        }
    }

    Ok(FcStdArchive { document, shapes })
}

impl FcStdArchive {
    /// Shape bytes for an object, resolved via its Shape property file ref.
    pub fn shape_of(&self, obj: &FcStdObject) -> Option<&[u8]> {
        self.shapes
            .get(obj.shape_file.as_ref()?)
            .map(|v| v.as_slice())
    }
}
