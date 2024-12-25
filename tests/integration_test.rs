use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use pretty_assertions::assert_eq;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn bin() -> Command {
    let mut cmd = Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")));
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

fn before_path(keyboard: &str) -> PathBuf {
    format!("testdata/before/{keyboard}/keymaps/default/keymap.c").into()
}

fn after_path(keyboard: &str) -> PathBuf {
    format!("testdata/after/{keyboard}/keymaps/default/keymap.c").into()
}

fn fmt_pipe(cmd: &mut Command, keyboard: &str) -> String {
    let mut cmd = cmd.spawn().unwrap();
    let keymap = std::fs::read_to_string(before_path(keyboard)).unwrap();
    cmd.stdin
        .take()
        .unwrap()
        .write_all(keymap.as_bytes())
        .unwrap();
    let out = cmd.wait_with_output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

fn fmt_path(cmd: &mut Command, keyboard: &str) -> String {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("keymap.c");
    std::fs::copy(before_path(keyboard), &path).unwrap();
    let cmd = cmd.arg(&path);
    let out = cmd.output().unwrap();
    assert!(out.status.success());
    std::fs::read_to_string(path).unwrap()
}

fn expected(keyboard: &str) -> String {
    std::fs::read_to_string(after_path(keyboard)).unwrap()
}

#[test]
fn test_help() {
    assert_cmd_snapshot!(bin().arg("--help"));
}

#[test]
fn test_fmt_dactyl_stdio() {
    let keyboard = "dactyl";
    let mut cmd = bin();
    let actual = fmt_pipe(&mut cmd, keyboard);
    assert_eq!(actual, expected(keyboard));
}

#[test]
fn test_fmt_moonlander_stdio() {
    let keyboard = "moonlander";
    let mut cmd = bin();
    let actual = fmt_pipe(&mut cmd, keyboard);
    assert_eq!(actual, expected(keyboard));
}

#[test]
fn test_fmt_dactyl_path() {
    let keyboard = "dactyl";
    let mut cmd = bin();
    let actual = fmt_path(&mut cmd, keyboard);
    assert_eq!(actual, expected(keyboard));
}

#[test]
fn test_fmt_moonlander_path() {
    let keyboard = "moonlander";
    let mut cmd = bin();
    let actual = fmt_path(&mut cmd, keyboard);
    assert_eq!(actual, expected(keyboard));
}
