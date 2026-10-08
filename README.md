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
just setup
```

That builds the library and does the three things it takes to cover everything:
installs it system-wide for native applications, puts a second copy where
sandboxes can reach it, and grants that copy to the flatpaks you have installed.
The system step asks for sudo; the rest runs as you.

The library installs as a GIO module, and every GTK application scans that
directory on startup, so there is nothing to configure afterwards. Applications
pick it up the next time they start.

```sh
just uninstall
```

Removes all three, and the uninstall is exact: nothing is left pointing at a file
that no longer exists.

The steps are also available on their own, and all of them can be repeated
safely:

| recipe | does |
| --- | --- |
| `just install` | `/usr/lib/<multiarch>/gio/modules`, for native applications |
| `just install-user` | `~/.local/lib/gio/modules`, the copy flatpaks can reach |
| `just flatpak-enable` | grants that copy to the installed flatpaks |
| `just flatpak-disable` | takes the grants back |
| `just check` | rustfmt, clippy and the tests |
| `just deb` | builds a package |

### About the flatpak grants

Sandboxed applications have their own `/usr` and never see the system copy,
which is why they need the second one plus a filesystem grant and
`GIO_EXTRA_MODULES`. That is the same shape COSMIC already uses to hand its GTK
colours to sandboxes: the theme arrives only because the global override grants
`xdg-config/gtk-4.0`, and the file vanishes from the sandbox the moment that
grant is denied.

The grants are written per application rather than globally, which is what makes
taking them back exact: `flatpak override --reset <id>` deletes that
application's override outright, while a global `--reset` would clear the whole
global override, and on a COSMIC install that is where the GTK theme grants
live.

An application that already carries an override of its own is left untouched in
both directions and reported, since adding ours there would leave the undo
unable to tell the two apart.

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
