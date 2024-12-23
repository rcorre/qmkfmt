# QMK Format

`qmkfmt` is a tool to format the `keymaps` section of a `keymap.c` file in [qmk](https://qmk.fm/).

# Clang Format

If `clang-format` is available on `$PATH`, `qmkfmt` will also invoke `clang-format` to format the rest of the file.

# Editor Setup

## Helix

Put the following in `.helix/languages.toml` at the root of the `qmk_firmware` repository:

```toml
[[language]]
name = "c"
auto-format = true
formatter = { command = "qmkfmt" }
```
