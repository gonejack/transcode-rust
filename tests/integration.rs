use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::process::{Command, Stdio};

const GBK_TEXT: &str = "这是一段gbk的中文版本\n这是一段gbk的中文版本";

fn transcode(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_transcode"))
        .args(args)
        .output()
        .expect("failed to run transcode");
    assert!(
        output.status.success(),
        "transcode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn transcode_stdin(args: &[&str], input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_transcode"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to run transcode");

    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "transcode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn test_gbk() {
    let out = transcode(&["-s", "gbk", "tests/gbk.txt"]);
    assert_eq!(out, GBK_TEXT);
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
    assert_eq!(out, GBK_TEXT);
}

#[test]
fn test_detect_only() {
    for (file, expected) in [
        ("tests/gbk.txt", "GB18030"),
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

#[test]
fn test_streaming_across_multibyte_boundary() {
    let expected = format!("{}{}", "a".repeat(8191), "这是一段中文".repeat(4096));
    let (input, _, had_errors) = encoding_rs::GBK.encode(&expected);
    assert!(!had_errors);

    let output = transcode_stdin(&["-s", "gbk"], &input);
    assert_eq!(output, expected.as_bytes());
}

#[test]
fn test_streaming_replaces_invalid_and_unmappable_input() {
    let decoded = transcode_stdin(&["-s", "utf-8"], &[b'a', 0xff, b'b']);
    assert_eq!(String::from_utf8(decoded).unwrap(), "a�b");

    let encoded = transcode_stdin(&["-s", "utf-8", "-t", "windows-1252"], "😀".as_bytes());
    assert_eq!(encoded, b"&#128512;");
}

#[test]
fn test_overwrite_uses_transcoded_output() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.txt");
    let expected = "覆盖写入测试".repeat(4096);
    let (input, _, had_errors) = encoding_rs::GBK.encode(&expected);
    assert!(!had_errors);
    fs::write(&path, input).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    #[cfg(unix)]
    let original_metadata = fs::metadata(&path).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_transcode"))
        .args(["-s", "gbk", "-w"])
        .arg(&path)
        .output()
        .expect("failed to run transcode");

    assert!(
        output.status.success(),
        "transcode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    #[cfg(unix)]
    {
        let metadata = fs::metadata(path).unwrap();
        assert_eq!(metadata.ino(), original_metadata.ino());
        assert_eq!(metadata.permissions().mode(), original_metadata.permissions().mode());
    }
}
