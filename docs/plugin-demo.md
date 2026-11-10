# The demo plugin

The `demo` plugin (crates/xfce-panel-demo) is the usage example of the
`xfce-panel` framework: a label whose text, font size and "shout" mode are
settings persisted in xfconf. It is the reference implementation and the guinea
pig for the ABI test.

## What it illustrates

- the [`panel_settings`](../crates/xfce-panel-demo/src/lib.rs) macro: declaring
  the settings, the wrapper, the `ParamSpec` properties, the accessors and the
  xfconf bindings in a single table;
- settings-driven rendering (`render`);
- a preferences dialog bound straight to xfconf;
- residency (`define_panel_plugin!(Demo)`);
- the minimal lifecycle (`construct`, `size_changed`, `configure_plugin`,
  `about`, `save`, `free_data`).

## The settings

The plugin depends on the framework with the `settings` feature enabled
(`xfce-panel = { workspace = true, features = ["settings"] }`): it is what pulls
the `xfconf` crate, the `Channel` type and the `panel_settings` macro. The
`hello` plugin does not enable it and links no xfconf.

`#[xfce_panel::panel_settings]` declares the settings table, each with its xfconf path and
default value:

| Setting | Type | xfconf path | Default |
|---|---|---|---|
| `shout` | `bool` | `/shout` | `false` |
| `font_size` | `i32` | `/font-size` | `14` |
| `greeting` | `Option<String>` | `/greeting` | `Hello from Rust!` |

The macro generates the wrapper, the `ParamSpec`s, the accessors and the
bindings. `PanelPlugin::save` stays a no-op: settings save on every change and
survive a panel restart.

## The rendering

`render(&settings, &label)` composes the label's markup: text in uppercase when
`shout`, size = `font_size` (floor of 1), text escaped via
`glib::markup_escape_text`. A `connect_notify_local` redraws the label on every
setting change.

## The configuration

`configure_plugin` opens a preferences dialog (a `Switch`, a `SpinButton`, an
`Entry`) bound **directly** to xfconf via `channel.bind(...)`: the binding loads
the stored value at bind time, saves every edit, and disconnects itself when the
dialog is destroyed (`xfconf-binding.c:456`).

## Residency and lifecycle

`define_panel_plugin!(Demo)` declares the plugin resident. Asking for it
is **mandatory** as soon as the plugin registers `glib` types: the `GType` and
its interned strings live in the `.so`, and `dlclose`-ing the library that owns
a registered type breaks the panel. `size_changed` applies a
`set_size_request`.

## The entry point

`define_panel_plugin!` exports the single C entry point the panel `dlsym`s,
`xfce_panel_module_init`: it fills `*make_resident` (guarded: the wrapper passes
`NULL`) and returns the plugin's `GType`. The `.desktop` file is
generated at build time by the crate's `build.rs` (via
`xfce_panel::build::write_desktop`) with `X-XFCE-API=2.0` and
`X-XFCE-Internal=TRUE`.

## Build and test

- `just install` copies the `.so` and the generated `.desktop` into
  `~/.local/lib` and `~/.local/share` (atomic `file.new` + `rename(2)` swap);
- `just refresh` quits and relaunches `xfce4-panel` to load it;
- the e2e test
  [crates/xfce-panel-demo/tests/settings_e2e.rs](../crates/xfce-panel-demo/tests/settings_e2e.rs)
  does a full settings round-trip through a private xfconf channel (real
  xfconfd), with a guard when xfconfd is absent;
- `just test-abi` uses the demo as a guinea pig (`nm -D` on the `.so`).
