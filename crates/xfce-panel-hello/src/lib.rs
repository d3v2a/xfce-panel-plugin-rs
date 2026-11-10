//! Smallest xfce4-panel plugin: one label, no settings, no xfconf.
//!
//! Shows the whole contract a plugin needs: a `PanelPlugin` impl naming its
//! `GType`, and [`define_panel_plugin!`](xfce_panel::define_panel_plugin)
//! emitting the `xfce_panel_module_init` entry point the panel `dlsym`s.

use xfce_panel::gtk::prelude::WidgetExt;
use xfce_panel::{PanelPlugin, Plugin, gtk};

#[derive(Default)]
pub struct Hello;

impl PanelPlugin for Hello {
    const TYPE_NAME: &'static str = "XfceRustHello";
    type State = ();

    fn construct(_state: &mut Self::State, plugin: &Plugin) {
        let label = gtk::Label::new(Some("Hello from Rust"));
        label.set_widget_name("xfce-panel-rs-label");
        plugin.add_widget(&label);
        label.show();
    }

    fn size_changed(_state: &mut Self::State, plugin: &Plugin, size: i32) -> bool {
        plugin.set_size_request(size, size);
        true
    }
}

// Resident: the `.so` registers a `GType`, so it must never be `dlclose`d.
xfce_panel::define_panel_plugin!(Hello);
