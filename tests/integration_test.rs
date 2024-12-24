#[cfg(test)]
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

#[test]
fn test_fmt_dactyl_stdio() {
    let mut cmd = Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")));
    let keymap = std::fs::read_to_string("testdata/keymap.c").unwrap();
    assert_cmd_snapshot!(cmd.pass_stdin(keymap));
}

#[test]
fn test_fmt_moonlander_stdio() {
    let mut cmd = Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")));
    let keymap = std::fs::read_to_string("testdata/keymap_moonlander.c").unwrap();
    assert_cmd_snapshot!(cmd.pass_stdin(keymap));
}
