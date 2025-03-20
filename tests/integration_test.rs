use core::str;
use insta::assert_snapshot;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use pretty_assertions::assert_eq;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn cmd(args: &[&str]) -> Command {
    let mut cmd = Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")));
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(args);
    cmd
}

fn before_path(keyboard: &str) -> PathBuf {
    format!("testdata/{keyboard}/keymaps/default/keymap.c").into()
}

fn fmt_pipe(keyboard: &str, args: &[&str]) -> String {
    let mut cmd = cmd(args).spawn().unwrap();
    let keymap = std::fs::read_to_string(before_path(keyboard)).unwrap();
    cmd.stdin
        .take()
        .unwrap()
        .write_all(keymap.as_bytes())
        .unwrap();
    let out = cmd.wait_with_output().unwrap();
    assert!(out.status.success(), "{:?}", str::from_utf8(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

fn fmt_path(keyboard: &str, args: &[&str]) -> String {
    let mut cmd = cmd(args);
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("keymap.c");
    std::fs::copy(before_path(keyboard), &path).unwrap();

    let cmd = cmd.arg(&path);
    let out = cmd.output().unwrap();
    assert!(out.status.success());

    let res = std::fs::read_to_string(&path).unwrap();

    // repeat, to ensure nothing changes with multiple formats
    let out = cmd.output().unwrap();
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(path).unwrap(), res);

    res
}

#[test]
fn test_help() {
    assert_cmd_snapshot!(cmd(&[]).arg("--help"));
}

#[test]
fn test_fmt_dactyl() {
    let keyboard = "dactyl";
    let args = &["--split-spaces=4"];

    let actual = fmt_pipe(keyboard, args);
    assert_eq!(actual, fmt_path(keyboard, args));
    assert_snapshot!(actual);
}

#[test]
fn test_fmt_moonlander() {
    let keyboard = "moonlander";
    let args = &[];

    let actual = fmt_pipe(keyboard, args);
    assert_eq!(actual, fmt_path(keyboard, args));
    assert_snapshot!(actual);
}

#[test]
fn test_fmt_moonlander_no_clang() {
    let keyboard = "moonlander";
    let args = &["--no-clang-format"];

    let actual = fmt_pipe(keyboard, args);
    assert_eq!(actual, fmt_path(keyboard, args));
    assert_snapshot!(actual);
}
