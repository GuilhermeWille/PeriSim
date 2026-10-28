//! Maleta pericial: escolher a ferramenta que vai para a mão, calçar/trocar luvas e colocar a
//! pulseira antiestática. Abre ao interagir com a maleta na cena (é preciso estar perto dela).
//! Cada escolha vira um comando do motor (`pegar`, `guardar`, `luvas`, `pulseira`), auditado.

use crate::audio::Cat;
use crate::ui::*;
use crate::App;
use macroquad::prelude::*;
use pericia_core::equip::{Gloves, Tool, TOOLS};
use std::collections::HashMap;

#[derive(Default)]
pub struct MaletaUi {
    pub open: bool,
}

/// Ícone (Blender → PNG) de cada ferramenta.
pub fn icon_name(t: Tool) -> &'static str {
    match t {
        Tool::Camera => "camera_foto",
        Tool::Oficio => "oficio_prancheta",
        Tool::Tesoura => "tesoura",
        Tool::Chave => "chave_phillips",
        Tool::Lanterna => "lanterna",
        Tool::Placas => "placa_vestigio",
        Tool::CaboSata => "cabo_sata",
        Tool::AdaptadorM2 => "adaptador_m2",
        Tool::LeitorSd => "leitor_sd",
        Tool::CaboUsb => "cabo_usb",
        Tool::CaboUsbC | Tool::CaboLightning => "cabo_branco",
        Tool::Bateria => "bateria_externa",
        Tool::Faraday => "faraday_bag",
        Tool::Aluminio => "papel_aluminio",
        Tool::Ejetor => "ejetor_sim",
        Tool::PendriveColeta => "pendrive_coleta",
        Tool::Saco => "saco_evidencia",
        Tool::FitaLacre => "fita_lacre",
        Tool::Etiquetas => "etiquetas_caneta",
    }
}

/// Ícones das ferramentas (assets/icons/*.png); ausentes = só o nome.
pub fn load_icons() -> HashMap<String, Texture2D> {
    let mut m = HashMap::new();
    let Some(dir) = crate::assets::assets_dir().map(|d| d.join("icons")) else { return m };
    let Ok(rd) = std::fs::read_dir(&dir) else { return m };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("png") {
            continue;
        }
        if let Ok(b) = std::fs::read(&p) {
            if let Ok(img) = Image::from_file_with_format(&b, Some(ImageFormat::Png)) {
                let tx = Texture2D::from_image(&img);
                tx.set_filter(FilterMode::Linear);
                m.insert(p.file_stem().and_then(|x| x.to_str()).unwrap_or("").to_string(), tx);
            }
        }
    }
    m
}

pub fn draw_icon(app: &App, name: &str, r: Rect) {
    if let Some(tx) = app.icons.get(name) {
        draw_texture_ex(tx, r.x, r.y, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
    }
}

pub fn open(app: &mut App) {
    if !app.maleta.open {
        app.audio.play("maleta_abrir", Cat::Efeitos, 0.9);
    }
    app.maleta.open = true;
    app.player.set_captured(false);
}

pub fn close(app: &mut App) {
    if app.maleta.open {
        app.audio.play("maleta_fechar", Cat::Efeitos, 0.8);
    }
    app.maleta.open = false;
}

/// Teclas da maleta (E ou Esc fecham). Retorna se continua aberta.
pub fn keys(app: &mut App) -> bool {
    if app.maleta.open && (app.ui.key_esc || is_key_pressed(KeyCode::E)) {
        app.ui.key_esc = false;
        close(app);
    }
    app.maleta.open
}

pub fn draw(app: &mut App, area: Rect) {
    if !app.maleta.open {
        return;
    }
    let Some(sim) = app.sim.as_ref() else { return };
    let hand = sim.equip.hand;
    let gloves = sim.equip.gloves.clone();
    let strap = sim.equip.strap;
    let s = app.ui.scale;
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.0, 0.0, 0.0, 0.55));
    let w = (area.w - 40.0 * s).min(1060.0 * s);
    let h = (area.h - 30.0 * s).min(700.0 * s);
    let p = rect(area.x + (area.w - w) / 2.0, area.y + (area.h - h) / 2.0, w, h);
    // casco e espuma
    crate::phone::rrect(p, 18.0 * s, Color::new(0.14, 0.15, 0.17, 1.0));
    let inner = rect(p.x + 14.0 * s, p.y + 14.0 * s, p.w - 28.0 * s, p.h - 28.0 * s);
    crate::phone::rrect(inner, 10.0 * s, Color::new(0.075, 0.078, 0.085, 1.0));
    app.ui.text("MALETA PERICIAL", inner.x + 16.0 * s, inner.y + 28.0 * s, 16.0, F::Bold, Color::new(0.98, 0.62, 0.22, 1.0));
    let na_mao = hand.map(pericia_core::equip::tool_name).unwrap_or("mãos livres");
    app.ui.text(&format!("Na mão: {}", na_mao), inner.x + 200.0 * s, inner.y + 28.0 * s, 13.0, F::Reg, T.text);
    let fr = rect(inner.x + inner.w - 150.0 * s, inner.y + 10.0 * s, 136.0 * s, 28.0 * s);
    let mut cmd: Option<String> = None;
    if app.ui.button(fr, "Fechar  (E)") {
        close(app);
        return;
    }
    // EPI: luvas e pulseira
    let ey = inner.y + 50.0 * s;
    let epi = rect(inner.x + 12.0 * s, ey, inner.w - 24.0 * s, 74.0 * s);
    app.ui.frame(epi, Color::new(0.10, 0.11, 0.125, 1.0), T.border);
    draw_icon(app, "caixa_luvas", rect(epi.x + 8.0 * s, epi.y + 5.0 * s, 64.0 * s, 64.0 * s));
    let (gtxt, gcol) = match &gloves {
        Gloves::Nenhuma => ("Luvas: não calçadas".to_string(), T.warn),
        Gloves::Limpas => ("Luvas: limpas".to_string(), T.ok),
        Gloves::Contaminadas(x) => (format!("Luvas: usadas em {} — troque!", x), T.bad),
    };
    app.ui.text(&gtxt, epi.x + 80.0 * s, epi.y + 24.0 * s, 13.0, F::Bold, gcol);
    let bw = 96.0 * s;
    let by = epi.y + 36.0 * s;
    if app.ui.button_c(rect(epi.x + 80.0 * s, by, bw, 28.0 * s), "Calçar", T.panel2, gloves == Gloves::Nenhuma) {
        cmd = Some("luvas calcar".into());
    }
    if app.ui.button_c(rect(epi.x + 80.0 * s + bw + 6.0 * s, by, bw, 28.0 * s), "Trocar", T.panel2, gloves != Gloves::Nenhuma) {
        cmd = Some("luvas trocar".into());
    }
    if app.ui.button_c(rect(epi.x + 80.0 * s + 2.0 * (bw + 6.0 * s), by, bw, 28.0 * s), "Retirar", T.panel2, gloves != Gloves::Nenhuma) {
        cmd = Some("luvas retirar".into());
    }
    let px = epi.x + epi.w * 0.55;
    draw_icon(app, "pulseira_esd", rect(px, epi.y + 5.0 * s, 64.0 * s, 64.0 * s));
    app.ui.text(if strap { "Pulseira antiestática: colocada" } else { "Pulseira antiestática: não" }, px + 72.0 * s, epi.y + 24.0 * s, 13.0, F::Bold, if strap { T.ok } else { T.dim });
    if app.ui.button(rect(px + 72.0 * s, by, 150.0 * s, 28.0 * s), if strap { "Retirar" } else { "Colocar e aterrar" }) {
        cmd = Some(if strap { "pulseira retirar" } else { "pulseira colocar" }.into());
    }
    // ferramentas
    let gy = epi.y + epi.h + 12.0 * s;
    let cols = 5usize;
    let gap = 8.0 * s;
    let cw = (inner.w - 24.0 * s - gap * (cols as f32 - 1.0)) / cols as f32;
    let rows = (TOOLS.len() + cols - 1) / cols;
    let ch = ((inner.y + inner.h - 46.0 * s - gy) / rows as f32 - gap).min(96.0 * s);
    let mut tip: Option<String> = None;
    for (k, (t, id, nome, uso)) in TOOLS.iter().enumerate() {
        let r = rect(inner.x + 12.0 * s + (k % cols) as f32 * (cw + gap), gy + (k / cols) as f32 * (ch + gap), cw, ch);
        let held = hand == Some(*t);
        let hov = app.ui.hover(r);
        let fill = if held { Color::new(0.17, 0.27, 0.40, 1.0) } else if hov { Color::new(0.16, 0.17, 0.19, 1.0) } else { Color::new(0.11, 0.115, 0.125, 1.0) };
        app.ui.frame(r, fill, if held { T.accent } else if hov { T.border } else { Color::new(0.2, 0.21, 0.24, 1.0) });
        let isz = (ch - 10.0 * s).min(64.0 * s);
        if held {
            app.ui.text("na mão", r.x + 8.0 * s, r.y + 16.0 * s, 10.0, F::Bold, T.accent);
        } else {
            draw_icon(app, icon_name(*t), rect(r.x + 4.0 * s, r.y + (ch - isz) / 2.0, isz, isz));
        }
        let tx = r.x + isz + 10.0 * s;
        let mw = r.x + r.w - tx - 6.0 * s;
        for (j, l) in app.ui.wrap(nome, 12.0, F::Bold, mw).iter().take(3).enumerate() {
            app.ui.text(l, tx, r.y + 20.0 * s + j as f32 * 15.0 * s, 12.0, F::Bold, T.text);
        }
        if hov {
            tip = Some(format!("{} — {} ('pegar {}')", nome, uso, id));
        }
        if app.ui.click_in(r) {
            app.ui.clicks += 1;
            cmd = Some(if held { "guardar".to_string() } else { format!("pegar {}", id) });
        }
    }
    let fy = inner.y + inner.h - 32.0 * s;
    if app.ui.button_c(rect(inner.x + 12.0 * s, fy, 220.0 * s, 26.0 * s), "Guardar o que está na mão", T.panel2, hand.is_some()) {
        cmd = Some("guardar".into());
    }
    let dica = tip.unwrap_or_else(|| "Uma ferramenta por vez: ao pegar outra, a anterior volta para a maleta. Instalados (cabos, Faraday bag, bateria) ficam no lugar.".into());
    app.ui.text(&crate::phone::clip_text(&app.ui, &dica, 11.5, F::Reg, inner.w - 260.0 * s), inner.x + 244.0 * s, fy + 18.0 * s, 11.5, F::Reg, T.dim);
    if let Some(c) = cmd {
        let out = app.run(&c);
        app.toast(out.lines().next().unwrap_or(""));
    }
    app.ui.click_consumed = true;
    app.ui.wheel = 0.0;
}
