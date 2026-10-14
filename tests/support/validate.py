#!/usr/bin/env python3
"""Hand a generated stylesheet to a real GTK parser and check every colour it names."""
import os
import re
import sys
import warnings

warnings.simplefilter("ignore")

TOOLKITS = {
    "gtk3": ("Gtk", "3.0", False),
    "gtk4": ("Gtk", "4.0", False),
    "adw": ("Gtk", "4.0", True),
}
TOKEN = re.compile(r"@([A-Za-z0-9_]+)")
DEFINED = re.compile(r"@define-color\s+([A-Za-z0-9_]+)")


SKIP = 77


def open_toolkit(toolkit):
    namespace, version, adwaita = TOOLKITS[toolkit]
    import gi

    gi.require_version(namespace, version)
    if adwaita:
        gi.require_version("Adw", "1")
    from gi.repository import Gtk

    if adwaita:
        from gi.repository import Adw

        Adw.init()
    else:
        Gtk.init_check()
    from gi.repository import Gdk

    if Gdk.Display.get_default() is None:
        raise RuntimeError(f"{toolkit} could not open a display")
    return Gtk, version


def describe(section):
    try:
        return section.to_string()
    except AttributeError:
        return "the stylesheet"


def load(Gtk, version, text):
    problems = []
    provider = Gtk.CssProvider()
    provider.connect(
        "parsing-error",
        lambda _provider, section, error: problems.append(
            f"parse error at {describe(section)}: {error.message}"
        ),
    )
    try:
        if version == "4.0":
            provider.load_from_string(text)
        else:
            provider.load_from_data(text.encode())
    except Exception as error:  # GTK3 raises instead of only signalling
        problems.append(f"parse error: {error}")

    window = Gtk.Window()
    if version == "4.0":
        display = window.get_display()
        Gtk.StyleContext.add_provider_for_display(display, provider, 801)
    else:
        Gtk.StyleContext.add_provider_for_screen(window.get_screen(), provider, 801)

    context = Gtk.Window().get_style_context()
    asked = set(TOKEN.findall(text)) - {"define"} - set(DEFINED.findall(text))
    for name in sorted(asked):
        if not context.lookup_color(name)[0]:
            problems.append(f"undefined colour: @{name}")
    return problems


def main():
    if len(sys.argv) != 3:
        print("usage: validate.py <gtk3|gtk4|adw> <directory>", file=sys.stderr)
        return 2
    toolkit, directory = sys.argv[1], sys.argv[2]
    sheets = sorted(f for f in os.listdir(directory) if f.endswith(".css"))
    if not sheets:
        print(f"no stylesheets in {directory}", file=sys.stderr)
        return 2
    try:
        Gtk, version = open_toolkit(toolkit)
    except Exception as error:
        print(f"skipping {toolkit}: {error}")
        return SKIP
    failures = 0
    for sheet in sheets:
        with open(os.path.join(directory, sheet), encoding="utf-8") as handle:
            text = handle.read()
        for problem in load(Gtk, version, text):
            print(f"{sheet}: {problem}")
            failures += 1
    print(f"checked {len(sheets)} stylesheets under {toolkit}, {failures} problem(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
