#!/usr/bin/env bash
# Check that installing, repeating, recovering and uninstalling all converge.
set -uo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

name=libcosmic-gtk-appearance
lib=libcosmic_gtk_appearance.so
multiarch="$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || gcc -print-multiarch)"

real_home="$HOME"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
prefix="$work/prefix"
fake_home="$work/home"
stub="$work/stub"
mkdir -p "$prefix" "$fake_home" "$stub"

cat > "$stub/flatpak" <<'STUB'
#!/bin/sh
case "${1:-}" in
    list) exit 0 ;;
    override) printf '%s\n' "$*" >> "${FLATPAK_STUB_LOG:-/dev/null}" ; exit 0 ;;
    *) exit 0 ;;
esac
STUB
chmod +x "$stub/flatpak"
export FLATPAK_STUB_LOG="$work/override.log"

module="$prefix/lib/$multiarch/gio/modules/$lib"
docs="$prefix/share/doc/$name"
data="$prefix/share/$name"
user_lib="$fake_home/.local/lib/$name/$lib"
user_data="$fake_home/.local/share/$name"
legacy="$fake_home/.local/lib/gio/modules/$lib"
session="$fake_home/.config/environment.d/95-cosmic-gtk-appearance.conf"
overrides="$fake_home/.local/share/flatpak/overrides/global"

failures=0
checks=0

ok() { checks=$((checks + 1)); printf '  ok    %s\n' "$1"; }
bad() { checks=$((checks + 1)); failures=$((failures + 1)); printf '  FAIL  %s\n' "$1"; }
exists() { [ -e "$1" ] && ok "$2" || bad "$2"; }
absent() { [ -e "$1" ] && bad "$2" || ok "$2"; }
holds() { grep -qF -- "$2" "$1" 2>/dev/null && ok "$3" || bad "$3"; }
lacks() { grep -qF -- "$2" "$1" 2>/dev/null && bad "$3" || ok "$3"; }

run() {
    env HOME="$fake_home" \
        XDG_DATA_HOME="$fake_home/.local/share" \
        XDG_CONFIG_HOME="$fake_home/.config" \
        RUSTUP_HOME="${RUSTUP_HOME:-$real_home/.rustup}" \
        CARGO_HOME="${CARGO_HOME:-$real_home/.cargo}" \
        PREFIX="$prefix" SUDO= PATH="$stub:$PATH" \
        just "$@" > "$work/out" 2>&1
    local code=$?
    [ "$code" = 0 ] || { printf '  FAIL  just %s exited %s\n' "$*" "$code"; sed 's/^/        /' "$work/out"; failures=$((failures + 1)); }
    checks=$((checks + 1))
    return 0
}

no_leftover_staging() {
    local found
    found="$(find "$prefix" "$fake_home" -name '*.new' 2>/dev/null)"
    [ -z "$found" ] && ok "$1" || { bad "$1"; printf '        %s\n' "$found"; }
}

everything_is_in_place() {
    exists "$module" 'the module is installed for the system'
    exists "$docs/README.md" 'the README is installed'
    exists "$docs/LICENSE" 'the licence is installed'
    exists "$data/examples/content-gtk3.css" 'the example stylesheets are installed'
    exists "$user_lib" 'the copy flatpaks reach is installed'
    exists "$user_data/examples/content-gtk4.css" 'the user examples are installed'
    holds "$session" 'ext_background_effect_manager_v1' 'the session setting names the blur manager'
    no_leftover_staging 'no staging file is left behind'
}

printf '%s\n' '== a clean install =='
run build
run install
run install-user
run session-enable
everything_is_in_place
cmp -s target/release/"$lib" "$module" && ok 'the installed module is the one just built' \
    || bad 'the installed module is the one just built'

printf '%s\n' '== installing again changes nothing =='
before="$(find "$prefix" "$fake_home" -type f | sort | xargs -r md5sum | md5sum)"
run install
run install-user
run session-enable
after="$(find "$prefix" "$fake_home" -type f | sort | xargs -r md5sum | md5sum)"
[ "$before" = "$after" ] && ok 'the second install leaves the same tree' \
    || bad 'the second install leaves the same tree'

printf '%s\n' '== an interrupted install is repaired =='
: > "$module"
printf 'GDK_WAYLAND_DISABLE=${GDK_WAYLAND_' > "$session"
touch "$module.new" "$fake_home/.local/lib/$name/$lib.new"
mkdir -p "$(dirname "$legacy")" && touch "$legacy"
rm -rf "$data/examples"
run install
run install-user
run session-enable
everything_is_in_place
cmp -s target/release/"$lib" "$module" && ok 'the truncated module was replaced' \
    || bad 'the truncated module was replaced'
absent "$legacy" 'the stale copy was cleared'

printf '%s\n' '== uninstalling keeps what we never wrote =='
mkdir -p "$(dirname "$overrides")"
cat > "$overrides" <<OVERRIDE
[Context]
filesystems=xdg-config/kdeglobals:ro;$fake_home/.local/lib/$name:ro;xdg-data/color-schemes:ro;xdg-config/cosmic/com.system76.CosmicTheme.Dark;

[Environment]
GIO_EXTRA_MODULES=$fake_home/.local/lib/$name
GDK_WAYLAND_DISABLE=ext_background_effect_manager_v1
EDITOR=vim
OVERRIDE
run uninstall
absent "$module" 'the system module is gone'
absent "$docs" 'the documentation directory is gone'
absent "$data" 'the data directory is gone'
absent "$user_lib" 'the user copy is gone'
absent "$user_data" 'the user data directory is gone'
absent "$session" 'the session setting is gone'
holds "$overrides" 'kdeglobals' 'a foreign filesystem grant survived'
holds "$overrides" 'color-schemes' 'a second foreign grant survived'
holds "$overrides" 'EDITOR=vim' 'a foreign environment variable survived'
lacks "$overrides" "$name" 'our own filesystem grant is gone'
lacks "$overrides" 'GIO_EXTRA_MODULES' 'our own module path is gone'

printf '%s\n' '== uninstalling again is still clean =='
run uninstall
absent "$module" 'nothing came back'
absent "$user_lib" 'nothing came back for the user'

printf '\n%s\n' "$checks checks, $failures failure(s)"
[ "$failures" = 0 ]
