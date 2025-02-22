# QMK Format

`qmkfmt` is a tool to format the `keymaps` section of a `keymap.c` file in [qmk](https://qmk.fm/).
It formats each `LAYOUT` entry under `keymaps` into a grid with aligned columns.

Bug reports, feature requests, and contributions are welcome!

# Installation

Pre-compiled binaries are available on the [releases page](https://github.com/rcorre/qmkfmt/releases).

To install the latest release from crates.io:

```sh
cargo install qmkfmt
```

To install the latest from git:

```sh
cargo install --git https://github.com/rcorre/qmkfmt
```

# Behavior

`qmkfmt` is designed to work out of the box with no configuration.
`qmkfmt` infers the number of rows from the number of lines in each `LAYOUT`.
If a row has less than the maximum number of columns (e.g. a thumb cluster), it is centered.

For example, the following keymap:

```c
[0] = LAYOUT_split_3x6_3(
KC_TAB,KC_Q,KC_W,KC_E,KC_R,KC_T,KC_Y,KC_U,KC_I,KC_O,KC_P,KC_PIPE,
KC_ESC,LALT_T(KC_A),RGUI_T(KC_S),LCTL_T(KC_D),LSFT_T(KC_F),KC_G,KC_H,RSFT_T(KC_J),RCTL_T(KC_K),RGUI_T(KC_L),RALT_T(KC_SCLN),KC_QUOT,
KC_LSFT,KC_Z,KC_X,KC_C,KC_V,KC_B,KC_N,KC_M,KC_COMM,KC_DOT,KC_SLSH,KC_RSFT,
KC_SPC,LT(LYR_NAV, KC_TAB),KC_ESC,LT(1, KC_ENT),KC_BSPC,KC_ESC
)
```

Will be reformatted to:

```c
[0] = LAYOUT_split_3x6_3(
    KC_TAB,  KC_Q,         KC_W,         KC_E,         KC_R,                KC_T,   KC_Y,          KC_U,         KC_I,         KC_O,         KC_P,            KC_PIPE,
    KC_ESC,  LALT_T(KC_A), RGUI_T(KC_S), LCTL_T(KC_D), LSFT_T(KC_F),        KC_G,   KC_H,          RSFT_T(KC_J), RCTL_T(KC_K), RGUI_T(KC_L), RALT_T(KC_SCLN), KC_QUOT,
    KC_LSFT, KC_Z,         KC_X,         KC_C,         KC_V,                KC_B,   KC_N,          KC_M,         KC_COMM,      KC_DOT,       KC_SLSH,         KC_RSFT,
                                         KC_SPC,       LT(LYR_NAV, KC_TAB), KC_ESC, LT(1, KC_ENT), KC_BSPC,      KC_ESC
)
```

If `clang-format` is available on `$PATH`, `qmkfmt` will invoke it to format the rest of the file.

# Configuration

The `--split-spaces` flag controls the given number of spaces in the center of each layout.
Pass `--split-spaces=0` if the keyboard is not split and you want no separation between halves.

# Usage

## CLI

Simply run `qmkfmt path/to/keymap.c` to format the file inline.
If not given a path, `qmkfmt` reads `stdin` and writes to `stdout`.

## Helix

Put the following in `.helix/languages.toml` at the root of the `qmk_firmware` repository:

```toml
[[language]]
name = "c"
auto-format = true
formatter = { command = "qmkfmt", args = ["--split-spaces=8"] }
```
