# xfce-panel-plugin-rs

Write `xfce4-panel` plugins (API 2.0) in Rust, on top of `libxfce4panel-2.0`.

## Quick start: a working plugin in 5 minutes

### 1. System libraries

The build probes these with `pkg-config`; install the `-dev`/`-devel`
equivalents of your distro:

| pkg-config name | What it is |
| --- | --- |
| `gtk+-3.0` | GTK 3 (the plugin is a `GtkEventBox`) |
| `libxfce4panel-2.0` | the plugin API |
| `libxfce4util-1.0` | Xfce utilities |
| `xfconf-1.0` | settings storage (D-Bus) — only with the `settings` feature |

Plus a Rust toolchain (**1.95+**, edition 2024) and `pkg-config`.

### 2. Build

```sh
cargo build --release      # cdylibs: target/release/lib{hello,demo}.so
```

Each plugin's `build.rs` also generates its `.desktop` (under
`target/release/build/*/`), so there is nothing to write by hand.

### 3. Install into your session

The panel scans the XDG data directories, so a per-user install is enough.
**Do not overwrite an in-place `.so`** (see [Reload safety](#reload-safety)) —
`just install` copies to `file.new` then swaps with `rename(2)`:

```sh
just install               # → ~/.local/lib/xfce4/panel/plugins/lib{hello,demo}.so
                           # → ~/.local/share/xfce4/panel/plugins/{hello,demo}.desktop
```

Without `just`, the same steps (the `.desktop` lands in the crate's
`build/…/out` directory):

```sh
mkdir -p ~/.local/lib/xfce4/panel/plugins ~/.local/share/xfce4/panel/plugins
src=target/release/libhello.so; dst=~/.local/lib/xfce4/panel/plugins/libhello.so
cp "$src" "$dst.new" && mv -f "$dst.new" "$dst"
cp "$(find target/release/build -name hello.desktop -print -quit)" \
   ~/.local/share/xfce4/panel/plugins/hello.desktop
```

### 4. Load it

```sh
just refresh               # = xfce4-panel -q, then relaunch
```

Then *Panel* → *Panel preferences* → **Add new items** → **Rust Hello**
(or **Rust Demo**).

### The whole plugin

`crates/xfce-panel-hello/src/lib.rs` — one label, no settings:

```rust
use xfce_panel::gtk::prelude::WidgetExt;
use xfce_panel::{PanelPlugin, Plugin, gtk};

#[derive(Default)]
pub struct Hello;

impl PanelPlugin for Hello {
    const TYPE_NAME: &'static str = "XfceRustHello";
    type State = ();

    fn construct(_state: &mut Self::State, plugin: &Plugin) {
        let label = gtk::Label::new(Some("Hello from Rust"));
        label.set_widget_name("xfce-panel-rs-label");
        plugin.add_widget(&label);
        label.show();
    }

    fn size_changed(_state: &mut Self::State, plugin: &Plugin, size: i32) -> bool {
        plugin.set_size_request(size, size);
        true
    }
}

// Resident: the `.so` registers a `GType`, so it must never be `dlclose`d.
xfce_panel::define_panel_plugin!(Hello);
```

The macro exports the one symbol the panel `dlsym`s
(`xfce_panel_module_init`); `construct` runs at the plugin's realize.

## Workspace

| Crate | Role |
| --- | --- |
| `crates/xfce-panel-sys` | Raw FFI bindings: `XfcePanelPlugin*` structs, `XfcePanelPluginClass` vtable, ABI entry points (`xfce_panel_module_init`). |
| `crates/xfce-panel` | Safe API: `PanelPlugin` trait, GType registration (`register_type` + `define_panel_plugin!` macro), xfconf settings (`Channel`), per-instance state (GData), `.desktop` generation (`build` feature). |
| `crates/xfce-panel-macros` | `#[xfce_panel::panel_settings]` attribute macro: the struct you declare becomes the `glib::Object` whose every field is an xfconf-bound property. |
| `crates/xfce-panel-demo` | Demo plugin (`cdylib` `libdemo.so`); its `build.rs` calls the `.desktop` generator from `xfce-panel`. |
| `crates/xfce-panel-hello` | Minimal plugin (`cdylib` `libhello.so`): one label, no settings — the contract of a plugin without xfconf. |

## Features

`xfce-panel` has two features, both **off** by default:

| Feature | Pulls | Use it for |
|---|---|---|
| `settings` | `xfconf` + the `xfce-panel-macros` proc-macro | xfconf-backed settings: `Channel`, `XfconfType`, `#[panel_settings]` |
| `build` | nothing | the plugin's `build.rs` (`.desktop` generation) |

```toml
[dependencies]
xfce-panel = { version = "0.1", features = ["settings"] }  # a plugin with settings
xfce-panel = "0.1"                                        # a plugin without any
```

Without `settings`, `xfce-panel` compiles against `libxfce4panel`,
`libxfce4util` and gtk only: no `xfconf` crate, no xfconfd, no session bus —
verified on the image with `readelf -d target/release/libhello.so | grep NEEDED`.

One caveat: **Cargo unifies features within a single workspace build**, so
`cargo build` (all members) compiles one `xfce-panel` with `settings` — because
`demo` asks for it — and `libxfconf-0.so.3` lands in `libhello.so` too. Build a
plugin on its own to get the xfconf-free image:
`cargo build --release -p xfce-panel-hello` (what the flake's `nix build .#hello`
does per plugin).

## Reference docs

The `docs/` files are the working notes behind this crate; every
claim is referenced `file:line` in the xfce4-panel sources.

| Document | Contents |
| --- | --- |
| [docs/quick-start.md](docs/quick-start.md) | Guided version of the quick start above, plus build/test/ABI. |
| [docs/xfce-panel-plugins.md](docs/xfce-panel-plugins.md) | The plugin contract, the exact `.desktop` keys, the full config → widget → destruction flow, the `wrapper-2.0` (external / Wayland) mode. |
| [docs/audit-ffi-rust.md](docs/audit-ffi-rust.md) | FFI audit: the 3 entry points, the vtable member by member, the Provider interface, the signals, C-type → Rust complexity, the bindgen verdict. |
| [docs/plugin-demo.md](docs/plugin-demo.md) | The `demo` plugin: xfconf settings, settings-driven rendering, the preferences dialog, residency, lifecycle, the e2e test. |

## Development

```sh
just build           # cargo build --release
just test            # unit tests
just test-abi        # ABI harness: dlopen the cdylib + GType type chain
just lint            # clippy -D warnings + cargo fmt --check
just debug           # panel restart with PANEL_DEBUG=module-factory,main
tools/panel-repro.sh label 1   # throwaway panel: Xvfb + private D-Bus + isolated XDG dirs
```

A Nix flake is provided for a pinned environment: `nix develop` (cargo,
clang, the xfce SDK, gdb), `nix build .#demo`, `nix run .#lint`,
`nix flake check`.

## Reload safety

`X-XFCE-Internal=TRUE` (what `xfce_panel::build` generates) means the plugin runs
**inside** the panel process: the `.so` is opened with `G_MODULE_BIND_LOCAL` and
made resident, so the panel keeps it mapped for its whole life — and a plugin
crash takes the panel down. Two rules follow:

- **Never overwrite a loaded `.so` in place.** `cp` writes through the same
  inode, so it rewrites the code pages the panel is executing. Install to
  `file.new` and swap with `rename(2)` — the old inode stays mapped, the next
  load picks up the new file. That is what `just install` does.
- **Restart with `xfce4-panel -q`, then relaunch.** `xfce4-panel -r` is a restart
  race: the relaunched instance finds `org.xfce.Panel` still owned and quits,
  leaving no panel at all. That is what `just refresh` does.

## Settings (xfconf)

Everything in this section is behind the `settings` feature (see
[Features](#features)).

`xfce_panel::Channel` wraps the plugin's xfconf channel
(`xfce4-panel`, base `/plugins/plugin-<id>/`). A settings table is declared with
the `#[xfce_panel::panel_settings]` attribute: the struct you declare **becomes**
the `glib::Object` (wrapper + `imp`), every `#[xfconf]` field becomes a
`ParamSpec` property with its accessors, and `bind_to` wires everything to the
channel:

```rust
#[xfce_panel::panel_settings]
pub struct Settings {
    /// Loud mode.
    #[xfconf(path = "/shout", default = false)]
    shout: bool,
    /// Font size in points.
    #[xfconf(path = "/font-size", default = 14)]
    font_size: i32,
    /// Greeting text.
    #[xfconf(path = "/greeting", default = "Hello from Rust!")]
    greeting: Option<String>,
}

let channel = Channel::new(plugin);          // None without a D-Bus session
settings.bind_to(&channel);                  // + reads the saved values back
```

The attribute consumes the struct: it does **not** emit the original struct,
which is why it is an attribute and not a `derive` (a derive must re-emit its
input, which would produce two types with the same name). The default is
**mandatory**: xfconf returns `NULL` while the key does not exist. Accepted types
are the ones `xfconf_g_property_bind` knows: `bool`, `i32`, `u32`, `i64`, `f64`,
`Option<String>`.

- The path is **relative to the base, `/` included**: GLib concatenates base +
  property with no separator (`xfconf-channel.c:44`). `"/shout"` becomes
  `/plugins/plugin-<id>/shout` (`common/panel-private.h:83`: the base is keyed on
  the plugin's unique id, not on its name).
- The binding is **non-destructive**: it reads the value when the key exists and
  otherwise keeps the object's default; it only creates the xfconf key on the
  first change. `bind_to` writes the `ParamSpec` default when the key is missing
  (`panel_properties_bind`'s `save = TRUE`) so that it shows up in
  `xfconf-query`. `Channel::ensure(path, value)` does the same by hand.
- **Deleting the xfconf key reloads the `ParamSpec` default**
  (`xfconf-binding.c:376`): the default the property declares, not `NULL`. An
  `Option<String>` declared with `default = None` therefore comes back as `None`,
  a property declared with a string comes back as that string — hence the
  mandatory default.
- **For the settings dialog, `channel.bind` widgets straight onto the channel** —
  not `g_object_bind_property` onto the settings object:

  ```rust
  let entry = gtk::Entry::new();
  channel.bind("/greeting", glib::Type::STRING, &entry, "text");
  ```

  The binding loads the saved value into the widget and sends widget changes to
  xfconf (hence the settings object, the following panel, `xfconf-query`). The
  binding dies with the widget (`xfconf-binding.c:456`): nothing to store,
  nothing to destroy. `g_object_bind_property` with `BIDIRECTIONAL` does the
  opposite: it copies the target's *initial* value into the source, so brand new
  widgets (`FALSE` / `0` / `""`) overwrite the settings at `bind` time.
- The bound object is a `glib::Object` (generated by `#[xfce_panel::panel_settings]`);
  repainting happens in `connect_notify_local` (glib's `connect_notify` closures
  require `Send`, GTK widgets are not `Send`).
- `save()` is a *no-op*: xfconf bindings save at the source, there is nothing to
  write in the callback.
- The `Channel` lives in the plugin state: its `Drop` runs `unbind_all` +
  `xfconf::shutdown` at `free_data`.

The demo exposes three persisted settings (`shout`, `font-size`, `greeting`),
edited by the "Properties" dialog (two-way bindings on Switch / SpinButton /
Entry) and readable with
`xfconf-query -c xfce4-panel -l -v -p /plugins/plugin-<id>` (the plugin id is in
`/panels/panel-N/plugin-ids`).

## Key ABI points

- The `.desktop` (`X-XFCE-Module`, `X-XFCE-API=2.0`) is generated by
  `xfce_panel::build` from the plugin's `build.rs` — never written by hand.
- `xfce_panel_module_init(module, &make_resident)` must return a GType
  **descending from `XfcePanelPlugin`**: the panel performs **no type check**
  , a wrong parent is a silent segfault. `just test-abi` is the
  guard against that case.
- `GTypeModule` is abstract: only the panel provides a concrete module
  (`XfcePanelModule`); never `g_object_new` on `GTypeModule` directly.
