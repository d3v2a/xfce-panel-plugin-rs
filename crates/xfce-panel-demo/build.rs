//! The `.desktop` file is produced by the `xfce-panel` crate itself, so every
//! plugin gets the same keys the panel reads (`panel-module.c:322-384`).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let extra = [("Version", env!("CARGO_PKG_VERSION"))];
    let mut spec = xfce_panel::build::DesktopSpec::new(
        "demo",
        "Rust Demo",
        "xfce4-panel plugin written in Rust",
    );
    spec.icon("application-x-executable")
        .internal(true)
        .unique(true)
        .extra(&extra);

    xfce_panel::build::write_desktop(&spec);
}
