{
  description = "Rust plugin stack for xfce4-panel (xfce-panel-sys, xfce-panel, demo plugin)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" ];

      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f system);

      # dev + runtime of the C libraries a panel plugin links against
      xfceSdk =
        pkgs: with pkgs; [
          glib
          glib.dev
          gtk3
          gtk3.dev
          libxfce4util
          libxfce4util.dev
          libxfce4ui
          libxfce4ui.dev
          xfconf
          xfconf.dev
          xfce4-panel
          xfce4-panel.dev
        ];
    in
    {
      version = "0.1.0";

      devShells = eachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            name = "xfce-panel-plugin-rs";

            packages =
              with pkgs;
              [
                # Rust toolchain: keep the flake pinned, not the system one
                cargo
                rustc
                clippy
                rustfmt
                rust-analyzer
                # build.rs: pkg-config probe + bindgen
                pkg-config
                clang
                rust-bindgen
                # rlottie-sys build.rs vendors rlottie C++ via cmake+ninja
                cmake
                ninja
                just
                gdb
                nixfmt
              ]
              ++ xfceSdk pkgs;

            shellHook = ''
              export CARGO_TERM_COLOR=always
              # cargo test harness for the ABI check (crates/xfce-panel-sys/tests/abi_smoke.rs)
              export XFCE_PANEL_PLUGIN_SO="$PWD/target/debug/libdemo.so:$PWD/target/debug/libhello.so"
              # link with nix's gcc (glibc 2.44): the nix glib needs >= 2.43,
              # the clang wrapper targets the system glibc (2.42).
              export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc
              export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C linker=gcc"
              echo "xfce4-panel $(nix eval --raw nixpkgs#xfce4-panel.version 2>/dev/null) SDK in PKG_CONFIG_PATH"
            '';

            # dlopen() from the ABI test resolves the plugin's Nix dependencies
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
              with pkgs;
              [
                glib
                gtk3
                libxfce4util
                libxfce4ui
                xfconf
                xfce4-panel
              ]
            );
          };
        }
      );

      packages = eachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          inherit (pkgs) lib;

          # One plugin package: builds crates/xfce-panel-<name> and installs the
          # .so + the .desktop its build.rs generated.
          pluginPackage =
            name:
            pkgs.rustPlatform.buildRustPackage {
              pname = "xfce-panel-${name}";
              version = self.version;

              src = lib.fileset.toSource {
                root = ./.;
                fileset = lib.fileset.unions [
                  ./Cargo.toml
                  ./Cargo.lock
                  ./crates
                ];
              };

              cargoLock.lockFile = ./Cargo.lock;
              cargoHash = "sha256-nNJiMju0j9LFFutdMTjUWuM3C74Rqfl/nbuTU4jorGA=";

              nativeBuildInputs = with pkgs; [
                pkg-config
                rustc
                cargo
              ];

              buildInputs = xfceSdk pkgs;

              buildAndTestSubdir = "crates/xfce-panel-${name}";

              installPhase = ''
                runHook preInstall

                mkdir -p "$out/lib/xfce4/panel/plugins" "$out/share/xfce4/panel/plugins"
                cp "$(find "$CARGO_TARGET_DIR" -name lib${name}.so -path '*/release/*' -print -quit)" \
                  "$out/lib/xfce4/panel/plugins/lib${name}.so"
                cp "$(find "$CARGO_TARGET_DIR" -name ${name}.desktop -path '*/release/build/*' -print -quit)" \
                  "$out/share/xfce4/panel/plugins/${name}.desktop"

                runHook postInstall
              '';

              meta = {
                description = "xfce4-panel plugin written in Rust (${name})";
                license = lib.licenses.mit;
                platforms = [ "x86_64-linux" ];
              };
            };
        in
        {
          # The demo plugin, installed where the panel looks for plugins.
          # Fill cargoHash with the hash nix reports on the first build
          # (`nix build .#demo` prints `hash required: ...`).
          demo = pluginPackage "demo";
          hello = pluginPackage "hello";

          # nix run .#lint — clippy with the system libs (glib, gtk, xfce4-panel)
          # visible through pkg-config, without going through `nix develop`.
          lint =
            let
              toolchain = with pkgs; [
                cargo
                rustc
                clippy
                pkg-config
                # nix's cc + ld: the nix libs (glib) target nixpkgs' glibc, the
                # system ld does not have the symbols (free_sized).
                stdenv.cc
              ];
              # The full closure of pkg-config paths computed by stdenv
              # (includes transitive deps: harfbuzz, freetype, etc.).
              pkgConfigPath = pkgs.runCommand "xfce-panel-lint-pkg-config" {
                nativeBuildInputs = [ pkgs.pkg-config ];
                buildInputs = xfceSdk pkgs;
              } ''echo "$PKG_CONFIG_PATH" > $out'';
            in
            pkgs.writeShellScriptBin "lint" ''
              export PATH="${pkgs.lib.makeBinPath toolchain}:$PATH"
              export PKG_CONFIG_PATH="$(cat ${pkgConfigPath})"
              exec cargo clippy --all-targets
            '';

          default = self.packages.${system}.demo;
        }
      );

      formatter = eachSystem (system: (import nixpkgs { inherit system; }).nixfmt);
    };
}
