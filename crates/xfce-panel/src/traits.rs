use xfce_panel_sys::gobject::GValue;
use xfce_panel_sys::panel::{XfcePanelPluginMode, XfceScreenPosition};

use crate::Plugin;

/// Panel display mode (work area, full width, …); alias of the C enum.
pub type PluginMode = XfcePanelPluginMode;
/// Position of the plugin along the panel; alias of the C enum.
pub type ScreenPosition = XfceScreenPosition;

/// The plugin contract: one Rust type, one exported symbol.
///
/// Every callback is invoked by the panel through the `XfcePanelPluginClass`
/// vtable. All of them are panic-guarded, so a `panic!` in plugin code cannot
/// unwind into the panel process.
///
/// Lifecycle: the panel creates the instance with the `name`, `unique-id`,
/// `display-name`, `comment` and `arguments` properties, realizes it, then calls
/// [`PanelPlugin::construct`] exactly once. Provider calls (`set_size`,
/// `set_mode`, …) can arrive before or after `construct`, in any order, and
/// `size_changed(-1)` means "size is automatic".
pub trait PanelPlugin: 'static {
    /// `GObject` type name, registered into the panel's `GTypeModule`.
    /// Must start with `Xfce` by convention.
    const TYPE_NAME: &'static str;

    /// Plugin state, allocated in the `GObject` instance at `instance_init`.
    type State: Default + 'static;

    /// Build the UI here: exactly one child widget on the plugin (it is a
    /// `GtkBin`). Also the place to opt into menu entries with
    /// [`Plugin::menu_show_configure`] and [`Plugin::menu_show_about`].
    fn construct(_state: &mut Self::State, _plugin: &Plugin) {}

    /// `size-changed`: return `true` if handled, `false` to let the panel force
    /// a square size request.
    fn size_changed(_state: &mut Self::State, _plugin: &Plugin, _size: i32) -> bool {
        false
    }

    /// `orientation-changed`: the panel switched between horizontal and vertical.
    fn orientation_changed(
        _state: &mut Self::State,
        _plugin: &Plugin,
        _orientation: gtk::Orientation,
    ) {
    }

    /// `screen-position-changed`: the plugin moved along the panel.
    fn screen_position_changed(
        _state: &mut Self::State,
        _plugin: &Plugin,
        _position: ScreenPosition,
    ) {
    }

    /// `mode-changed` (API 4.10+).
    fn mode_changed(_state: &mut Self::State, _plugin: &Plugin, _mode: PluginMode) {}

    /// `nrows-changed` (API 4.10+).
    fn nrows_changed(_state: &mut Self::State, _plugin: &Plugin, _nrows: u32) {}

    /// `hidden-event` (API 4.20.1+).
    fn hidden_event(_state: &mut Self::State, _plugin: &Plugin, _hidden: bool) {}

    /// Persist configuration. With xfconf bindings this is a no-op: the
    /// bindings save on change.
    fn save(_state: &mut Self::State, _plugin: &Plugin) {}

    /// "About" dialog. Opt in with [`Plugin::menu_show_about`] in `construct`.
    fn about(_state: &mut Self::State, _plugin: &Plugin) {}

    /// "Properties" dialog. Opt in with [`Plugin::menu_show_configure`] in
    /// `construct` — the panel does not derive the menu entry from this slot.
    fn configure_plugin(_state: &mut Self::State, _plugin: &Plugin) {}

    /// The panel is about to remove the plugin.
    fn removed(_state: &mut Self::State, _plugin: &Plugin) {}

    /// Final cleanup, on `GObject` dispose.
    fn free_data(_state: &mut Self::State, _plugin: &Plugin) {}

    /// Remote events (D-Bus in external mode).
    ///
    /// `value` is an opaque `GValue` whose content depends on the event name;
    /// returning `false` is the documented acceptable answer (docs/audit-ffi-rust.md §J).
    fn remote_event(
        _state: &mut Self::State,
        _plugin: &Plugin,
        _name: &str,
        _value: *const GValue,
    ) -> bool {
        false
    }
}
