# QMK Format

`qmkfmt` is a tool to format the `keymaps` section of a `keymap.c` file in [qmk](https://qmk.fm/).
It formats each `LAYOUT` entry under `keymaps` into a grid with aligned columns.
If the `--split-spaces` argument is passed, it inserts the given nubmer of spaces in the center of each layout.
If a row has less than the maximum number of columns (e.g. a thumb cluster), it is centered.

If `clang-format` is available on `$PATH`, `qmkfmt` will invoke it to format the rest of the file.

`qmkfmt` infers the number of rows from the number of lines in each `LAYOUT`.

Bug reports, feature requests, and contributions are welcome!

# Installation

Pre-compiled binaries are available on the [releases page](https://github.com/rcorre/qmkfmt/releases).
To install the latest release from crates.io, run `cargo install qmkfmt`
To install the latest from git, run `cargo install --git https://github.com/rcorre/qmkfmt`

# Editor Setup

## Helix

Put the following in `.helix/languages.toml` at the root of the `qmk_firmware` repository:

```toml
[[language]]
name = "c"
auto-format = true
formatter = { command = "qmkfmt", args = ["--split-spaces=8"] }
```
