#!/bin/sh
# Remove o Perícia Sim (mantém seus casos salvos em ~/.local/share/pericia-sim).
rm -rf "$HOME/.local/share/pericia-sim-app"
rm -f "$HOME/.local/bin/pericia-sim" "$HOME/.local/share/applications/pericia-sim.desktop" "$HOME/.local/share/icons/hicolor/256x256/apps/pericia-sim.png"
echo "Perícia Sim removido. Casos salvos preservados em ~/.local/share/pericia-sim"
