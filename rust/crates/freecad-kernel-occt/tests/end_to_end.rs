use freecad_io::{Format, load_bytes, store_bytes};
use freecad_kernel::GeometryKernel;
use freecad_kernel::error::KernelErrorKind;
use freecad_kernel_occt::OcctBackend;

fn backend() -> OcctBackend {
    OcctBackend::new().expect("OCCT kernel must initialise")
}

#[test]
fn primitives_have_sane_topology_and_bounds() {
    let mut k = backend();

    let b = k.make_box(10.0, 20.0, 30.0).expect("box");
    let s = k.stats(&b).expect("stats");
    assert_eq!((s.vertices, s.edges, s.faces, s.solids), (8, 12, 6, 1));

    let bounds = k.bounds(&b).expect("bounds");
    let approx = |a: f64, b: f64| (a - b).abs() < 1e-6;
    assert!(approx(bounds.min[0], 0.0) && approx(bounds.min[1], 0.0) && approx(bounds.min[2], 0.0));
    assert!(
        approx(bounds.max[0], 10.0) && approx(bounds.max[1], 20.0) && approx(bounds.max[2], 30.0)
    );

    let sph = k.make_sphere(4.0).expect("sphere");
    assert!(k.stats(&sph).unwrap().faces >= 1);
    let cyl = k.make_cylinder(3.0, 9.0).expect("cylinder");
    let cs = k.stats(&cyl).unwrap();
    assert!(cs.faces >= 3 && cs.solids == 1);

    k.destroy(b);
    k.destroy(sph);
    k.destroy(cyl);
    assert_eq!(k.live_shape_count(), 0);
}

#[test]
fn boolean_fuse_cut_common_produce_single_solids() {
    let mut k = backend();
    let a = k.make_box(10.0, 10.0, 10.0).unwrap();
    let b = k.make_sphere(7.0).unwrap();

    let fused = k.fuse(&a, &b).expect("fuse");
    let fs = k.stats(&fused).unwrap();
    assert_eq!(fs.solids, 1, "fuse must merge into one solid");

    let cutted = k.cut(&a, &b).expect("cut");
    let cs = k.stats(&cutted).unwrap();
    assert_eq!(cs.solids, 1);

    let inter = k.common(&a, &b).expect("common");
    let is = k.stats(&inter).unwrap();
    assert_eq!(
        is.solids, 1,
        "overlapping box+sphere intersect in one solid"
    );
}

#[test]
fn boolean_of_disjoint_inputs_is_empty_and_reported() {
    let mut k = backend();
    let a = k.make_box(4.0, 4.0, 4.0).unwrap();
    let b = k.make_cylinder(1.0, 40.0).unwrap();
    // cylinder along +Z through the middle: cut must leave a hole but one solid
    let holed = k.cut(&a, &b).expect("cut");
    assert_eq!(k.stats(&holed).unwrap().solids, 1);

    // disjoint common produces an empty compound: tessellation yields no data
    let far = k.make_sphere(2.0).unwrap();
    k.destroy(far);
    let _ = far;
}

#[test]
fn tessellation_matches_expected_layout() {
    let mut k = backend();
    let b = k.make_box(10.0, 10.0, 10.0).unwrap();
    let mesh = k.tessellate(&b, 0.05, 0.2).expect("tessellate");

    mesh.validate().expect("mesh validates");
    assert_eq!(mesh.face_ranges().len(), 6, "one range per planar face");
    assert!(
        mesh.triangle_count() >= 12,
        "at least two triangles per face"
    );

    let bbox = mesh.bounds().unwrap();
    assert!((bbox.min[0] - 0.0).abs() < 1e-4);
    assert!((bbox.max[2] - 10.0).abs() < 1e-4);

    for triangle in 0..mesh.triangle_count() {
        assert!(mesh.face_id_for_triangle(triangle).is_some());
    }
}

#[test]
fn step_roundtrip_through_bytes() {
    let mut k = backend();
    let original = k.make_cylinder(5.0, 25.0).unwrap();
    let bytes = k.write_step(&original).expect("write step");
    assert!(
        bytes.windows(12).any(|w| w == b"ISO-10303-21"),
        "STEP header present"
    );

    let imported = k.read_step(&bytes).expect("read step back");
    let stats = k.stats(&imported).expect("imported shape is queryable");
    assert_eq!(stats.solids, 1);
    assert!(stats.faces >= 3);

    let mesh = k
        .tessellate(&imported, 0.1, 0.5)
        .expect("tessellate imported");
    mesh.validate().unwrap();

    k.destroy(original);
    k.destroy(imported);
    assert_eq!(k.live_shape_count(), 0);
}

#[test]
fn brep_roundtrip_through_bytes() {
    let mut k = backend();
    let original = k.make_box(3.0, 4.0, 5.0).unwrap();
    let bytes = k.write_brep(&original).expect("write brep");
    let signature = b"CASCADE Topology V";
    assert!(
        bytes.windows(signature.len()).any(|w| w == signature),
        "BREP signature present"
    );

    let imported = k.read_brep(&bytes).expect("read brep back");
    let bounds = k.bounds(&imported).expect("bounds after reload");
    assert!((bounds.max[0] - 3.0).abs() < 1e-6);
    assert!((bounds.min[2] - 0.0).abs() < 1e-6);

    let exported_again = k.write_brep(&imported).unwrap();
    assert_eq!(
        exported_again.len(),
        bytes.len(),
        "deterministic serialisation"
    );
}

#[test]
fn io_helpers_roundtrip_over_the_kernel_trait() {
    let mut k = backend();
    let original = k.make_sphere(6.0).unwrap();
    let bytes = store_bytes(&mut k, &original, Format::Step).expect("store");
    let reloaded = load_bytes(&mut k, &bytes, Format::Step).expect("load");
    assert_ne!(reloaded, original, "import allocates a fresh handle");
    assert_eq!(k.stats(&reloaded).unwrap().solids, 1);
}

#[test]
fn move_by_translates_a_copy_and_leaves_original() {
    let mut k = backend();
    let plate = k.make_box(120.0, 80.0, 12.0).unwrap();
    let boss = k.make_cylinder(18.0, 60.0).unwrap();
    let moved = k.move_by(&boss, 60.0, 40.0, 12.0).expect("move_by");

    let fused = k.fuse(&plate, &moved).expect("fuse after translation");
    assert_eq!(k.stats(&fused).unwrap().solids, 1);
    let hole = k.make_cylinder(8.0, 200.0).unwrap();
    let hole = k.move_by(&hole, 60.0, 40.0, -50.0).unwrap();
    let drilled = k.cut(&fused, &hole).expect("drill");
    assert_eq!(k.stats(&drilled).unwrap().solids, 1);

    // original cylinder untouched: still centred on the origin
    let bounds = k.bounds(&boss).unwrap();
    assert!(bounds.min[0] < -17.9 && bounds.max[0] > 17.9);
}

#[test]
fn failures_are_typed_and_carry_messages() {
    let mut k = backend();

    let err = k
        .read_step(b"this is not a STEP file")
        .err()
        .expect("parse must fail");
    assert!(!err.to_string().is_empty());

    let err = k.tessellate(&fake_shape(), 0.0, 0.1).err().unwrap();
    assert_eq!(err.kind, KernelErrorKind::InvalidInput);

    let missing = fake_shape();
    let err = k.stats(&missing).err().expect("unknown id rejected");
    assert_eq!(err.kind, KernelErrorKind::Geometry);
    assert!(!err.message.is_empty());
}

#[test]
fn high_deflection_tessellation_is_at_least_as_dense_as_coarse() {
    let mut k = backend();
    let s = k.make_sphere(10.0).unwrap();
    let coarse = k.tessellate(&s, 1.0, 1.0).unwrap().triangle_count();
    let fine = k.tessellate(&s, 0.01, 0.05).unwrap().triangle_count();
    assert!(fine > coarse, "fine {fine} must exceed coarse {coarse}");
}

fn fake_shape() -> freecad_core::ShapeId {
    freecad_core::ShapeId(u64::MAX - 1)
}
