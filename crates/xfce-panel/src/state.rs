use std::os::raw::c_void;

use xfce_panel_sys::glib::GQuark;
use xfce_panel_sys::glib::g_quark_from_static_string;
use xfce_panel_sys::gobject::{GObject, g_object_get_qdata, g_object_set_qdata_full};
use xfce_panel_sys::panel::XfcePanelPlugin;

/// Marker for the state stored in the plugin instance (`GData`).
///
/// Any `Default + 'static` type qualifies, so `type State = ();` is a valid
/// stateless plugin.
pub trait PluginState: Default + 'static {}
impl<T: Default + 'static> PluginState for T {}

fn state_quark() -> GQuark {
    static KEY: &[u8] = b"xfce-panel-rs:state\0";
    unsafe { g_quark_from_static_string(KEY.as_ptr().cast()) }
}

pub(crate) unsafe extern "C" fn free_state<S: 'static>(p: *mut c_void) {
    drop(unsafe { Box::from_raw(p.cast::<S>()) });
}

/// Hands the state back to `GLib`.
///
/// Setting the qdata to `NULL` runs the destroy notify registered by
/// `init_state` exactly once and clears the pointer. `free_data` must use this
/// instead of freeing the state directly: the notify also runs at object
/// finalization, and freeing twice corrupts the heap.
pub(crate) unsafe fn clear_state(plugin: *mut XfcePanelPlugin) {
    let object = plugin.cast::<GObject>();
    unsafe {
        g_object_set_qdata_full(
            object,
            state_quark(),
            std::ptr::null_mut(),
            None::<unsafe extern "C" fn(p: *mut c_void)>,
        );
    };
}

pub(crate) unsafe fn init_state<S: Default + 'static>(plugin: *mut XfcePanelPlugin) {
    let object = plugin.cast::<GObject>();
    let boxed = Box::new(S::default());
    unsafe {
        g_object_set_qdata_full(
            object,
            state_quark(),
            Box::into_raw(boxed).cast(),
            Some(free_state::<S>),
        );
    }
}

pub(crate) unsafe fn state_mut<'a, S: Default + 'static>(
    plugin: *mut XfcePanelPlugin,
) -> &'a mut S {
    let object = plugin.cast::<GObject>();
    let mut ptr = unsafe { g_object_get_qdata(object, state_quark()) }.cast::<S>();
    if ptr.is_null() {
        unsafe { init_state::<S>(plugin) };
        ptr = unsafe { g_object_get_qdata(object, state_quark()) }.cast::<S>();
    }
    unsafe { &mut *ptr }
}
