use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

use glib::translate::{from_glib, from_glib_none};
use gtk::prelude::*;

use xfce_panel_sys::glib as sysglib;
use xfce_panel_sys::gtk as gtk_sys;
use xfce_panel_sys::panel as syspanel;
use xfce_panel_sys::provider as sysprov;

use crate::traits::{PluginMode, ScreenPosition};

/// Safe handle on the plugin widget.
///
/// `XfcePanelPlugin` *is* a `GtkEventBox` (a `GtkBin`: exactly one child) with
/// `visible-window = FALSE`, so it is transparent and the panel paints the
/// background. This type adds no state of its own — it is a typed view over the
/// C instance.
#[repr(transparent)]
pub struct Plugin(syspanel::XfcePanelPlugin);

impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Plugin").field(&self.as_ptr()).finish()
    }
}

impl Plugin {
    /// Borrow the plugin from the raw pointer handed to a vtable callback.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a live `XfcePanelPlugin` owned by the panel.
    #[inline]
    pub unsafe fn from_ptr<'a>(ptr: *mut syspanel::XfcePanelPlugin) -> &'a Self {
        unsafe { &*(ptr as *const Self) }
    }

    /// Recover the plugin from its own widget — the top-level `GtkEventBox` the
    /// panel created, i.e. one obtained from [`Plugin::widget`] (or a clone /
    /// `connect_*` handler on it). `Plugin` is `#[repr(transparent)]` over the
    /// `XfcePanelPlugin`, which *is* that widget, so this is a zero-cost view.
    ///
    /// Passing a widget that is *not* the plugin's own widget (a child, an
    /// unrelated window) is a programming error and undefined behaviour: the
    /// caller is responsible for handing back the widget that came from the
    /// plugin.
    #[inline]
    #[must_use]
    pub fn from_widget(widget: &gtk::Widget) -> &Self {
        unsafe { &*(widget.as_ptr() as *const Self) }
    }

    /// Raw pointer to the underlying `XfcePanelPlugin` instance.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *mut syspanel::XfcePanelPlugin {
        std::ptr::from_ref::<Self>(self) as *mut _
    }

    /// The plugin as a GTK widget (refcounted handle).
    #[must_use]
    pub fn widget(&self) -> gtk::Widget {
        unsafe { from_glib_none(self.as_ptr().cast::<gtk_sys::GtkWidget>()) }
    }

    /// Borrow this plugin's state — the [`PanelPlugin::State`](crate::PanelPlugin::State)
    /// allocated at `instance_init`.
    ///
    /// Lets a callback installed outside the vtable (a `notify` on a settings
    /// object, a button click) reach the state. `S` must be the state type the
    /// plugin declared: passing another one is undefined behaviour.
    #[allow(
        clippy::mut_from_ref,
        reason = "the plugin is a view over a C GObject: its state lives in the \
                  instance GData, which is interior-mutable and only touched from \
                  the panel main loop"
    )]
    #[must_use]
    pub fn state<S: Default + 'static>(&self) -> &mut S {
        unsafe { crate::state::state_mut::<S>(self.as_ptr()) }
    }

    fn c_string(ptr: *const c_char) -> String {
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned()
        }
    }

    /// `X-XFCE-Module` name from the `.desktop` file.
    #[must_use]
    pub fn name(&self) -> String {
        Self::c_string(unsafe { syspanel::xfce_panel_plugin_get_name(self.as_ptr()) })
    }

    /// `Name` from the `.desktop` file.
    #[must_use]
    pub fn display_name(&self) -> String {
        Self::c_string(unsafe { syspanel::xfce_panel_plugin_get_display_name(self.as_ptr()) })
    }

    /// `Comment` from the `.desktop` file.
    #[must_use]
    pub fn comment(&self) -> String {
        Self::c_string(unsafe { syspanel::xfce_panel_plugin_get_comment(self.as_ptr()) })
    }

    /// Instance id, key of `/plugins/plugin-<id>` in the xfconf channel.
    #[must_use]
    pub fn unique_id(&self) -> i32 {
        unsafe { syspanel::xfce_panel_plugin_get_unique_id(self.as_ptr()) }
    }

    /// `/plugins/plugin-<id>`: base path for xfconf property bindings.
    #[must_use]
    pub fn property_base(&self) -> String {
        Self::c_string(unsafe { syspanel::xfce_panel_plugin_get_property_base(self.as_ptr()) })
    }

    /// Extra arguments from the panelrc (never freed; docs/audit-ffi-rust.md §F).
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut ptr = unsafe { syspanel::xfce_panel_plugin_get_arguments(self.as_ptr()) };
        if ptr.is_null() {
            return out;
        }
        unsafe {
            while !(*ptr).is_null() {
                out.push(Self::c_string(*ptr));
                ptr = ptr.add(1);
            }
        }
        out
    }

    /// Length along the panel, in pixels.
    #[must_use]
    pub fn size(&self) -> i32 {
        unsafe { syspanel::xfce_panel_plugin_get_size(self.as_ptr()) }
    }

    /// Icon size for the panel, in pixels.
    #[must_use]
    pub fn icon_size(&self) -> i32 {
        unsafe { syspanel::xfce_panel_plugin_get_icon_size(self.as_ptr()) }
    }

    /// Number of rows the plugin spans (columns on a vertical panel).
    #[must_use]
    pub fn nrows(&self) -> u32 {
        unsafe { syspanel::xfce_panel_plugin_get_nrows(self.as_ptr()) }
    }

    /// Panel display mode (work area, full width, …).
    #[must_use]
    pub fn mode(&self) -> PluginMode {
        unsafe { syspanel::xfce_panel_plugin_get_mode(self.as_ptr()) }
    }

    /// Position of the plugin along the panel.
    #[must_use]
    pub fn screen_position(&self) -> ScreenPosition {
        unsafe { syspanel::xfce_panel_plugin_get_screen_position(self.as_ptr()) }
    }

    /// Panel orientation (horizontal / vertical).
    #[must_use]
    pub fn orientation(&self) -> gtk::Orientation {
        unsafe { from_glib(syspanel::xfce_panel_plugin_get_orientation(self.as_ptr())) }
    }

    /// Whether the panel is locked (its preferences window is open).
    #[must_use]
    pub fn is_locked(&self) -> bool {
        unsafe { syspanel::xfce_panel_plugin_get_locked(self.as_ptr()) != 0 }
    }

    /// Whether the plugin expands to fill the panel's available space.
    #[must_use]
    pub fn is_expand(&self) -> bool {
        unsafe { syspanel::xfce_panel_plugin_get_expand(self.as_ptr()) != 0 }
    }

    /// Set whether the plugin expands to fill the panel's available space.
    pub fn set_expand(&self, expand: bool) {
        unsafe {
            syspanel::xfce_panel_plugin_set_expand(self.as_ptr(), sysglib::gboolean::from(expand));
        }
    }

    /// Whether the plugin may shrink below its natural size.
    #[must_use]
    pub fn is_shrink(&self) -> bool {
        unsafe { syspanel::xfce_panel_plugin_get_shrink(self.as_ptr()) != 0 }
    }

    /// Set whether the plugin may shrink below its natural size.
    pub fn set_shrink(&self, shrink: bool) {
        unsafe {
            syspanel::xfce_panel_plugin_set_shrink(self.as_ptr(), sysglib::gboolean::from(shrink));
        }
    }

    /// Whether the plugin is in small mode (compact rendering).
    #[must_use]
    pub fn is_small(&self) -> bool {
        unsafe { syspanel::xfce_panel_plugin_get_small(self.as_ptr()) != 0 }
    }

    /// Set whether the plugin is in small mode (compact rendering).
    pub fn set_small(&self, small: bool) {
        unsafe {
            syspanel::xfce_panel_plugin_set_small(self.as_ptr(), sysglib::gboolean::from(small));
        }
    }

    /// Add the single child of the plugin (`GtkBin`). Call `show` yourself.
    pub fn add_widget<W: IsA<gtk::Widget>>(&self, widget: &W) {
        unsafe {
            gtk_sys::gtk_container_add(
                self.as_ptr().cast::<gtk_sys::GtkContainer>(),
                widget.as_ptr().cast::<gtk_sys::GtkWidget>(),
            );
        }
    }

    /// Add a widget to the panel's action area (panel-side, not inside the plugin).
    pub fn add_action_widget<W: IsA<gtk::Widget>>(&self, widget: &W) {
        unsafe {
            syspanel::xfce_panel_plugin_add_action_widget(
                self.as_ptr(),
                widget.as_ptr().cast::<gtk_sys::GtkWidget>(),
            );
        }
    }

    /// Request a minimum size (pixels) on the plugin widget.
    pub fn set_size_request(&self, width: i32, height: i32) {
        unsafe {
            gtk_sys::gtk_widget_set_size_request(
                self.as_ptr().cast::<gtk_sys::GtkWidget>(),
                width,
                height,
            );
        }
    }

    /// Show the "Properties" menu entry. Without this call the panel never shows
    /// it, even when `configure_plugin` is implemented (docs/audit-ffi-rust.md §K.3).
    pub fn menu_show_configure(&self) {
        unsafe { syspanel::xfce_panel_plugin_menu_show_configure(self.as_ptr()) }
    }

    /// Show the "About" menu entry; same opt-in rule as
    /// [`Plugin::menu_show_configure`].
    pub fn menu_show_about(&self) {
        unsafe { syspanel::xfce_panel_plugin_menu_show_about(self.as_ptr()) }
    }

    /// Insert a custom item into the plugin's right-click menu.
    pub fn menu_insert_item<W: IsA<gtk::MenuItem>>(&self, item: &W) {
        unsafe {
            syspanel::xfce_panel_plugin_menu_insert_item(
                self.as_ptr(),
                item.as_ptr().cast::<gtk_sys::GtkMenuItem>(),
            );
        }
    }

    /// Register a menu so the panel can position it relative to the plugin.
    pub fn register_menu(&self, menu: &gtk::Menu) {
        unsafe { syspanel::xfce_panel_plugin_register_menu(self.as_ptr(), menu.as_ptr().cast()) }
    }

    /// Place `window` next to the plugin with the panel's popup hints (utility
    /// window, keep-above, Wayland layer attach, auto-close on focus loss).
    pub fn popup_window<W: IsA<gtk::Window>, G: IsA<gtk::Widget>>(&self, window: &W, widget: &G) {
        unsafe {
            syspanel::xfce_panel_plugin_popup_window(
                self.as_ptr(),
                window.as_ptr().cast::<gtk_sys::GtkWindow>(),
                widget.as_ptr().cast::<gtk_sys::GtkWidget>(),
            );
        }
    }

    /// Give keyboard focus to `widget`.
    pub fn focus_widget<W: IsA<gtk::Widget>>(&self, widget: &W) {
        unsafe {
            syspanel::xfce_panel_plugin_focus_widget(
                self.as_ptr(),
                widget.as_ptr().cast::<gtk_sys::GtkWidget>(),
            );
        }
    }

    /// Keep the panel from auto-hiding while a popup is open.
    pub fn block_autohide(&self, blocked: bool) {
        unsafe {
            syspanel::xfce_panel_plugin_block_autohide(
                self.as_ptr(),
                sysglib::gboolean::from(blocked),
            );
        }
    }

    /// Ask the panel for the plugin's rc-file path (transfer full, freed here).
    #[must_use]
    pub fn lookup_rc_file(&self) -> Option<PathBuf> {
        Self::take_string(unsafe { syspanel::xfce_panel_plugin_lookup_rc_file(self.as_ptr()) })
    }

    /// Ask the panel for the plugin's config path, optionally creating it.
    #[must_use]
    pub fn save_location(&self, create: bool) -> Option<PathBuf> {
        Self::take_string(unsafe {
            syspanel::xfce_panel_plugin_save_location(
                self.as_ptr(),
                sysglib::gboolean::from(create),
            )
        })
    }

    fn take_string(ptr: *mut c_char) -> Option<PathBuf> {
        if ptr.is_null() {
            return None;
        }
        let path = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { sysglib::g_free(ptr.cast()) };
        Some(PathBuf::from(path))
    }

    /// Send a provider signal to the panel (expand, remove, preferences, …).
    pub fn emit(&self, signal: sysprov::XfcePanelPluginProviderSignal) {
        unsafe {
            sysprov::xfce_panel_plugin_provider_emit_signal(
                self.as_ptr().cast::<sysprov::XfcePanelPluginProvider>(),
                signal,
            );
        }
    }

    /// Ask the panel to remove this plugin.
    pub fn remove(&self) {
        unsafe { syspanel::xfce_panel_plugin_remove(self.as_ptr()) }
    }
}
