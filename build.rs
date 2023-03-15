use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    if cfg!(target_os = "windows") {
        let manifest = new_manifest(".manifest");
        embed_manifest(manifest).expect("unable to embed manifest file");
        let mut res = winres::WindowsResource::new();
        res.set_icon_with_id("icon.ico", "ICON");
        res.compile().unwrap();
    }
    //println!("cargo:rerun-if-changed=build.rs");
}
