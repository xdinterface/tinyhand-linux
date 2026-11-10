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
    install -Dm644 -t $out/share/tinyhand/hands assets/hands/*.png
    install -Dm644 -t $out/share/tinyhand/sounds assets/sounds/*.wav
    install -Dm644 -t $out/share/tinyhand/icons assets/icons/tinyhand-*-symbolic.svg
    install -Dm644 -t $out/share/icons/hicolor/symbolic/apps assets/icons/tinyhand-*-symbolic.svg
    install -Dm644 -t $out/share/icons/hicolor/scalable/apps assets/icons/io.github.xdinterface.TinyHand.svg
    install -Dm644 -t $out/share/applications data/*.desktop
    mkdir -p $out/share/gnome-shell/extensions
    cp -r gnome-extension/tinyhand@xdinterface.github.io $out/share/gnome-shell/extensions/
  '';

  meta = {
    description = "A tiny hand on a stick that points at things on your screen";
    homepage = "https://github.com/xdinterface/tinyhand-linux";
    license = lib.licenses.mit;
    mainProgram = "tinyhand";
    platforms = lib.platforms.linux;
  };
}
