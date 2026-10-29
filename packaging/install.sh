#!/bin/sh
# Instala o Perícia Sim para o usuário atual (sem sudo): executável em ~/.local/bin,
# ícone e atalho no menu de aplicativos.
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
BIN="$HOME/.local/bin"; APPS="$HOME/.local/share/applications"; ICONS="$HOME/.local/share/icons/hicolor/256x256/apps"
mkdir -p "$BIN" "$APPS" "$ICONS"
install -m 755 "$DIR/pericia-sim" "$BIN/pericia-sim"
install -m 644 "$DIR/pericia-sim.png" "$ICONS/pericia-sim.png"
sed -e "s|@BIN@|$BIN/pericia-sim|" -e "s|@ICON@|pericia-sim|" "$DIR/pericia-sim.desktop" > "$APPS/pericia-sim.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" >/dev/null 2>&1 || true
echo "Perícia Sim instalado. Abra pelo menu de aplicativos ou execute: $BIN/pericia-sim"
