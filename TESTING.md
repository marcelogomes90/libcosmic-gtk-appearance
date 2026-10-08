# Testing notes

`./try.sh <app>` turns the log on. `./try.sh --content <app>` also appends the
translucent-content stylesheet for the right toolkit, picked by inspecting the
target binary.

## What the log should say

```
[cosmic-gtk-appearance] toolkit GTK4, map signal 71, COSMIC theme loaded, frosted glass on
[cosmic-gtk-appearance] bound ext_background_effect_manager_v1 v1
[cosmic-gtk-appearance] manager bound, caps=0x1
[cosmic-gtk-appearance] blur attached to window 0x...
[cosmic-gtk-appearance] stylesheet applied (GTK4, frosted glass on)
```

`COSMIC theme loaded` means the accent and the radius came from your config; if
it says `not found`, the styling fell back to GTK colours. `caps=0x1` is
`capability.blur`. The order matters: `blur attached` comes before
`stylesheet applied`, because the styling only goes in once the effect exists.
If the log stops at `manager bound`, the app opens normally with no
translucency — that is the expected behaviour when blur is unavailable.

## With frosted glass turned off

In Settings → Appearance → Frosted glass, turn the window toggle off. The log
should shrink to two lines:

```
[cosmic-gtk-appearance] toolkit GTK4, ..., frosted glass off
[cosmic-gtk-appearance] stylesheet applied (GTK4, frosted glass off)
```

The window opens **opaque**, keeping only the decoration styling. The shim never
even talks to the compositor in that case.

## What to look for on screen

- All four corners, at different window sizes: no sharp strip on any side, and
  no blur halo outside the window.
- Resizing by dragging a corner: the blur keeps up with no lag.
- Active versus inactive window: the decoration icons are accent-coloured on the
  active one and grey on the inactive one.
- Hovering the window buttons: the background appears with a 2px radius, not a
  circle, following `corner_radii` in your theme.
- Maximising with super+↑: with `frosted_maximized_apps` set to `false`, the
  window should go opaque while maximised and return to glass when restored.
  This one could not be exercised automatically.

## GTK4 with libadwaita

| command | note |
| --- | --- |
| `./try.sh baobab` | sidebar and rows; good for resize testing |
| `./try.sh file-roller` | exercises the menu-button icon colour |
| `./try.sh gnome-font-viewer` | content fills the window; use `--content` |
| `./try.sh simple-scan` | |
| `./try.sh yelp` | navigation sidebar |
| `./try.sh gnome-help` | |
| `./try.sh evince FILE.pdf` | only works with the library in `/usr/lib` (AppArmor) |
| `./try.sh zenity --info --text=hi` | small dialog, two seconds |

## GTK3

| command | note |
| --- | --- |
| `./try.sh gnome-disks` | split header bar |
| `./try.sh seahorse` | two header bars side by side |
| `./try.sh gucharmap` | no app title bar, plus a menu bar |
| `./try.sh nm-connection-editor` | plain list window |
| `./try.sh popsicle-gtk` | |
| `./try.sh gnome-power-statistics` | maps two popup windows at startup |

Open a menu in any of these: the menu must stay opaque and unblurred.

Two that do not work and are not bugs. `gnome-terminal` is a D-Bus client, so
the window belongs to `gnome-terminal-server`, which does not inherit the
`LD_PRELOAD`. `firefox` loads GTK3 through `dlopen` together with `libxul`,
after the shim's constructor has already run and found no glib — and an already
running instance takes over any new `firefox` anyway.

## GTK4 without libadwaita

Most GTK4 applications ship libadwaita too. The reliable bench for the plain
case is `gtk4-widget-factory`, from the `gtk-4-examples` package.

## Flatpak

Two `--filesystem` grants are needed: one for the library and one for the COSMIC
theme — without the second the sandbox sees neither the accent colour nor the
button radius.

```sh
P=$PWD
flatpak run \
  --filesystem=$P:ro \
  --filesystem=xdg-config/cosmic:ro \
  --env=LD_PRELOAD=$P/target/release/libcosmic_gtk_appearance.so \
  --env=COSMIC_GTK_APPEARANCE_DEBUG=1 \
  com.vysp3r.ProtonPlus
```

Swap the application id as needed. Sandboxes vary: some grant home access and
would find the theme anyway, others grant nothing, so both `--filesystem` flags
are the safe default.

Most other GTK-linking flatpaks are Electron, CEF, Gecko or Qt apps: the shim
attaches, but the window is painted by the embedded engine and nothing changes.

## A process without GTK

```sh
COSMIC_GTK_APPEARANCE_DEBUG=1 \
  LD_PRELOAD=$PWD/target/release/libcosmic_gtk_appearance.so ls
```

Should print nothing: the constructor does one `dlsym` for `g_idle_add` and
gives up.
