//! `XfcePanelPlugin`, its class vtable and the plugin entry points.

use std::os::raw::{c_char, c_int};

pub use crate::glib::{GType, gboolean, gint, guint};
use crate::gobject::{GTypeModule, GValue};

pub const XFCE_PANEL_PLUGIN_ICON_PADDING: c_int = 4;

/// `XfcePanelPluginMode` — `libxfce4panel-enums.h`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfcePanelPluginMode {
    Horizontal = 0,
    Vertical = 1,
    Deskbar = 2,
}

/// `XfceScreenPosition` — `libxfce4panel-enums.h`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum XfceScreenPosition {
    None = 0,
    NW_H = 1,
    N = 2,
    NE_H = 3,
    NW_V = 4,
    W = 5,
    SW_V = 6,
    NE_V = 7,
    E = 8,
    SE_V = 9,
    SW_H = 10,
    S = 11,
    SE_H = 12,
    FloatingH = 13,
    FloatingV = 14,
}

impl XfceScreenPosition {
    /// `xfce_screen_position_is_horizontal()`.
    #[inline]
    pub fn is_horizontal(self) -> bool {
        matches!(
            self,
            Self::NW_H | Self::N | Self::NE_H | Self::SW_H | Self::S | Self::SE_H | Self::FloatingH
        )
    }
}

/// Opaque private data of `XfcePanelPlugin`; never dereferenced by plugins.
#[repr(C)]
pub struct XfcePanelPluginPrivate {
    _private: [u8; 0],
}

/// `struct _XfcePanelPlugin` — a `GtkEventBox` subclass, the plugin widget itself.
#[repr(C)]
pub struct XfcePanelPlugin {
    pub parent: gtk_sys::GtkEventBox,
    pub priv_: *mut XfcePanelPluginPrivate,
}

/// `struct _XfcePanelPluginClass` — the vtable filled by `class_init`.
#[repr(C)]
pub struct XfcePanelPluginClass {
    pub parent_class: gtk_sys::GtkEventBoxClass,
    pub construct: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin)>,
    pub screen_position_changed:
        Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin, position: XfceScreenPosition)>,
    pub size_changed:
        Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin, size: gint) -> gboolean>,
    pub orientation_changed:
        Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin, orientation: c_int)>,
    pub free_data: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin)>,
    pub save: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin)>,
    pub about: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin)>,
    pub configure_plugin: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin)>,
    pub removed: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin)>,
    pub remote_event: Option<
        unsafe extern "C" fn(
            plugin: *mut XfcePanelPlugin,
            name: *const c_char,
            value: *const GValue,
        ) -> gboolean,
    >,
    pub mode_changed:
        Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin, mode: XfcePanelPluginMode)>,
    pub nrows_changed: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin, rows: guint)>,
    pub hidden_event: Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin, hidden: gboolean)>,
    pub reserved1: Option<unsafe extern "C" fn()>,
}

/// Legacy `XfcePanelPluginFunc` callback (API 1.x registration macros).
pub type XfcePanelPluginFunc = Option<unsafe extern "C" fn(plugin: *mut XfcePanelPlugin)>;

/// `XfcePanelPluginPreInit` — wrapper-only, runs before `gtk_init()`.
pub type XfcePanelPluginPreInit =
    Option<unsafe extern "C" fn(argc: gint, argv: *mut *mut c_char) -> gboolean>;

/// Legacy `PluginConstructFunc` — `xfce_panel_module_construct`.
pub type PluginConstructFunc = Option<
    unsafe extern "C" fn(
        name: *const c_char,
        unique_id: gint,
        display_name: *const c_char,
        comment: *const c_char,
        arguments: *mut *mut c_char,
        screen: *mut gdk_sys::GdkScreen,
    ) -> *mut gtk_sys::GtkWidget,
>;

/// `PluginInitFunc` — `xfce_panel_module_init`, the modern GTypeModule ABI.
pub type PluginInitFunc =
    Option<unsafe extern "C" fn(module: *mut GTypeModule, make_resident: *mut gboolean) -> GType>;

unsafe extern "C" {
    pub fn xfce_panel_plugin_get_type() -> GType;

    /// `XFCE_PANEL_CHANNEL_NAME` — the xfconf channel plugins write to
    /// (`xfce-panel-convenience.h:40`).
    pub fn xfce_panel_get_channel_name() -> *const c_char;

    pub fn xfce_panel_plugin_get_name(plugin: *mut XfcePanelPlugin) -> *const c_char;
    pub fn xfce_panel_plugin_get_display_name(plugin: *mut XfcePanelPlugin) -> *const c_char;
    pub fn xfce_panel_plugin_get_comment(plugin: *mut XfcePanelPlugin) -> *const c_char;
    pub fn xfce_panel_plugin_get_unique_id(plugin: *mut XfcePanelPlugin) -> gint;
    pub fn xfce_panel_plugin_get_property_base(plugin: *mut XfcePanelPlugin) -> *const c_char;
    pub fn xfce_panel_plugin_get_arguments(plugin: *mut XfcePanelPlugin) -> *const *const c_char;

    pub fn xfce_panel_plugin_get_size(plugin: *mut XfcePanelPlugin) -> gint;
    pub fn xfce_panel_plugin_get_expand(plugin: *mut XfcePanelPlugin) -> gboolean;
    pub fn xfce_panel_plugin_set_expand(plugin: *mut XfcePanelPlugin, expand: gboolean);
    pub fn xfce_panel_plugin_get_shrink(plugin: *mut XfcePanelPlugin) -> gboolean;
    pub fn xfce_panel_plugin_set_shrink(plugin: *mut XfcePanelPlugin, shrink: gboolean);
    pub fn xfce_panel_plugin_get_small(plugin: *mut XfcePanelPlugin) -> gboolean;
    pub fn xfce_panel_plugin_set_small(plugin: *mut XfcePanelPlugin, small: gboolean);
    pub fn xfce_panel_plugin_get_icon_size(plugin: *mut XfcePanelPlugin) -> gint;
    pub fn xfce_panel_plugin_get_orientation(plugin: *mut XfcePanelPlugin) -> c_int;
    pub fn xfce_panel_plugin_get_mode(plugin: *mut XfcePanelPlugin) -> XfcePanelPluginMode;
    pub fn xfce_panel_plugin_get_nrows(plugin: *mut XfcePanelPlugin) -> guint;
    pub fn xfce_panel_plugin_get_screen_position(
        plugin: *mut XfcePanelPlugin,
    ) -> XfceScreenPosition;

    pub fn xfce_panel_plugin_take_window(
        plugin: *mut XfcePanelPlugin,
        window: *mut gtk_sys::GtkWindow,
    );
    pub fn xfce_panel_plugin_add_action_widget(
        plugin: *mut XfcePanelPlugin,
        widget: *mut gtk_sys::GtkWidget,
    );
    pub fn xfce_panel_plugin_menu_insert_item(
        plugin: *mut XfcePanelPlugin,
        item: *mut gtk_sys::GtkMenuItem,
    );
    pub fn xfce_panel_plugin_menu_show_configure(plugin: *mut XfcePanelPlugin);
    pub fn xfce_panel_plugin_menu_show_about(plugin: *mut XfcePanelPlugin);
    pub fn xfce_panel_plugin_menu_destroy(plugin: *mut XfcePanelPlugin);

    pub fn xfce_panel_plugin_get_locked(plugin: *mut XfcePanelPlugin) -> gboolean;
    pub fn xfce_panel_plugin_remove(plugin: *mut XfcePanelPlugin);
    pub fn xfce_panel_plugin_register_menu(
        plugin: *mut XfcePanelPlugin,
        menu: *mut gtk_sys::GtkMenu,
    );
    pub fn xfce_panel_plugin_arrow_type(plugin: *mut XfcePanelPlugin) -> c_int;
    pub fn xfce_panel_plugin_position_widget(
        plugin: *mut XfcePanelPlugin,
        menu_widget: *mut gtk_sys::GtkWidget,
        attach_widget: *mut gtk_sys::GtkWidget,
        x: *mut gint,
        y: *mut gint,
    );
    pub fn xfce_panel_plugin_popup_window(
        plugin: *mut XfcePanelPlugin,
        window: *mut gtk_sys::GtkWindow,
        widget: *mut gtk_sys::GtkWidget,
    );
    pub fn xfce_panel_plugin_focus_widget(
        plugin: *mut XfcePanelPlugin,
        widget: *mut gtk_sys::GtkWidget,
    );
    pub fn xfce_panel_plugin_block_autohide(plugin: *mut XfcePanelPlugin, blocked: gboolean);

    /// Transfer full: `g_free()` the returned pointer.
    pub fn xfce_panel_plugin_lookup_rc_file(plugin: *mut XfcePanelPlugin) -> *mut c_char;
    /// Transfer full: `g_free()` the returned pointer.
    pub fn xfce_panel_plugin_save_location(
        plugin: *mut XfcePanelPlugin,
        create: gboolean,
    ) -> *mut c_char;
}

/// Convenience: `XFCE_TYPE_PANEL_PLUGIN`.
#[inline]
pub fn xfce_type_panel_plugin() -> GType {
    unsafe { xfce_panel_plugin_get_type() }
}

/// Convenience: `xfce_screen_position_is_horizontal()`.
#[inline]
pub fn xfce_screen_position_is_horizontal(position: XfceScreenPosition) -> bool {
    position.is_horizontal()
}

pub use crate::glib::g_free;
