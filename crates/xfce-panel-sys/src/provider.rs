//! The `XfcePanelPluginProvider` interface and the panel<->plugin protocol.
//!
//! Implemented by the base class `XfcePanelPlugin`; a plugin subclass inherits
//! every slot. The panel calls these functions (never the class vtable directly),
//! and in external mode the `wrapper-2.0` process relays them over D-Bus.

use std::os::raw::{c_char, c_uint};

use crate::glib::{GType, gboolean, gint, guint};
use crate::gobject::{GTypeInterface, GValue};

use crate::panel::{XfcePanelPluginMode, XfceScreenPosition};

/// `struct _XfcePanelPluginProviderInterface` — 18 slots, inherited by plugins.
#[repr(C)]
pub struct XfcePanelPluginProviderInterface {
    pub parent: GTypeInterface,
    pub get_name:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider) -> *const c_char>,
    pub get_unique_id: Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider) -> gint>,
    pub hidden_event:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, hidden: gboolean)>,
    pub set_size: Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, size: gint)>,
    pub set_icon_size:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, icon_size: gint)>,
    pub set_dark_mode:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, dark_mode: gboolean)>,
    pub set_mode: Option<
        unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, mode: XfcePanelPluginMode),
    >,
    pub set_nrows:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, rows: guint)>,
    pub set_screen_position: Option<
        unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, position: XfceScreenPosition),
    >,
    pub save: Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider)>,
    pub get_show_configure:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider) -> gboolean>,
    pub show_configure: Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider)>,
    pub get_show_about:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider) -> gboolean>,
    pub show_about: Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider)>,
    pub removed: Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider)>,
    pub remote_event: Option<
        unsafe extern "C" fn(
            provider: *mut XfcePanelPluginProvider,
            name: *const c_char,
            value: *const GValue,
            handle: *mut guint,
        ) -> gboolean,
    >,
    pub set_locked:
        Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider, locked: gboolean)>,
    pub ask_remove: Option<unsafe extern "C" fn(provider: *mut XfcePanelPluginProvider)>,
}

/// Opaque provider instance (the plugin object, or the panel-side wrapper).
#[repr(C)]
pub struct XfcePanelPluginProvider {
    pub parent: crate::gobject::GObject,
}

/// `XfcePanelPluginProviderSignal` — plugin -> panel requests.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfcePanelPluginProviderSignal {
    MovePlugin = 0,
    ExpandPlugin,
    CollapsePlugin,
    SmallPlugin,
    UnsmallPlugin,
    LockPanel,
    UnlockPanel,
    RemovePlugin,
    AddNewItems,
    PanelPreferences,
    PanelLogout,
    PanelAbout,
    PanelHelp,
    ShowConfigure,
    ShowAbout,
    FocusPlugin,
    ShrinkPlugin,
    UnshrinkPlugin,
}

/// `XfcePanelPluginProviderPropType` — panel -> plugin properties and actions.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XfcePanelPluginProviderPropType {
    SetSize = 0,
    SetIconSize,
    SetDarkMode,
    SetMode,
    SetScreenPosition,
    SetBackgroundAlpha,
    SetNrows,
    SetLocked,
    SetSensitive,
    ActionRemoved,
    ActionSave,
    ActionQuit,
    ActionQuitForRestart,
    ActionBackgroundUnset,
    ActionShowConfigure,
    ActionShowAbout,
    ActionAskRemove,
    EventHidden,
    SetOpacity,
    SetBackgroundColor,
    SetBackgroundImage,
    SetMonitor,
    SetGeometry,
}

/// Anonymous `PLUGIN_EXIT_*` enum — `provider.h:133-143` (bindgen blind spot).
pub mod plugin_exit {
    pub const SUCCESS: i32 = 0;
    pub const FAILURE: i32 = 1;
    pub const ARGUMENTS_FAILED: i32 = 2;
    pub const PREINIT_FAILED: i32 = 3;
    pub const CHECK_FAILED: i32 = 4;
    pub const NO_PROVIDER: i32 = 5;
    pub const NAME_LOST: i32 = 6;
    pub const SUCCESS_AND_RESTART: i32 = 7;
}

/// Anonymous `PLUGIN_ARGV_*` enum — `provider.h:146-156` (bindgen blind spot).
pub mod plugin_argv {
    pub const ARGV_0: usize = 0;
    pub const FILENAME: usize = 1;
    pub const UNIQUE_ID: usize = 2;
    pub const SOCKET_ID: usize = 3;
    pub const NAME: usize = 4;
    pub const DISPLAY_NAME: usize = 5;
    pub const COMMENT: usize = 6;
    pub const ARGUMENTS: usize = 7;
}

unsafe extern "C" {
    pub fn xfce_panel_plugin_provider_get_type() -> GType;

    pub fn xfce_panel_plugin_provider_get_name(
        provider: *mut XfcePanelPluginProvider,
    ) -> *const c_char;
    pub fn xfce_panel_plugin_provider_get_unique_id(provider: *mut XfcePanelPluginProvider)
    -> gint;
    pub fn xfce_panel_plugin_provider_emit_hidden_event(
        provider: *mut XfcePanelPluginProvider,
        hidden: gboolean,
    );
    pub fn xfce_panel_plugin_provider_set_size(provider: *mut XfcePanelPluginProvider, size: gint);
    pub fn xfce_panel_plugin_provider_set_icon_size(
        provider: *mut XfcePanelPluginProvider,
        icon_size: gint,
    );
    pub fn xfce_panel_plugin_provider_set_dark_mode(
        provider: *mut XfcePanelPluginProvider,
        dark_mode: gboolean,
    );
    pub fn xfce_panel_plugin_provider_set_mode(
        provider: *mut XfcePanelPluginProvider,
        mode: XfcePanelPluginMode,
    );
    pub fn xfce_panel_plugin_provider_set_nrows(
        provider: *mut XfcePanelPluginProvider,
        rows: guint,
    );
    pub fn xfce_panel_plugin_provider_set_screen_position(
        provider: *mut XfcePanelPluginProvider,
        position: XfceScreenPosition,
    );
    pub fn xfce_panel_plugin_provider_save(provider: *mut XfcePanelPluginProvider);
    pub fn xfce_panel_plugin_provider_emit_signal(
        provider: *mut XfcePanelPluginProvider,
        provider_signal: XfcePanelPluginProviderSignal,
    );
    pub fn xfce_panel_plugin_provider_get_show_configure(
        provider: *mut XfcePanelPluginProvider,
    ) -> gboolean;
    pub fn xfce_panel_plugin_provider_show_configure(provider: *mut XfcePanelPluginProvider);
    pub fn xfce_panel_plugin_provider_get_show_about(
        provider: *mut XfcePanelPluginProvider,
    ) -> gboolean;
    pub fn xfce_panel_plugin_provider_show_about(provider: *mut XfcePanelPluginProvider);
    pub fn xfce_panel_plugin_provider_removed(provider: *mut XfcePanelPluginProvider);
    pub fn xfce_panel_plugin_provider_remote_event(
        provider: *mut XfcePanelPluginProvider,
        name: *const c_char,
        value: *const GValue,
        handle: *mut c_uint,
    ) -> gboolean;
    pub fn xfce_panel_plugin_provider_set_locked(
        provider: *mut XfcePanelPluginProvider,
        locked: gboolean,
    );
    pub fn xfce_panel_plugin_provider_ask_remove(provider: *mut XfcePanelPluginProvider);
}
