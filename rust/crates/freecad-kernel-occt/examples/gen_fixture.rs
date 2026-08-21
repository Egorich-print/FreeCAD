use freecad_kernel::GeometryKernel;
fn main() {
    let mut k = freecad_kernel_occt::OcctBackend::new().unwrap();
    let plate = k.make_box(120.0, 80.0, 12.0).unwrap();
    let boss = k.make_cylinder(18.0, 60.0).unwrap();
    let boss = k.move_by(&boss, 60.0, 40.0, 12.0).unwrap();
    let fused = k.fuse(&plate, &boss).unwrap();
    let hole = k.make_cylinder(8.0, 200.0).unwrap();
    let hole = k.move_by(&hole, 60.0, 40.0, -50.0).unwrap();
    let drilled = k.cut(&fused, &hole).unwrap();
    let bytes = k.write_step(&drilled).unwrap();
    std::fs::write(
        "crates/freecad-kernel-occt/tests/fixtures/demo_part.step",
        bytes,
    )
    .unwrap();
    println!("fixture written");
}
