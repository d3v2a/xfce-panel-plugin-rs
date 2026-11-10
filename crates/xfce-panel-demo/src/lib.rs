//! Demo panel plugin: a label whose text, font size and "shout" mode are
//! xfconf-backed settings.
//!
//! [`panel_settings`](xfce_panel::panel_settings) declares the settings type —
//! wrapper, `ParamSpec` properties, accessors and xfconf bindings in one table
//! — so they save on change, follow the panel's naming convention and survive a
//! panel restart: [`PanelPlugin::save`] stays a
//! no-op.

use xfce_panel::glib::object::ObjectExt;
use xfce_panel::gtk::prelude::{
    ContainerExt, DialogExt, GridExt, GtkWindowExt, LabelExt, WidgetExt,
};
use xfce_panel::{Channel, PanelPlugin, Plugin, glib, gtk};

/// Editable settings of the demo plugin, mirrored in xfconf.
#[xfce_panel::panel_settings]
pub struct Settings {
    /// Loud mode.
    #[xfconf(path = "/shout", default = false)]
    shout: bool,
    /// Font size in points.
    #[xfconf(path = "/font-size", default = 14)]
    font_size: i32,
    /// Greeting text.
    #[xfconf(path = "/greeting", default = "Hello from Rust!")]
    greeting: Option<String>,
}

pub fn render(settings: &Settings, label: &gtk::Label) {
    let greeting = settings.greeting().unwrap_or_default();
    let text = if settings.shout() {
        greeting.to_uppercase()
    } else {
        greeting
    };
    label.set_markup(&format!(
        r#"<span font="Sans {}">{}</span>"#,
        settings.font_size().max(1),
        glib::markup_escape_text(&text)
    ));
}

/// Per-instance state, dropped at `free_data`: dropping the [`Channel`] unbinds
/// the xfconf bindings.
#[derive(Default)]
pub struct DemoState {
    channel: Option<Channel>,
    settings: Option<Settings>,
}

#[derive(Default)]
pub struct Demo;

impl PanelPlugin for Demo {
    const TYPE_NAME: &'static str = "XfceRustDemo";
    type State = DemoState;

    fn construct(state: &mut Self::State, plugin: &Plugin) {
        let label = gtk::Label::new(None);
        label.set_widget_name("xfce-panel-rs-label");
        plugin.add_widget(&label);
        label.show();

        let settings = Settings::new();
        render(&settings, &label);

        let (label, settings2) = (label.clone(), settings.clone());
        settings.connect_notify_local(None, move |_, _| render(&settings2, &label));

        let channel = Channel::new(plugin);
        if let Some(channel) = &channel {
            settings.bind_to(channel);
        }

        state.channel = channel;
        state.settings = Some(settings);

        plugin.menu_show_configure();
        plugin.menu_show_about();
    }

    fn size_changed(_state: &mut Self::State, plugin: &Plugin, size: i32) -> bool {
        plugin.set_size_request(size, size);
        true
    }

    fn configure_plugin(state: &mut Self::State, plugin: &Plugin) {
        let Some(channel) = &state.channel else {
            return;
        };

        let dialog = gtk::Dialog::with_buttons::<gtk::Window>(
            Some(&format!("{} preferences", plugin.name())),
            None,
            gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[("Close", gtk::ResponseType::Close)],
        );
        dialog.set_default_size(280, -1);

        let shout = gtk::Switch::new();
        let size = gtk::SpinButton::with_range(1.0, 72.0, 1.0);
        let greeting = gtk::Entry::new();

        let grid = gtk::Grid::new();
        grid.set_row_spacing(6);
        grid.set_column_spacing(12);
        grid.attach(&gtk::Label::new(Some("Shout")), 0, 0, 1, 1);
        grid.attach(&shout, 1, 0, 1, 1);
        grid.attach(&gtk::Label::new(Some("Font size")), 0, 1, 1, 1);
        grid.attach(&size, 1, 1, 1, 1);
        grid.attach(&gtk::Label::new(Some("Greeting")), 0, 2, 1, 1);
        grid.attach(&greeting, 1, 2, 1, 1);
        dialog.content_area().add(&grid);

        // Widgets bound straight to xfconf: the binding loads the stored value
        // into the widget at bind time, saves every edit, and disconnects
        // itself when the dialog is destroyed (`xfconf-binding.c:456`).
        channel.bind("/shout", glib::Type::BOOL, &shout, "active");
        channel.bind("/font-size", glib::Type::I32, &size, "value");
        channel.bind("/greeting", glib::Type::STRING, &greeting, "text");

        // The delete-event handler of `GtkDialog` emits `response` with
        // `GTK_RESPONSE_DELETE_EVENT` and destroys the window itself, so only
        // the button's response is handled here: calling `close()` on every
        // response would re-enter this handler.
        dialog.connect_response(|dialog, response| {
            if response == gtk::ResponseType::Close {
                dialog.close();
            }
        });
        dialog.show_all();
    }

    fn about(_state: &mut Self::State, plugin: &Plugin) {
        println!("[xfce-panel-rs] about: plugin `{}`", plugin.name());
    }

    fn save(_state: &mut Self::State, plugin: &Plugin) {
        println!("[xfce-panel-rs] save: {}", plugin.name());
    }

    fn free_data(_state: &mut Self::State, plugin: &Plugin) {
        println!("[xfce-panel-rs] free_data: {}", plugin.name());
    }
}

// Resident: the panel forces residency for API 2.0 plugins with
// `X-XFCE-Internal=TRUE` (panel-module.c:228) and never `dlclose`s them, so
// declaring it keeps the plugin honest about its lifetime.
xfce_panel::define_panel_plugin!(Demo);
