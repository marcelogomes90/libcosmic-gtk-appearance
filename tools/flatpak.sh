#!/usr/bin/env bash
set -u

action="${1:-}"
modules="${2:-}"
overrides="${XDG_DATA_HOME:-$HOME/.local/share}/flatpak/overrides"

apps() { flatpak list --app --columns=application 2>/dev/null; }

case "$action" in
enable)
  [ -d "$modules" ] || { echo "run 'make install-user' first" >&2; exit 1; }
  for app in $(apps); do
    if [ -e "$overrides/$app" ]; then
      echo "skipping $app: it already has an override of its own"
      continue
    fi
    flatpak override --user \
      --filesystem="$modules":ro \
      --filesystem=xdg-config/cosmic:ro \
      --env=GIO_EXTRA_MODULES="$modules" \
      "$app" && echo "enabled for $app"
  done
  ;;
disable)
  for app in $(apps); do
    [ -e "$overrides/$app" ] || continue
    if grep -q 'GIO_EXTRA_MODULES' "$overrides/$app" 2>/dev/null &&
       ! grep -qvE '^\[|^$|GIO_EXTRA_MODULES|filesystems=' "$overrides/$app" 2>/dev/null; then
      flatpak override --user --reset "$app" && echo "disabled for $app"
    else
      echo "leaving $app alone: its override holds settings that are not ours"
    fi
  done
  ;;
*)
  echo "usage: $0 {enable|disable} <modules-dir>" >&2
  exit 1
  ;;
esac
