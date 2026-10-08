# libcosmic-gtk-appearance

Makes GTK applications follow the COSMIC desktop's appearance settings.

## What it does

COSMIC can frost the background behind any window, and its own applications use
it. GTK applications never do: neither libgtk nor libadwaita asks the compositor
for the effect, and both paint an opaque background.

This is a small shared library you preload into GTK applications. It reads your
COSMIC appearance settings and applies them: frosted glass when it is enabled,
the accent colour on the window decorations, and the theme's corner radius. It
follows the configuration rather than imposing a look — turn frosted glass off
and windows stay solid.

Works with GTK3 and GTK4.

## How it works

A constructor schedules an idle callback; by the time it runs, GTK is up. From
there a hook on `GtkWidget::map` catches every window that appears, and public
GDK API leads to its `wl_surface`. The library asks the compositor for the blur
through `ext_background_effect_v1`, and installs a stylesheet that makes the
window translucent and flattens the title bar.

If anything fails — no compositor support, an unrecognised toolkit, frosted
glass turned off — it gives up quietly and the application opens exactly as it
would without it.

## Installing

```sh
make
sudo make install     # /usr/lib/<multiarch>/gio/modules
make deb              # builds a .deb
```

That is the whole setup for applications on the system. The library installs as
a GIO module, every GTK application scans that directory on startup, and it
applies itself from there. No environment variable, no desktop file to edit.
Applications pick it up the next time they start.

To remove it, `sudo make uninstall` or just delete the file; nothing else points
at it and nothing needs undoing.

### Flatpaks

Sandboxed applications have their own `/usr` and never see the system copy, so
they need a second one and a grant:

```sh
make install-user     # ~/.local/lib/gio/modules

flatpak override --user \
  --filesystem="$HOME/.local/lib/gio/modules":ro \
  --filesystem=xdg-config/cosmic:ro \
  --env=GIO_EXTRA_MODULES="$HOME/.local/lib/gio/modules" \
  com.example.App
```

Mind the quoting: in zsh, `"$VAR:ro"` is read as the `:r` history modifier and
silently mangles the path, leaving the grant pointing at a directory that does
not exist. Quote the variable and leave `:ro` outside, as above.

Naming an application confines the override to it, which also makes it reversible
in one step:

```sh
flatpak override --user --reset com.example.App
```

That deletes the application's override file outright. Run `--reset` **without**
an application id and it clears the global override instead, taking with it
whatever else lives there — on a COSMIC install that already holds the grants
for GTK themes and colour schemes. Note too that resetting an application that
already had an override of its own discards those settings along with these.

Applying the override globally works as well, but then only the blunt `--reset`
undoes it, and the selective flags are worse than they look: `--nofilesystem`
and `--unset-env` do not remove an entry, they append a negation that actively
denies the path from then on.

## Settings

| variable | effect |
| --- | --- |
| `COSMIC_GTK_APPEARANCE_DEBUG=1` | log every step to stderr |
| `COSMIC_GTK_APPEARANCE_ALPHA=0.78` | force frosted glass at that opacity |
| `COSMIC_GTK_APPEARANCE_CSS_EXTRA=f.css` | append rules to the stylesheet |
| `COSMIC_GTK_APPEARANCE_CSS=f.css` | replace the stylesheet |
| `COSMIC_GTK_APPEARANCE_NO_CSS=1` | blur only, leave styling alone |
| `COSMIC_GTK_APPEARANCE_DISABLE=1` | turn the library off for that process |

`./try.sh <app>` runs something with logging on, and `./try.sh --content <app>`
adds the stylesheets from `examples/`, which also let the blur through text
views.

## Limits

- Applications that only hand a request to a background service are out of
  reach: the window belongs to a process that never inherited the preload.
- So are those that load GTK through `dlopen` after startup, since the library
  looks for GTK as the process begins.
- Popovers, menus and tooltips are separate surfaces and stay opaque on purpose.
- An application painting its own opaque background still wins, because GTK
  flattens every layer before the compositor sees it.

## Licence

LGPL-3.0-or-later. The library is loaded into the address space of other
programs, including proprietary ones; the LGPL keeps the copyleft on this code
without reaching into whatever it ends up next to. `LICENSE` holds the LGPL-3
text, which incorporates by reference the GPL-3 in `LICENSE.GPL-3`.
