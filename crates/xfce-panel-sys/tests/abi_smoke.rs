//! ABI harness: loads a plugin cdylib the way the panel does and checks the
//! GType it registers.
//!
//! The panel performs **no type check** on the object returned by
//! `xfce_panel_module_init` (docs/audit-ffi-rust.md §A) — a wrong parent type is a silent
//! segfault. This test is the guard for that: it dlopens the plugin, registers
//! its type in a fresh `GTypeModule`, verifies the parent chain reaches
//! `XfcePanelPlugin`, then instantiates the plugin with the construct
//! properties exactly like `xfce_panel_module_new` does.
//!
//! The panel's `gtk_init_check` lives in the **panel's** binary: gtk-rs
//! keeps its own per-binary initialized flag, so a plugin touching widgets
//! in `construct` panics unless `xfce-panel` claimed GTK as initialized
//! (`gtk::set_initialized` in `register_type`). Instantiation exercises
//! that path (skipped when no display is available).
//!
//! Run with: `just test-abi` (sets `XFCE_PANEL_PLUGIN_SO`).

use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};
use std::path::{Path, PathBuf};

use xfce_panel_sys::glib::{GType, gpointer};

use xfce_panel_sys::gobject::{self, GTypeModule};
use xfce_panel_sys::panel;

const RTLD_NOW: i32 = 2;
const RTLD_LOCAL: i32 = 0;

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: i32) -> *mut std::ffi::c_void;
    fn dlsym(handle: *mut std::ffi::c_void, symbol: *const c_char) -> *mut std::ffi::c_void;
    fn dlerror() -> *const c_char;
}

unsafe extern "C" fn module_load(_module: *mut GTypeModule) -> xfce_panel_sys::glib::gboolean {
    1
}
unsafe extern "C" fn module_unload(_module: *mut GTypeModule) {}

unsafe extern "C" fn module_class_init(class: gpointer, _data: gpointer) {
    unsafe {
        let klass = class as *mut gobject::GTypeModuleClass;
        (*klass).load = Some(module_load);
        (*klass).unload = Some(module_unload);
    }
}

/// `GTypeModule` is abstract — the panel passes a concrete subclass
/// (`XfcePanelModule`). Register a minimal one for the harness; `load`/`unload`
/// are the vfuncs `g_type_module_use`/`unuse` dispatch to (gtypemodule.c).
/// GType names are process-global: register the harness type once, then create
/// one module instance per plugin.
fn harness_type() -> GType {
    static TYPE: std::sync::OnceLock<GType> = std::sync::OnceLock::new();
    *TYPE.get_or_init(|| unsafe {
        let parent = gobject::g_type_module_get_type();
        assert_ne!(parent, 0, "g_type_module_get_type returned 0");
        let ty = gobject::g_type_register_static_simple(
            parent,
            c"AbiTestModule".as_ptr(),
            std::mem::size_of::<gobject::GTypeModuleClass>() as c_uint,
            Some(module_class_init),
            std::mem::size_of::<GTypeModule>() as c_uint,
            None,
            0,
        );
        assert_ne!(ty, 0, "failed to register AbiTestModule");
        ty
    })
}

fn new_module() -> *mut GTypeModule {
    unsafe { gobject::g_object_new(harness_type(), std::ptr::null()) as *mut GTypeModule }
}

fn check_plugin(so: &Path) {
    let c_path = CString::new(so.to_string_lossy().as_bytes()).unwrap();
    unsafe { dlerror() };
    let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
    let err = unsafe { dlerror() };
    let err = if err.is_null() {
        "unknown".to_string()
    } else {
        unsafe { CStr::from_ptr(err) }
            .to_string_lossy()
            .into_owned()
    };
    assert!(!handle.is_null(), "dlopen({}) failed: {err}", so.display());

    let sym = unsafe { dlsym(handle, c"xfce_panel_module_init".as_ptr()) };
    assert!(
        !sym.is_null(),
        "plugin exports no xfce_panel_module_init symbol"
    );

    let init: panel::PluginInitFunc = unsafe { std::mem::transmute(sym) };
    let init = init.expect("xfce_panel_module_init is NULL");

    let module = new_module();
    assert!(!module.is_null());
    assert_ne!(unsafe { gobject::g_type_module_use(module) }, 0);

    let mut make_resident = 0;
    let ty = unsafe { init(module, &mut make_resident) };
    assert_ne!(ty, 0, "xfce_panel_module_init returned an invalid GType");

    let name = unsafe { CStr::from_ptr(gobject::g_type_name(ty)) };
    assert!(
        name.to_string_lossy().starts_with("Xfce"),
        "plugin type {:?} must be named Xfce*",
        name.to_string_lossy()
    );

    let mut parent = unsafe { gobject::g_type_parent(ty) };
    let root = panel::xfce_type_panel_plugin();
    let mut depth = 0;
    while parent != 0 && parent != root {
        parent = unsafe { gobject::g_type_parent(parent) };
        depth += 1;
        assert!(depth < 32, "type parent chain too deep");
    }
    assert_eq!(
        parent, root,
        "plugin type is not a descendant of XfcePanelPlugin"
    );

    // Residency is a plugin choice; the panel forces it for API 1.0 modules
    // (panel-module.c:228), so both answers are ABI-legal.
    assert!(
        make_resident == 0 || make_resident == 1,
        "make_resident must be a gboolean, got {make_resident}"
    );
    assert_ne!(unsafe { gobject::g_type_is_a(ty, root) }, 0);

    // The panel calls gtk_init_check on the main thread before dlopen;
    // replicate it, then instantiate with the construct properties.
    // Without a display gtk_init_check fails and the panel itself would
    // not load plugins — skip that half.
    if unsafe { xfce_panel_sys::gtk::gtk_init_check(std::ptr::null_mut(), std::ptr::null_mut()) }
        == 0
    {
        eprintln!("skip instantiation: no display for gtk_init_check");
    } else {
        let stem = so
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .trim_start_matches("lib")
            .to_string();
        let plugin_name = CString::new(stem.clone()).unwrap();
        let display_name = CString::new(format!("Rust {}", stem)).unwrap();
        let instance = unsafe {
            gobject::g_object_new(
                ty,
                c"name".as_ptr(),
                plugin_name.as_ptr(),
                c"unique-id".as_ptr(),
                1i32,
                c"display-name".as_ptr(),
                display_name.as_ptr(),
                std::ptr::null::<c_char>(),
            )
        };
        assert!(
            !instance.is_null(),
            "g_object_new failed — construct panicked (gtk-rs not initialized?)"
        );
        unsafe {
            gobject::g_object_ref_sink(instance);
            // The panel tears a plugin down by calling the free_data vfunc
            // explicitly (xfce_panel_plugin_provider_destroy), then unref'ing
            // the instance: the qdata destroy notify runs after free_data.
            let class = gobject::g_type_class_ref(ty) as *mut panel::XfcePanelPluginClass;
            if let Some(free_data) = (*class).free_data {
                free_data(instance as *mut panel::XfcePanelPlugin);
            }
            gobject::g_object_unref(instance);
        };
    }

    unsafe { gobject::g_type_module_unuse(module) };
}

/// `XFCE_PANEL_PLUGIN_SO` is a colon-separated list of cdylib paths, so the
/// harness walks every plugin in the workspace.
#[test]
fn plugin_loads_like_the_panel_does() {
    let paths = match env::var_os("XFCE_PANEL_PLUGIN_SO") {
        Some(value) => value
            .to_string_lossy()
            .split(':')
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
        None => {
            eprintln!("skip: set XFCE_PANEL_PLUGIN_SO to the plugin cdylib path");
            return;
        }
    };
    assert!(!paths.is_empty(), "XFCE_PANEL_PLUGIN_SO is empty");

    for so in &paths {
        check_plugin(so);
    }
}
