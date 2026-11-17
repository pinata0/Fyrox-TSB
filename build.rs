fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=EOS_SDK_DIR");
    #[cfg(feature = "eos")]
    configure_eos();
}

#[cfg(feature = "eos")]
fn configure_eos() {
    use std::{env, fs, path::PathBuf};
    assert_eq!(
        env::var("TARGET").unwrap(),
        "x86_64-pc-windows-msvc",
        "The EOS bridge currently supports Windows x64/MSVC only"
    );
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let sdk = if let Some(path) = env::var_os("EOS_SDK_DIR") {
        let path = PathBuf::from(path);
        if path.join("SDK/Include/eos_sdk.h").is_file() {
            path.join("SDK")
        } else {
            path
        }
    } else {
        // Optional local discovery: keep vendor code outside this repository.
        let tools = root.parent().unwrap().join("tools");
        println!("cargo:rerun-if-changed={}", tools.display());
        let candidates: Vec<_> = fs::read_dir(&tools)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("SDK"))
            .filter(|path| path.join("Include/eos_sdk.h").is_file())
            .collect();
        assert_eq!(candidates.len(), 1, "Set EOS_SDK_DIR to the extracted SDK folder (containing Include, Lib, Bin); automatic discovery requires exactly one SDK under ../tools");
        candidates[0].clone()
    };
    let sdk = sdk.canonicalize().expect("EOS_SDK_DIR does not exist");
    // MSVC's include/link options do not consistently accept verbatim paths.
    let sdk_text = sdk.to_str().expect("EOS SDK path must be valid Unicode");
    let sdk = if let Some(unc) = sdk_text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{unc}"))
    } else {
        PathBuf::from(sdk_text.strip_prefix(r"\\?\").unwrap_or(sdk_text))
    };
    for relative in [
        "Include/eos_sdk.h",
        "Lib/EOSSDK-Win64-Shipping.lib",
        "Bin/EOSSDK-Win64-Shipping.dll",
    ] {
        assert!(sdk.join(relative).is_file(), "Missing EOS file: {relative}");
    }
    println!("cargo:rerun-if-changed=native/eos_bridge.c");
    println!("cargo:rerun-if-changed={}", sdk.join("Include").display());
    println!("cargo:rerun-if-changed={}", sdk.join("Lib").display());
    let runtime = sdk.join("Bin/EOSSDK-Win64-Shipping.dll");
    println!("cargo:rerun-if-changed={}", runtime.display());
    cc::Build::new()
        .file("native/eos_bridge.c")
        .include(sdk.join("Include"))
        .warnings(true)
        .compile("tsb_eos_bridge");
    println!(
        "cargo:rustc-link-search=native={}",
        sdk.join("Lib").display()
    );
    println!("cargo:rustc-link-lib=dylib=EOSSDK-Win64-Shipping");

    // Cargo's OUT_DIR ends in <profile>/build/<package>/out, including custom targets.
    // Both executable and test processes need the SDK DLL alongside their .exe.
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let profile = out.ancestors().nth(3).unwrap();
    for directory in [profile.to_path_buf(), profile.join("deps")] {
        fs::create_dir_all(&directory).expect("Cannot create Cargo output directory");
        let destination = directory.join("EOSSDK-Win64-Shipping.dll");
        if fs::read(&destination).ok() != Some(fs::read(&runtime).expect("Cannot read EOS DLL")) {
            fs::copy(&runtime, destination)
                .expect("Cannot copy EOS DLL; close running EOS programs");
        }
    }
}
