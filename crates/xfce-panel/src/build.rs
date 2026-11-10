//! Generation of the `.desktop` file the panel uses to discover a plugin.
//!
//! This module is meant to be called from a plugin crate's `build.rs`:
//!
//! ```ignore
//! // crates/my-plugin/build.rs
//! fn main() {
//!     let mut spec = xfce_panel::build::DesktopSpec::new("my-plugin", "My Plugin", "…");
//!     xfce_panel::build::write_desktop(&spec);
//! }
//! ```
//!
//! The file is written to `OUT_DIR/<module>.desktop`. The panel reads it from
//! `<datadir>/xfce4/panel/plugins/`, the `.so` from `<libdir>/xfce4/panel/plugins/`
//! — `just install` and the flake package do that copy.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Contents of the `[Xfce Panel]` group of a plugin `.desktop`.
#[derive(Debug)]
pub struct DesktopSpec<'a> {
    /// `X-XFCE-Module`: the `.so` stem (`g_module_build_path` adds `lib`/`.so`).
    pub module: &'a str,
    /// `Name`: display name shown in the plugin dialog and menu header.
    pub name: &'a str,
    /// `Comment`: one-liner shown next to the name.
    pub comment: &'a str,
    /// `Icon`.
    pub icon: &'a str,
    /// `X-XFCE-Internal`: allow running inside the panel process (X11 only;
    /// Wayland always goes through `wrapper-2.0`).
    pub internal: bool,
    /// `X-XFCE-Unique`: allow several instances with distinct unique ids.
    pub unique: bool,
    /// `X-XFCE-API`: must be `2.0`, the panel refuses anything else.
    pub api: &'a str,
    /// Extra keys appended verbatim.
    pub extra: &'a [(&'a str, &'a str)],
}

impl<'a> DesktopSpec<'a> {
    /// Builds a spec with panel defaults: `internal` and `unique` true,
    /// `api = "2.0"`, a generic icon, no extra keys.
    #[must_use]
    pub fn new(module: &'a str, name: &'a str, comment: &'a str) -> Self {
        Self {
            module,
            name,
            comment,
            icon: "application-x-executable",
            internal: true,
            unique: true,
            api: "2.0",
            extra: &[],
        }
    }

    /// Sets `Icon`.
    #[allow(
        clippy::must_use_candidate,
        reason = "builder used statement-style in build scripts"
    )]
    pub fn icon(&mut self, icon: &'a str) -> &mut Self {
        self.icon = icon;
        self
    }

    /// Sets `X-XFCE-Internal`.
    #[allow(
        clippy::must_use_candidate,
        reason = "builder used statement-style in build scripts"
    )]
    pub fn internal(&mut self, internal: bool) -> &mut Self {
        self.internal = internal;
        self
    }

    /// Sets `X-XFCE-Unique`.
    #[allow(
        clippy::must_use_candidate,
        reason = "builder used statement-style in build scripts"
    )]
    pub fn unique(&mut self, unique: bool) -> &mut Self {
        self.unique = unique;
        self
    }

    /// Sets `X-XFCE-API` (the panel only accepts `2.0`).
    #[allow(
        clippy::must_use_candidate,
        reason = "builder used statement-style in build scripts"
    )]
    pub fn api(&mut self, api: &'a str) -> &mut Self {
        self.api = api;
        self
    }

    /// Appends extra `key=value` lines verbatim.
    #[allow(
        clippy::must_use_candidate,
        reason = "builder used statement-style in build scripts"
    )]
    pub fn extra(&mut self, extra: &'a [(&'a str, &'a str)]) -> &mut Self {
        self.extra = extra;
        self
    }

    fn validate(&self) -> Result<(), String> {
        if self.module.is_empty() {
            return Err("module must not be empty".to_string());
        }
        if !self
            .module
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        {
            return Err(format!(
                "module {:?} must be [a-z0-9-_] (it becomes the .so file name)",
                self.module
            ));
        }
        if self.name.is_empty() {
            return Err("name must not be empty".to_string());
        }
        if self.api != "2.0" {
            return Err(format!(
                "api {:?} is refused by the panel (only 2.0 is accepted)",
                self.api
            ));
        }
        for (key, _) in self.extra {
            if key.contains(['=', '\n']) {
                return Err(format!("extra key {key:?} is not a valid desktop key"));
            }
        }
        Ok(())
    }
}

/// Renders the `.desktop` content.
///
/// # Panics
///
/// Panics if `spec` is invalid: empty module, module outside `[a-z0-9-_]`,
/// empty name, `api` other than `2.0`, or an extra key containing `=`/newline.
#[must_use]
pub fn generate(spec: &DesktopSpec<'_>) -> String {
    spec.validate().expect("invalid DesktopSpec");

    let mut out = String::new();
    out.push_str("[Xfce Panel]\n");
    out.push_str("Type=X-XFCE-PanelPlugin\n");
    out.push_str("Encoding=UTF-8\n");
    let _ = writeln!(out, "Name={}", spec.name);
    let _ = writeln!(out, "Comment={}", spec.comment);
    let _ = writeln!(out, "Icon={}", spec.icon);
    let _ = writeln!(out, "X-XFCE-Module={}", spec.module);
    let _ = writeln!(out, "X-XFCE-API={}", spec.api);
    let _ = writeln!(
        out,
        "X-XFCE-Internal={}",
        if spec.internal { "TRUE" } else { "FALSE" }
    );
    let _ = writeln!(
        out,
        "X-XFCE-Unique={}",
        if spec.unique { "TRUE" } else { "FALSE" }
    );
    for (key, value) in spec.extra {
        let _ = writeln!(out, "{key}={value}");
    }
    out
}

/// Writes `<module>.desktop` into `OUT_DIR` and returns its path.
///
/// # Panics
///
/// Panics if `OUT_DIR` is unset (i.e. not running in a build script), or if
/// `spec` is invalid (see [`generate`]).
#[allow(
    clippy::must_use_candidate,
    reason = "called as a statement from build scripts"
)]
pub fn write_desktop(spec: &DesktopSpec<'_>) -> PathBuf {
    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR is set for build scripts");
    write_desktop_to(spec, Path::new(&out_dir))
}

/// Writes `<module>.desktop` into `dir` and returns its path.
///
/// # Panics
///
/// Panics if the file cannot be written, or if `spec` is invalid (see
/// [`generate`]).
#[must_use]
pub fn write_desktop_to(spec: &DesktopSpec<'_>, dir: &Path) -> PathBuf {
    let path = dir.join(format!("{}.desktop", spec.module));
    std::fs::write(&path, generate(spec))
        .unwrap_or_else(|err| panic!("cannot write {}: {err}", path.display()));
    println!("cargo:info=generated {}", path.display());
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_keys_the_panel_reads() {
        let mut spec = DesktopSpec::new("demo", "Rust Demo", "A plugin in Rust");
        spec.icon("clock");
        let text = generate(&spec);
        assert!(text.starts_with("[Xfce Panel]\n"));
        assert!(text.contains("Type=X-XFCE-PanelPlugin\n"));
        assert!(text.contains("X-XFCE-Module=demo\n"));
        assert!(text.contains("X-XFCE-API=2.0\n"));
        assert!(text.contains("X-XFCE-Internal=TRUE\n"));
        assert!(text.contains("X-XFCE-Unique=TRUE\n"));
        assert!(text.contains("Icon=clock\n"));
    }

    #[test]
    #[should_panic(expected = "refused by the panel")]
    fn refuses_the_api_the_panel_would_reject() {
        let mut spec = DesktopSpec::new("demo", "n", "c");
        spec.api("1.0");
        generate(&spec);
    }

    #[test]
    fn refuses_module_names_that_cannot_be_a_shared_object() {
        let spec = DesktopSpec::new("Bad Name", "n", "c");
        assert!(spec.validate().is_err());
    }
}
