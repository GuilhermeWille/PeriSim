#!/bin/sh
# Instala o Perícia Sim para o usuário atual (sem sudo), em qualquer distribuição
# (Arch/Omarchy, Ubuntu, Fedora…): o programa e seus assets ficam em
# ~/.local/share/pericia-sim-app, com atalho em ~/.local/bin e no menu de aplicativos.
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
APP="$HOME/.local/share/pericia-sim-app"
BIN="$HOME/.local/bin"; APPS="$HOME/.local/share/applications"; ICONS="$HOME/.local/share/icons/hicolor/256x256/apps"
mkdir -p "$APP" "$BIN" "$APPS" "$ICONS"
install -m 755 "$DIR/pericia-sim" "$APP/pericia-sim"
rm -rf "$APP/assets.novo" && cp -r "$DIR/assets" "$APP/assets.novo"
[ -d "$APP/assets" ] && mv "$APP/assets" "$APP/assets.antigo"
mv "$APP/assets.novo" "$APP/assets" && rm -rf "$APP/assets.antigo"
ln -sf "$APP/pericia-sim" "$BIN/pericia-sim"
install -m 644 "$DIR/pericia-sim.png" "$ICONS/pericia-sim.png"
sed -e "s|@BIN@|$APP/pericia-sim|" -e "s|@ICON@|pericia-sim|" "$DIR/pericia-sim.desktop" > "$APPS/pericia-sim.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" >/dev/null 2>&1 || true
echo "Perícia Sim instalado. Abra pelo menu de aplicativos (no Omarchy: Super+Espaço) ou execute: pericia-sim"
# avisos úteis (não bloqueiam a instalação)
if ! ldconfig -p 2>/dev/null | grep -q libasound.so.2; then
  echo "Aviso: sem libasound (som). No Arch/Omarchy: sudo pacman -S alsa-lib pipewire-alsa"
fi
