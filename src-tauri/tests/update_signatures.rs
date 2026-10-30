use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use std::path::Path;

#[test]
fn embedded_key_accepts_signed_files_and_rejects_modified_bytes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let public_key = base64::engine::general_purpose::STANDARD
        .decode(config["plugins"]["updater"]["pubkey"].as_str().unwrap())
        .unwrap();
    let public_key = PublicKey::decode(std::str::from_utf8(&public_key).unwrap()).unwrap();
    let mut paths = vec![root.join("tests/fixtures/update-probe.txt")];
    if let Some(installer) = std::env::var_os("APP_FLASH_VERIFY_INSTALLER") {
        paths.push(installer.into());
    }
    for path in paths {
        let mut bytes = std::fs::read(&path).unwrap();
        let signature = std::fs::read_to_string(format!("{}.sig", path.display())).unwrap();
        let signature = base64::engine::general_purpose::STANDARD
            .decode(signature.trim())
            .unwrap();
        let signature = Signature::decode(std::str::from_utf8(&signature).unwrap()).unwrap();
        assert!(public_key.verify(&bytes, &signature, false).is_ok());
        let expected_version = if path.ends_with("update-probe.txt") {
            "0.1.0"
        } else {
            config["version"].as_str().unwrap()
        };
        assert!(signature
            .trusted_comment()
            .contains(&format!("\tversion:{expected_version}")));
        bytes[0] ^= 1;
        assert!(public_key.verify(&bytes, &signature, false).is_err());
    }
}
