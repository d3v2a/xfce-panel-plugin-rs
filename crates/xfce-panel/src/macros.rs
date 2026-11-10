//! Public declaratives of the crate.
//!
//! [`panel_settings`](crate::panel_settings) declares the type exposed to
//! xfconf, [`define_panel_plugin!`] declares the plugin on the C side.

/// Declares the C entry point of an `xfce4-panel` plugin.
///
/// Defines `xfce_panel_module_init`, the only symbol the panel loader looks
/// for: it registers the plugin type with the loader's `GTypeModule`
/// (`panel-module.c:216`) and returns its `GType`. The panel then instantiates
/// that type with `g_initable_new (gtype, NULL, &error, "plugin", module,
/// "display", display, "id", id, NULL)` (`panel-object.c:305`) — hence the
/// construction through `glib::Object::new()` with the `plugin`, `display`,
/// `id` properties.
///
/// Residency: the second argument writes into `make_resident` (the panel passes
/// a non-NULL `gboolean*` for API 1.0 and `NULL` for `wrapper-2.0`, hence the
/// guard). The one-argument form is **resident by default**: `register_type`
/// always registers a `GType`, and that `GType` and the interned strings
/// (`g_intern_static_string`, `panel-module.c:34`) live in the `.so`, so
/// `dlclose`-ing the one that owns a registered type breaks the panel. Passing
/// `false` is almost never what you want: it is only for a plugin that registers
/// no `GType` at all.
///
/// ```ignore
/// xfce_panel::define_panel_plugin!(MyPlugin);        // resident (default)
/// xfce_panel::define_panel_plugin!(MyPlugin, false); // not resident: only if you register no GType
/// ```
#[macro_export]
macro_rules! define_panel_plugin {
    ($ty:ty) => {
        $crate::define_panel_plugin!($ty, true);
    };
    ($ty:ty, $resident:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn xfce_panel_module_init(
            module: *mut $crate::sys::gobject::GTypeModule,
            make_resident: *mut $crate::sys::glib::gboolean,
        ) -> $crate::sys::glib::GType {
            if !make_resident.is_null() {
                unsafe { *make_resident = $resident as $crate::sys::glib::gboolean };
            }
            unsafe { $crate::register_type::<$ty>(module) }
        }
    };
}
