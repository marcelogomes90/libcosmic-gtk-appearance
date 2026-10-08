NAME      := libcosmic-gtk-appearance
LIB       := libcosmic_gtk_appearance.so
VERSION   := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
MULTIARCH ?= $(shell dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || gcc -print-multiarch)
DEB_ARCH  ?= $(shell dpkg-architecture -qDEB_HOST_ARCH 2>/dev/null || echo amd64)
MAINTAINER ?= Marcelo <marcelo.sobrinho@outlook.com>

PREFIX  ?= /usr
LIBDIR  ?= $(PREFIX)/lib/$(MULTIARCH)/gio/modules
DOCDIR  ?= $(PREFIX)/share/doc/$(NAME)
DATADIR ?= $(PREFIX)/share/$(NAME)

USER_LIBDIR  ?= $(HOME)/.local/lib/gio/modules
USER_DATADIR ?= $(HOME)/.local/share/$(NAME)

CARGO   ?= cargo
DEBROOT := target/deb/$(NAME)

.PHONY: all build check install install-user uninstall uninstall-user deb clean

all: build

build:
	$(CARGO) build --release --locked

check:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --release --all-targets -- -D warnings
	$(CARGO) test --release --locked

install: build
	install -D -m 0644 target/release/$(LIB) $(DESTDIR)$(LIBDIR)/$(LIB)
	install -D -m 0644 README.md $(DESTDIR)$(DOCDIR)/README.md
	install -D -m 0644 LICENSE $(DESTDIR)$(DOCDIR)/LICENSE
	install -D -m 0644 LICENSE.GPL-3 $(DESTDIR)$(DOCDIR)/LICENSE.GPL-3
	install -d -m 0755 $(DESTDIR)$(DATADIR)
	install -d -m 0755 $(DESTDIR)$(DATADIR)/examples
	install -m 0644 examples/*.css $(DESTDIR)$(DATADIR)/examples/
	@echo
	@echo "Installed $(LIBDIR)/$(LIB)"
	@echo "GTK applications pick it up on their next start. Nothing else to do."

install-user: build
	install -D -m 0644 target/release/$(LIB) $(USER_LIBDIR)/$(LIB)
	install -d -m 0755 $(USER_DATADIR)
	install -d -m 0755 $(USER_DATADIR)/examples
	install -m 0644 examples/*.css $(USER_DATADIR)/examples/
	@echo
	@echo "Installed $(USER_LIBDIR)/$(LIB)"
	@echo "This copy is for flatpaks; point GIO_EXTRA_MODULES at the directory."

uninstall:
	rm -f $(DESTDIR)$(LIBDIR)/$(LIB)
	rm -rf $(DESTDIR)$(DOCDIR) $(DESTDIR)$(DATADIR)

uninstall-user:
	rm -f $(USER_LIBDIR)/$(LIB)
	rm -rf $(USER_DATADIR)

deb: build
	rm -rf $(DEBROOT)
	$(MAKE) install DESTDIR=$(CURDIR)/$(DEBROOT) PREFIX=/usr
	install -d -m 0755 $(DEBROOT)/DEBIAN
	printf '%s\n' \
	  'Package: $(NAME)' \
	  'Version: $(VERSION)' \
	  'Section: x11' \
	  'Priority: optional' \
	  'Architecture: $(DEB_ARCH)' \
	  'Depends: libc6, libwayland-client0' \
	  'Maintainer: $(MAINTAINER)' \
	  'Description: Make GTK windows follow the COSMIC appearance settings' \
	  ' An LD_PRELOAD shim that lets GTK3 and GTK4 applications pick up the' \
	  ' COSMIC desktop appearance: frosted glass when it is enabled, the accent' \
	  ' colour on window decorations, and the theme corner radius. It follows the' \
	  ' configuration rather than forcing a look, and stays inert when the blur' \
	  ' is unavailable.' \
	  > $(DEBROOT)/DEBIAN/control
	dpkg-deb --root-owner-group --build $(DEBROOT) target/deb/$(NAME)_$(VERSION)_$(DEB_ARCH).deb
	@echo
	@ls -l target/deb/*.deb

clean:
	$(CARGO) clean
