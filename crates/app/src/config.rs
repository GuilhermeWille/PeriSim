//! Menu de configurações (no menu principal e dentro do jogo: botão "Configurações" ou Esc).
//! Áudio, gráficos, controles e interface. Grava em `config.json` ao fechar; nada aqui
//! altera o caso em andamento.

use crate::audio::Cat;
use crate::save::{self, Settings};
use crate::ui::*;
use crate::App;
use macroquad::prelude::*;

#[derive(Default)]
pub struct ConfigUi {
    pub open: bool,
    pub tab: u8,
    /// Escala da interface em edição (aplicada ao soltar o controle, para o layout não pular).
    pending_scale: Option<f32>,
}

const TABS: [&str; 4] = ["Áudio", "Gráficos", "Controles", "Interface"];

pub fn open(app: &mut App) {
    app.config.open = true;
    app.player.set_captured(false);
    app.ui.drag = None;
}

pub fn close(app: &mut App) {
    if let Some(v) = app.config.pending_scale.take() {
        app.settings.ui_scale = v;
    }
    app.config.open = false;
    app.ui.drag = None;
    save::save_settings(&app.settings);
}

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

/// Linha "rótulo ······ [controle] valor".
fn slider_row(app: &mut App, id: &str, x: f32, y: f32, w: f32, label: &str, v: &mut f32, lo: f32, hi: f32, fmt: impl Fn(f32) -> String) -> bool {
    let s = app.ui.scale;
    app.ui.text(label, x, y + 18.0 * s, 13.0, F::Reg, T.text);
    let sx = x + w * 0.38;
    let sw = w * 0.46;
    let ch = app.ui.slider(id, rect(sx, y + 6.0 * s, sw, 16.0 * s), v, lo, hi);
    app.ui.text(&fmt(*v), sx + sw + 16.0 * s, y + 18.0 * s, 12.5, F::Bold, T.dim);
    ch
}

pub fn draw(app: &mut App) {
    if !app.config.open {
        return;
    }
    let s = app.ui.scale;
    let (sw, sh) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
    let w = (760.0 * s).min(sw - 24.0 * s);
    let h = (630.0 * s).min(sh - 24.0 * s);
    let p = rect((sw - w) / 2.0, (sh - h) / 2.0, w, h);
    app.ui.frame(p, T.panel, T.accent2);
    app.ui.text("Configurações", p.x + 18.0 * s, p.y + 30.0 * s, 18.0, F::Bold, T.text);
    app.ui.text("Valem para todos os casos · gravadas ao fechar", p.x + 18.0 * s, p.y + 50.0 * s, 11.0, F::Reg, T.dim);
    // abas à esquerda
    let tw = 150.0 * s;
    for (k, n) in TABS.iter().enumerate() {
        if app.ui.tab(rect(p.x + 14.0 * s, p.y + 70.0 * s + k as f32 * 40.0 * s, tw, 34.0 * s), n, app.config.tab == k as u8) {
            app.config.tab = k as u8;
        }
    }
    let cx = p.x + tw + 34.0 * s;
    let cw = p.x + p.w - cx - 18.0 * s;
    let mut y = p.y + 70.0 * s;
    let row = 40.0 * s;
    let mut st = app.settings.clone();
    match app.config.tab {
        0 => {
            slider_row(app, "vm", cx, y, cw, "Volume geral", &mut st.vol_master, 0.0, 1.0, pct);
            y += row;
            slider_row(app, "va", cx, y, cw, "Ambiente", &mut st.vol_ambient, 0.0, 1.0, pct);
            y += row;
            if slider_row(app, "ve", cx, y, cw, "Efeitos e objetos", &mut st.vol_effects, 0.0, 1.0, pct) && app.ui.drag.is_none() {
                app.audio.play("interagir_gabinete", Cat::Efeitos, 1.0);
            }
            y += row;
            slider_row(app, "vu", cx, y, cw, "Interface e tablet", &mut st.vol_ui, 0.0, 1.0, pct);
            y += row;
            slider_row(app, "vmu", cx, y, cw, "Música (jazz noir)", &mut st.vol_music, 0.0, 1.0, pct);
            y += row + 6.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Música ambiente", &mut st.music_on);
            y += 38.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Silenciar tudo", &mut st.muted);
            y += 38.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Som dos passos", &mut st.footsteps);
            y += 46.0 * s;
            if app.ui.button(rect(cx, y, 170.0 * s, 30.0 * s), "Testar efeito") {
                app.audio.play("foto", Cat::Efeitos, 1.0);
            }
            if app.ui.button(rect(cx + 180.0 * s, y, 170.0 * s, 30.0 * s), "Testar tablet") {
                app.audio.play("tablet_sacar", Cat::Interface, 1.0);
            }
            y += 44.0 * s;
            let status = app.audio.status.lock().unwrap().clone();
            let ok = app.audio.ok.load(std::sync::atomic::Ordering::SeqCst);
            let line = format!("Saída de som: {}", status);
            for l in app.ui.wrap(&line, 11.5, F::Reg, cw) {
                app.ui.text(&l, cx, y + 12.0 * s, 11.5, F::Reg, if ok { T.ok } else { T.warn });
                y += 16.0 * s;
            }
            app.ui.text(&format!("{} sons carregados de assets/sounds", app.audio.clip_count()), cx, y + 12.0 * s, 11.5, F::Reg, T.dim);
        }
        1 => {
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Cena 3D (desligada = modo 2D leve)", &mut st.render3d);
            y += 38.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Sombras em tempo real", &mut st.shadows);
            y += 38.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Economia de energia (redesenha só com entrada)", &mut st.low_power);
            y += 46.0 * s;
            slider_row(app, "fov", cx, y, cw, "Campo de visão", &mut st.fov_deg, 55.0, 95.0, |v| format!("{:.0}°", v));
            y += row + 10.0 * s;
            for l in app.ui.wrap("Sem placa de vídeo, desligar as sombras é o que mais alivia o processador; o modo 2D dispensa a cena 3D por completo.", 11.5, F::Reg, cw) {
                app.ui.text(&l, cx, y + 12.0 * s, 11.5, F::Reg, T.dim);
                y += 16.0 * s;
            }
        }
        2 => {
            slider_row(app, "sens", cx, y, cw, "Sensibilidade do mouse", &mut st.mouse_sens, 0.2, 3.0, |v| format!("{:.1}×", v));
            y += row + 4.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Inverter eixo vertical", &mut st.invert_y);
            y += 48.0 * s;
            app.ui.text("Teclas", cx, y + 12.0 * s, 13.0, F::Bold, T.text);
            y += 24.0 * s;
            for (k, d) in [
                ("W A S D / setas", "andar (Shift corre)"),
                ("Botão direito ou Tab", "olhar / capturar o mouse"),
                ("Clique ou E", "interagir com o objeto na mira / abrir a maleta"),
                ("T", "tirar ou guardar o tablet"),
                ("G", "devolver à maleta a ferramenta da mão"),
                ("Esc", "fechar janelas; no ambiente, abrir estas configurações"),
                ("Page Up / Page Down", "olhar para cima / para baixo pelo teclado"),
            ] {
                app.ui.text(k, cx, y + 12.0 * s, 12.0, F::Bold, T.dim);
                app.ui.text(d, cx + 200.0 * s, y + 12.0 * s, 12.0, F::Reg, T.text);
                y += 21.0 * s;
            }
        }
        _ => {
            let mut sc = app.config.pending_scale.unwrap_or(st.ui_scale);
            slider_row(app, "esc", cx, y, cw, "Escala da interface", &mut sc, 0.7, 1.8, pct);
            if app.ui.drag.as_deref() == Some("esc") {
                app.config.pending_scale = Some(sc);
            } else if (sc - st.ui_scale).abs() > 1e-4 {
                st.ui_scale = (sc * 20.0).round() / 20.0;
                app.config.pending_scale = None;
            }
            y += row + 6.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Dicas de teclas no canto da tela", &mut st.show_hints);
            y += 38.0 * s;
            app.ui.toggle(rect(cx, y, cw, 32.0 * s), "Mostrar as mãos do perito", &mut st.held_tablet);
        }
    }
    // rodapé
    let by = p.y + p.h - 46.0 * s;
    if app.ui.button(rect(p.x + 18.0 * s, by, 190.0 * s, 32.0 * s), "Restaurar padrões") {
        st = Settings::default();
        app.config.pending_scale = None;
    }
    let close_r = rect(p.x + p.w - 168.0 * s, by, 150.0 * s, 32.0 * s);
    let esc = app.ui.key_esc;
    app.settings = st;
    app.audio.set_volumes(app.settings.volumes());
    if app.ui.button_c(close_r, "Fechar  (Esc)", T.accent2, true) || esc {
        app.ui.key_esc = false;
        close(app);
    }
    app.ui.click_consumed = true;
}
