//! End-to-end check of the settings pipeline against the real xfconf daemon:
//! xfconf property -> bound GObject property -> `notify` -> widget repaint.
//!
//! Needs a session bus with `xfconfd` (the nix dev shell provides one). Skips
//! itself when there is none.
//!
//! One test, several scenarios: gtk 3 may only be used from the thread that
//! initialised it, and libtest runs each test on its own thread.

use std::process::Command;
use std::time::{Duration, Instant};

use xfce_panel::gtk::prelude::*;
use xfce_panel::{Channel, glib, gtk};

const CHANNEL: &str = "xfce4-panel";
const ID: i32 = 9999;

fn pump(millis: u64) {
    let ctx = glib::MainContext::default();
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(millis) {
        while ctx.iteration(false) {}
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn cleanup() {
    let base = format!("/plugins/plugin-{ID}");
    let _ = Command::new("xfconf-query")
        .args(["-c", CHANNEL, "-p", &base, "-r", "-R"])
        .output();
}

/// A settings object wired to a label exactly like the demo plugin does:
/// `render` on `notify`, which is also the xfconf -> widget path.
fn settings_with_label() -> (demo::Settings, gtk::Label) {
    let settings: demo::Settings = glib::Object::new();
    let label = gtk::Label::new(None);
    demo::render(&settings, &label);
    let wired = label.clone();
    settings.connect_notify_local(None, move |settings, _| demo::render(settings, &wired));
    (settings, label)
}

/// Defaults are invisible in xfconf until seeded, and a saved value must win
/// over the compiled default.
fn defaults(channel: &Channel) {
    cleanup();
    let (settings, _label) = settings_with_label();
    channel.bind("/font-size", glib::Type::I32, &settings, "font-size");
    channel.bind("/greeting", glib::Type::STRING, &settings, "greeting");
    pump(300);

    assert!(
        channel.ensure("/font-size", settings.font_size()),
        "the default must be seeded"
    );
    assert!(
        channel.ensure("/greeting", settings.greeting().unwrap_or_default()),
        "the default must be seeded"
    );
    pump(300);
    assert_eq!(
        channel.get::<i32>("/font-size"),
        Some(settings.font_size()),
        "a seeded default must reach xfconf"
    );

    let (settings, label) = settings_with_label();
    channel.bind("/greeting", glib::Type::STRING, &settings, "greeting");
    pump(300);
    channel
        .set("/greeting", "already saved")
        .then_some(())
        .expect("write a saved value");
    pump(300);
    assert!(
        !channel.ensure("/greeting", String::from("default")),
        "an existing property must not be reseeded"
    );
    assert_eq!(
        settings.greeting().as_deref(),
        Some("already saved"),
        "the saved value must win over the default"
    );
    assert_eq!(label.text().as_str(), "already saved");
}

/// The three directions: object -> xfconf, xfconf -> object -> widget,
/// widget -> xfconf.
fn round_trips(channel: &Channel) {
    cleanup();
    let (settings, label) = settings_with_label();
    channel.bind("/shout", glib::Type::BOOL, &settings, "shout");
    channel.bind("/font-size", glib::Type::I32, &settings, "font-size");
    channel.bind("/greeting", glib::Type::STRING, &settings, "greeting");
    pump(300);

    settings.set_greeting(Some(String::from("hello")));
    settings.set_font_size(11);
    pump(500);
    assert_eq!(
        channel.get::<String>("/greeting").as_deref(),
        Some("hello"),
        "setting a property must write xfconf"
    );
    assert_eq!(channel.get::<i32>("/font-size"), Some(11));

    channel.set("/shout", true);
    channel.set("/greeting", "world");
    pump(800);
    assert_eq!(
        settings.greeting().as_deref(),
        Some("world"),
        "xfconf must drive the object"
    );
    assert!(settings.shout());
    assert_eq!(
        label.text().as_str(),
        "WORLD",
        "the widget must follow the settings"
    );

    settings.set_shout(false);
    pump(500);
    assert_eq!(channel.get::<bool>("/shout"), Some(false));
    assert_eq!(label.text().as_str(), "world");
}

/// The configure dialog case: plain widgets bound to the xfconf channel.
fn dialog(channel: &Channel) {
    cleanup();
    let (settings, label) = settings_with_label();
    channel.bind("/greeting", glib::Type::STRING, &settings, "greeting");
    channel.bind("/shout", glib::Type::BOOL, &settings, "shout");
    pump(300);

    let greeting = gtk::Entry::new();
    let shout = gtk::Switch::new();
    channel.bind("/greeting", glib::Type::STRING, &greeting, "text");
    channel.bind("/shout", glib::Type::BOOL, &shout, "active");
    pump(300);

    greeting.set_text("from the dialog");
    shout.set_active(true);
    pump(600);
    assert_eq!(
        channel.get::<String>("/greeting").as_deref(),
        Some("from the dialog"),
        "the dialog must write xfconf"
    );
    assert_eq!(channel.get::<bool>("/shout"), Some(true));
    assert_eq!(
        label.text().as_str(),
        "FROM THE DIALOG",
        "the widget must follow the dialog"
    );

    channel.set("/greeting", "from xfconf-query");
    pump(600);
    assert_eq!(
        greeting.text().as_str(),
        "from xfconf-query",
        "xfconf must come back into the dialog"
    );
}

/// Opening the dialog must show the saved values, not blank ones. A
/// `BIDIRECTIONAL` `g_object_bind_property` seeds the source from the
/// target's initial value, so a freshly built `Entry` wipes the saved setting;
/// binding the widget to the channel instead loads the saved value into the
/// widget and keeps the widget as the only writer.
fn dialog_opens_with_saved_values(channel: &Channel) {
    cleanup();
    let (settings, _label) = settings_with_label();
    channel.bind("/greeting", glib::Type::STRING, &settings, "greeting");
    channel.bind("/shout", glib::Type::BOOL, &settings, "shout");
    channel.bind("/font-size", glib::Type::I32, &settings, "font-size");
    pump(300);
    channel.set("/greeting", "saved");
    channel.set("/shout", true);
    channel.set("/font-size", 20);
    pump(600);

    let greeting = gtk::Entry::new();
    let shout = gtk::Switch::new();
    let size = gtk::SpinButton::with_range(1.0, 72.0, 1.0);
    channel.bind("/greeting", glib::Type::STRING, &greeting, "text");
    channel.bind("/shout", glib::Type::BOOL, &shout, "active");
    channel.bind("/font-size", glib::Type::I32, &size, "value");
    pump(400);

    assert_eq!(greeting.text().as_str(), "saved", "the dialog must load");
    assert!(shout.is_active());
    assert_eq!(size.value() as i32, 20);
    assert_eq!(
        settings.greeting().as_deref(),
        Some("saved"),
        "opening the dialog must not clobber the settings"
    );
    assert_eq!(
        channel.get::<String>("/greeting").as_deref(),
        Some("saved"),
        "opening the dialog must not clobber xfconf"
    );
    assert_eq!(channel.get::<i32>("/font-size"), Some(20));

    greeting.set_text("typed in the dialog");
    shout.set_active(false);
    pump(600);
    assert_eq!(
        settings.greeting().as_deref(),
        Some("typed in the dialog"),
        "the dialog must reach the running plugin"
    );
    assert!(
        !settings.shout(),
        "the dialog must reach the running plugin"
    );
}

/// Deleting the xfconf property makes the binding fall back to the
/// `ParamSpec` default (`xfconf-binding.c:376`), which is the default the
/// property declares — not `NULL`. The plugin must survive it, the panel owns
/// this process, and the repaint must follow.
fn deletion(channel: &Channel) {
    cleanup();
    let (settings, label) = settings_with_label();
    channel.bind("/greeting", glib::Type::STRING, &settings, "greeting");
    pump(300);
    settings.set_greeting(Some(String::from("live")));
    pump(300);
    assert_eq!(label.text().as_str(), "live");

    cleanup();
    pump(600);
    assert_eq!(
        settings.greeting().as_deref(),
        Some("Hello from Rust!"),
        "the deleted property must fall back to its declared ParamSpec default"
    );
    assert_eq!(
        label.text().as_str(),
        "Hello from Rust!",
        "the fallback must repaint the widget"
    );
}

/// Widgets bound straight to the xfconf channel: the C binding lives on a
/// `notify::` handler with a `GClosureNotify` (`xfconf-binding.c:456`), so it
/// dies with the widget — nothing to unbind by hand.
fn dialog_bound_to_channel(channel: &Channel) {
    cleanup();
    let (settings, _label) = settings_with_label();
    channel.bind("/greeting", glib::Type::STRING, &settings, "greeting");
    channel.bind("/font-size", glib::Type::I32, &settings, "font-size");
    pump(300);
    channel.set("/greeting", "saved");
    channel.set("/font-size", 20);
    pump(600);

    let greeting = gtk::Entry::new();
    let size = gtk::SpinButton::with_range(1.0, 72.0, 1.0);
    channel.bind("/greeting", glib::Type::STRING, &greeting, "text");
    channel.bind("/font-size", glib::Type::I32, &size, "value");
    pump(600);

    assert_eq!(
        greeting.text().as_str(),
        "saved",
        "the widget must load from xfconf"
    );
    assert_eq!(size.value() as i32, 20);

    greeting.set_text("edited");
    pump(600);
    assert_eq!(
        channel.get::<String>("/greeting").as_deref(),
        Some("edited"),
        "the widget must write xfconf"
    );
    assert_eq!(
        settings.greeting().as_deref(),
        Some("edited"),
        "the settings object must follow"
    );

    // Destroying the dialog widgets must not disturb the channel.
    greeting.unparent();
    size.unparent();
    drop(greeting);
    drop(size);
    pump(400);
    channel.set("/greeting", "after destroy");
    pump(600);
    assert_eq!(
        channel.get::<String>("/greeting").as_deref(),
        Some("after destroy"),
        "a destroyed widget must not break the channel"
    );
    assert_eq!(settings.greeting().as_deref(), Some("after destroy"));
}

#[test]
fn xfconf_backed_settings() {
    if gtk::init().is_err() {
        eprintln!("ignored: no display for gtk");
        return;
    }
    let Some(channel) = Channel::with_base(CHANNEL, &format!("/plugins/plugin-{ID}")) else {
        eprintln!("ignored: no xfconf daemon on this session bus");
        return;
    };

    defaults(&channel);
    round_trips(&channel);
    dialog(&channel);
    dialog_opens_with_saved_values(&channel);
    dialog_bound_to_channel(&channel);
    deletion(&channel);

    channel.unbind_all();
    cleanup();
}
