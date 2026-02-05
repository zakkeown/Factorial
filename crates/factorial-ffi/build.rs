use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = PathBuf::from(&crate_dir);
    let output_file = output_dir.join("factorial.h");

    // Only regenerate when sources change.
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let config = cbindgen::Config::from_file("cbindgen.toml")
        .expect("failed to read cbindgen.toml");

    match cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(&output_file);
        }
        Err(cbindgen::Error::ParseSyntaxError { .. }) => {
            // During `cargo test` the crate may be compiled as rlib with cfg(test),
            // which cbindgen cannot parse. This is expected and not an error.
            eprintln!("cbindgen: skipping header generation (parse error, likely cfg(test))");
        }
        Err(e) => {
            panic!("cbindgen failed: {e:?}");
        }
    }
}
