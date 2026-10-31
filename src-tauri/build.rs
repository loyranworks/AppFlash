fn main() {
    for name in [
        "APPLE_SIGNING_IDENTITY",
        "APP_FLASH_SIGNING_KEYCHAIN",
        "APP_FLASH_REQUIRE_STABLE_SIGNING",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");
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
        if let Ok(identity) = std::env::var("APPLE_SIGNING_IDENTITY") {
            assert!(
                !identity.is_empty() && identity != "-",
                "macOS releases require the fixed certificate identity"
            );
            let mut sign = std::process::Command::new("codesign");
            sign.args([
                "--force",
                "--sign",
                &identity,
                "--identifier",
                "local.appflash.desktop.native",
                "--timestamp=none",
            ]);
            if let Some(keychain) = std::env::var_os("APP_FLASH_SIGNING_KEYCHAIN") {
                sign.arg("--keychain").arg(keychain);
            }
            assert!(
                sign.arg(&output)
                    .status()
                    .expect("Sign macOS helper")
                    .success(),
                "macOS helper signing failed"
            );
            let verify = std::process::Command::new("python3")
                .arg("../tools/verify-macos-signing.py")
                .arg(&output)
                .args(["--identifier", "local.appflash.desktop.native"])
                .status()
                .expect("Verify stable helper identity");
            assert!(
                verify.success(),
                "macOS helper has an unexpected signing identity"
            );
        } else {
            assert!(
                std::env::var("APP_FLASH_REQUIRE_STABLE_SIGNING").as_deref() != Ok("1"),
                "Fixed macOS signing identity is required"
            );
        }
        println!("cargo:rustc-env=APP_FLASH_NATIVE={}", output.display());
        println!("cargo:rerun-if-changed=native/macos.swift");
    }
    tauri_build::build();
}
