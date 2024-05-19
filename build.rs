use std::{os::unix::ffi::OsStringExt, path::Path};

fn gen_inc_path(path: &Path, name: &str) {
    let full_path = path.canonicalize().unwrap();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(
        format!("{out_dir}/{name}"),
        full_path.into_os_string().into_vec(),
    )
    .unwrap();
}

fn main() {
    gen_inc_path(Path::new("targets"), "target_path.inc");
    gen_inc_path(Path::new("configs"), "config_path.inc");

    println!("cargo::rerun-if-changed=targets");
    println!("cargo::rerun-if-changed=configs");
}
