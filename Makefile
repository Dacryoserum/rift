PREFIX ?= ~/.local
BIN    := $(PREFIX)/bin
BINARY := rift

VERSION := $(shell cargo metadata --no-deps --format-version 1 | sed 's/.*"version":"\([^"]*\)".*/\1/')
TARGET  := $(shell rustc -vV 2>/dev/null | sed -n 's/^host: //p')
ARCHIVE := rift-v$(VERSION)-$(TARGET).tar.gz

.PHONY: all build release install uninstall check test clean dist

all: build

build:
	cargo build

release:
	cargo build --release

install: release
	@mkdir -p $(BIN)
	cp target/release/$(BINARY) $(BIN)/$(BINARY)
	@echo "Installed to $(BIN)/$(BINARY)"

uninstall:
	rm -f $(BIN)/$(BINARY)
	@echo "Removed $(BIN)/$(BINARY)"

check:
	cargo fmt --check
	cargo clippy -- -D warnings

test:
	cargo test

dist: release
	tar -czf $(ARCHIVE) -C target/release $(BINARY)
	@echo "Created $(ARCHIVE)"

clean:
	cargo clean
	rm -f rift-*.tar.gz
