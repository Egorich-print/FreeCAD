use freecad_io::fcstd;

fn repo_rel(p: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../")
        .join(p)
}

#[test]
fn scan_repo_fixtures() {
    let candidates = [
        "data/tests/ProjectTest.FCStd",
        "data/examples/draft_test_objects.FCStd",
        "data/examples/EngineBlock.FCStd",
        "tests/src/Mod/PartDesign/App/TestModels/TwoLengthsPadWithExpression.FCStd",
    ];
    for rel in candidates {
        let path = repo_rel(rel);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                println!("MISSING  {rel}");
                continue;
            }
        };
        match fcstd::open_archive(&bytes) {
            Ok(archive) => {
                let objs = archive.document.objects.len();
                let shape_objects = archive.document.shape_objects().count();
                let with_pl = archive
                    .document
                    .objects
                    .iter()
                    .filter(|o| o.placement.is_some())
                    .count();
                println!("OK {rel}: objects={objs} shapes={shape_objects} placements={with_pl}");
            }
            Err(e) => println!("FAIL {rel}: {e}"),
        }
    }
}
