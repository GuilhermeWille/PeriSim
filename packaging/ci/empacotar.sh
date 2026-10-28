#!/usr/bin/env bash
# Empacota o executável já compilado (target/release) para a plataforma informada.
# Uso: packaging/ci/empacotar.sh linux-x86_64 | windows-x86_64 | macos-universal
set -euo pipefail
plat="$1"
ver=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
nome="pericia-sim-${ver}-${plat}"
out="dist-ci"
rm -rf "$out" && mkdir -p "$out/$nome"
d="$out/$nome"
case "$plat" in
  linux-*)
    cp target/release/pericia-sim "$d/"
    strip "$d/pericia-sim" || true
    cp -r assets docs README.md crates/app/assets/DEJAVU-LICENSE.txt LICENSE \
          packaging/install.sh packaging/uninstall.sh packaging/pericia-sim.desktop packaging/pericia-sim.png "$d/"
    tar -C "$out" -czf "$out/$nome.tar.gz" "$nome"
    ;;
  windows-*)
    cp target/release/pericia-sim.exe "$d/"
    cp -r assets docs README.md crates/app/assets/DEJAVU-LICENSE.txt LICENSE "$d/"
    printf '%s\r\n' "Perícia Sim ${ver} — Windows" "" \
      "1. Extraia a pasta inteira (o jogo precisa da pasta assets ao lado do .exe)." \
      "2. Abra pericia-sim.exe." \
      "3. Se o Windows SmartScreen avisar (aplicativo sem assinatura digital): Mais informações > Executar assim mesmo." \
      "" "Configurações e salvamentos ficam em %APPDATA%\\pericia-sim." > "$d/LEIA-ME-WINDOWS.txt"
    (cd "$out" && 7z a -tzip "$nome.zip" "$nome" > /dev/null)
    ;;
  macos-*)
    app="$d/Perícia Sim.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp target/release/pericia-sim "$app/Contents/MacOS/"
    cp -r assets "$app/Contents/Resources/"
    sed "s/@VERSION@/${ver}/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"
    codesign --force --deep --sign - "$app"
    cp -r docs README.md crates/app/assets/DEJAVU-LICENSE.txt LICENSE "$d/"
    printf '%s\n' "Perícia Sim ${ver} — macOS (Apple Silicon e Intel)" "" \
      "1. Arraste \"Perícia Sim.app\" para Aplicativos (ou abra direto desta pasta)." \
      "2. Na primeira vez: clique com o botão direito no app > Abrir > Abrir." \
      "   O app não é assinado por um desenvolvedor Apple; se o macOS disser que está danificado, rode no Terminal:" \
      "   xattr -dr com.apple.quarantine \"/Applications/Perícia Sim.app\"" "" \
      "Configurações e salvamentos ficam em ~/Library/Application Support/pericia-sim." > "$d/LEIA-ME-MACOS.txt"
    (cd "$out" && ditto -c -k --keepParent "$nome" "$nome.zip")
    ;;
  *) echo "plataforma desconhecida: $plat" >&2; exit 1 ;;
esac
rm -rf "$d"
ls -la "$out"
