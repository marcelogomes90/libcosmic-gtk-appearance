# libcosmic-gtk-appearance

Makes GTK windows follow the COSMIC desktop's appearance settings. Works on both
GTK3 and GTK4.

## The problem

`cosmic-comp` can blur the background behind any window: it implements
`ext_background_effect_v1` and KWin's `org_kde_kwin_blur`, and not just for the
shell's own surfaces — it works per `wl_surface`, for any client. That is how
`cosmic-files` gets the effect: it binds the protocol and asks for it itself.

GTK never asks. Neither libgtk nor libadwaita binds either protocol, and on top
of that they paint an opaque background. Both halves are missing, and neither is
reachable through a theme, a gsetting or a compositor flag.

This shim enters the process through `LD_PRELOAD` and closes both: it injects a
stylesheet that makes the window translucent and asks the compositor for a blur
region. It also strips the background from the title bar and flattens the window
buttons, which then only show a background on hover, using the colour and the
corner radius read from your COSMIC theme.

The shim **follows** the configuration rather than imposing a look. If frosted
glass is turned off under Appearance, nothing is made translucent — only the
decoration styling is applied, and the compositor is never contacted. The
translucency is applied only after the blur is actually in place: if the
compositor does not offer the effect, if the toolkit is not recognised, or if any
step fails, the shim gives up quietly and the app opens exactly as it would
without it.

## Installing

```sh
make && sudo make install   # /usr/lib/<multiarch>
make install-user           # ~/.local/lib, no root
make deb                    # target/deb/libcosmic-gtk-appearance_<version>_<arch>.deb
make check                  # rustfmt, clippy and the unit tests
```

Prefer the system path. Applications confined by AppArmor — `evince` among them —
refuse to map a preloaded library from a user directory, so `install-user` leaves
those out.

Installing does not enable anything. Pick one of these:

```sh
# one app: copy its .desktop into ~/.local/share/applications/ and prefix Exec
Exec=env LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libcosmic_gtk_appearance.so baobab

# flatpaks, once for all of them
flatpak override --user \
  --filesystem=/usr/lib/x86_64-linux-gnu/libcosmic_gtk_appearance.so:ro \
  --filesystem=xdg-config/cosmic:ro \
  --env=LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libcosmic_gtk_appearance.so

# the whole session: one line in /etc/environment
LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libcosmic_gtk_appearance.so
```

On a COSMIC session started by greetd, `/etc/environment` is the file that
actually reaches the apps, because `/etc/pam.d/cosmic-greeter` runs
`pam_env.so readenv=1`; `~/.config/environment.d` does not apply, since
`cosmic-session` is not a child of the systemd user manager. That last option
injects the library into every process on the system — the shim bails out after
a single `dlsym` when there is no glib, but it is still the most invasive choice.

## Usage

```sh
cargo build --release
LD_PRELOAD=$PWD/target/release/libcosmic_gtk_appearance.so baobab
```

While iterating, `./try.sh baobab` turns the log on, and `./try.sh --content
baobab` also makes the content area translucent, not just the chrome.

| variable | effect |
| --- | --- |
| `COSMIC_GTK_APPEARANCE_DEBUG=1` | log every step to stderr |
| `COSMIC_GTK_APPEARANCE_ALPHA=0.78` | force frosted glass at that opacity, ignoring the config |
| `COSMIC_GTK_APPEARANCE_CSS_EXTRA=f.css` | append rules to the default stylesheet |
| `COSMIC_GTK_APPEARANCE_CSS=f.css` | replace the default stylesheet entirely |
| `COSMIC_GTK_APPEARANCE_NO_CSS=1` | blur only, leave styling alone |
| `COSMIC_GTK_APPEARANCE_DISABLE=1` | turn the shim off for that process |

The stylesheet is installed at priority 801, just above
`GTK_STYLE_PROVIDER_PRIORITY_USER`. That is why it is injected instead of written
to a file: `~/.config/gtk-*/gtk.css` is rewritten by cosmic-settings on every
theme change.

## How it works

A constructor in `.init_array` schedules an idle callback; by the time the idle
runs, GTK is up. From there an emission hook on `GtkWidget::map` catches every
window that appears, and the path down to the `wl_surface` is public GDK API. The
protocol is spoken straight to libwayland, with the `wl_interface` tables built
by hand.

Nothing is interposed. The obvious approach would be to interpose
`wl_proxy_marshal_array_flags`, the non-variadic funnel every request goes
through, but it does not work: inside libwayland `wl_proxy_marshal_flags` calls
`array_flags` directly, without going through the PLT, so GDK's requests would
never reach the hook. And `marshal_flags` itself is variadic, which cannot be
forwarded.

## What the documentation does not tell you

**The blur region is anchored at the window geometry, and clipped by it.** The
spec talks about surface-local coordinates and about clipping to the surface
size, but cosmic-comp applies the region from the window geometry's corner —
already offset by the CSD shadow margin — and discards whatever falls outside it.
Sending the client's real origin pushed everything one extra shadow-width inwards
and left an unblurred strip along the left and top edges. Since the clipping
happens at the geometry, the shim sends a rectangle deliberately larger than any
window starting at `(0, 0)`: it always covers the whole window, never bleeds into
a halo, and the region stays immutable — no recomputation on resize, which is
where the missing edges and the blur that only came back on click came from.

**`set_blur_region(NULL)` removes the effect**, it does not apply it to the whole
surface: the initial region is empty. This is the opposite of
`org_kde_kwin_blur`, where null means the entire surface.

**The region is double-buffered and only takes effect on the surface's next
commit.** Because it is set from an idle callback, outside the paint cycle, a
`queue_draw` right afterwards is needed for a frame to carry it to the
compositor.

**`g_signal_lookup("map", GTK_TYPE_WIDGET)` can return 0.**
`gtk_widget_get_type()` registers the type but does not instantiate the class,
and signals are only installed in `class_init`. If the idle runs before the app
creates its first widget, the lookup fails; a `g_type_class_ref` beforehand fixes
it.

**In GTK3 a dialog's CSS node is `dialog`, not `window`.** Asking GTK for a
`GtkFileChooserDialog`'s style path returns
`dialog:dir-ltr.background.csd.tiled...`, so every `window ...` selector misses
it. The `decoration` node then never gets the tint while the plain `headerbar`
rule still matches, which leaves the chrome with no colour at all and the live
blurred content showing straight through the labels and icons. The glass rules
have to spell out `window`, `dialog` and `messagedialog`; on GTK4 the same case
is covered by `dialog.background`.

**In GTK3, menus and tooltips are `GtkWindow`s.** Checking `GTK_IS_WINDOW` is not
enough: an open menu is a popup-type window, and treating it as a normal one
makes the shim request blur for the menu and the stylesheet make its background
translucent. `gnome-power-statistics`, for instance, maps two of those at
startup. The cut is `gtk_window_get_window_type() == GTK_WINDOW_TOPLEVEL`, plus a
`:not(.popup)` on the glass rules, because GTK3 tags those windows with the
`popup` class.

**You cannot target the node that carries the icon.** Every header bar button
nests its icon differently: `windowcontrols > button > image` catches the window
controls, but a `GtkMenuButton`'s icon sits outside the reach of `headerbar
button image` and keeps inheriting the header bar's colour. The colour goes on
the buttons and travels down by inheritance, with labels reset to
`@headerbar_fg_color` so the text does not turn into the accent colour too.

**Inside a flatpak, `XDG_CONFIG_HOME` points at the app's private config**
(`~/.var/app/<id>/config`), not at yours. Reading the theme from there always
fails in the sandbox and the styling falls back to GTK colours, with no accent.
The shim tries `XDG_CONFIG_HOME/cosmic` and then `$HOME/.config/cosmic`, keeping
the first one that actually holds the theme — and the app needs
`--filesystem=xdg-config/cosmic:ro`.

**The decoration styling comes straight from the COSMIC config**, not from GTK:
`corner_radii.radius_s` for the radius, `icon_button.{on,hover,pressed}` for the
button and `accent.base` for the active window's icon, read from
`~/.config/cosmic/com.system76.CosmicTheme.*/v2/`. If those files are not
readable, the stylesheet falls back to GTK colours.

**`frosted` is not an on/off switch.** It is a `BlurStrength` — typically
`Medium` — and it sits next to an `alpha_map` that turns each level into an
opacity. The booleans beside it are what enables the effect: `frosted_windows`
for app windows, plus `frosted_panel`, `frosted_applets` and
`frosted_system_interface` for the rest of the shell. Looking only at `frosted`
would make the shim believe it is always on.

**The exact opacity does not come from the `alpha_map`** but from
`transparent_background.base` — the ready-made colour libcosmic uses for windows
(`#1A1B26C2`, i.e. 0.76). The `alpha_map` entry for the same level gives 0.79;
using it would leave GTK windows out of step with COSMIC ones. The sidebar
equivalent is `transparent_primary.base`. Do not read that token's alpha as a
statement about containers: it is fully opaque, yet `cosmic-files` is visibly
glassy right through its icon grid, so libcosmic clearly does not take the
content background from there.

**`frosted_maximized_apps` usually comes as `false`**: COSMIC does not frost a
maximized window. That can be honoured without tracking any state, because both
toolkits put a `maximized` class on the window node — `window.maximized` on GTK4,
`.maximized decoration` on GTK3.

## Limits

- **Apps confined by AppArmor refuse the library when it sits in a user path.**
  `evince` has a profile at `/etc/apparmor.d/usr.bin.evince`, and
  `abstractions/base` only allows mapping `.so` files from `/lib`, `/usr/lib` and
  `/usr/lib/<multiarch>`. Running it from `~/Projects` or from `/tmp` makes
  `ld.so` answer `cannot be preloaded (failed to map segment from shared object)`
  and the app opens with nothing applied. For those, the library has to be
  installed in `/usr/lib/x86_64-linux-gnu/` — which is the practical reason the
  install step is not just housekeeping.
- Apps that merely delegate to a service are out of reach. `gnome-terminal` is a
  D-Bus client: the window belongs to `gnome-terminal-server`, D-Bus activated,
  which does not inherit the `LD_PRELOAD`. It only works with a session-wide
  preload.
- Apps that load GTK through `dlopen` after start are out of reach too. The
  Firefox binary links only libc and libpthread; GTK3 arrives together with
  `libxul`, long after our constructor has run and found no glib. On top of that
  an already running instance takes over any new `firefox`, and Firefox paints
  its own interface with Gecko — even attached, there would be nothing to see.
- Popovers, menus and tooltips are separate surfaces and get no blur. The
  stylesheet deliberately leaves them opaque so they do not end up transparent
  with nothing behind them.
- An app that paints its own background stays opaque inside, and that matches
  COSMIC: `gnome-font-viewer` fills the window with a container drawn in
  `view_bg_color`, which the theme defines as solid. Use
  `COSMIC_GTK_APPEARANCE_CSS_EXTRA` with the files under `examples/` to depart from
  that.
- One proxy per window leaks at process exit. The `destroy` request is not sent
  from the surface's finalize handler because the `wl_display` may already be
  gone by then; swapping surfaces across a hide/show cycle destroys the previous
  one normally.

## Licence

LGPL-3.0-or-later. This library is preloaded into the address space of other
programs, including proprietary ones; the LGPL keeps the copyleft on this code
without reaching into whatever it ends up loaded next to. `LICENSE` holds the
LGPL-3 text, which incorporates by reference the GPL-3 in `LICENSE.GPL-3`.
