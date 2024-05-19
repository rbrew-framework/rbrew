use std::{os::unix::ffi::OsStringExt, path::Path};

fn main() {
    let path = Path::new("targets");
    let full_path = path.canonicalize().unwrap();
    std::fs::write(
        "generated/target_path.inc",
        full_path.into_os_string().into_vec(),
    )
    .unwrap();

    println!("cargo::rerun-if-changed=generated");
}
