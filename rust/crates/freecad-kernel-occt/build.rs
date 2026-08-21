use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn target_os() -> String {
    env::var("CARGO_CFG_TARGET_OS").unwrap_or_default()
}

fn is_android() -> bool {
    target_os() == "android"
}

fn occt_root() -> PathBuf {
    if let Ok(root) = env::var("OCCT_ROOT") {
        return PathBuf::from(root);
    }
    if is_android() {
        panic!(
            "Android build requires OCCT_ROOT pointing at an NDK-built OCCT install \
             (see rust/android/build_occt_ndk.sh)"
        );
    }
    let brew_prefix = Command::new("brew")
        .args(["--prefix", "opencascade"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|prefix| !prefix.is_empty());
    if let Some(prefix) = brew_prefix {
        return PathBuf::from(prefix);
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
         and rebuild.",
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

/// Toolkits of the freecad kernel shim in **dependency order**
/// (dependents first, so static archives resolve left to right).
const TOOLKITS: &[&str] = &[
    "TKDESTEP",
    "TKDE",
    "TKXSBase",
    "TKMesh",
    "TKShHealing",
    "TKBool",
    "TKBO",
    "TKPrim",
    "TKTopAlgo",
    "TKGeomAlgo",
    "TKBRep",
    "TKGeomBase",
    "TKG3d",
    "TKG2d",
    "TKMath",
    "TKernel",
];

/// Legacy (pre-7.8) names mapped onto the modern list for desktop distros.
fn legacy_name(kit: &str) -> Option<&'static str> {
    match kit {
        "TKDESTEP" => Some("TKSTEP"),
        "TKDE" => None,
        _ => None,
    }
}

fn archive_exists(lib_dir: &Path, name: &str) -> bool {
    lib_dir.join(format!("lib{name}.a")).exists()
}

fn dylib_exists(lib_dir: &Path, name: &str) -> bool {
    if lib_dir.join(format!("lib{name}.dylib")).exists() || lib_dir.join(format!("lib{name}.so")).exists() {
        return true;
    }
    lib_dir
        .read_dir()
        .map(|entries| {
            entries.filter_map(Result::ok).any(|entry| {
                let file_name = entry.file_name();
                let name_owned = file_name.to_string_lossy();
                name_owned.starts_with(&format!("lib{name}."))
                    && (name_owned.ends_with(".so") || name_owned.ends_with(".dylib"))
            })
        })
        .unwrap_or(false)
}

fn main() {
    let android = is_android();
    let root = occt_root();
    let headers = header_dir(&root);
    let libs = lib_dir(&root);

    let mut kits: Vec<String> = Vec::new();
    for kit in TOOLKITS {
        let resolved = if android {
            // NDK install layout only carries the modern 7.9 names.
            if archive_exists(&libs, kit) {
                kit.to_string()
            } else {
                panic!("static OCCT toolkit {kit}.a not found in {}", libs.display());
            }
        } else {
            match (dylib_exists(&libs, kit), legacy_name(kit).is_some_and(|l| dylib_exists(&libs, l))) {
                (true, _) => kit.to_string(),
                (false, true) => legacy_name(kit).unwrap().to_string(),
                (false, false) => continue, // optional kit absent on this distro
            }
        };
        kits.push(resolved);
    }
    assert!(
        !kits.is_empty(),
        "no OCCT toolkits resolved in {}",
        libs.display()
    );

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

    let kind = if android { "static" } else { "dylib" };
    for kit in &kits {
        println!("cargo:rustc-link-lib={kind}={kit}");
    }

    if android {
        // libc++/libc++abi come from the NDK clang++ driver used as rustc
        // linker (see rust/android/build_rust.sh); `log` covers OCCT's
        // Android diagnostics.
        println!("cargo:rustc-link-lib=dylib=log");
    }
}
