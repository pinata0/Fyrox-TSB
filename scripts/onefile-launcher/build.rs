fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/ui/icons/erpin.res");
    if std::env::var("TARGET").is_ok_and(|target| target.ends_with("windows-msvc")) {
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/ui/icons/erpin.res");
        println!("cargo:rustc-link-arg={}", icon.display());
    }
}
