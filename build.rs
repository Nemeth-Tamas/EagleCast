use embed_manifest::{embed_manifest, manifest::DpiAwareness, new_manifest};

fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = new_manifest("EagleCast").dpi_awareness(DpiAwareness::System);

        embed_manifest(manifest).expect("failed to embed EagleCast Windows manifest");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
