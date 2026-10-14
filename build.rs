fn main() {
    // Embed the app icon in the exe (Windows only).
    #[cfg(windows)]
    embed_resource::compile("assets/icon.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.rc");
}
