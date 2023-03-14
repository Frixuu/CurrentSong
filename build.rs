use std::env;

use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = new_manifest(".manifest");
        embed_manifest(manifest).expect("unable to embed manifest file");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
