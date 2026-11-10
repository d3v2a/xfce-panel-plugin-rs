set shell := ["bash", "-cu"]

prefix := env('HOME') / ".local"

# build the workspace (cdylib + rlibs)
build:
    cargo build --release

# unit tests (no panel required)
test:
    cargo test

# debug build: the cdylibs the ABI harness dlopens (target/debug)
build-debug:
    cargo build

test-abi: build-debug
    MALLOC_CHECK_=3 XFCE_PANEL_PLUGIN_SO="$(pwd)/target/debug/libdemo.so:$(pwd)/target/debug/libhello.so" cargo test -p xfce-panel-sys --test abi_smoke

# clippy + fmt check
lint:
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check

fmt:
    cargo fmt
    nixfmt flake.nix

# install the .so + generated .desktop into ~/.local (the panel scans XDG dirs)
#
# Files are swapped atomically (copy to a sibling, then rename(2)). A running
# panel has the previous .so mapped and made resident: writing over that file in
# place (plain cp) rewrites the code pages the panel is executing, and the next
# call into the plugin - the save handler of a panel restart - jumps into the
# foreign image and segfaults. rename(2) leaves the old inode mapped.
install: build
    @mkdir -p {{prefix}}/lib/xfce4/panel/plugins {{prefix}}/share/xfce4/panel/plugins
    @for p in demo hello; do \
        src="target/release/lib$p.so"; \
        dst="{{prefix}}/lib/xfce4/panel/plugins/lib$p.so"; \
        cp "$src" "$dst.new" && mv -f "$dst.new" "$dst"; \
        src="$(find target/release/build -name "$p.desktop" -print -quit)"; \
        dst="{{prefix}}/share/xfce4/panel/plugins/$p.desktop"; \
        cp "$src" "$dst.new" && mv -f "$dst.new" "$dst"; \
    done
    @echo "installed in {{prefix}}:"
    @find {{prefix}}/lib/xfce4/panel/plugins {{prefix}}/share/xfce4/panel/plugins -name '*demo*' -o -name '*hello*'

# reload the panel: quit then relaunch.
# `xfce4-panel -r` is a restart race (the relaunched instance finds
# org.xfce.Panel still owned and quits, leaving no panel), see docs/README.md.
refresh:
    xfce4-panel -q || true
    setsid xfce4-panel >/dev/null 2>&1 &

# reload with the panel module factory's debug logs
debug:
    xfce4-panel -q || true
    PANEL_DEBUG=module-factory,main setsid xfce4-panel >/dev/null 2>&1 &

clean:
    cargo clean
