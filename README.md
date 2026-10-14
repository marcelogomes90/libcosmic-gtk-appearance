# libcosmic-gtk-appearance

Makes GTK applications follow the COSMIC desktop's appearance settings.

![Concept art: a GTK window opaque on the left, frosted against the COSMIC desktop on the right](docs/before-and-after.png)

## What it does

COSMIC can frost the background behind a window, and its own applications use
it. GTK applications never do: neither libgtk nor libadwaita asks the
compositor for the effect, and both paint an opaque background.

This is a small shared library that loads into GTK3 and GTK4 applications. It
reads your COSMIC appearance settings and applies them: frosted glass when it is
enabled, the palette of the live theme, and COSMIC's own colours, corner radius
and spacing on the window decorations. Change the theme, the accent colour, the
density or the frosted setting and open windows follow along.

It follows the configuration rather than imposing a look. Turn frosted glass off
and windows stay solid; where the blur is out of reach — an X11 window, a
compositor that does not offer the effect — the application opens exactly as it
would without the library.

## Installing

```sh
just setup
```

That builds the library and installs it: the system copy for native
applications, a second one where sandboxes can reach it, the grant that hands it
to flatpaks, and the session setting that lets it drive the blur. GTK 4.23.3 and
newer claim the blur protocol themselves, and the region they set lands a shadow
margin off under COSMIC, so `GDK_WAYLAND_DISABLE` keeps GTK out of the way. The
system step asks for sudo; the rest runs as you.

The library installs as a GIO module, which every GTK application scans on
startup, so there is nothing to configure afterwards. Applications pick it up
the next time they start — flatpaks included, even ones installed next month —
and the ones your session starts from your next login.

```sh
just uninstall
```

takes all of it back, exactly: nothing is left pointing at a file that no longer
exists. Every step is also available on its own, and all of them can be repeated
safely:

| recipe | does |
| --- | --- |
| `just install` | the system copy, for native applications |
| `just install-user` | the copy flatpaks can reach |
| `just flatpak-enable` | grants that copy to every flatpak |
| `just flatpak-disable` | takes the grant back |
| `just session-enable` | hands the blur protocol to the library |
| `just status` | reports what is in place and what drives the blur |
| `just check` | rustfmt, clippy and the tests |
| `just test-install` | installs, repeats and uninstalls against a throwaway prefix |
| `just deb` | builds a package |

## Settings

| variable | effect |
| --- | --- |
| `COSMIC_GTK_APPEARANCE_DEBUG=1` | log every step to stderr |
| `COSMIC_GTK_APPEARANCE_ALPHA=0.78` | force frosted glass at that opacity |
| `COSMIC_GTK_APPEARANCE_CSS_EXTRA=f.css` | append rules to the stylesheet |
| `COSMIC_GTK_APPEARANCE_CSS=f.css` | replace the stylesheet |
| `COSMIC_GTK_APPEARANCE_NO_CSS=1` | blur only, leave styling alone |
| `COSMIC_GTK_APPEARANCE_DISABLE=1` | turn the library off for that process |

`COSMIC_GTK_APPEARANCE_CSS` replaces the rules, not the colours: the palette
COSMIC generated still goes in front of whatever the file holds, so `@window_bg_color`
and the rest keep resolving.

`./try.sh <app>` runs something with logging on, and `./try.sh --content <app>`
adds the stylesheets from `examples/`, which also let the blur through text
views. For a flatpak, pass the variable through the sandbox:
`flatpak run --env=COSMIC_GTK_APPEARANCE_DEBUG=1 <id>`.
`tests/support/sample.py <3|4|adw>` is a small window per toolkit to try them against.

## What gets painted

Every value below is read from the COSMIC configuration. None of them is a
number this library chose.

| what | where it comes from |
| --- | --- |
| window background | the alpha of `transparent_background.base` |
| sidebars and sidebar panes | the alpha of `transparent_primary.base`, which is `FF` by default, so a sidebar stays solid over the frosted window |
| the title bar above a sidebar | the same value as the sidebar below it |
| title bars, toolbars and window handles | cleared, so the window background reaches the top of the window |
| window control icons | `accent_text` or `accent.base`, and the hover and pressed tints from `icon_button` |
| icon buttons in a title bar | the same accent, because they are the same kind of control; filled buttons keep their own colour |
| the radius behind a window control | `corner_radii.radius_xl` |
| title bar height and padding | `CosmicTk/v1/header_size` |
| spacing between window controls | `spacing.space_xxs` |
| maximized and fullscreen windows | `frosted_maximized_apps`, which turns them solid again |
| text views and pictures | `view_bg_color`, so text keeps its contrast and an image shows against a surface rather than against the desktop |
| menu bars | `window_bg_color`, whichever way the window is focused |

Finding the title bar that belongs to a sidebar means looking at the widgets, not
only at the CSS: libadwaita marks the pane the title bar sits in, and GTK3
applications mark the sidebar's divider instead, so there the title bar is the one
just before that divider. Nothing else about the widget tree is touched.

These are left alone on purpose:

- popovers, menus and tooltips, which are separate Wayland surfaces the blur
  could not reach behind anyway;
- cards and boxed lists, because COSMIC's secondary container is solid —
  `transparent_secondary.base` carries no alpha;
- the window corner radius and the active-window hint, which cosmic-comp draws
  for every window whatever toolkit it came from;
- a `GtkSeparator` an application marks as a sidebar, because libhandy marks the
  divider that way rather than the sidebar itself;
- surfaces the application makes translucent itself, which stay as it made them.
  A container with the `osd` class is the usual case: libadwaita carries that
  class into the popovers inside it, so such a popover is translucent with or
  without this library. It only reads differently here, because what shows
  through it is now the desktop instead of a solid window.

## Limits

- An application that paints its own window background wins, because GTK
  flattens every layer before the compositor sees it. GTK is only the frame for
  Gecko and for Chromium, so Firefox, Thunderbird and Electron applications never
  turn frosted. Gecko goes one step further and reads the GTK colours to paint
  them itself, so a window left without a background colour reaches it as a hole
  in the header bar and in the status strip; the library recognises `libxul.so`
  and leaves those processes untouched.
- Popovers, menus and tooltips are separate surfaces and stay opaque on purpose,
  and so is anything drawn into a `picture`, so an image shows against a solid
  background rather than against the desktop.
- X11 and XWayland windows get the colours and the decorations, never the blur.
- Only the `gtk.css` COSMIC generates is followed; a hand-written one is left
  alone, and its colours reach a window once, when it opens.
- A window that is not a GTK window is not touched at all.
- The stylesheet provider and the blur manager are held for the life of the
  process on purpose. They are the only references this library does not give
  back.

## Licence

LGPL-3.0-or-later. The library is loaded into the address space of other
programs, including proprietary ones; the LGPL keeps the copyleft on this code
without reaching into whatever it ends up next to. `LICENSE` holds the LGPL-3
text, which incorporates by reference the GPL-3 in `LICENSE.GPL-3`.
