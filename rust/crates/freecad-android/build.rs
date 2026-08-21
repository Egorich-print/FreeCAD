fn main() {
    // libandroid.so provides ANativeWindow_* used by the viewer feature.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-lib=dylib=android");
    }
}
