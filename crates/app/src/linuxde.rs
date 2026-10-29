//! Ambiente de trabalho do "Perícia Linux" na Estação Pericial: tela de login, painel
//! superior (menu Aplicativos, lista de janelas, rede, relógio), ícones na área de
//! trabalho e janelas móveis/redimensionáveis com Terminal (bash), Arquivos, Editor de
//! texto e os aplicativos do simulador (Navegador, Documento, Manual...).

use crate::desktop;
use crate::ui::*;
use crate::App;
use macroquad::prelude::*;
use pericia_core::os::{self, shell::run_line, Kind, Os, Shell};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WinKind {
    Terminal(usize),
    Files,
    Editor,
    Navegador,
    Documento,
    Manual,
    Mentor,
    Custodia,
    Linha,
    Caso,
    Evidencias,
    Avaliacao,
}

impl WinKind {
    pub fn title(self) -> String {
        match self {
            WinKind::Terminal(_) => "Terminal — perito@estacao-pericial".into(),
            WinKind::Files => "Arquivos".into(),
            WinKind::Editor => "Editor de texto".into(),
            WinKind::Navegador => "Navegador — rede emulada do caso".into(),
            WinKind::Documento => "Documento pericial".into(),
            WinKind::Manual => "Manual dos POPs".into(),
            WinKind::Mentor => "Mentor".into(),
            WinKind::Custodia => "Cadeia de custódia".into(),
            WinKind::Linha => "Linha do tempo".into(),
            WinKind::Caso => "Caso".into(),
            WinKind::Evidencias => "Visualizador de evidências".into(),
            WinKind::Avaliacao => "Avaliação".into(),
        }
    }
    fn short(self) -> &'static str {
        match self {
            WinKind::Terminal(_) => "Terminal",
            WinKind::Files => "Arquivos",
            WinKind::Editor => "Editor",
            WinKind::Navegador => "Navegador",
            WinKind::Documento => "Documento",
            WinKind::Manual => "Manual",
            WinKind::Mentor => "Mentor",
            WinKind::Custodia => "Custódia",
            WinKind::Linha => "Linha do tempo",
            WinKind::Caso => "Caso",
            WinKind::Evidencias => "Evidências",
            WinKind::Avaliacao => "Avaliação",
        }
    }
}

pub struct Win {
    pub id: u32,
    pub kind: WinKind,
    pub r: Rect,
    pub min: bool,
    pub max: bool,
    pub scroll: f32,
}

pub struct LTerm {
    pub sh: Shell,
    pub buf: String,
    pub input: String,
    pub scroll: f32,
    pub hist_idx: usize,
}

pub struct LinuxDe {
    pub os: Option<Os>,
    pub os_case: String,
    pub logged: bool,
    pub pass: String,
    pub login_msg: String,
    pub wins: Vec<Win>,
    pub terms: Vec<LTerm>,
    pub menu_open: bool,
    drag: Option<(u32, Vec2, bool)>,
    pub files_cwd: String,
    pub files_sel: Option<String>,
    files_scroll: f32,
    pub editor_path: String,
    pub editor_buf: String,
    pub editor_ro: bool,
    pub editor_orig: String,
    editor_scroll: f32,
    pub editor_msg: String,
    next_id: u32,
    last_click: (f64, String),
    pub snapshot: Option<Texture2D>,
    pub preview_age: u32,
}

impl LinuxDe {
    pub fn new() -> Self {
        LinuxDe {
            os: None,
            os_case: String::new(),
            logged: false,
            pass: String::new(),
            login_msg: String::new(),
            wins: vec![],
            terms: vec![],
            menu_open: false,
            drag: None,
            files_cwd: "/home/perito".into(),
            files_sel: None,
            files_scroll: 0.0,
            editor_path: String::new(),
            editor_buf: String::new(),
            editor_ro: true,
            editor_orig: String::new(),
            editor_scroll: 0.0,
            editor_msg: String::new(),
            next_id: 1,
            last_click: (0.0, String::new()),
            snapshot: None,
            preview_age: 0,
        }
    }
}

/// Garante que o SO exista para o caso atual.
pub fn ensure_os(app: &mut App) {
    let Some(sim) = &app.sim else { return };
    if app.de.os.is_none() || app.de.os_case != sim.case.id {
        let mut o = Os::new(sim);
        o.sync(sim);
        app.de.os = Some(o);
        app.de.os_case = sim.case.id.clone();
        app.de.logged = false;
        app.de.wins.clear();
        app.de.terms.clear();
        app.de.files_cwd = "/home/perito".into();
    }
    if let (Some(o), Some(sim)) = (&mut app.de.os, &app.sim) {
        o.sync(sim);
    }
}

// --------------------------------------------------------------------- janelas

pub fn open(app: &mut App, kind: WinKind) {
    // aplicativos de instância única: apenas trazem a janela à frente
    if !matches!(kind, WinKind::Terminal(_)) {
        if let Some(i) = app.de.wins.iter().position(|w| w.kind == kind) {
            let mut w = app.de.wins.remove(i);
            w.min = false;
            app.de.wins.push(w);
            return;
        }
    }
    let kind = if let WinKind::Terminal(_) = kind {
        let home = app.de.os.as_ref().map(|o| o.home.clone()).unwrap_or_else(|| "/home/perito".into());
        let motd = app.de.os.as_ref().and_then(|o| o.local_node("/etc/motd")).and_then(|n| match n {
            os::Node::File(f) => Some(f.data.clone()),
            _ => None,
        });
        let first = app.de.terms.is_empty();
        app.de.terms.push(LTerm { sh: Shell::new(&home), buf: if first { motd.unwrap_or_default() } else { String::new() }, input: String::new(), scroll: f32::MAX, hist_idx: 0 });
        WinKind::Terminal(app.de.terms.len() - 1)
    } else {
        kind
    };
    if kind == WinKind::Documento {
        desktop::load_laudo(app);
    }
    let n = app.de.wins.len() as f32;
    let s = app.ui.scale;
    let (w, h) = match kind {
        WinKind::Terminal(_) => (760.0, 440.0),
        WinKind::Files => (820.0, 480.0),
        WinKind::Editor => (720.0, 480.0),
        WinKind::Documento | WinKind::Navegador | WinKind::Evidencias => (960.0, 560.0),
        _ => (720.0, 480.0),
    };
    let id = app.de.next_id;
    app.de.next_id += 1;
    app.de.wins.push(Win { id, kind, r: Rect::new((60.0 + n * 34.0) * s, (40.0 + n * 28.0) * s, w * s, h * s), min: false, max: false, scroll: 0.0 });
    if let WinKind::Terminal(_) = kind {
        app.ui.focus = Some(format!("lterm{}", id));
    }
}

fn close(app: &mut App, id: u32) {
    if let Some(i) = app.de.wins.iter().position(|w| w.id == id) {
        if app.de.wins[i].kind == WinKind::Documento {
            desktop::sync_laudo(app);
        }
        app.de.wins.remove(i);
    }
}

struct Muted {
    clicked: bool,
    wheel: f32,
    chars: Vec<char>,
    enter: bool,
    back: bool,
    up: bool,
    down: bool,
    tab: bool,
    paste: Option<String>,
}

fn mute(ui: &mut Ui) -> Muted {
    let m = Muted { clicked: ui.clicked, wheel: ui.wheel, chars: std::mem::take(&mut ui.chars), enter: ui.key_enter, back: ui.key_back, up: ui.key_up, down: ui.key_down, tab: ui.key_tab, paste: ui.paste.take() };
    ui.clicked = false;
    ui.wheel = 0.0;
    ui.key_enter = false;
    ui.key_back = false;
    ui.key_up = false;
    ui.key_down = false;
    ui.key_tab = false;
    m
}

fn unmute(ui: &mut Ui, m: Muted) {
    ui.clicked = m.clicked;
    ui.wheel = m.wheel;
    ui.chars = m.chars;
    ui.key_enter = m.enter;
    ui.key_back = m.back;
    ui.key_up = m.up;
    ui.key_down = m.down;
    ui.key_tab = m.tab;
    ui.paste = m.paste;
}

fn clip(r: Option<Rect>) {
    unsafe {
        let gl = get_internal_gl();
        gl.quad_gl.scissor(r.map(|r| (r.x as i32, r.y as i32, r.w.max(0.0) as i32, r.h.max(0.0) as i32)));
    }
}

// --------------------------------------------------------------------- desenho principal

pub fn draw(app: &mut App, area: Rect) {
    ensure_os(app);
    let s = app.ui.scale;
    wallpaper(app, area);
    if !app.de.logged {
        login(app, area);
        return;
    }
    let panel_h = 28.0 * s;
    let desk = Rect::new(area.x, area.y + panel_h, area.w, area.h - panel_h);
    // limita janelas à área útil
    for w in app.de.wins.iter_mut() {
        if w.max {
            continue;
        }
        w.r.w = w.r.w.min(desk.w - 10.0);
        w.r.h = w.r.h.min(desk.h - 10.0);
        w.r.x = w.r.x.clamp(0.0, (desk.w - 60.0 * s).max(0.0));
        w.r.y = w.r.y.clamp(0.0, (desk.h - 30.0 * s).max(0.0));
    }
    let abs_r = |w: &Win| -> Rect { if w.max { desk } else { Rect::new(desk.x + w.r.x, desk.y + w.r.y, w.r.w, w.r.h) } };
    // janela sob o mouse (de cima para baixo)
    let hover = app.de.wins.iter().enumerate().rev().find(|(_, w)| !w.min && abs_r(w).contains(app.ui.mouse)).map(|(i, _)| i);
    let menu_r = Rect::new(area.x + 4.0 * s, area.y + panel_h, 250.0 * s, 0.0);
    let in_menu = app.de.menu_open && Rect::new(menu_r.x, menu_r.y, menu_r.w, 420.0 * s).contains(app.ui.mouse);
    if app.ui.clicked && !in_menu {
        if let Some(i) = hover {
            let top = app.de.wins.len() - 1;
            if i != top {
                let w = app.de.wins.remove(i);
                if let WinKind::Terminal(_) = w.kind {
                    app.ui.focus = Some(format!("lterm{}", w.id));
                } else {
                    app.ui.focus = None;
                }
                app.de.wins.push(w);
                app.ui.click_consumed = true; // o clique só ativa a janela
            }
        }
    }
    let hover_any = hover.is_some();
    desktop_icons(app, desk, hover_any || app.de.menu_open);
    // janelas (de trás para frente)
    let n = app.de.wins.len();
    let mut to_close = None;
    for i in 0..n {
        if app.de.wins[i].min {
            continue;
        }
        let is_top = i == n - 1 && !app.de.menu_open;
        let saved = if is_top { None } else { Some(mute(&mut app.ui)) };
        let r = abs_r(&app.de.wins[i]);
        if let Some(c) = draw_window(app, i, r, is_top) {
            to_close = Some(c);
        }
        if let Some(m) = saved {
            unmute(&mut app.ui, m);
        }
    }
    if let Some(id) = to_close {
        close(app, id);
    }
    // arrastar / redimensionar
    if let Some((id, off, resize)) = app.de.drag {
        if is_mouse_button_down(MouseButton::Left) {
            if let Some(w) = app.de.wins.iter_mut().find(|w| w.id == id) {
                if resize {
                    w.r.w = (app.ui.mouse.x - desk.x - w.r.x).max(320.0 * s);
                    w.r.h = (app.ui.mouse.y - desk.y - w.r.y).max(200.0 * s);
                } else {
                    w.r.x = app.ui.mouse.x - desk.x - off.x;
                    w.r.y = app.ui.mouse.y - desk.y - off.y;
                }
            }
            app.ui.any_input = true;
        } else {
            app.de.drag = None;
        }
    }
    top_panel(app, area, panel_h);
}

fn wallpaper(app: &mut App, area: Rect) {
    // degradê azul-petróleo com o logotipo da distribuição
    let steps = 24;
    for k in 0..steps {
        let t = k as f32 / steps as f32;
        let c = Color::new(0.05 + 0.04 * t, 0.12 + 0.10 * t, 0.17 + 0.12 * t, 1.0);
        draw_rectangle(area.x, area.y + area.h * t, area.w, area.h / steps as f32 + 1.0, c);
    }
    let s = app.ui.scale;
    let cx = area.x + area.w * 0.72;
    let cy = area.y + area.h * 0.62;
    draw_circle_lines(cx, cy, 70.0 * s, 3.0 * s, Color::new(1.0, 1.0, 1.0, 0.08));
    draw_circle_lines(cx, cy, 52.0 * s, 2.0 * s, Color::new(1.0, 1.0, 1.0, 0.06));
    draw_line(cx + 48.0 * s, cy + 48.0 * s, cx + 92.0 * s, cy + 92.0 * s, 10.0 * s, Color::new(1.0, 1.0, 1.0, 0.07));
    app.ui.text("Perícia Linux", cx - 64.0 * s, cy + 128.0 * s, 20.0, F::Bold, Color::new(1.0, 1.0, 1.0, 0.16));
    app.ui.text("12.4 \"Lacre\"", cx - 32.0 * s, cy + 148.0 * s, 12.0, F::Reg, Color::new(1.0, 1.0, 1.0, 0.14));
}

fn login(app: &mut App, area: Rect) {
    let s = app.ui.scale;
    let card = Rect::new(area.x + area.w / 2.0 - 180.0 * s, area.y + area.h / 2.0 - 150.0 * s, 360.0 * s, 300.0 * s);
    app.ui.frame(card, Color::new(0.10, 0.12, 0.15, 1.0), T.border);
    let cx = card.x + card.w / 2.0;
    draw_circle(cx, card.y + 62.0 * s, 34.0 * s, T.accent2);
    draw_circle(cx, card.y + 54.0 * s, 12.0 * s, T.text);
    draw_circle(cx, card.y + 82.0 * s, 18.0 * s, T.text);
    draw_rectangle(cx - 34.0 * s, card.y + 86.0 * s, 68.0 * s, 14.0 * s, T.accent2);
    let name = "perito";
    let w = app.ui.measure(name, 16.0, F::Bold);
    app.ui.text(name, cx - w / 2.0, card.y + 128.0 * s, 16.0, F::Bold, T.text);
    let mut pass = std::mem::take(&mut app.de.pass);
    if app.ui.focus.is_none() {
        app.ui.focus = Some("login".into());
    }
    let enter = password_input(app, Rect::new(card.x + 40.0 * s, card.y + 150.0 * s, card.w - 80.0 * s, 32.0 * s), &mut pass);
    app.de.pass = pass;
    let go = app.ui.button_c(Rect::new(card.x + 40.0 * s, card.y + 192.0 * s, card.w - 80.0 * s, 32.0 * s), "Entrar", T.accent2, true) || enter;
    if go {
        if app.de.pass == "perito" {
            app.de.logged = true;
            app.de.pass.clear();
            app.de.login_msg.clear();
            if let Some(sim) = &mut app.sim {
                sim.rec_pub("login", "perito@estacao-pericial", "sessão gráfica iniciada");
            }
            open(app, WinKind::Terminal(0));
        } else {
            app.de.login_msg = "Senha incorreta. Tente novamente.".into();
            app.de.pass.clear();
        }
    }
    let msg = if app.de.login_msg.is_empty() { "Senha da estação do laboratório: perito".to_string() } else { app.de.login_msg.clone() };
    let mw = app.ui.measure(&msg, 11.5, F::Reg);
    app.ui.text(&msg, cx - mw / 2.0, card.y + 250.0 * s, 11.5, F::Reg, if app.de.login_msg.is_empty() { T.dim } else { T.bad });
    let clock = app.sim.as_ref().map(|x| pericia_core::util::fmt_hm(x.clock, pericia_core::util::BRT)).unwrap_or_default();
    let cw = app.ui.measure(&clock, 40.0, F::Reg);
    app.ui.text(&clock, area.x + area.w / 2.0 - cw / 2.0, card.y - 30.0 * s, 40.0, F::Reg, Color::new(1.0, 1.0, 1.0, 0.85));
}

fn password_input(app: &mut App, r: Rect, buf: &mut String) -> bool {
    if app.ui.click_in(r) {
        app.ui.focus = Some("login".into());
    }
    let focused = app.ui.focus.as_deref() == Some("login");
    app.ui.frame(r, T.term_bg, if focused { T.accent } else { T.border });
    let mut enter = false;
    if focused {
        for c in app.ui.chars.clone() {
            buf.push(c);
        }
        if app.ui.key_back {
            buf.pop();
        }
        enter = app.ui.key_enter;
    }
    let dots: String = "●".repeat(buf.chars().count());
    let s = app.ui.scale;
    if buf.is_empty() {
        app.ui.text("Senha", r.x + 10.0 * s, r.y + r.h / 2.0 + 5.0 * s, 13.0, F::Reg, T.dim);
    } else {
        app.ui.text(&dots, r.x + 10.0 * s, r.y + r.h / 2.0 + 5.0 * s, 13.0, F::Reg, T.text);
    }
    enter
}

fn top_panel(app: &mut App, area: Rect, h: f32) {
    let s = app.ui.scale;
    draw_rectangle(area.x, area.y, area.w, h, Color::new(0.06, 0.07, 0.09, 1.0));
    // menu Aplicativos
    let mb = Rect::new(area.x + 4.0 * s, area.y + 2.0 * s, 110.0 * s, h - 4.0 * s);
    let hov = app.ui.hover(mb);
    if hov || app.de.menu_open {
        draw_rectangle(mb.x, mb.y, mb.w, mb.h, T.accent2);
    }
    draw_circle(mb.x + 12.0 * s, mb.y + mb.h / 2.0, 6.0 * s, T.accent);
    app.ui.text("Aplicativos", mb.x + 24.0 * s, mb.y + mb.h / 2.0 + 5.0 * s, 12.5, F::Bold, T.text);
    let toggled = app.ui.click_in(mb);
    // lista de janelas
    let mut x = mb.x + mb.w + 10.0 * s;
    let n = app.de.wins.len();
    let mut action: Option<(usize, u8)> = None;
    for (i, w) in app.de.wins.iter().enumerate() {
        let label = w.kind.short();
        let bw = (app.ui.measure(label, 12.0, F::Reg) + 22.0 * s).min(150.0 * s);
        let r = Rect::new(x, area.y + 3.0 * s, bw, h - 6.0 * s);
        if x + bw > area.x + area.w - 260.0 * s {
            break;
        }
        let top = i == n - 1 && !w.min;
        app.ui.frame(r, if top { Color::new(0.20, 0.26, 0.34, 1.0) } else { Color::new(0.12, 0.13, 0.16, 1.0) }, if top { T.accent } else { T.border });
        app.ui.text(label, r.x + 10.0 * s, r.y + r.h / 2.0 + 5.0 * s, 12.0, F::Reg, if w.min { T.dim } else { T.text });
        if app.ui.click_in(r) {
            action = Some((i, if w.min { 0 } else if top { 1 } else { 2 }));
        }
        x += bw + 4.0 * s;
    }
    if let Some((i, a)) = action {
        match a {
            1 => app.de.wins[i].min = true,
            _ => {
                let mut w = app.de.wins.remove(i);
                w.min = false;
                app.de.wins.push(w);
            }
        }
    }
    // área de notificação: rede, relógio
    let (clock, date) = app.sim.as_ref().map(|x| (pericia_core::util::fmt_hm(x.clock, pericia_core::util::BRT), pericia_core::util::fmt_date(x.clock, pericia_core::util::BRT))).unwrap_or_default();
    let net = app.sim.as_ref().map(|x| if x.case.net.is_some() { format!("rede do caso ({})", x.net_conn) } else { "sem rede".into() }).unwrap_or_default();
    let right = format!("⇅ {}    perito    {} {}", net, date, clock);
    let rw = app.ui.measure(&right, 12.0, F::Reg);
    app.ui.text(&right, area.x + area.w - rw - 12.0 * s, area.y + h / 2.0 + 5.0 * s, 12.0, F::Reg, T.text);
    if toggled {
        app.de.menu_open = !app.de.menu_open;
    }
    if app.de.menu_open {
        apps_menu(app, Rect::new(area.x + 4.0 * s, area.y + h, 260.0 * s, 0.0), toggled);
    }
}

const MENU: &[(&str, &str, WinKind)] = &[
    ("Sistema", "Terminal", WinKind::Terminal(0)),
    ("Sistema", "Arquivos", WinKind::Files),
    ("Acessórios", "Editor de texto", WinKind::Editor),
    ("Internet", "Navegador (rede do caso)", WinKind::Navegador),
    ("Perícia", "Caso e quesitos", WinKind::Caso),
    ("Perícia", "Visualizador de evidências", WinKind::Evidencias),
    ("Perícia", "Linha do tempo", WinKind::Linha),
    ("Perícia", "Cadeia de custódia", WinKind::Custodia),
    ("Perícia", "Documento pericial", WinKind::Documento),
    ("Perícia", "Avaliação", WinKind::Avaliacao),
    ("Ajuda", "Manual dos POPs", WinKind::Manual),
    ("Ajuda", "Mentor", WinKind::Mentor),
];

fn apps_menu(app: &mut App, at: Rect, just_opened: bool) {
    let s = app.ui.scale;
    let ih = 26.0 * s;
    let mut rows = 0.0;
    let mut last_cat = "";
    for (cat, _, _) in MENU {
        if *cat != last_cat {
            rows += 0.8;
            last_cat = cat;
        }
        rows += 1.0;
    }
    let r = Rect::new(at.x, at.y, at.w, rows * ih + 12.0 * s);
    app.ui.frame(r, Color::new(0.09, 0.10, 0.13, 1.0), T.border);
    let mut y = r.y + 6.0 * s;
    let mut last_cat = "";
    let mut chosen = None;
    for (cat, label, kind) in MENU {
        if *cat != last_cat {
            app.ui.text(cat, r.x + 10.0 * s, y + ih * 0.6, 10.5, F::Bold, T.dim);
            y += ih * 0.8;
            last_cat = cat;
        }
        let ir = Rect::new(r.x + 4.0 * s, y, r.w - 8.0 * s, ih);
        if app.ui.hover(ir) {
            draw_rectangle(ir.x, ir.y, ir.w, ir.h, T.accent2);
        }
        app.ui.text(label, ir.x + 18.0 * s, ir.y + ih * 0.68, 12.5, F::Reg, T.text);
        if app.ui.click_in(ir) {
            chosen = Some(*kind);
        }
        y += ih;
    }
    if let Some(k) = chosen {
        app.de.menu_open = false;
        open(app, k);
    } else if app.ui.clicked && !app.ui.click_consumed && !just_opened && !r.contains(app.ui.mouse) {
        app.de.menu_open = false;
    }
}

fn icon(app: &Ui, kind: &str, r: Rect) {
    let s = app.scale;
    match kind {
        "folder" => {
            draw_rectangle(r.x, r.y + r.h * 0.12, r.w * 0.42, r.h * 0.18, Color::from_rgba(214, 166, 60, 255));
            draw_rectangle(r.x, r.y + r.h * 0.24, r.w, r.h * 0.66, Color::from_rgba(236, 190, 84, 255));
        }
        "terminal" => {
            draw_rectangle(r.x, r.y + r.h * 0.1, r.w, r.h * 0.8, Color::from_rgba(30, 32, 36, 255));
            draw_rectangle_lines(r.x, r.y + r.h * 0.1, r.w, r.h * 0.8, 1.5, T.dim);
            app.text(">_", r.x + r.w * 0.16, r.y + r.h * 0.62, 13.0, F::Mono, T.term_fg);
        }
        "web" => {
            draw_circle(r.x + r.w / 2.0, r.y + r.h / 2.0, r.w * 0.42, Color::from_rgba(60, 130, 210, 255));
            draw_circle_lines(r.x + r.w / 2.0, r.y + r.h / 2.0, r.w * 0.42, 1.5, T.text);
            draw_line(r.x + r.w * 0.08, r.y + r.h / 2.0, r.x + r.w * 0.92, r.y + r.h / 2.0, 1.5, T.text);
        }
        "doc" => {
            draw_rectangle(r.x + r.w * 0.15, r.y, r.w * 0.7, r.h, Color::from_rgba(240, 240, 244, 255));
            for k in 0..4 {
                draw_line(r.x + r.w * 0.25, r.y + r.h * (0.28 + k as f32 * 0.16), r.x + r.w * 0.75, r.y + r.h * (0.28 + k as f32 * 0.16), 1.5 * s, Color::from_rgba(120, 130, 150, 255));
            }
        }
        "drive" => {
            draw_rectangle(r.x, r.y + r.h * 0.3, r.w, r.h * 0.45, Color::from_rgba(160, 166, 176, 255));
            draw_circle(r.x + r.w * 0.82, r.y + r.h * 0.52, 2.5 * s, Color::from_rgba(80, 220, 120, 255));
        }
        _ => {
            draw_rectangle(r.x + r.w * 0.2, r.y, r.w * 0.6, r.h, Color::from_rgba(220, 224, 232, 255));
        }
    }
}

fn desktop_icons(app: &mut App, desk: Rect, blocked: bool) {
    let s = app.ui.scale;
    let items: Vec<(&str, &str, &str)> = vec![
        ("folder", "Pasta pessoal", "home"),
        ("terminal", "Terminal", "term"),
        ("web", "Navegador", "web"),
        ("doc", "LEIA-ME.txt", "readme"),
        ("doc", "Documento pericial", "laudo"),
        ("folder", "Caso atual", "caso"),
    ];
    let cell = 92.0 * s;
    for (k, (ic, label, act)) in items.iter().enumerate() {
        let r = Rect::new(desk.x + 14.0 * s, desk.y + 12.0 * s + k as f32 * cell, 78.0 * s, cell - 6.0 * s);
        let hov = !blocked && app.ui.hover(r);
        if hov {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(1.0, 1.0, 1.0, 0.10));
        }
        icon(&app.ui, ic, Rect::new(r.x + r.w / 2.0 - 20.0 * s, r.y + 6.0 * s, 40.0 * s, 40.0 * s));
        let lw = app.ui.measure(label, 11.0, F::Reg);
        app.ui.text(label, r.x + (r.w - lw).max(0.0) / 2.0, r.y + 64.0 * s, 11.0, F::Reg, T.text);
        if !blocked && app.ui.click_in(r) {
            let now = get_time();
            let dbl = app.de.last_click.1 == *act && now - app.de.last_click.0 < 0.45;
            app.de.last_click = (now, act.to_string());
            if dbl {
                let case_dir = app.de.os.as_ref().map(|o| o.case_dir.clone()).unwrap_or_default();
                match *act {
                    "home" => {
                        app.de.files_cwd = "/home/perito".into();
                        open(app, WinKind::Files);
                    }
                    "caso" => {
                        app.de.files_cwd = case_dir;
                        open(app, WinKind::Files);
                    }
                    "term" => open(app, WinKind::Terminal(0)),
                    "web" => open(app, WinKind::Navegador),
                    "laudo" => open(app, WinKind::Documento),
                    _ => open_in_editor(app, "/home/perito/Área de Trabalho/LEIA-ME.txt"),
                }
            }
        }
    }
}

/// Desenha a janela i; retorna Some(id) se o usuário pediu para fechar.
fn draw_window(app: &mut App, i: usize, r: Rect, is_top: bool) -> Option<u32> {
    let s = app.ui.scale;
    let id = app.de.wins[i].id;
    let kind = app.de.wins[i].kind;
    // sombra e moldura
    draw_rectangle(r.x + 4.0 * s, r.y + 6.0 * s, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_rectangle(r.x, r.y, r.w, r.h, T.panel);
    let th = 30.0 * s;
    let title = Rect::new(r.x, r.y, r.w, th);
    draw_rectangle(title.x, title.y, title.w, title.h, if is_top { Color::from_rgba(44, 50, 60, 255) } else { Color::from_rgba(32, 36, 42, 255) });
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, if is_top { T.accent2 } else { T.border });
    let t = kind.title();
    let tw = app.ui.measure(&t, 12.5, F::Bold);
    app.ui.text(&t, r.x + (r.w - tw) / 2.0, r.y + th * 0.66, 12.5, F::Bold, if is_top { T.text } else { T.dim });
    // botões: minimizar, maximizar, fechar
    let bs = 18.0 * s;
    let mut close_req = None;
    let btns = [("–", 0u8), ("□", 1), ("×", 2)];
    for (k, (g, a)) in btns.iter().enumerate() {
        let br = Rect::new(r.x + r.w - (3 - k) as f32 * (bs + 8.0 * s) - 4.0 * s, r.y + (th - bs) / 2.0, bs, bs);
        let hov = app.ui.hover(br);
        draw_circle(br.x + bs / 2.0, br.y + bs / 2.0, bs / 2.0, if hov && *a == 2 { Color::from_rgba(220, 70, 60, 255) } else if hov { Color::from_rgba(80, 88, 100, 255) } else { Color::from_rgba(58, 64, 74, 255) });
        app.ui.text(g, br.x + bs * 0.27, br.y + bs * 0.78, 12.0, F::Bold, T.text);
        if app.ui.click_in(br) {
            match a {
                0 => app.de.wins[i].min = true,
                1 => app.de.wins[i].max = !app.de.wins[i].max,
                _ => close_req = Some(id),
            }
        }
    }
    // arrastar pela barra de título (duplo clique maximiza)
    if is_top && app.ui.clicked && !app.ui.click_consumed && title.contains(app.ui.mouse) {
        let now = get_time();
        let key = format!("title{}", id);
        if app.de.last_click.1 == key && now - app.de.last_click.0 < 0.4 {
            app.de.wins[i].max = !app.de.wins[i].max;
        } else if !app.de.wins[i].max {
            app.de.drag = Some((id, app.ui.mouse - vec2(r.x, r.y) + vec2(0.0, 0.0) - vec2(0.0, 0.0), false));
            let w = &app.de.wins[i];
            app.de.drag = Some((id, app.ui.mouse - vec2(w.r.x, w.r.y), false));
        }
        app.de.last_click = (now, key);
        app.ui.click_consumed = true;
    }
    // canto de redimensionamento
    let grip = Rect::new(r.x + r.w - 14.0 * s, r.y + r.h - 14.0 * s, 14.0 * s, 14.0 * s);
    let body = Rect::new(r.x + 6.0 * s, r.y + th + 6.0 * s, r.w - 12.0 * s, r.h - th - 12.0 * s);
    clip(Some(Rect::new(r.x, r.y + th, r.w, r.h - th)));
    match kind {
        WinKind::Terminal(t) => terminal(app, i, t, body, is_top),
        WinKind::Files => files(app, body),
        WinKind::Editor => editor(app, body),
        WinKind::Navegador => desktop::navegador(app, body),
        WinKind::Documento => desktop::laudo(app, body),
        WinKind::Manual => desktop::manual(app, body),
        WinKind::Evidencias => desktop::evidencias(app, body),
        WinKind::Caso | WinKind::Mentor | WinKind::Custodia | WinKind::Linha | WinKind::Avaliacao => {
            let txt = app.sim.as_ref().map(|sim| match kind {
                WinKind::Caso => pericia_core::tools::describe_case(sim),
                WinKind::Mentor => pericia_core::assess::hint(sim),
                WinKind::Custodia => format!("{}\n\nAUDITORIA ENCADEADA (SHA-256)\n{}", pericia_core::tools::custody(sim), sim.audit.render(pericia_core::util::BRT)),
                WinKind::Linha => pericia_core::tools::timeline_text(sim),
                _ => {
                    if app.desk.eval_text.is_empty() {
                        format!("O caso ainda não foi finalizado (Documento pericial → Emitir e finalizar).\n\n{}", pericia_core::assess::progress(sim))
                    } else {
                        app.desk.eval_text.clone()
                    }
                }
            }).unwrap_or_default();
            let mono = matches!(kind, WinKind::Custodia | WinKind::Linha);
            let mut sc = app.de.wins[i].scroll;
            app.ui.scroll_text(body, &txt, &mut sc, if mono { 11.5 } else { 12.5 }, if mono { F::Mono } else { F::Reg }, if mono { T.term_fg } else { T.text }, false);
            app.de.wins[i].scroll = sc;
        }
    }
    clip(None);
    if !app.de.wins[i].max {
        draw_triangle(vec2(grip.x + grip.w, grip.y), vec2(grip.x + grip.w, grip.y + grip.h), vec2(grip.x, grip.y + grip.h), T.border);
        if is_top && app.ui.clicked && !app.ui.click_consumed && grip.contains(app.ui.mouse) {
            app.de.drag = Some((id, Vec2::ZERO, true));
            app.ui.click_consumed = true;
        }
    }
    close_req
}

// --------------------------------------------------------------------- terminal

fn terminal(app: &mut App, wi: usize, t: usize, body: Rect, is_top: bool) {
    let s = app.ui.scale;
    let wid = app.de.wins[wi].id;
    let fid = format!("lterm{}", wid);
    if is_top && app.ui.click_in(body) {
        app.ui.focus = Some(fid.clone());
    }
    let focused = is_top && app.ui.focus.as_deref() == Some(fid.as_str());
    draw_rectangle(body.x - 6.0 * s, body.y - 6.0 * s, body.w + 12.0 * s, body.h + 12.0 * s, Color::from_rgba(18, 20, 24, 255));
    // entrada de teclado
    if focused {
        let mut enter = false;
        {
            let ui = &mut app.ui;
            let term = &mut app.de.terms[t];
            for c in ui.chars.clone() {
                term.input.push(c);
            }
            if let Some(p) = &ui.paste {
                term.input.push_str(&p.replace('\n', " "));
            }
            if ui.key_back {
                term.input.pop();
            }
            if ui.key_up && !term.sh.history.is_empty() {
                term.hist_idx = term.hist_idx.saturating_sub(1).min(term.sh.history.len() - 1);
                term.input = term.sh.history[term.hist_idx].clone();
            }
            if ui.key_down {
                term.hist_idx = (term.hist_idx + 1).min(term.sh.history.len());
                term.input = term.sh.history.get(term.hist_idx).cloned().unwrap_or_default();
            }
            if ui.ctrl && is_key_pressed(KeyCode::L) {
                term.buf.clear();
                term.input.clear();
            }
            if ui.ctrl && is_key_pressed(KeyCode::C) {
                let p = term.sh.prompt(app.de.os.as_ref().unwrap());
                term.buf.push_str(&format!("{}{}^C\n", p, term.input));
                term.input.clear();
            }
            if ui.key_enter {
                enter = true;
            }
        }
        if app.ui.key_tab {
            complete(app, t);
        }
        if enter {
            let line = app.de.terms[t].input.clone();
            app.de.terms[t].input.clear();
            exec_line(app, t, &line);
        }
    }
    // texto: buffer + linha de prompt
    let prompt = app.de.os.as_ref().map(|o| app.de.terms[t].sh.prompt(o)).unwrap_or_default();
    let cursor = if focused && (get_time() * 2.0) as i64 % 2 == 0 { "█" } else { " " };
    let full = format!("{}{}{}{}", app.de.terms[t].buf, prompt, app.de.terms[t].input, cursor);
    let size = 12.5;
    let lines = app.ui.wrap(&full, size, F::Mono, body.w - 12.0 * s);
    let lh = app.ui.s(size * 1.32);
    let visible = (body.h / lh).floor().max(1.0) as usize;
    let maxs = lines.len().saturating_sub(visible) as f32;
    let term = &mut app.de.terms[t];
    if is_top && app.ui.hover(body) && app.ui.wheel != 0.0 {
        term.scroll = (term.scroll.min(maxs) - app.ui.wheel.signum() * 3.0).clamp(0.0, maxs);
    }
    if focused && (app.ui.key_enter || !app.ui.chars.is_empty()) {
        term.scroll = maxs;
    }
    term.scroll = term.scroll.clamp(0.0, maxs);
    let start = term.scroll as usize;
    for (k, l) in lines.iter().skip(start).take(visible).enumerate() {
        let col = if l.starts_with("perito@") { Color::from_rgba(120, 220, 140, 255) } else { Color::from_rgba(220, 224, 228, 255) };
        app.ui.text(l, body.x + 4.0 * s, body.y + lh * (k as f32 + 0.85), size, F::Mono, col);
    }
}

pub fn exec_line(app: &mut App, t: usize, line: &str) {
    let App { sim, de, cmdlog, .. } = app;
    let (Some(sim), Some(os)) = (sim.as_mut(), de.os.as_mut()) else { return };
    let term = &mut de.terms[t];
    let prompt = term.sh.prompt(os);
    term.buf.push_str(&format!("{}{}\n", prompt, line));
    let cwd = term.sh.cwd.clone();
    let out = run_line(os, sim, &mut term.sh, line);
    if !line.trim().is_empty() {
        sim.os_shell_audit(line.trim(), if term.sh.last_code == 0 { "ok" } else { "código de saída diferente de zero" });
        cmdlog.push(format!("\u{1}sh {}\t{}", cwd, line));
    }
    term.buf.push_str(&out);
    term.scroll = f32::MAX;
    term.hist_idx = term.sh.history.len();
    if term.sh.clear_requested {
        term.sh.clear_requested = false;
        term.buf.clear();
    }
    // limita o histórico de tela
    if term.buf.len() > 200_000 {
        let cut = term.buf.len() - 150_000;
        let cut = term.buf[cut..].find('\n').map(|k| cut + k + 1).unwrap_or(cut);
        term.buf.drain(..cut);
    }
    let exit = term.sh.exit_requested;
    let _ = crate::save::save_current(sim, cmdlog);
    if exit {
        if let Some(i) = de.wins.iter().position(|w| w.kind == WinKind::Terminal(t)) {
            de.wins.remove(i);
        }
        de.terms[t].sh.exit_requested = false;
    }
    if line.trim_start().starts_with("pericia finalizar") || line.trim_start().starts_with("pericia avaliar") {
        let txt = pericia_core::assess::render(sim, &pericia_core::assess::evaluate(sim));
        app.finish_case(txt);
    }
}

/// Completa com Tab: comando (primeira palavra) ou caminho (demais).
fn complete(app: &mut App, t: usize) {
    let App { sim, de, .. } = app;
    let (Some(sim), Some(os)) = (sim.as_ref(), de.os.as_ref()) else { return };
    let term = &mut de.terms[t];
    let input = term.input.clone();
    let (head, last) = match input.rfind(' ') {
        Some(k) => (input[..=k].to_string(), input[k + 1..].to_string()),
        None => (String::new(), input.clone()),
    };
    let candidates: Vec<String> = if head.trim().is_empty() || head.trim() == "sudo" {
        os::cmds::names().into_iter().filter(|c| c.starts_with(&last)).map(|c| format!("{} ", c)).collect()
    } else {
        let expanded = if let Some(r) = last.strip_prefix('~') { format!("{}{}", term.sh.env.get("HOME").cloned().unwrap_or_default(), r) } else { last.clone() };
        let (dir, pre) = match expanded.rfind('/') {
            Some(k) => (expanded[..=k].to_string(), expanded[k + 1..].to_string()),
            None => (String::new(), expanded.clone()),
        };
        let abs_dir = if dir.is_empty() { term.sh.cwd.clone() } else { pericia_core::os::shell::abs(&term.sh, &dir) };
        let shown_dir = match last.rfind('/') {
            Some(k) => last[..=k].to_string(),
            None => String::new(),
        };
        os.list(sim, &abs_dir)
            .unwrap_or_default()
            .into_iter()
            .filter(|(n, _)| n.starts_with(&pre) && (!n.starts_with('.') || pre.starts_with('.')))
            .map(|(n, k)| {
                let esc = n.replace(' ', "\\ ");
                format!("{}{}{}", shown_dir, esc, if k.is_dir() { "/" } else { " " })
            })
            .collect()
    };
    if candidates.len() == 1 {
        term.input = format!("{}{}", head, candidates[0]);
    } else if candidates.len() > 1 {
        // prefixo comum + lista
        let mut common = candidates[0].clone();
        for c in &candidates[1..] {
            while !c.starts_with(&common) {
                common.pop();
            }
        }
        if common.len() > last.len() {
            term.input = format!("{}{}", head, common);
        } else {
            let p = term.sh.prompt(os);
            term.buf.push_str(&format!("{}{}\n{}\n", p, input, pericia_core::os::cmds::columns(&candidates.iter().map(|c| c.trim_end().rsplit('/').next().unwrap_or(c).to_string()).collect::<Vec<_>>(), 90).trim_end()));
        }
    }
}

// --------------------------------------------------------------------- arquivos

fn files(app: &mut App, body: Rect) {
    let s = app.ui.scale;
    let side_w = 170.0 * s;
    let side = Rect::new(body.x, body.y, side_w, body.h);
    draw_rectangle(side.x, side.y, side.w, side.h, Color::from_rgba(26, 29, 34, 255));
    let case_dir = app.de.os.as_ref().map(|o| o.case_dir.clone()).unwrap_or_default();
    let mut places: Vec<(String, String)> = vec![
        ("Pasta pessoal".into(), "/home/perito".into()),
        ("Área de Trabalho".into(), "/home/perito/Área de Trabalho".into()),
        ("Documentos".into(), "/home/perito/Documentos".into()),
        ("Encaminhados".into(), "/home/perito/Encaminhados".into()),
        ("Caso atual".into(), case_dir),
        ("Sistema de arquivos".into(), "/".into()),
    ];
    if let Some(o) = &app.de.os {
        for m in &o.mounts {
            places.push((format!("⏏ {}", os::basename(&m.point)), m.point.clone()));
        }
    }
    let mut y = side.y + 8.0 * s;
    for (label, path) in places {
        let r = Rect::new(side.x + 4.0 * s, y, side.w - 8.0 * s, 26.0 * s);
        let active = app.de.files_cwd == path;
        if active {
            draw_rectangle(r.x, r.y, r.w, r.h, T.accent2);
        } else if app.ui.hover(r) {
            draw_rectangle(r.x, r.y, r.w, r.h, T.panel2);
        }
        app.ui.text(&label, r.x + 8.0 * s, r.y + 18.0 * s, 12.0, F::Reg, T.text);
        if app.ui.click_in(r) {
            app.de.files_cwd = path;
            app.de.files_sel = None;
            app.de.files_scroll = 0.0;
        }
        y += 28.0 * s;
    }
    let main = Rect::new(body.x + side_w + 6.0 * s, body.y, body.w - side_w - 6.0 * s, body.h);
    // barra de caminho
    let bar = Rect::new(main.x, main.y, main.w, 28.0 * s);
    app.ui.frame(bar, T.term_bg, T.border);
    let up = Rect::new(bar.x + 2.0 * s, bar.y + 2.0 * s, 28.0 * s, bar.h - 4.0 * s);
    if app.ui.button(up, "↑") && app.de.files_cwd != "/" {
        app.de.files_cwd = os::dirname(&app.de.files_cwd);
        app.de.files_sel = None;
    }
    app.ui.text(&app.de.files_cwd.clone(), bar.x + 40.0 * s, bar.y + 19.0 * s, 12.0, F::Mono, T.text);
    let term_b = Rect::new(bar.x + bar.w - 150.0 * s, bar.y + 2.0 * s, 148.0 * s, bar.h - 4.0 * s);
    if app.ui.button(term_b, "Abrir terminal aqui") {
        let cwd = app.de.files_cwd.clone();
        open(app, WinKind::Terminal(0));
        if let Some(t) = app.de.terms.last_mut() {
            t.sh.cwd = cwd;
        }
    }
    let list_r = Rect::new(main.x, bar.y + bar.h + 4.0 * s, main.w, main.h - bar.h - 30.0 * s);
    app.ui.frame(list_r, T.term_bg, T.border);
    let entries: Vec<(String, Kind)> = match (&app.de.os, &app.sim) {
        (Some(o), Some(sim)) => match o.list(sim, &app.de.files_cwd) {
            Ok(v) => v.into_iter().filter(|(n, _)| !n.starts_with('.')).collect(),
            Err(e) => {
                app.ui.text(&format!("Não foi possível exibir: {}", e), list_r.x + 10.0 * s, list_r.y + 22.0 * s, 12.0, F::Reg, T.bad);
                vec![]
            }
        },
        _ => vec![],
    };
    let mut sorted = entries;
    sorted.sort_by(|a, b| b.1.is_dir().cmp(&a.1.is_dir()).then(a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    let rh = 24.0 * s;
    let visible = ((list_r.h - 26.0 * s) / rh) as usize;
    let maxs = sorted.len().saturating_sub(visible) as f32;
    if app.ui.hover(list_r) && app.ui.wheel != 0.0 {
        app.de.files_scroll = (app.de.files_scroll - app.ui.wheel.signum() * 3.0).clamp(0.0, maxs);
    }
    app.de.files_scroll = app.de.files_scroll.clamp(0.0, maxs);
    // cabeçalho
    app.ui.text("Nome", list_r.x + 34.0 * s, list_r.y + 17.0 * s, 11.0, F::Bold, T.dim);
    app.ui.text("Tamanho", list_r.x + list_r.w * 0.62, list_r.y + 17.0 * s, 11.0, F::Bold, T.dim);
    app.ui.text("Modificado", list_r.x + list_r.w * 0.76, list_r.y + 17.0 * s, 11.0, F::Bold, T.dim);
    let mut open_path: Option<(String, bool)> = None;
    for (k, (name, kind)) in sorted.iter().skip(app.de.files_scroll as usize).take(visible).enumerate() {
        let r = Rect::new(list_r.x + 2.0 * s, list_r.y + 24.0 * s + k as f32 * rh, list_r.w - 4.0 * s, rh);
        let path = if app.de.files_cwd == "/" { format!("/{}", name) } else { format!("{}/{}", app.de.files_cwd, name) };
        let sel = app.de.files_sel.as_deref() == Some(path.as_str());
        if sel {
            draw_rectangle(r.x, r.y, r.w, r.h, T.accent2);
        } else if app.ui.hover(r) {
            draw_rectangle(r.x, r.y, r.w, r.h, T.panel2);
        }
        let ic = match kind {
            Kind::Dir | Kind::MDir { .. } => "folder",
            Kind::Image { .. } | Kind::Dev { .. } => "drive",
            _ => "doc",
        };
        icon(&app.ui, ic, Rect::new(r.x + 6.0 * s, r.y + 3.0 * s, 18.0 * s, 18.0 * s));
        app.ui.text(name, r.x + 32.0 * s, r.y + rh * 0.7, 12.0, F::Reg, T.text);
        let (size, mt) = match kind {
            Kind::File { size, mtime, .. } | Kind::MFile { size, mtime, .. } | Kind::Image { size, mtime, .. } => (pericia_core::util::fmt_bytes(*size), pericia_core::util::fmt_dt(*mtime, pericia_core::util::BRT)),
            _ => ("—".into(), String::new()),
        };
        app.ui.text(&size, list_r.x + list_r.w * 0.62, r.y + rh * 0.7, 11.5, F::Reg, T.dim);
        app.ui.text(&mt, list_r.x + list_r.w * 0.76, r.y + rh * 0.7, 11.5, F::Reg, T.dim);
        if app.ui.click_in(r) {
            let now = get_time();
            if app.de.last_click.1 == path && now - app.de.last_click.0 < 0.45 {
                open_path = Some((path.clone(), kind.is_dir()));
            }
            app.de.last_click = (now, path.clone());
            app.de.files_sel = Some(path);
        }
    }
    let status = format!("{} itens  ·  duplo clique abre  ·  volumes de evidência são somente leitura", sorted.len());
    app.ui.text(&status, main.x + 4.0 * s, main.y + main.h - 8.0 * s, 11.0, F::Reg, T.dim);
    if let Some((p, is_dir)) = open_path {
        if is_dir {
            app.de.files_cwd = p;
            app.de.files_sel = None;
            app.de.files_scroll = 0.0;
        } else {
            open_in_editor(app, &p);
        }
    }
}

pub fn open_in_editor(app: &mut App, path: &str) {
    let (Some(o), Some(sim)) = (app.de.os.as_mut(), app.sim.as_mut()) else { return };
    let kind = o.kind(sim, path);
    match kind {
        Some(Kind::Image { .. }) | Some(Kind::Dev { .. }) => {
            app.de.editor_msg = "Imagens e dispositivos não são abertos no editor: monte com 'mount -o ro,loop'.".into();
            app.de.editor_path = path.to_string();
            app.de.editor_buf.clear();
            app.de.editor_ro = true;
        }
        _ => match o.read(sim, path) {
            Ok(d) => {
                let ro = o.mount_for(path).is_some() || !path.starts_with("/home/perito/") || path.contains("/Encaminhados/");
                app.de.editor_path = path.to_string();
                app.de.editor_buf = d.clone();
                app.de.editor_orig = d;
                app.de.editor_ro = ro;
                app.de.editor_msg = if ro { "Somente leitura (evidência, material encaminhado ou arquivo do sistema).".into() } else { String::new() };
                app.de.editor_scroll = 0.0;
                let _ = crate::save::save_current(sim, &app.cmdlog);
            }
            Err(e) => {
                app.de.editor_msg = format!("Não foi possível abrir {}: {}", path, e);
            }
        },
    }
    open(app, WinKind::Editor);
}

fn editor(app: &mut App, body: Rect) {
    let s = app.ui.scale;
    let bar = Rect::new(body.x, body.y, body.w, 28.0 * s);
    let dirty = app.de.editor_buf != app.de.editor_orig;
    let title = if app.de.editor_path.is_empty() { "(sem arquivo)".to_string() } else { format!("{}{}", app.de.editor_path, if dirty { " •" } else { "" }) };
    app.ui.text(&title, bar.x + 4.0 * s, bar.y + 18.0 * s, 12.0, F::Mono, T.text);
    let save_r = Rect::new(bar.x + bar.w - 90.0 * s, bar.y, 90.0 * s, bar.h - 2.0 * s);
    let can_save = !app.de.editor_ro && !app.de.editor_path.is_empty() && dirty;
    if app.ui.button_c(save_r, "Salvar", T.accent2, can_save) || (can_save && app.ui.ctrl && is_key_pressed(KeyCode::S)) {
        let App { sim, de, cmdlog, .. } = app;
        if let (Some(sim), Some(o)) = (sim.as_mut(), de.os.as_mut()) {
            match o.write(sim, &de.editor_path, &de.editor_buf, false, "editado no Editor de texto") {
                Ok(()) => {
                    de.editor_orig = de.editor_buf.clone();
                    de.editor_msg = "Salvo.".into();
                    sim.rec_pub("editor-salvar", &de.editor_path, "arquivo de trabalho salvo");
                    cmdlog.push(format!("\u{1}wr {}\t{}", de.editor_path, de.editor_buf.replace('\\', "\\\\").replace('\n', "\\n")));
                    let _ = crate::save::save_current(sim, cmdlog);
                }
                Err(e) => de.editor_msg = format!("Erro ao salvar: {}", e),
            }
        }
    }
    let area = Rect::new(body.x, body.y + bar.h + 2.0 * s, body.w, body.h - bar.h - 24.0 * s);
    if app.de.editor_ro {
        let t = app.de.editor_buf.clone();
        let mut sc = app.de.editor_scroll;
        app.ui.scroll_text(area, &t, &mut sc, 12.0, F::Mono, T.text, false);
        app.de.editor_scroll = sc;
    } else {
        let mut b = std::mem::take(&mut app.de.editor_buf);
        let mut sc = app.de.editor_scroll;
        app.ui.text_area("editor", area, &mut b, &mut sc);
        app.de.editor_buf = b;
        app.de.editor_scroll = sc;
    }
    let msg = app.de.editor_msg.clone();
    app.ui.text(&msg, body.x + 4.0 * s, body.y + body.h - 6.0 * s, 11.0, F::Reg, T.dim);
}

// --------------------------------------------------------------------- miniatura para o monitor 3D

/// Captura a área de trabalho já desenhada na tela (sem renderizar texto fora da tela,
/// o que corromperia o cache de glifos) para exibir no monitor da cena 3D.
pub fn capture(app: &mut App, area: Rect) {
    let img = get_screen_data();
    let sh = screen_height();
    let y = (sh - area.y - area.h).max(0.0);
    let sub = img.sub_image(Rect::new(area.x.max(0.0), y, area.w.min(img.width as f32), area.h.min(img.height as f32)));
    let tex = Texture2D::from_image(&sub);
    tex.set_filter(FilterMode::Linear);
    app.de.snapshot = Some(tex);
}
