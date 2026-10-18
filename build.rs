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
    } else if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let kits = std::path::PathBuf::from(
            std::env::var_os("ProgramFiles(x86)")
                .unwrap_or_else(|| "C:/Program Files (x86)".into()),
        )
        .join("Windows Kits/10/bin");
        let mut candidates: Vec<_> = std::fs::read_dir(kits)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path().join("x64/rc.exe"))
            .filter(|p| p.is_file())
            .collect();
        candidates.sort();
        let compiler = candidates.pop().unwrap_or_else(|| "rc.exe".into());
        let output = output.with_extension("res");
        let status = std::process::Command::new(compiler)
            .arg("/nologo")
            .arg("/fo")
            .arg(&output)
            .arg("assets/SSMP.rc")
            .status()
            .expect("Windows SDK resource compiler is required");
        assert!(status.success(), "SSMP Windows resource compilation failed");
        println!("cargo:rustc-link-arg-bin=ssmp-desktop={}", output.display());
    }
}
