//! Safe Rust API for xfce4-panel plugins.
//!
//! A plugin is a Rust type implementing [`PanelPlugin`], exported with
//! [`define_panel_plugin!`], which emits the `xfce_panel_module_init` symbol the
//! panel (and `wrapper-2.0`) probes. The plugin state lives in the `GObject`
//! instance (`GData`), the vtable slots are trampolines that catch panics.
//!
//! ```ignore
//! use xfce_panel::prelude::*;
//!
//! #[derive(Default)]
//! struct MyPlugin;
//!
//! impl PanelPlugin for MyPlugin {
//!     const TYPE_NAME: &'static str = "XfceMyPlugin";
//!     fn construct(_state: &mut Self::State, plugin: &Plugin) {
//!         plugin.add_widget(&gtk::Label::new(Some("hello")));
//!     }
//! }
//!
//! xfce_panel::define_panel_plugin!(MyPlugin);
//! ```

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(
    noop_method_call,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    missing_docs,
    missing_debug_implementations,
    clippy::pedantic
)]
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]

#[cfg(feature = "build")]
pub mod build;

mod macros;
mod plugin;
mod registration;
#[cfg(feature = "settings")]
mod settings;
mod state;
mod traits;

#[doc(hidden)]
pub use xfce_panel_sys as sys;

/// Re-export of `glib` and `gtk`: a plugin only needs to depend on
/// `xfce-panel` and use `xfce_panel::glib` / `xfce_panel::gtk`. It no longer
/// pins its own `glib`/`gtk` versions (a near-mandatory dependency to keep in
/// lockstep with the binding crate) — the version is the one `xfce-panel`
/// uses, guaranteed compatible.
pub use glib;
pub use gtk;

#[doc(hidden)]
pub use crate::registration::register_type;
#[cfg(feature = "settings")]
pub use crate::settings::Channel;
#[cfg(feature = "settings")]
pub use crate::settings::XfconfType;
pub use crate::state::PluginState;
pub use crate::traits::{PanelPlugin, PluginMode, ScreenPosition};

pub use crate::plugin::Plugin;
#[cfg(feature = "settings")]
pub use xfce_panel_macros::panel_settings;

/// The prelude: the traits, types and GTK/GLib re-exports a plugin needs.
pub mod prelude {
    pub use super::PanelPlugin;
    pub use super::Plugin;
    pub use super::glib;
    pub use super::gtk;
    pub use super::{PluginMode, ScreenPosition};
}
