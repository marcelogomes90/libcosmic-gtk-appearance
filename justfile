name       := "libcosmic-gtk-appearance"
lib        := "libcosmic_gtk_appearance.so"
version    := `sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1`
multiarch  := `dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || gcc -print-multiarch`
deb_arch   := `dpkg-architecture -qDEB_HOST_ARCH 2>/dev/null || echo amd64`
maintainer := env('MAINTAINER', 'Marcelo <marcelo.sobrinho@outlook.com>')

prefix       := env('PREFIX', '/usr')
libdir       := prefix / 'lib' / multiarch / 'gio/modules'
docdir       := prefix / 'share/doc' / name
datadir      := prefix / 'share' / name
user_libdir  := home_directory() / '.local/lib' / name
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

# Install for native applications and for every flatpak
setup: install install-user flatpak-enable session-enable

[private]
stage destdir sudo='':
    {{sudo}} install -D -m 0644 target/release/{{lib}} {{destdir}}{{libdir}}/{{lib}}
    {{sudo}} install -D -m 0644 README.md {{destdir}}{{docdir}}/README.md
    {{sudo}} install -D -m 0644 LICENSE {{destdir}}{{docdir}}/LICENSE
    {{sudo}} install -D -m 0644 LICENSE.GPL-3 {{destdir}}{{docdir}}/LICENSE.GPL-3
    {{sudo}} install -d -m 0755 {{destdir}}{{datadir}}/examples
    {{sudo}} install -m 0644 examples/*.css {{destdir}}{{datadir}}/examples/

# Install for applications running on the system
install: build (stage '' 'sudo')
    @echo 'Installed {{libdir}}/{{lib}} — GTK applications pick it up on their next start.'

# Install the copy flatpaks can reach
install-user: build
    rm -f {{home_directory()}}/.local/lib/gio/modules/{{lib}}
    install -D -m 0644 target/release/{{lib}} {{user_libdir}}/{{lib}}
    install -d -m 0755 {{user_datadir}}/examples
    install -m 0644 examples/*.css {{user_datadir}}/examples/

# Hand the blur protocol to this library in applications the session starts
session-enable:
    install -d -m 0755 {{session_envd}}
    printf '%s\n' 'GDK_WAYLAND_DISABLE=${GDK_WAYLAND_DISABLE}:ext_background_effect_manager_v1' > {{session_env}}
    @echo 'Native applications pick that up at your next login.'

# Give it back to GTK
[private]
session-disable:
    rm -f {{session_env}}

# Grant the user copy to every flatpak, present and future
flatpak-enable:
    tools/flatpak.sh enable {{user_libdir}}

# Take that grant back
flatpak-disable:
    tools/flatpak.sh disable {{user_libdir}}

# Remove everything this installed
uninstall: flatpak-disable session-disable uninstall-user uninstall-system

[private]
uninstall-system:
    sudo rm -f {{libdir}}/{{lib}}
    sudo rm -rf {{docdir}} {{datadir}}

[private]
uninstall-user:
    rm -f {{user_libdir}}/{{lib}} {{home_directory()}}/.local/lib/gio/modules/{{lib}}
    rm -rf {{user_libdir}} {{user_datadir}}

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
