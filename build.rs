fn main() {
    println!("cargo:rerun-if-changed=assets/SSMP.rc");
    println!("cargo:rerun-if-changed=assets/SSMP.manifest");
    println!("cargo:rerun-if-changed=assets/SSMP.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || std::env::var_os("CARGO_FEATURE_DESKTOP").is_none()
    {
        return;
    }
    let output =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("ssmp-resource.o");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let compiler = std::env::var_os("SSMP_WINDRES").unwrap_or_else(|| "windres".into());
        let result = std::process::Command::new(compiler)
            .args(["-i", "assets/SSMP.rc", "-O", "coff", "-o"])
            .arg(&output)
            .status();
        if result.is_ok_and(|s| s.success()) {
            println!("cargo:rustc-link-arg-bin=ssmp-desktop={}", output.display());
        } else {
            println!(
                "cargo:warning=Windows resource compiler unavailable; building without an embedded icon."
            );
        }
    }
}
