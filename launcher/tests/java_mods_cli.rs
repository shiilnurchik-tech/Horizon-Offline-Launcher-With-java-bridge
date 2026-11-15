//! The file-management portion of the bridge, not the Java-mods runtime.
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_horizon-launcher"))
        .args(args)
        .env("HORIZON_HOME", root.join("game"))
        .env("HORIZON_LAUNCHER_HOME", root.join("launcher"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("HORIZON_LAUNCHER_TRASH", root.join("trash"))
        .env("HORIZON_LAUNCHER_NO_SELF_UPDATE", "1")
        .env("HORIZON_NO_AUDIO", "1")
        .env("HORIZON_TELEMETRY_URL", "off")
        .env("HORIZON_NO_KEYCHAIN", "1")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .output()
        .unwrap()
}

#[test]
fn manage_fabric_jars_and_explain_how_to_enable_the_bridge() {
    let root = std::env::temp_dir().join(format!("horizon-java-mods-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("example.jar");
    let mut jar = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    jar.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default()).unwrap();
    jar.write_all(br#"{"id":"example","name":"Example","version":"1"}"#).unwrap();
    jar.finish().unwrap();

    let file = path.to_str().unwrap();
    for args in [vec!["--json", "java-mods", "add", file], vec!["--json", "java-mods", "list"]] {
        let output = run(&root, &args);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["bridge_integrated"], false);
        assert_eq!(result["fabric_cli_available"], true);
        assert_eq!(result["mods"][0]["id"], "example");
    }
    let output = run(&root, &["java-mods", "remove", "example"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stderr).contains("play --fabric"));
    assert!(root.join("trash/example.jar").is_file());
    let _ = std::fs::remove_dir_all(root);
}
