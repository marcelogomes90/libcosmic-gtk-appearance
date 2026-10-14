#!/usr/bin/env python3
"""A minimal window per toolkit, shaped like the applications this library has to get right."""
import sys
import warnings

warnings.simplefilter("ignore")

ROWS = ["Keyrings", "GnuPG keys", "Secure Shell", "Certificates"]


def gtk3():
    import gi

    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk

    window = Gtk.Window(title="sample gtk3", default_width=800, default_height=500)
    pane = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
    header = Gtk.HeaderBar(title="Sidebar", show_close_button=False)
    listing = Gtk.ListBox()
    for row in ROWS:
        listing.add(Gtk.Label(label=row, xalign=0, margin=8))
    scrolled = Gtk.ScrolledWindow(width_request=220)
    scrolled.add(listing)
    pane.pack_start(header, False, False, 0)
    pane.pack_start(scrolled, True, True, 0)

    divider = Gtk.Separator(orientation=Gtk.Orientation.VERTICAL)
    divider.get_style_context().add_class("sidebar")

    content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
    content_header = Gtk.HeaderBar(title="Content", show_close_button=True)
    search = Gtk.SearchBar(search_mode_enabled=True)
    search.add(Gtk.SearchEntry())
    view = Gtk.TextView()
    view.get_buffer().set_text("A text view stays solid so the text keeps its contrast.")
    content.pack_start(content_header, False, False, 0)
    content.pack_start(search, False, False, 0)
    content.pack_start(view, True, True, 0)

    split = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
    split.pack_start(pane, False, False, 0)
    split.pack_start(divider, False, False, 0)
    split.pack_start(content, True, True, 0)
    window.add(split)
    window.connect("destroy", Gtk.main_quit)
    window.show_all()
    Gtk.main()


def gtk4(with_adwaita):
    import gi

    gi.require_version("Gtk", "4.0")
    if with_adwaita:
        gi.require_version("Adw", "1")
    from gi.repository import Gtk, Gio

    def build_content():
        listing = Gtk.ListBox()
        listing.add_css_class("navigation-sidebar")
        for row in ROWS:
            listing.append(Gtk.Label(label=row, xalign=0, margin_top=8, margin_bottom=8))
        return listing

    menu = Gio.Menu()
    menu.append("Something", "app.nothing")
    menu_button = Gtk.MenuButton(icon_name="open-menu-symbolic", menu_model=menu)
    chooser = Gtk.DropDown.new_from_strings(["Select", "Resize", "Crop"])

    if with_adwaita:
        from gi.repository import Adw

        Adw.init()
        sidebar_header = Adw.HeaderBar()
        sidebar_pane = Adw.ToolbarView()
        sidebar_pane.add_top_bar(sidebar_header)
        sidebar_pane.set_content(Gtk.ScrolledWindow(child=build_content()))
        content_header = Adw.HeaderBar()
        content_header.pack_end(menu_button)
        content_header.pack_start(chooser)
        body = Adw.ToolbarView()
        body.add_top_bar(content_header)
        view = Gtk.TextView()
        view.get_buffer().set_text("A text view stays solid so the text keeps its contrast.")
        body.set_content(view)
        split = Adw.NavigationSplitView(
            sidebar=Adw.NavigationPage(child=sidebar_pane, title="Sidebar"),
            content=Adw.NavigationPage(child=body, title="Content"),
        )
        window = Adw.ApplicationWindow(default_width=900, default_height=560)
        window.set_content(split)
    else:
        window = Gtk.Window(title="sample gtk4", default_width=900, default_height=560)
        header = Gtk.HeaderBar()
        header.pack_end(menu_button)
        header.pack_start(chooser)
        window.set_titlebar(header)
        sidebar = Gtk.ScrolledWindow(child=build_content(), width_request=220)
        sidebar.add_css_class("sidebar")
        view = Gtk.TextView()
        view.get_buffer().set_text("A text view stays solid so the text keeps its contrast.")
        split = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        split.append(sidebar)
        split.append(view)
        window.set_child(split)

    held = {"open": True}
    window.connect("close-request", lambda *_: (held.update(open=False), False)[1])
    window.present()
    from gi.repository import GLib

    loop = GLib.MainLoop()
    window.connect("close-request", lambda *_: (loop.quit(), False)[1])
    loop.run()


def main():
    which = sys.argv[1] if len(sys.argv) > 1 else "4"
    if which == "3":
        gtk3()
    elif which == "4":
        gtk4(False)
    elif which == "adw":
        gtk4(True)
    else:
        print("usage: sample.py <3|4|adw>", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
