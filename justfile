name       := "libcosmic-gtk-appearance"
lib        := "libcosmic_gtk_appearance.so"
version    := `sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1`
multiarch  := `dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || gcc -print-multiarch`
deb_arch   := `dpkg-architecture -qDEB_HOST_ARCH 2>/dev/null || echo amd64`
maintainer := env('MAINTAINER', 'Marcelo <marcelo.sobrinho@outlook.com>')
sudo       := env('SUDO', 'sudo')

prefix       := env('PREFIX', '/usr')
libdir       := prefix / 'lib' / multiarch / 'gio/modules'
docdir       := prefix / 'share/doc' / name
datadir      := prefix / 'share' / name
user_libdir  := home_directory() / '.local/lib' / name
legacy_lib   := home_directory() / '.local/lib/gio/modules' / lib
session_envd := home_directory() / '.config/environment.d'
session_env  := session_envd / '95-cosmic-gtk-appearance.conf'
user_datadir := home_directory() / '.local/share' / name
debroot      := 'target/deb' / name

[private]
default:
    @just --list

build:
    cargo build --release --locked

check:
    cargo fmt --all -- --check
    cargo clippy --release --all-targets -- -D warnings
    cargo test --release --locked

# Check that installing, repeating and uninstalling all converge
test-install:
    tests/install.sh

# Install for native applications and for every flatpak
setup: install install-user flatpak-enable session-enable
    @printf '\n%s\n' 'Done. What that means:'
    @printf '  %s\n' 'applications started from now on pick the library up'
    @printf '  %s\n' 'applications already open keep the copy they loaded'
    @printf '  %s\n' 'the blur setting for your session needs a new login'
    @printf '  %s\n' 'run `just status` to see what is in place'

# Copy the library to one tree, atomically so running applications keep their mapping
[private]
stage destdir sudo='':
    #!/usr/bin/env bash
    set -euo pipefail
    place() {
        local source=$1 target=$2
        {{sudo}} install -D -m 0644 "$source" "$target.new"
        {{sudo}} mv -f "$target.new" "$target"
    }
    place target/release/{{lib}} '{{destdir}}{{libdir}}/{{lib}}'
    place README.md '{{destdir}}{{docdir}}/README.md'
    place LICENSE '{{destdir}}{{docdir}}/LICENSE'
    place LICENSE.GPL-3 '{{destdir}}{{docdir}}/LICENSE.GPL-3'
    {{sudo}} install -d -m 0755 '{{destdir}}{{datadir}}/examples'
    for sheet in examples/*.css; do
        place "$sheet" "{{destdir}}{{datadir}}/examples/$(basename "$sheet")"
    done

# Install for applications running on the system
install: build (stage '' sudo)
    @printf '%s\n' 'installed {{libdir}}/{{lib}}'
    @printf '%s\n' '          {{docdir}} and {{datadir}}'
    @printf '%s\n' 'GTK applications pick it up on their next start; open ones keep the old copy.'

# Install the copy flatpaks can reach
install-user: build
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -e '{{legacy_lib}}' ]; then
        rm -f '{{legacy_lib}}'
        printf '%s\n' 'removed the copy this tool used to install at {{legacy_lib}}'
    fi
    install -D -m 0644 target/release/{{lib}} '{{user_libdir}}/{{lib}}.new'
    mv -f '{{user_libdir}}/{{lib}}.new' '{{user_libdir}}/{{lib}}'
    install -d -m 0755 '{{user_datadir}}/examples'
    install -m 0644 examples/*.css '{{user_datadir}}/examples/'
    printf '%s\n' 'installed {{user_libdir}}/{{lib}} — this is the copy flatpaks reach'

# Hand the blur protocol to this library in applications the session starts
session-enable:
    #!/usr/bin/env bash
    set -euo pipefail
    install -d -m 0755 '{{session_envd}}'
    printf '%s\n' 'GDK_WAYLAND_DISABLE=${GDK_WAYLAND_DISABLE}:ext_background_effect_manager_v1' \
        > '{{session_env}}.new'
    mv -f '{{session_env}}.new' '{{session_env}}'
    printf '%s\n' 'wrote {{session_env}}'
    printf '%s\n' 'Applications your session starts pick that up at your next login.'

# Give it back to GTK
[private]
session-disable:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -e '{{session_env}}' ]; then
        rm -f '{{session_env}}'
        printf '%s\n' 'removed {{session_env}}'
    else
        printf '%s\n' 'no session setting was in place'
    fi

# Grant the user copy to every flatpak, present and future
flatpak-enable:
    tools/flatpak.sh enable '{{user_libdir}}'

# Take that grant back
flatpak-disable:
    tools/flatpak.sh disable '{{user_libdir}}'

# Remove everything this installed
uninstall: flatpak-disable session-disable uninstall-user uninstall-system
    @printf '\n%s\n' 'Nothing of this library is left pointing at a file that no longer exists.'
    @printf '%s\n' 'Applications already open keep what they loaded until they restart.'

[private]
uninstall-system:
    #!/usr/bin/env bash
    set -euo pipefail
    for tree in '{{docdir}}' '{{datadir}}'; do
        case "$tree" in
            */{{name}}) ;;
            *) printf '%s\n' "refusing to remove $tree, it is not a {{name}} directory" >&2; exit 1 ;;
        esac
    done
    removed=0
    if [ -e '{{libdir}}/{{lib}}' ]; then
        {{sudo}} rm -f '{{libdir}}/{{lib}}'
        printf '%s\n' 'removed {{libdir}}/{{lib}}'
        removed=1
    fi
    for tree in '{{docdir}}' '{{datadir}}'; do
        if [ -d "$tree" ]; then
            {{sudo}} rm -rf "$tree"
            printf '%s\n' "removed $tree"
            removed=1
        fi
    done
    [ "$removed" = 1 ] || printf '%s\n' 'nothing was installed for the system'

[private]
uninstall-user:
    #!/usr/bin/env bash
    set -euo pipefail
    removed=0
    for leftover in '{{user_libdir}}/{{lib}}' '{{legacy_lib}}'; do
        if [ -e "$leftover" ]; then
            rm -f "$leftover"
            printf '%s\n' "removed $leftover"
            removed=1
        fi
    done
    for tree in '{{user_libdir}}' '{{user_datadir}}'; do
        if [ -d "$tree" ]; then
            rm -rf "$tree"
            printf '%s\n' "removed $tree"
            removed=1
        fi
    done
    [ "$removed" = 1 ] || printf '%s\n' 'nothing was installed for your user'

# Report what is in place and what is actually driving your session
status:
    #!/usr/bin/env bash
    set -u
    say() { printf '  %-7s %s\n' "$1" "$2"; }
    present() { [ -e "$1" ] && say yes "$2" || say no "$2"; }
    printf '%s\n' 'Installed:'
    present '{{libdir}}/{{lib}}' 'system copy at {{libdir}}/{{lib}}'
    present '{{user_libdir}}/{{lib}}' 'copy flatpaks reach at {{user_libdir}}/{{lib}}'
    present '{{legacy_lib}}' 'stale copy at {{legacy_lib}} (run `just install-user` to clear)'
    present '{{session_env}}' 'session setting at {{session_env}}'
    overrides="${XDG_DATA_HOME:-$HOME/.local/share}/flatpak/overrides/global"
    if grep -qF '{{user_libdir}}' "$overrides" 2>/dev/null; then
        say yes 'flatpak grant in the global override'
    else
        say no 'flatpak grant in the global override'
    fi
    printf '\n%s\n' 'Driving the blur:'
    if printf '%s' "${GDK_WAYLAND_DISABLE:-}" | grep -q 'ext_background_effect_manager_v1'; then
        say yes 'GDK_WAYLAND_DISABLE names the manager in this shell'
    elif [ -e '{{session_env}}' ]; then
        say no 'the session setting is installed but has not reached this shell'
        printf '          %s\n' 'environment.d only reaches what the systemd user session starts;'
        printf '          %s\n' 'the library claims the protocol itself for anything else.'
    else
        say no 'nothing hands the blur to this library yet'
    fi
    printf '\n%s\n' 'COSMIC appearance:'
    for leaf in gtk-3.0 gtk-4.0; do
        sheet="${XDG_CONFIG_HOME:-$HOME/.config}/$leaf/gtk.css"
        if grep -qF 'GENERATED BY COSMIC' "$sheet" 2>/dev/null; then
            say yes "COSMIC palette for $leaf"
        else
            say no "COSMIC palette for $leaf (the library falls back to the GTK colours)"
        fi
    done
    root="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic"
    dark=$(cat "$root/com.system76.CosmicTheme.Mode/v1/is_dark" 2>/dev/null || echo unknown)
    mode=$([ "$dark" = false ] && echo Light || echo Dark)
    frosted=$(cat "$root/com.system76.CosmicTheme.$mode/v2/frosted_windows" 2>/dev/null || echo unknown)
    say "$frosted" "frosted_windows in the $mode theme"

# Build a .deb
deb: build (stage debroot)
    install -d -m 0755 {{debroot}}/DEBIAN
    printf '%s\n' \
      'Package: {{name}}' \
      'Version: {{version}}' \
      'Section: x11' \
      'Priority: optional' \
      'Architecture: {{deb_arch}}' \
      'Depends: libc6, libwayland-client0' \
      'Maintainer: {{maintainer}}' \
      'Description: Make GTK windows follow the COSMIC appearance settings' \
      ' A GIO module that lets GTK3 and GTK4 applications pick up the COSMIC' \
      ' desktop appearance: frosted glass when it is enabled, the accent colour' \
      ' on window decorations, and the theme corner radius. It follows the' \
      ' configuration rather than forcing a look, and stays inert when the blur' \
      ' is unavailable.' \
      > {{debroot}}/DEBIAN/control
    dpkg-deb --root-owner-group --build {{debroot}} target/deb/{{name}}_{{version}}_{{deb_arch}}.deb
    @ls -l target/deb/*.deb

clean:
    cargo clean
