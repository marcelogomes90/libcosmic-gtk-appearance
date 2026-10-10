# libcosmic-gtk-appearance

Makes GTK applications follow the COSMIC desktop's appearance settings.

## What it does

COSMIC can frost the background behind any window, and its own applications use
it. GTK applications never do: neither libgtk nor libadwaita asks the compositor
for the effect, and both paint an opaque background.

This is a small shared library that loads into GTK applications. It reads your
COSMIC appearance settings and applies them: frosted glass when it is enabled,
the accent colour on the window decorations, and the theme's corner radius. It
follows the configuration rather than imposing a look — turn frosted glass off
and windows stay solid.

Works with GTK3 and GTK4.

## How it works

A constructor runs when the library is loaded and schedules an idle callback;
by the time it runs, the main loop is up. That callback asks whether `libgtk-4`
or `libgtk-3` is resident and returns without doing anything if neither is: the
library is loaded into every program that uses GIO, and this is where everything
that is not a GTK application stops.

Asking in the callback rather than in the constructor is deliberate. GIO scans
its module directory the moment a program first touches an extension point,
which under PyGObject is a few instructions before `gi` imports GTK — so a
constructor that insisted on seeing GTK would miss `ibus-setup`,
`system-config-printer` and every other application written against the
bindings. By the time a main loop is running, an application that uses GTK has
loaded it.

Symbols are resolved through that library's own handle rather than the global
symbol table, which is what makes applications that `dlopen` GTK themselves
work — `nvidia-settings` loads it privately, so nothing it brought in is
globally visible.

From there a hook on `GtkWidget::map` catches every window that appears, and
public GDK API leads to its `wl_surface`. The library asks the compositor for
the blur through `ext_background_effect_v1`, and installs a stylesheet that
makes the window translucent and flattens the title bar.

GTK 4.23.3 learned that protocol itself, and it claims the surface's effect
object whether or not anything asked for blur. A second one is the
`background_effect_exists` error, which is fatal — the client is disconnected,
so the application does not open at all. Worse, the region GTK sets is the
window geometry rectangle in surface coordinates, and COSMIC reads a blur region
from the geometry origin rather than from the surface origin: the blur lands one
shadow margin to the right and below, leaving a strip along the left edge and
another along the top unblurred. No stylesheet closes that gap, because GTK
never lets the shadow margin fall below the twelve pixels it keeps for the
resize handles, and it derives both the geometry and the blur region from the
same rectangle.

So the library asks GDK to leave the interface alone. `GDK_WAYLAND_DISABLE`
names Wayland interfaces GDK should not bind, and with
`ext_background_effect_manager_v1` on that list GTK never creates an effect
object for its surfaces. The library then speaks the protocol itself, as it does
to an older GTK, with a region that covers the whole surface — which no offset
can push off the window. GDK reads the variable when it opens the display, so it
has to be set before the application starts: the install writes it into the
flatpak override and into the session environment. The library also sets it when
it happens to be loaded before GTK is, which is where PyGObject applications
land. When it arrives too late the CSS route remains: `backdrop-filter: blur()`
on the window, the property GTK added alongside the protocol, with the strips
that come with it.

The manager is bound either way, because its capabilities are how we know the
compositor can blur at all before making a window translucent.

COSMIC hands its palette to GTK by generating `~/.config/gtk-3.0/gtk.css` and
`~/.config/gtk-4.0/gtk.css`, and GTK reads that file once, on startup: change
the theme and every window already open keeps the old colours until it is
restarted. The stylesheet this library installs sits above the user one, so it
carries the colour definitions along and reloads them with everything else.

It then watches the COSMIC configuration and the stylesheet COSMIC generates,
and follows both: change the theme, the accent colour or the frosted glass
setting and open windows pick it up without being restarted. Applying a theme
rewrites dozens of files, so the watcher waits for the writes to settle and
reloads once; and the stylesheet is only handed back to GTK when the text it
composes actually changed, so a configuration write that touches nothing visible
costs nothing.

A window on the X11 backend — an XWayland application, anything started with
`GDK_BACKEND=x11` — still gets the colours, the flat title bar and the accent on
the window controls. It cannot get the blur: the protocol is addressed to a
`wl_surface`, and an X11 client never holds its own.

If anything else fails — no compositor support, an unrecognised toolkit, frosted
glass turned off — it gives up quietly and the application opens exactly as it
would without it.

## Installing

```sh
just setup
```

That builds the library and does what it takes to cover everything: installs it
system-wide for native applications, puts a second copy where sandboxes can
reach it, grants that copy to flatpaks, and hands the blur protocol to the
library in both. The system step asks for sudo; the rest runs as you. Flatpaks
are covered the next time they start, and applications the session starts from
your next login.

The library installs as a GIO module, and every GTK application scans that
directory on startup, so there is nothing to configure afterwards. Applications
pick it up the next time they start, and a flatpak installed next month is
covered without running anything again.

```sh
just uninstall
```

Removes all three, and the uninstall is exact: nothing is left pointing at a
file that no longer exists.

The steps are also available on their own, and all of them can be repeated
safely:

| recipe | does |
| --- | --- |
| `just install` | `/usr/lib/<multiarch>/gio/modules`, for native applications |
| `just install-user` | `~/.local/lib/libcosmic-gtk-appearance`, the copy flatpaks can reach |
| `just flatpak-enable` | grants that copy to every flatpak |
| `just flatpak-disable` | takes the grant back |
| `just check` | rustfmt, clippy and the tests |
| `just deb` | builds a package |

### About the flatpak grant

Sandboxed applications have their own `/usr` and never see the system copy,
which is why they need the second one plus a filesystem grant and
`GIO_EXTRA_MODULES`. That is the same shape COSMIC already uses to hand its GTK
colours to sandboxes: the theme arrives only because the global override grants
`xdg-config/gtk-4.0`, and the file vanishes from the sandbox the moment that
grant is denied.

The grant is written once, into the global override, which is what makes it
cover flatpaks that are not installed yet. Four entries are added:

| entry | why |
| --- | --- |
| `--filesystem=<module dir>:ro` | the library itself, read-only |
| `--env=GIO_EXTRA_MODULES=<module dir>` | so GIO scans that directory |
| `--filesystem=xdg-config/cosmic/com.system76.CosmicTheme.Mode` | which of the two themes is live |
| `--filesystem=xdg-config/cosmic/com.system76.CosmicTheme.{Dark,Light}` | the opacities, radius and decoration colours |

Reaching the module directory is enough for a GTK application to be styled, but
not to be frosted: the opacity values exist only in the COSMIC configuration,
and neither the generated `gtk.css` nor the settings portal carries them.

Those three COSMIC directories are granted read-write rather than read-only,
which reads backwards for a library that only reads them. A flatpak override
narrows what the manifest gave: a read-only grant lands as a deeper bind mount
on top of a writable one and takes the write away, so granting them read-only
would stop `cosmic-ext-tweaks` from applying a theme and any application holding
`--filesystem=host` from writing there. Read-write leaves every application
exactly the access it came with. The cost is that a sandboxed application can
write those three directories, which is to say it can change your theme.

Nothing is written per application, and `flatpak override --reset` is never
used. Taking the grant back removes those four entries from the global override
and leaves the rest of it alone, which matters on a COSMIC install because that
is where the GTK colour grants live. Earlier versions of this tool wrote one
override per application; `just flatpak-enable` clears those as it goes.

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
views. For a flatpak, pass the variable through the sandbox:
`flatpak run --env=COSMIC_GTK_APPEARANCE_DEBUG=1 <id>`.

## Limits

- An application that paints its own window background wins, because GTK
  flattens every layer before the compositor sees it. GTK is only the frame for
  Gecko and for Chromium, and both paint opaque: Thunderbird and Firefox,
  Electron applications like Discord, Slack and Spotify. The blur is attached
  and the stylesheet is loaded, and neither reaches the pixels. Making those
  translucent is work inside the application — for the Gecko family,
  `toolkit.legacyUserProfileCustomizations.stylesheets` with a `userChrome.css`
  that clears the chrome background, and `widget.wayland.opaque-region.enabled`
  set to false so the compositor is told the surface is not opaque.
- Popovers, menus and tooltips are separate surfaces and stay opaque on purpose,
  and so is anything drawn into a `picture`, so an image editor shows its image
  against a solid background rather than against the desktop. The area around a
  picture belongs to whatever scrolls it and stays translucent; GTK CSS has no
  way to say "the scrolled window that holds an image", so an application that
  wants that too needs a rule of its own through
  `COSMIC_GTK_APPEARANCE_CSS_EXTRA`.
- On a GTK that carries the protocol, cosmic-comp currently places the blur
  20px off: it reads the blur region as relative to the window geometry, while
  the protocol defines it as surface-local, and the two differ by exactly the
  shadow margin GTK leaves around a client-side decorated window. The result is
  a strip along the left and top edges that is translucent but not blurred.
  Nothing here can correct it — the region is GTK's to send — and it is one
  subtraction away in the compositor.
- X11 and XWayland windows get the colours and the decorations but never the
  blur.
- Only the `gtk.css` COSMIC generates is followed; a hand-written one is left
  alone, and its colours reach a window once, when it opens.
- Nothing here reaches a window that is not a GTK window: a Qt or libcosmic
  application loads the module and it returns without looking at anything.

## Licence

LGPL-3.0-or-later. The library is loaded into the address space of other
programs, including proprietary ones; the LGPL keeps the copyleft on this code
without reaching into whatever it ends up next to. `LICENSE` holds the LGPL-3
text, which incorporates by reference the GPL-3 in `LICENSE.GPL-3`.
