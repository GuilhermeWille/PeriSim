//! Quadro de provas (cortiça na parede): o jogador fixa, à mão, o que tem do caso atual —
//! fotos que ele mesmo tirou na cena, bilhetes escritos na hora e anotações do tablet — e
//! liga os cartões com barbante vermelho. Abre ao interagir com o quadro (E).
//!
//! As fotos são capturas reais da visão do perito no instante do comando `fotografar`
//! (sem a mão nem a interface), gravadas em `saves/fotos/<caso>/`. O quadro fica em
//! `saves/quadro.json`. Nada disso altera o caso, a auditoria ou a nota.

use crate::save;
use crate::ui::*;
use crate::App;
use macroquad::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub id: u32,
    /// "foto" | "bilhete" | "anotacao"
    pub kind: String,
    pub text: String,
    /// Arquivo da foto (nome dentro da pasta de fotos do caso).
    pub foto: Option<String>,
    /// Posição no quadro (0..1 da esquerda/topo).
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Foto {
    pub file: String,
    pub legenda: String,
    pub hora: String,
}

#[derive(Default)]
pub struct BoardUi {
    pub open: bool,
    pub case_id: String,
    pub cards: Vec<Card>,
    pub links: Vec<(u32, u32)>,
    pub fotos: Vec<Foto>,
    pub tex: HashMap<String, Texture2D>,
    drag: Option<(u32, Vec2)>,
    link_from: Option<u32>,
    mode: u8, // 0 mover · 1 barbante · 2 remover
    draft: String,
    pub pending_photo: Option<String>,
    pub flash_until: f64,
}

fn dir_for(case: &str) -> std::path::PathBuf {
    save::data_dir().join("saves").join("fotos").join(case)
}

fn board_path() -> std::path::PathBuf {
    save::data_dir().join("saves").join("quadro.json")
}

pub fn to_json(b: &BoardUi) -> String {
    json!({
        "formato": 1,
        "case_id": b.case_id,
        "cards": b.cards.iter().map(|c| json!({"id": c.id, "kind": c.kind, "text": c.text, "foto": c.foto, "x": c.x, "y": c.y})).collect::<Vec<_>>(),
        "links": b.links.iter().map(|(a, c)| json!([a, c])).collect::<Vec<_>>(),
        "fotos": b.fotos.iter().map(|f| json!({"file": f.file, "legenda": f.legenda, "hora": f.hora})).collect::<Vec<_>>(),
    })
    .to_string()
}

/// Lê o quadro; de outro caso ou ilegível = vazio.
pub fn from_json(case_id: &str, s: &str) -> (Vec<Card>, Vec<(u32, u32)>, Vec<Foto>) {
    let Ok(v) = serde_json::from_str::<Value>(s) else { return Default::default() };
    if v["case_id"].as_str() != Some(case_id) {
        return Default::default();
    }
    let cards = v["cards"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    Some(Card {
                        id: c["id"].as_u64()? as u32,
                        kind: c["kind"].as_str()?.to_string(),
                        text: c["text"].as_str().unwrap_or("").to_string(),
                        foto: c["foto"].as_str().map(|x| x.to_string()),
                        x: c["x"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0) as f32,
                        y: c["y"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0) as f32,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let links = v["links"].as_array().map(|a| a.iter().filter_map(|l| Some((l[0].as_u64()? as u32, l[1].as_u64()? as u32))).collect()).unwrap_or_default();
    let fotos = v["fotos"]
        .as_array()
        .map(|a| a.iter().filter_map(|f| Some(Foto { file: f["file"].as_str()?.to_string(), legenda: f["legenda"].as_str().unwrap_or("").to_string(), hora: f["hora"].as_str().unwrap_or("").to_string() })).collect())
        .unwrap_or_default();
    (cards, links, fotos)
}

impl BoardUi {
    pub fn bind_case(&mut self, case_id: &str, fresh: bool) {
        self.case_id = case_id.to_string();
        self.tex.clear();
        self.open = false;
        let (c, l, f) = if fresh { Default::default() } else { std::fs::read_to_string(board_path()).map(|s| from_json(case_id, &s)).unwrap_or_default() };
        self.cards = c;
        self.links = l;
        self.fotos = f;
        if fresh {
            self.persist();
        }
    }

    pub fn persist(&self) {
        let _ = std::fs::create_dir_all(save::data_dir().join("saves"));
        let _ = save::write_atomic(&board_path(), to_json(self).as_bytes());
    }

    fn next_id(&self) -> u32 {
        self.cards.iter().map(|c| c.id).max().unwrap_or(0) + 1
    }

    /// Posição livre para um cartão novo (espalha em grade, com leve variação).
    fn free_spot(&self) -> (f32, f32) {
        let n = self.cards.len();
        let col = n % 5;
        let row = (n / 5) % 3;
        (0.12 + col as f32 * 0.19 + (n as f32 * 0.37).sin() * 0.02, 0.2 + row as f32 * 0.3 + (n as f32 * 0.71).cos() * 0.03)
    }

    pub fn pin(&mut self, kind: &str, text: &str, foto: Option<String>) -> u32 {
        let (x, y) = self.free_spot();
        let id = self.next_id();
        self.cards.push(Card { id, kind: kind.into(), text: text.into(), foto, x, y });
        self.persist();
        id
    }

    pub fn texture(&mut self, file: &str) -> Option<Texture2D> {
        if let Some(t) = self.tex.get(file) {
            return Some(t.clone());
        }
        let b = std::fs::read(dir_for(&self.case_id).join(file)).ok()?;
        let img = Image::from_file_with_format(&b, Some(ImageFormat::Png)).ok()?;
        let t = Texture2D::from_image(&img);
        t.set_filter(FilterMode::Linear);
        self.tex.insert(file.to_string(), t.clone());
        Some(t)
    }

    /// Grava a foto a partir da tela (região da visão 3D, logo após desenhar a cena).
    pub fn capture(&mut self, view: Rect, legenda: &str, hora: &str) {
        let img = get_screen_data();
        let (sw, sh) = (img.width as i32, img.height as i32);
        let (x0, y0) = (view.x.max(0.0) as i32, view.y.max(0.0) as i32);
        let (w, h) = ((view.w as i32).min(sw - x0), (view.h as i32).min(sh - y0));
        if w < 16 || h < 16 {
            return;
        }
        // recorte central 4:3 e redução para 400×300 (média de blocos)
        let cw = w.min(h * 4 / 3);
        let ch = cw * 3 / 4;
        let cx = x0 + (w - cw) / 2;
        let cy = y0 + (h - ch) / 2;
        let (ow, oh) = (400i32, 300i32);
        let mut out = vec![0u8; (ow * oh * 4) as usize];
        for oy in 0..oh {
            for ox in 0..ow {
                let sx = cx + ox * cw / ow;
                // a tela vem de baixo para cima
                let sy_top = cy + oy * ch / oh;
                let sy = sh - 1 - sy_top;
                let i = ((sy * sw + sx) * 4) as usize;
                let o = ((oy * ow + ox) * 4) as usize;
                if i + 3 < img.bytes.len() {
                    out[o..o + 3].copy_from_slice(&img.bytes[i..i + 3]);
                }
                out[o + 3] = 255;
            }
        }
        let dir = dir_for(&self.case_id);
        let _ = std::fs::create_dir_all(&dir);
        let file = format!("foto_{:02}.png", self.fotos.len() + 1);
        // export_png grava de baixo para cima (como a tela): inverte as linhas antes
        let row = (ow * 4) as usize;
        let mut flipped = Vec::with_capacity(out.len());
        for r in (0..oh as usize).rev() {
            flipped.extend_from_slice(&out[r * row..(r + 1) * row]);
        }
        let im = Image { bytes: flipped, width: ow as u16, height: oh as u16 };
        im.export_png(&dir.join(&file).to_string_lossy());
        self.fotos.push(Foto { file, legenda: legenda.to_string(), hora: hora.to_string() });
        self.persist();
    }
}

/// Área do quadro em tela (cortiça), dentro do painel.
fn board_rect(p: Rect, s: f32) -> Rect {
    rect(p.x + 16.0 * s, p.y + 54.0 * s, p.w - 300.0 * s, p.h - 70.0 * s)
}

fn card_size(c: &Card, b: Rect) -> Vec2 {
    let k = b.w / 1000.0;
    if c.kind == "foto" { vec2(150.0 * k, 135.0 * k) } else { vec2(140.0 * k, 105.0 * k) }
}

fn card_rect(c: &Card, b: Rect) -> Rect {
    let sz = card_size(c, b);
    rect(b.x + c.x * b.w - sz.x / 2.0, b.y + c.y * b.h - sz.y / 2.0, sz.x, sz.y)
}

pub fn open(app: &mut App) {
    app.board.open = true;
    app.player.set_captured(false);
    app.audio.play("papel", crate::audio::Cat::Efeitos, 0.6);
}

pub fn close(app: &mut App) {
    app.board.open = false;
    app.board.drag = None;
    app.board.link_from = None;
    if app.ui.focus.as_deref() == Some("bnote") {
        app.ui.focus = None;
    }
    app.board.persist();
}

pub fn keys(app: &mut App) -> bool {
    if app.board.open {
        let typing = app.ui.focus.as_deref() == Some("bnote");
        if app.ui.key_esc {
            app.ui.key_esc = false;
            if typing {
                app.ui.focus = None;
            } else {
                close(app);
            }
        } else if !typing && is_key_pressed(KeyCode::E) {
            close(app);
        }
    }
    app.board.open
}

fn draw_card(app: &mut App, c: &Card, r: Rect, hl: bool) {
    let s = app.ui.scale;
    // sombra
    draw_rectangle(r.x + 3.0 * s, r.y + 4.0 * s, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.3));
    if c.kind == "foto" {
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.97, 0.96, 0.93, 1.0));
        let ph = rect(r.x + 6.0 * s, r.y + 6.0 * s, r.w - 12.0 * s, (r.w - 12.0 * s) * 0.75);
        match c.foto.as_ref().and_then(|f| app.board.texture(f)) {
            Some(t) => draw_texture_ex(&t, ph.x, ph.y, WHITE, DrawTextureParams { dest_size: Some(vec2(ph.w, ph.h)), ..Default::default() }),
            None => draw_rectangle(ph.x, ph.y, ph.w, ph.h, Color::new(0.2, 0.2, 0.22, 1.0)),
        }
        let t = crate::phone::clip_text(&app.ui, &c.text, 9.5, F::Reg, r.w - 10.0 * s);
        app.ui.text(&t, r.x + 5.0 * s, r.y + r.h - 7.0 * s, 9.5, F::Reg, Color::new(0.15, 0.15, 0.15, 1.0));
    } else {
        let col = if c.kind == "anotacao" { Color::new(0.80, 0.90, 1.0, 1.0) } else { Color::new(1.0, 0.93, 0.55, 1.0) };
        draw_rectangle(r.x, r.y, r.w, r.h, col);
        let mut y = r.y + 15.0 * s;
        for l in app.ui.wrap(&c.text, 10.5, F::Reg, r.w - 10.0 * s).iter().take(((r.h - 8.0 * s) / (13.0 * s)) as usize) {
            app.ui.text(l, r.x + 5.0 * s, y, 10.5, F::Reg, Color::new(0.12, 0.1, 0.05, 1.0));
            y += 13.0 * s;
        }
    }
    if hl {
        draw_rectangle_lines(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0, 2.0, T.highlight);
    }
    // alfinete
    draw_circle(r.x + r.w / 2.0, r.y + 5.0 * s, 5.0 * s, Color::new(0.8, 0.1, 0.1, 1.0));
}

pub fn draw(app: &mut App, area: Rect) {
    if !app.board.open {
        return;
    }
    let s = app.ui.scale;
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.0, 0.0, 0.0, 0.6));
    let w = (area.w - 30.0 * s).min(1280.0 * s);
    let h = (area.h - 24.0 * s).min(760.0 * s);
    let p = rect(area.x + (area.w - w) / 2.0, area.y + (area.h - h) / 2.0, w, h);
    app.ui.frame(p, Color::new(0.16, 0.12, 0.09, 1.0), Color::new(0.35, 0.25, 0.15, 1.0));
    app.ui.text("QUADRO DE PROVAS", p.x + 16.0 * s, p.y + 30.0 * s, 17.0, F::Bold, Color::new(1.0, 0.82, 0.5, 1.0));
    app.ui.text("arraste os cartões · barbante liga dois cartões · nada aqui altera o caso ou a nota", p.x + 236.0 * s, p.y + 30.0 * s, 11.0, F::Reg, T.dim);
    let b = board_rect(p, s);
    // cortiça
    let cork = app.renderer.as_ref().and_then(|r| r.tex.get(&crate::render::Tex::CorkPlain).cloned());
    match cork {
        Some(t) => draw_texture_ex(&t, b.x, b.y, WHITE, DrawTextureParams { dest_size: Some(vec2(b.w, b.h)), ..Default::default() }),
        None => draw_rectangle(b.x, b.y, b.w, b.h, Color::new(0.68, 0.5, 0.32, 1.0)),
    }
    draw_rectangle_lines(b.x, b.y, b.w, b.h, 6.0 * s, Color::new(0.35, 0.24, 0.14, 1.0));
    // barbantes
    let cards = app.board.cards.clone();
    for (a, c) in app.board.links.clone() {
        let (Some(ca), Some(cc)) = (cards.iter().find(|x| x.id == a), cards.iter().find(|x| x.id == c)) else { continue };
        let (ra, rc) = (card_rect(ca, b), card_rect(cc, b));
        draw_line(ra.x + ra.w / 2.0, ra.y + 5.0 * s, rc.x + rc.w / 2.0, rc.y + 5.0 * s, 3.0 * s, Color::new(0.75, 0.08, 0.1, 0.95));
    }
    // cartões
    let mouse = app.ui.mouse;
    let mut hovered: Option<u32> = None;
    for c in cards.iter().rev() {
        if card_rect(c, b).contains(mouse) {
            hovered = Some(c.id);
            break;
        }
    }
    for c in &cards {
        let hl = hovered == Some(c.id) || app.board.link_from == Some(c.id);
        draw_card(app, c, card_rect(c, b), hl);
    }
    // interação com o quadro
    let clicked = app.ui.clicked && !app.ui.click_consumed && b.contains(mouse);
    if clicked {
        app.ui.click_consumed = true;
        if let Some(id) = hovered {
            match app.board.mode {
                1 => match app.board.link_from.take() {
                    Some(a) if a != id => {
                        if !app.board.links.iter().any(|l| *l == (a, id) || *l == (id, a)) {
                            app.board.links.push((a, id));
                            app.board.persist();
                        }
                    }
                    Some(_) => {}
                    None => app.board.link_from = Some(id),
                },
                2 => {
                    app.board.cards.retain(|c| c.id != id);
                    app.board.links.retain(|(a, c)| *a != id && *c != id);
                    app.board.persist();
                }
                _ => {
                    let c = cards.iter().find(|x| x.id == id).unwrap();
                    let r = card_rect(c, b);
                    app.board.drag = Some((id, mouse - vec2(r.x + r.w / 2.0, r.y + r.h / 2.0)));
                }
            }
        }
    }
    if let Some((id, off)) = app.board.drag {
        if is_mouse_button_down(MouseButton::Left) {
            let p2 = mouse - off;
            if let Some(c) = app.board.cards.iter_mut().find(|x| x.id == id) {
                c.x = ((p2.x - b.x) / b.w).clamp(0.05, 0.95);
                c.y = ((p2.y - b.y) / b.h).clamp(0.08, 0.92);
            }
            app.ui.any_input = true;
        } else {
            app.board.drag = None;
            app.board.persist();
        }
    }
    // barra lateral
    let sx = b.x + b.w + 12.0 * s;
    let sw = p.x + p.w - sx - 14.0 * s;
    let mut y = p.y + 54.0 * s;
    if app.ui.button(rect(p.x + p.w - 150.0 * s, p.y + 10.0 * s, 136.0 * s, 28.0 * s), "Fechar  (E)") {
        close(app);
        return;
    }
    for (k, nome) in ["Mover", "Barbante", "Remover"].iter().enumerate() {
        let bw = (sw - 8.0 * s) / 3.0;
        if app.ui.tab(rect(sx + k as f32 * (bw + 4.0 * s), y, bw, 26.0 * s), nome, app.board.mode == k as u8) {
            app.board.mode = k as u8;
            app.board.link_from = None;
        }
    }
    y += 36.0 * s;
    app.ui.text("Novo bilhete", sx, y + 12.0 * s, 12.0, F::Bold, T.text);
    y += 18.0 * s;
    let mut buf = std::mem::take(&mut app.board.draft);
    let enter = app.ui.input("bnote", rect(sx, y, sw, 28.0 * s), &mut buf, "escreva e tecle Enter");
    app.board.draft = buf;
    y += 32.0 * s;
    if (app.ui.button(rect(sx, y, sw, 26.0 * s), "Fixar bilhete") || enter) && !app.board.draft.trim().is_empty() {
        let t = std::mem::take(&mut app.board.draft);
        app.board.pin("bilhete", t.trim(), None);
        app.ui.focus = None;
    }
    y += 36.0 * s;
    app.ui.text(&format!("Fotos do caso ({})", app.board.fotos.len()), sx, y + 12.0 * s, 12.0, F::Bold, T.text);
    y += 18.0 * s;
    if app.board.fotos.is_empty() {
        for l in app.ui.wrap("Fotografe objetos e vestígios com a câmera: cada foto fica disponível aqui.", 11.0, F::Reg, sw) {
            app.ui.text(&l, sx, y + 12.0 * s, 11.0, F::Reg, T.dim);
            y += 15.0 * s;
        }
    }
    let tw = (sw - 8.0 * s) / 3.0;
    let fotos = app.board.fotos.clone();
    let mut pin_foto: Option<Foto> = None;
    for (k, f) in fotos.iter().enumerate().take(9) {
        let r = rect(sx + (k % 3) as f32 * (tw + 4.0 * s), y + (k / 3) as f32 * (tw * 0.75 + 6.0 * s), tw, tw * 0.75);
        match app.board.texture(&f.file) {
            Some(t) => draw_texture_ex(&t, r.x, r.y, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), ..Default::default() }),
            None => draw_rectangle(r.x, r.y, r.w, r.h, T.panel2),
        }
        let pinned = app.board.cards.iter().any(|c| c.foto.as_deref() == Some(f.file.as_str()));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, if pinned { T.ok } else if app.ui.hover(r) { T.accent } else { T.border });
        if app.ui.click_in(r) && !pinned {
            pin_foto = Some(f.clone());
        }
    }
    if let Some(f) = pin_foto {
        let txt = format!("{} · {}", f.legenda, f.hora);
        app.board.pin("foto", &txt, Some(f.file));
        app.audio.play("papel", crate::audio::Cat::Interface, 0.6);
    }
    y += ((fotos.len().min(9) + 2) / 3) as f32 * (tw * 0.75 + 6.0 * s) + 10.0 * s;
    app.ui.text("Anotações do tablet", sx, y + 12.0 * s, 12.0, F::Bold, T.text);
    y += 18.0 * s;
    let notas = app.tablet.notas.clone();
    if notas.is_empty() {
        app.ui.text("(nenhuma)", sx, y + 12.0 * s, 11.0, F::Reg, T.dim);
    }
    for n in notas.iter().rev() {
        if y > p.y + p.h - 40.0 * s {
            break;
        }
        let txt = format!("{}: {}", n.tag, n.texto.replace('\n', " "));
        let pinned = app.board.cards.iter().any(|c| c.kind == "anotacao" && c.text == txt);
        let label = crate::phone::clip_text(&app.ui, &txt, 13.0, F::Reg, sw - 16.0 * s);
        if app.ui.button_c(rect(sx, y, sw, 26.0 * s), &label, T.panel2, !pinned) {
            app.board.pin("anotacao", &txt, None);
        }
        y += 30.0 * s;
    }
    app.ui.click_consumed = true;
    app.ui.wheel = 0.0;
}

/// Cartões do quadro na parede da cena 3D (fotos com a imagem; bilhetes e anotações em cor).
/// `origin` = canto superior esquerdo útil do quadro; `right`/`down` = eixos do quadro no mundo.
pub fn draw_3d(app: &mut App, r: &mut crate::render::Renderer, origin: Vec3, right: Vec3, down: Vec3, size: Vec2, normal: Vec3) {
    let cards = app.board.cards.clone();
    let off = normal * 0.004;
    let quad = |c: Vec3, w: f32, h: f32| -> [Vec3; 4] {
        let (hx, hy) = (right * (w / 2.0), down * (h / 2.0));
        [c - hx - hy + off, c + hx - hy + off, c + hx + hy + off, c - hx + hy + off]
    };
    let pos = |c: &Card| origin + right * (c.x * size.x) + down * (c.y * size.y);
    for (a, b) in &app.board.links {
        let (Some(ca), Some(cb)) = (cards.iter().find(|x| x.id == *a), cards.iter().find(|x| x.id == *b)) else { continue };
        let (pa, pb) = (pos(ca) - down * 0.05, pos(cb) - down * 0.05);
        let d = pb - pa;
        let len = d.length();
        let n = (normal.cross(d) / len.max(0.001)) * 0.002;
        r.quad(crate::render::Tex::White, [pa - n + off * 1.5, pb - n + off * 1.5, pb + n + off * 1.5, pa + n + off * 1.5], [vec2(0.0, 0.0); 4], normal, Color::new(0.7, 0.05, 0.07, 1.0), 0.0, true);
    }
    for c in &cards {
        let p = pos(c);
        if c.kind == "foto" {
            r.quad(crate::render::Tex::White, quad(p, 0.15, 0.135), [vec2(0.0, 0.0); 4], normal, Color::new(0.95, 0.94, 0.9, 1.0), 0.1, false);
            if let Some(t) = c.foto.as_ref().and_then(|f| app.board.texture(f)) {
                let q = quad(p - down * 0.008 + off, 0.134, 0.1);
                r.extra.push((t, q, WHITE));
            }
        } else {
            let col = if c.kind == "anotacao" { Color::new(0.8, 0.9, 1.0, 1.0) } else { Color::new(1.0, 0.92, 0.55, 1.0) };
            r.quad(crate::render::Tex::Label, quad(p, 0.12, 0.09), [vec2(0.0, 0.0), vec2(0.4, 0.0), vec2(0.4, 0.3), vec2(0.0, 0.3)], normal, col, 0.05, false);
        }
        r.cyl(p - down * 0.055 + normal * 0.01, 0.006, 0.012, crate::render::m(crate::render::Tex::Plastic, Color::new(0.8, 0.1, 0.1, 1.0), 0.3, 0.6));
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn quadro_ida_e_volta_e_isolado_por_caso() {
        let b = BoardUi {
            case_id: "C501-D2-00000007".into(),
            cards: vec![Card { id: 1, kind: "bilhete".into(), text: "lacre \"ok\"".into(), foto: None, x: 0.3, y: 0.4 }, Card { id: 2, kind: "foto".into(), text: "ITEM-01".into(), foto: Some("foto_01.png".into()), x: 0.6, y: 0.5 }],
            links: vec![(1, 2)],
            fotos: vec![Foto { file: "foto_01.png".into(), legenda: "ITEM-01".into(), hora: "08:40".into() }],
            ..Default::default()
        };
        let j = to_json(&b);
        let (c, l, f) = from_json("C501-D2-00000007", &j);
        assert_eq!((c, l, f), (b.cards.clone(), b.links.clone(), b.fotos.clone()));
        assert_eq!(from_json("C502-D1-00000001", &j), (vec![], vec![], vec![]));
    }
}
