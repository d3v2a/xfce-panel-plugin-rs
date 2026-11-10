use std::ffi::OsStr;

fn link(pkg: &str) {
    match pkg_config::Config::new().probe(pkg) {
        Ok(lib) => {
            for path in &lib.link_paths {
                println!("cargo:rustc-link-search=native={}", path.display());
            }
            for l in &lib.libs {
                println!("cargo:rustc-link-lib={l}");
            }
        }
        Err(err) => {
            println!("cargo::warning=package `{pkg}` not found via pkg-config: {err}");
            println!(
                "cargo::warning=enter the dev shell (`nix develop`) or export PKG_CONFIG_PATH"
            );
        }
    }
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    for pkg in ["libxfce4panel-2.0", "libxfce4util-1.0"] {
        link(pkg);
    }
    println!("cargo:rustc-link-arg=-Wl,-z,defs");
    let _ = OsStr::new("");
}
