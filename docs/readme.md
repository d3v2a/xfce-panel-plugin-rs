# Documentation — xfce4-panel plugins in Rust

This repository provides the `xfce-panel` framework (and its `xfce-panel-macros`
crate) for writing xfce4-panel plugins in Rust. This documentation covers the
panel's plugin contract, an FFI audit of its API, the orchestration pattern of a
resident plugin, and the `demo` plugin as a complete worked example. Every claim
is referenced file:line.

| Document | Contents |
|---|---|
| [quick-start.md](./quick-start.md) | A minimal "hello world" plugin: the code, building, testing, installing, and running. |
| [xfce-panel-plugins.md](./xfce-panel-plugins.md) | Building an xfce4-panel plugin (`.desktop` + `G_MODULE_EXPORT`), the exact contract (`xfce_panel_module_init` / legacy `_construct` / `XfcePanelPlugin` vtable), the full config → widget → destruction flow, the role of the wrapper-2.0 (external/Wayland) |
| [audit-ffi-rust.md](./audit-ffi-rust.md) | Rust FFI audit of the plugin API: the 3 exported entry points (exact signatures), the `XfcePanelPluginClass` vtable member by member, the Provider interface, the signals, the post-instantiation execution contract, a C-type → Rust-complexity table, the bindgen verdict and ranked complications (no blocker) |
| [plugin-demo.md](./plugin-demo.md) | The `demo` plugin, a usage example of the framework: xfconf settings via `#[xfce_panel::panel_settings]`, settings-driven rendering, a preferences dialog bound to xfconf, residency, the lifecycle, the e2e test |

Sources analysed: `xfce4-panel` (C) for the plugin contract, and this repository's code.