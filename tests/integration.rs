use std::process::Command;

fn transcode(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_transcode"))
        .args(args)
        .output()
        .expect("failed to run transcode");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn test_gbk() {
    let out = transcode(&["-s", "gbk", "tests/gbk.txt"]);
    assert_eq!(out, "这是一段gbk的中文版本");
}

#[test]
fn test_gb18030() {
    let out = transcode(&["-s", "gb18030", "tests/gb18030.txt"]);
    assert!(out.contains("测试文件"));
}

#[test]
fn test_ibm866() {
    let out = transcode(&["-s", "ibm866", "tests/ibm866.txt"]);
    assert_eq!(out.trim(), "Привет, мир!");
}

#[test]
fn test_auto_detect_gbk() {
    let out = transcode(&["tests/gbk.txt"]);
    assert_eq!(out, "这是一段gbk的中文版本");
}

#[test]
fn test_detect_only() {
    for (file, expected) in [
        ("tests/gbk.txt", "GB2312"),
        ("tests/gb18030.txt", "GB18030"),
        ("tests/ibm866.txt", "IBM866"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_transcode"))
            .args(["-d", file])
            .output()
            .unwrap();
        let out = String::from_utf8(output.stdout).unwrap();
        assert!(out.contains(expected), "{file}: expected {expected}, got {out}");
    }
}
