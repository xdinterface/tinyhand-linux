{
  lib,
  rustPlatform,
  pkg-config,
  wrapGAppsHook4,
  gtk4,
  gtk4-layer-shell,
  libadwaita,
  alsa-lib,
}:
rustPlatform.buildRustPackage {
  pname = "tinyhand";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../Makefile
      ../crates
      ../assets
      ../data
      ../gnome-extension
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = ["-p" "tinyhand"];

  nativeBuildInputs = [pkg-config wrapGAppsHook4];
  buildInputs = [gtk4 gtk4-layer-shell libadwaita alsa-lib];

  preCheck = ''
    export TINYHAND_DATA_DIR=$PWD/assets
  '';

  postInstall = ''
    make install-data PREFIX=$out
  '';

  meta = {
    description = "A tiny hand on a stick that points at things on your screen";
    homepage = "https://github.com/xdinterface/tinyhand-linux";
    license = lib.licenses.mit;
    mainProgram = "tinyhand";
    platforms = lib.platforms.linux;
  };
}
