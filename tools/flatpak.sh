#!/usr/bin/env bash
set -u

action=${1:-}
modules=${2:-}
legacy="$HOME/.local/lib/gio/modules"
overrides="${XDG_DATA_HOME:-$HOME/.local/share}/flatpak/overrides"
themes=(com.system76.CosmicTheme.Mode com.system76.CosmicTheme.Dark com.system76.CosmicTheme.Light)

command -v flatpak >/dev/null || { echo "flatpak is not installed, nothing to do"; exit 0; }

apps() { flatpak list --app --columns=application 2>/dev/null; }

ours() {
    local entries="$modules;$modules:ro;$legacy;$legacy:ro"
    local theme
    for theme in "${themes[@]}"; do
        entries="$entries;xdg-config/cosmic/$theme;xdg-config/cosmic/$theme:ro"
    done
    printf '%s' "$entries"
}

prune() {
    local file=$1
    [ -f "$file" ] || return 1
    awk -v drop_fs="$(ours)" -v drop_env="GIO_EXTRA_MODULES" '
        function keep(token,   n, i, listed) {
            n = split(drop_fs, listed, ";")
            for (i = 1; i <= n; i++) if (listed[i] != "" && listed[i] == token) return 0
            return 1
        }
        /^\[/ { section = $0; order[++sections] = section; next }
        /=/ {
            key = substr($0, 1, index($0, "=") - 1)
            value = substr($0, index($0, "=") + 1)
            if (key == "filesystems") {
                kept = ""
                n = split(value, tokens, ";")
                for (i = 1; i <= n; i++)
                    if (tokens[i] != "" && keep(tokens[i])) kept = kept tokens[i] ";"
                if (kept == "") next
                $0 = key "=" kept
            } else if (section == "[Environment]" && key == drop_env) next
            body[section] = body[section] $0 "\n"
            next
        }
        END {
            blank = 0
            for (i = 1; i <= sections; i++) {
                s = order[i]
                if (body[s] == "" || seen[s]) continue
                seen[s] = 1
                if (blank++) print ""
                print s
                printf "%s", body[s]
            }
        }
    ' "$file" > "$file.new" || { rm -f "$file.new"; return 1; }
    if [ -s "$file.new" ]; then
        mv "$file.new" "$file"
    else
        rm -f "$file.new" "$file"
    fi
}

forget_per_app() {
    local app cleared=0
    for app in $(apps); do
        grep -qF -e "$modules" -e "$legacy" -e 'xdg-config/cosmic/com.system76.CosmicTheme' \
            "$overrides/$app" 2>/dev/null || continue
        prune "$overrides/$app" && cleared=$((cleared + 1))
    done
    [ "$cleared" -gt 0 ] && echo "cleared $cleared per-application override(s) this tool had written"
    return 0
}

case "$action" in
enable)
    [ -d "$modules" ] || { echo "run 'just install-user' first" >&2; exit 1; }
    forget_per_app
    prune "$overrides/global"
    grants=(--filesystem="$modules":ro --env=GIO_EXTRA_MODULES="$modules")
    for theme in "${themes[@]}"; do
        grants+=(--filesystem=xdg-config/cosmic/"$theme")
    done
    flatpak override --user "${grants[@]}" \
        && echo "granted to every flatpak, installed or not yet, through the global override"
    ;;
disable)
    forget_per_app
    prune "$overrides/global"
    echo "revoked from every flatpak"
    ;;
*)
    echo "usage: $0 {enable|disable} <modules-dir>" >&2
    exit 1
    ;;
esac
