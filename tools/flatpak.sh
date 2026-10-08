#!/usr/bin/env bash
set -u

action="${1:-}"
modules="${2:-}"
overrides="${XDG_DATA_HOME:-$HOME/.local/share}/flatpak/overrides"

command -v flatpak >/dev/null || { echo "flatpak is not installed, nothing to do"; exit 0; }

apps() { flatpak list --app --columns=application 2>/dev/null; }

# An override is ours alone when every line in it is one we wrote.
ours_alone() {
    local file=$1 line rest
    [ -f "$file" ] || return 1
    while IFS= read -r line; do
        case "$line" in
            '' | '['*) ;;
            filesystems=*)
                rest=${line#filesystems=}
                rest=${rest//"$modules:ro;"/}
                rest=${rest//'xdg-config/cosmic:ro;'/}
                [ -n "$rest" ] && return 1
                ;;
            GIO_EXTRA_MODULES=*) ;;
            *) return 1 ;;
        esac
    done < "$file"
    return 0
}

case "$action" in
enable)
    [ -d "$modules" ] || { echo "run 'just install-user' first" >&2; exit 1; }
    for app in $(apps); do
        if [ -e "$overrides/$app" ] && ! ours_alone "$overrides/$app"; then
            echo "skipped $app, it carries an override of its own"
            continue
        fi
        flatpak override --user \
            --filesystem="$modules":ro \
            --filesystem=xdg-config/cosmic:ro \
            --env=GIO_EXTRA_MODULES="$modules" \
            "$app" && echo "granted to $app"
    done
    ;;
disable)
    for app in $(apps); do
        [ -e "$overrides/$app" ] || continue
        if ours_alone "$overrides/$app"; then
            flatpak override --user --reset "$app" && echo "revoked from $app"
        else
            echo "left $app alone, its override carries more than ours"
        fi
    done
    ;;
*)
    echo "usage: $0 {enable|disable} <modules-dir>" >&2
    exit 1
    ;;
esac
