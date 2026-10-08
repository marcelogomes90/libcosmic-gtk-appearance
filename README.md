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
sudo make install     # /usr/lib/<multiarch>
make install-user     # ~/.local/lib
make deb              # builds a .deb
```

Both locations matter. Applications confined by AppArmor, such as `evince`, only
load libraries from `/usr/lib`. Flatpaks are the opposite: `/usr` is reserved by
Flatpak and cannot be shared into a sandbox, so they need the copy in
`~/.local/lib`.

Installing does not enable anything. Pick what suits you:

```sh
# one application: copy its .desktop to ~/.local/share/applications/
# and prefix the Exec line
Exec=env LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libcosmic_gtk_appearance.so baobab

# every flatpak at once
flatpak override --user \
  --filesystem=~/.local/lib/libcosmic_gtk_appearance.so:ro \
  --filesystem=xdg-config/cosmic:ro \
  --env=LD_PRELOAD=$HOME/.local/lib/libcosmic_gtk_appearance.so

# the whole session: one line in /etc/environment
LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libcosmic_gtk_appearance.so
```

The session-wide option injects the library into every process; it costs one
`dlsym` where there is no glib, but it is the most invasive choice.

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
