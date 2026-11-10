//! Raw FFI bindings to the `libxfce4panel` 2.0 plugin API.
//!
//! Layouts are hand-written against the public headers of xfce4-panel 4.20
//! (`libxfce4panel/xfce-panel-plugin.h`, `xfce-panel-plugin-provider.h`,
//! `libxfce4panel-enums.h`), on top of the `glib-sys` / `gobject-sys` / `gtk-sys`
//! bindings that provide the parent GObject and GTK types.
//!
//! Nothing here is safe: use the `xfce-panel` crate, which wraps this surface.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub use gdk_sys as gdk;
pub use glib_sys as glib;
pub use gobject_sys as gobject;
pub use gtk_sys as gtk;

pub mod panel;
pub mod provider;

pub use glib::{GType, gboolean, gint, gpointer, guint};
pub use gobject::GTypeFlags;
