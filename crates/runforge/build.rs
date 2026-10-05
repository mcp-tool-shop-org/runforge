fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }
    let manifest =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
            .join("app.manifest");
    println!("cargo:rustc-link-arg-bin=runforge=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bin=runforge=/MANIFESTUAC:no");
    println!(
        "cargo:rustc-link-arg-bin=runforge=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
