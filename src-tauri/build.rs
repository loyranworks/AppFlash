fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("app-flash-native");
        let status = std::process::Command::new("swiftc")
            .args([
                "-target",
                &format!(
                    "{}-apple-macosx12.0",
                    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64") {
                        "arm64"
                    } else {
                        "x86_64"
                    }
                ),
            ])
            .args(["-O", "native/macos.swift", "-o"])
            .arg(&output)
            .status()
            .expect("Install Xcode command-line tools to compile the macOS adapter");
        assert!(status.success(), "macOS adapter compilation failed");
        println!("cargo:rustc-env=APP_FLASH_NATIVE={}", output.display());
        println!("cargo:rerun-if-changed=native/macos.swift");
    }
    tauri_build::build();
}
