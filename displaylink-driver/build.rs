use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let evdi_root = manifest_dir.join("..").join("evdi_source");
    let library_dir = evdi_root.join("library");
    let header_path = library_dir.join("evdi_lib.h");

    println!("cargo:rustc-link-search={}", library_dir.display());
    println!("cargo:rustc-link-lib=evdi");
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
