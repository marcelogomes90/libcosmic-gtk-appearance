#!/usr/bin/env bash
set -u

here="$(cd "$(dirname "$0")" && pwd)"
library="$here/target/release/libcosmic_gtk_appearance.so"
[ -f "$library" ] || { echo "build it first: cargo build --release" >&2; exit 1; }

translucent=0
[ "${1:-}" = "--content" ] && { translucent=1; shift; }
[ $# -gt 0 ] || { echo "usage: $0 [--content] <command> [args...]" >&2; exit 1; }

if [ "$translucent" = 1 ]; then
  target="$(command -v "$1" 2>/dev/null || true)"
  extra="$here/examples/content-gtk4.css"
  if [ -n "$target" ] \
     && ! /bin/grep -qF 'libgtk-4.so.1' "$target" 2>/dev/null \
     &&   /bin/grep -qF 'libgtk-3.so.0' "$target" 2>/dev/null; then
    extra="$here/examples/content-gtk3.css"
  fi
  echo "[try.sh] extra stylesheet: $(basename "$extra")" >&2
  export COSMIC_GTK_APPEARANCE_CSS_EXTRA="$extra"
fi

exec env COSMIC_GTK_APPEARANCE_DEBUG=1 LD_PRELOAD="$library" "$@"
