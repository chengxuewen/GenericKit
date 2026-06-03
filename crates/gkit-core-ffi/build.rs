use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = crate_dir.join("generated");

    std::fs::create_dir_all(&out_dir).ok();

    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .unwrap_or_default();

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_dir.join("gkit_core.h"));
}
