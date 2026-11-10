use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

use glib::translate::from_glib;

use xfce_panel_sys::gobject::{GTypeInfo, GTypeInstance, GTypeModule, GValue};
use xfce_panel_sys::gtk as sysgtk;
use xfce_panel_sys::panel as syspanel;

use crate::plugin::Plugin;
use crate::state::{clear_state, init_state, state_mut};
use crate::traits::PanelPlugin;

// GLib does not copy the GTypeInfo: the table must live at least until
// g_type_module_register_type. GLib consumes the structure synchronously
// during registration, so a shared static is valid.
#[repr(transparent)]
pub(crate) struct SyncTypeInfo(pub GTypeInfo);
unsafe impl Sync for SyncTypeInfo {}

static mut INFO: SyncTypeInfo = SyncTypeInfo(GTypeInfo {
    class_size: 0,
    base_init: None,
    base_finalize: None,
    class_init: None,
    class_finalize: None,
    class_data: std::ptr::null(),
    instance_size: 0,
    n_preallocs: 0,
    instance_init: None,
    value_table: std::ptr::null(),
});

/// Registers `T` inside the panel's `GTypeModule`.
///
/// Registers `T` inside the panel's `GTypeModule`.
///
/// Marks gtk-rs as initialized: the panel called `gtk_init_check` on the
/// main thread before `dlopen`, so the host process already owns a live
/// GTK — the plugin must not call `gtk::init` itself, only claim the
/// state (gtk-rs keeps its own per-binary `INITIALIZED` flag).
///
/// # Safety
///
/// `module` must be the live `GTypeModule` handed to `xfce_panel_module_init`,
/// `T::TYPE_NAME` a valid `GObject` type name, and the caller must be the
/// panel's main thread (where GTK was initialized).
///
/// # Panics
///
/// Panics if `T::TYPE_NAME` contains a nul byte (not a valid C string).
pub unsafe fn register_type<T: PanelPlugin>(module: *mut GTypeModule) -> syspanel::GType {
    let name = std::ffi::CString::new(T::TYPE_NAME).expect("TYPE_NAME must be a valid C string");
    unsafe {
        if !gtk::is_initialized() {
            gtk::set_initialized();
        }
        INFO.0 = GTypeInfo {
            class_size: size_of::<syspanel::XfcePanelPluginClass>() as u16,
            base_init: None,
            base_finalize: None,
            class_init: Some(class_init::<T>),
            class_finalize: None,
            class_data: std::ptr::null(),
            instance_size: size_of::<syspanel::XfcePanelPlugin>() as u16,
            n_preallocs: 0,
            instance_init: Some(instance_init::<T>),
            value_table: std::ptr::null(),
        };
        xfce_panel_sys::gobject::g_type_module_register_type(
            module,
            syspanel::xfce_panel_plugin_get_type(),
            name.as_ptr(),
            (&raw const INFO).cast(),
            0,
        )
    }
}

unsafe extern "C" fn class_init<T: PanelPlugin>(class: *mut c_void, _data: *mut c_void) {
    let class = unsafe { &mut *class.cast::<syspanel::XfcePanelPluginClass>() };
    class.construct = Some(on_construct::<T>);
    class.size_changed = Some(on_size_changed::<T>);
    class.orientation_changed = Some(on_orientation_changed::<T>);
    class.screen_position_changed = Some(on_screen_position_changed::<T>);
    class.mode_changed = Some(on_mode_changed::<T>);
    class.nrows_changed = Some(on_nrows_changed::<T>);
    class.hidden_event = Some(on_hidden::<T>);
    class.save = Some(on_save::<T>);
    class.about = Some(on_about::<T>);
    class.configure_plugin = Some(on_configure_plugin::<T>);
    class.removed = Some(on_removed::<T>);
    class.free_data = Some(on_free_data::<T>);
    class.remote_event = Some(on_remote_event::<T>);
}

unsafe extern "C" fn instance_init<T: PanelPlugin>(
    instance: *mut GTypeInstance,
    _data: *mut c_void,
) {
    let plugin = instance.cast::<syspanel::XfcePanelPlugin>();
    unsafe { init_state::<T::State>(plugin) };
}

fn guard<R: Default>(f: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_default()
}

unsafe extern "C" fn on_construct<T: PanelPlugin>(plugin: *mut syspanel::XfcePanelPlugin) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::construct(state, plugin);
    });
}

unsafe extern "C" fn on_size_changed<T: PanelPlugin>(
    plugin: *mut syspanel::XfcePanelPlugin,
    size: c_int,
) -> syspanel::gboolean {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        syspanel::gboolean::from(T::size_changed(state, plugin, size))
    })
}

unsafe extern "C" fn on_orientation_changed<T: PanelPlugin>(
    plugin: *mut syspanel::XfcePanelPlugin,
    orientation: sysgtk::GtkOrientation,
) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::orientation_changed(state, plugin, unsafe { from_glib(orientation) });
    });
}

unsafe extern "C" fn on_screen_position_changed<T: PanelPlugin>(
    plugin: *mut syspanel::XfcePanelPlugin,
    pos: syspanel::XfceScreenPosition,
) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::screen_position_changed(state, plugin, pos);
    });
}

unsafe extern "C" fn on_mode_changed<T: PanelPlugin>(
    plugin: *mut syspanel::XfcePanelPlugin,
    mode: syspanel::XfcePanelPluginMode,
) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::mode_changed(state, plugin, mode);
    });
}

unsafe extern "C" fn on_nrows_changed<T: PanelPlugin>(
    plugin: *mut syspanel::XfcePanelPlugin,
    nrows: c_uint,
) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::nrows_changed(state, plugin, nrows);
    });
}

unsafe extern "C" fn on_hidden<T: PanelPlugin>(
    plugin: *mut syspanel::XfcePanelPlugin,
    hidden: syspanel::gboolean,
) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::hidden_event(state, plugin, hidden != 0);
    });
}

unsafe extern "C" fn on_save<T: PanelPlugin>(plugin: *mut syspanel::XfcePanelPlugin) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::save(state, plugin);
    });
}

unsafe extern "C" fn on_about<T: PanelPlugin>(plugin: *mut syspanel::XfcePanelPlugin) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::about(state, plugin);
    });
}

unsafe extern "C" fn on_configure_plugin<T: PanelPlugin>(plugin: *mut syspanel::XfcePanelPlugin) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::configure_plugin(state, plugin);
    });
}

unsafe extern "C" fn on_removed<T: PanelPlugin>(plugin: *mut syspanel::XfcePanelPlugin) {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        T::removed(state, plugin);
    });
}

unsafe extern "C" fn on_free_data<T: PanelPlugin>(plugin: *mut syspanel::XfcePanelPlugin) {
    guard(|| unsafe {
        let plugin_ref = &*(plugin as *const Plugin);
        T::free_data(state_mut::<T::State>(plugin), plugin_ref);
        clear_state(plugin);
    });
}

unsafe extern "C" fn on_remote_event<T: PanelPlugin>(
    plugin: *mut syspanel::XfcePanelPlugin,
    name: *const c_char,
    value: *const GValue,
) -> syspanel::gboolean {
    guard(|| {
        let plugin = unsafe { &*(plugin as *const Plugin) };
        let state = unsafe { state_mut::<T::State>(plugin.as_ptr()) };
        let name = unsafe { std::ffi::CStr::from_ptr(name) };
        syspanel::gboolean::from(T::remote_event(
            state,
            plugin,
            &name.to_string_lossy(),
            value,
        ))
    })
}
