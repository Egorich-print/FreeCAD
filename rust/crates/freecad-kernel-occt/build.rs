use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn occt_root() -> PathBuf {
    if let Ok(root) = env::var("OCCT_ROOT") {
        return PathBuf::from(root);
    }
    if let Ok(output) = Command::new("brew")
        .args(["--prefix", "opencascade"])
        .output()
    {
        if output.status.success() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                return PathBuf::from(prefix);
            }
        }
    }
    PathBuf::from("/opt/homebrew/opt/opencascade")
}

fn header_dir(root: &Path) -> PathBuf {
    let candidate = root.join("include/opencascade");
    if candidate.join("TopoDS_Shape.hxx").exists() {
        return candidate;
    }
    let flat = root.join("include");
    if flat.join("TopoDS_Shape.hxx").exists() {
        return flat;
    }
    panic!(
        "OCCT headers not found under {}.\nSet OCCT_ROOT to an OCCT install prefix \
         (e.g. `brew install opencascade` or an NDK cross-build) and rebuild.",
        root.display()
    );
}

fn lib_dir(root: &Path) -> PathBuf {
    for candidate in [root.join("lib"), root.join("lib64")] {
        if candidate.is_dir() {
            return candidate;
        }
    }
    panic!("OCCT library directory not found under {}", root.display());
}

fn toolkit_exists(lib_dir: &Path, name: &str) -> bool {
    let found_dylib = lib_dir.join(format!("lib{name}.dylib")).exists();
    let found_so = lib_dir.join(format!("lib{name}.so")).exists();
    let found_versioned = lib_dir
        .read_dir()
        .map(|entries| {
            entries.filter_map(Result::ok).any(|entry| {
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                name.starts_with(&format!("lib{name}."))
                    && (name.ends_with(".so") || name.ends_with(".dylib"))
            })
        })
        .unwrap_or(false);
    found_dylib || found_so || found_versioned
}

fn required_toolkits(lib_dir: &Path) -> Vec<&'static str> {
    let mut kits = vec![
        "TKernel",
        "TKMath",
        "TKG2d",
        "TKG3d",
        "TKGeomBase",
        "TKGeomAlgo",
        "TKBRep",
        "TKTopAlgo",
        "TKPrim",
        "TKBO",
        "TKBool",
        "TKShHealing",
        "TKMesh",
        "TKXSBase",
    ];
    if toolkit_exists(lib_dir, "TKDESTEP") {
        kits.push("TKDESTEP");
    } else {
        for legacy in [
            "TKSTEPBase",
            "TKSTEPAttr",
            "TKSTEP209",
            "TKSTEP207",
            "TKSTEP",
        ] {
            if toolkit_exists(lib_dir, legacy) {
                kits.push(legacy);
            }
        }
    }
    kits
}

fn main() {
    let root = occt_root();
    let headers = header_dir(&root);
    let libs = lib_dir(&root);

    let kits = required_toolkits(&libs);
    for kit in &kits {
        assert!(
            toolkit_exists(&libs, kit),
            "OCCT toolkit {kit} not found in {}",
            libs.display()
        );
    }

    let mut build = cxx_build::bridges(["src/lib.rs"]);
    build
        .file("cpp/occt_shim.cpp")
        .include("cpp")
        .include(&headers)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .warnings_into_errors(false)
        .compile("freecad_kernel_occt_cpp");

    println!("cargo:rerun-if-env-changed=OCCT_ROOT");
    println!("cargo:rerun-if-changed=cpp/occt_shim.h");
    println!("cargo:rerun-if-changed=cpp/occt_shim.cpp");
    println!("cargo:rustc-link-search=native={}", libs.display());
    for kit in &kits {
        println!("cargo:rustc-link-lib=dylib={kit}");
    }
}
