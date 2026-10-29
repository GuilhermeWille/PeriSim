//! Tela do aparelho celular (sistema "FoneOS", fictício). Desenha o aparelho em mãos,
//! sobre a cena, com o que o motor diz que a tela mostra (`pericia_core::phone::view`).
//!
//! Não há teclado de desbloqueio: a tela bloqueada só mostra relógio, estado e o atalho
//! de emergência (informações de emergência e o código de serviço *#06# do IMEI).
//! Cada aplicativo aberto vira o comando `tela <item> <app>`, registrado na auditoria.

use crate::ui::*;
use crate::App;
use macroquad::prelude::*;
use pericia_core::phone::{view, PhoneView, Screen, APPS};

#[derive(Default)]
pub struct PhoneUi {
    pub open: Option<String>,
    pub app: String,
    pub sub: Option<usize>,
    pub scroll: usize,
    pub emergency: bool,
    pub tab: u8,
}

impl PhoneUi {
    pub fn close(&mut self) {
        *self = PhoneUi::default();
    }
}

struct Ctx {
    sr: Rect,
    u: f32,
    scale: f32,
}

impl Ctx {
    fn x(&self, v: f32) -> f32 {
        self.sr.x + v * self.u
    }
    fn y(&self, v: f32) -> f32 {
        self.sr.y + v * self.u
    }
    fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        rect(self.x(x), self.y(y), w * self.u, h * self.u)
    }
    /// tamanho de fonte (em unidades da tela) para `Ui::text`, que aplica a escala da interface
    fn fs(&self, px: f32) -> f32 {
        px * self.u / self.scale
    }
}

fn rrect(r: Rect, rad: f32, c: Color) {
    // sem sobreposição de primitivas, para que cores translúcidas fiquem uniformes
    let rad = rad.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    draw_rectangle(r.x + rad, r.y, r.w - 2.0 * rad, r.h, c);
    draw_rectangle(r.x, r.y + rad, rad, r.h - 2.0 * rad, c);
    draw_rectangle(r.x + r.w - rad, r.y + rad, rad, r.h - 2.0 * rad, c);
    let corners = [
        (vec2(r.x + rad, r.y + rad), std::f32::consts::PI),
        (vec2(r.x + r.w - rad, r.y + rad), 1.5 * std::f32::consts::PI),
        (vec2(r.x + r.w - rad, r.y + r.h - rad), 0.0),
        (vec2(r.x + rad, r.y + r.h - rad), 0.5 * std::f32::consts::PI),
    ];
    let n = 10;
    for (o, a0) in corners {
        for i in 0..n {
            let a = a0 + std::f32::consts::FRAC_PI_2 * i as f32 / n as f32;
            let b = a0 + std::f32::consts::FRAC_PI_2 * (i + 1) as f32 / n as f32;
            draw_triangle(o, o + vec2(a.cos(), a.sin()) * rad, o + vec2(b.cos(), b.sin()) * rad, c);
        }
    }
}

/// Pinta, na cor da moldura, o que sobra fora dos cantos arredondados da tela.
fn corner_mask(r: Rect, rad: f32, c: Color) {
    let corners = [
        (vec2(r.x + rad, r.y + rad), vec2(r.x, r.y), std::f32::consts::PI),
        (vec2(r.x + r.w - rad, r.y + rad), vec2(r.x + r.w, r.y), 1.5 * std::f32::consts::PI),
        (vec2(r.x + r.w - rad, r.y + r.h - rad), vec2(r.x + r.w, r.y + r.h), 0.0),
        (vec2(r.x + rad, r.y + r.h - rad), vec2(r.x, r.y + r.h), 0.5 * std::f32::consts::PI),
    ];
    let n = 12;
    for (o, corner, a0) in corners {
        for i in 0..n {
            let a = a0 + std::f32::consts::FRAC_PI_2 * i as f32 / n as f32;
            let b = a0 + std::f32::consts::FRAC_PI_2 * (i + 1) as f32 / n as f32;
            draw_triangle(corner, o + vec2(a.cos(), a.sin()) * rad, o + vec2(b.cos(), b.sin()) * rad, c);
        }
    }
}

fn hex(v: u32) -> Color {
    Color::from_rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255)
}

fn gradient(r: Rect, top: Color, bottom: Color) {
    let n = 48;
    for k in 0..n {
        let t = k as f32 / (n - 1) as f32;
        let c = Color::new(top.r + (bottom.r - top.r) * t, top.g + (bottom.g - top.g) * t, top.b + (bottom.b - top.b) * t, 1.0);
        draw_rectangle(r.x, r.y + r.h * k as f32 / n as f32, r.w, r.h / n as f32 + 1.0, c);
    }
}

fn clip_text(ui: &Ui, s: &str, size: f32, f: F, maxw: f32) -> String {
    if ui.measure(s, size, f) <= maxw {
        return s.to_string();
    }
    let mut l: String = s.to_string();
    while l.chars().count() > 1 && ui.measure(&format!("{}…", l), size, f) > maxw {
        l.pop();
    }
    format!("{}…", l)
}

const W: f32 = 360.0;
const H: f32 = 760.0;
const NAV: f32 = 44.0; // barra de navegação

/// Desenha o aparelho e trata a interação. Chamado por cima da área de hardware.
pub fn draw(app: &mut App, area: Rect) {
    let Some(id) = app.phone.open.clone() else { return };
    let Some(v) = app.sim.as_ref().and_then(|s| view(s, &id)) else {
        app.phone.close();
        return;
    };
    let s = app.ui.scale;
    // escurece a cena: o aparelho está nas mãos do perito
    draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.0, 0.0, 0.0, 0.62));
    let ph_h = (area.h - 30.0 * s).min(860.0 * s);
    let ph_w = ph_h * (W + 26.0) / (H + 34.0);
    let body = rect(area.x + area.w * 0.5 - ph_w * 0.5 - 120.0 * s, area.y + (area.h - ph_h) / 2.0, ph_w, ph_h);
    let u = (body.w - 26.0 * body.w / (W + 26.0)) / W;
    let bez = 13.0 * u;
    let sr = rect(body.x + bez, body.y + 17.0 * u, W * u, H * u);
    let c = Ctx { sr, u, scale: s };
    // corpo, botões laterais e moldura
    rrect(rect(body.x - 3.0 * u, body.y + 120.0 * u, 6.0 * u, 50.0 * u), 2.0 * u, hex(0x2a2c30));
    rrect(rect(body.x + body.w - 3.0 * u, body.y + 150.0 * u, 6.0 * u, 80.0 * u), 2.0 * u, hex(0x2a2c30));
    rrect(body, 38.0 * u, hex(0x1d1f23));
    rrect(rect(body.x + 2.0 * u, body.y + 2.0 * u, body.w - 4.0 * u, body.h - 4.0 * u), 36.0 * u, hex(0x0b0c0e));
    // tela
    let screen_bg = match v.screen {
        Screen::Off => hex(0x050506),
        _ => hex(0x101418),
    };
    rrect(sr, 26.0 * u, screen_bg);
    let mut acted: Option<String> = None;
    match v.screen {
        Screen::Off => {
            // reflexo sutil no vidro
            draw_triangle(vec2(sr.x, sr.y + sr.h * 0.1), vec2(sr.x + sr.w * 0.7, sr.y), vec2(sr.x, sr.y + sr.h * 0.5), Color::new(1.0, 1.0, 1.0, 0.025));
        }
        Screen::Setup => setup(app, &c),
        Screen::Locked { afu } => locked(app, &c, &v, afu, &mut acted),
        Screen::Home => {
            if app.phone.app.is_empty() {
                home(app, &c, &v);
            } else {
                app_screen(app, &c, &v, &mut acted);
            }
        }
    }
    if v.screen != Screen::Off {
        status_bar(app, &c, &v);
        nav_bar(app, &c, &v);
    }
    corner_mask(sr, 26.0 * u, hex(0x0b0c0e));
    // câmera frontal (furo) e etiqueta sobre ela (POP 5.05)
    let cam = vec2(c.x(W / 2.0), c.y(14.0));
    draw_circle(cam.x, cam.y, 5.5 * u, hex(0x050505));
    draw_circle(cam.x + 1.5 * u, cam.y - 1.5 * u, 1.6 * u, hex(0x1b2a3a));
    if v.camera_covered {
        rrect(rect(cam.x - 16.0 * u, cam.y - 9.0 * u, 32.0 * u, 18.0 * u), 3.0 * u, hex(0xf2efe6));
        app.ui.text("PERÍCIA", cam.x - 13.0 * u, cam.y + 3.0 * u, c.fs(7.0), F::Bold, hex(0x8a2a20));
    }
    if v.screen_broken {
        cracks(&c);
    }
    // painel lateral de apoio (fora do aparelho)
    side(app, &v, rect(body.x + body.w + 30.0 * s, body.y + 20.0 * s, (area.x + area.w - (body.x + body.w + 30.0 * s) - 20.0 * s).min(360.0 * s), body.h - 40.0 * s), &mut acted);
    if app.ui.key_esc {
        app.phone.close();
    }
    if let Some(cmd) = acted {
        let out = app.run(&cmd);
        let first = out.lines().next().unwrap_or("").to_string();
        app.toast(&first);
    }
}

fn cracks(c: &Ctx) {
    let col = Color::new(1.0, 1.0, 1.0, 0.35);
    let o = vec2(c.x(250.0), c.y(560.0));
    let rays = [(-200.0, -260.0), (90.0, -330.0), (110.0, 170.0), (-150.0, 190.0), (-250.0, -40.0), (60.0, 40.0)];
    for (k, (dx, dy)) in rays.iter().enumerate() {
        let mut p = o;
        for seg in 1..=4 {
            let t = seg as f32 / 4.0;
            let wob = ((k * 7 + seg * 3) % 5) as f32 - 2.0;
            let q = vec2(o.x + dx * t * c.u + wob * 6.0 * c.u, o.y + dy * t * c.u - wob * 4.0 * c.u);
            let q = vec2(q.x.clamp(c.sr.x, c.sr.x + c.sr.w), q.y.clamp(c.sr.y, c.sr.y + c.sr.h));
            draw_line(p.x, p.y, q.x, q.y, 1.2, col);
            p = q;
        }
    }
    draw_circle_lines(o.x, o.y, 9.0 * c.u, 1.0, col);
}

fn status_bar(app: &mut App, c: &Ctx, v: &PhoneView) {
    let fg = WHITE;
    app.ui.text(&v.time, c.x(22.0), c.y(20.0), c.fs(13.0), F::Bold, fg);
    // bateria
    let bx = c.x(W - 50.0);
    let by = c.y(10.0);
    draw_rectangle_lines(bx, by, 24.0 * c.u, 12.0 * c.u, 1.5, fg);
    draw_rectangle(bx + 24.0 * c.u, by + 3.5 * c.u, 2.0 * c.u, 5.0 * c.u, fg);
    let lvl = v.battery as f32 / 100.0;
    let bc = if v.battery <= 15 { hex(0xff5a4e) } else if v.charging { hex(0x5ad27a) } else { fg };
    draw_rectangle(bx + 2.0 * c.u, by + 2.0 * c.u, 20.0 * c.u * lvl, 8.0 * c.u, bc);
    app.ui.text(&format!("{}%", v.battery), c.x(W - 88.0), c.y(20.0), c.fs(11.0), F::Reg, fg);
    if v.charging {
        app.ui.text("⚡", c.x(W - 102.0), c.y(20.0), c.fs(11.0), F::Reg, hex(0x5ad27a));
    }
    // rede
    let mut x = W - 128.0;
    if v.airplane {
        app.ui.text("✈", c.x(x), c.y(21.0), c.fs(13.0), F::Reg, fg);
    } else {
        for k in 0..4 {
            let h = 3.0 + k as f32 * 2.5;
            let on = v.cellular;
            draw_rectangle(c.x(x + k as f32 * 4.0), c.y(20.0 - h), 2.6 * c.u, h * c.u, if on { fg } else { Color::new(1.0, 1.0, 1.0, 0.3) });
        }
        if !v.cellular {
            app.ui.text("×", c.x(x + 14.0), c.y(20.0), c.fs(10.0), F::Bold, fg);
        }
        x -= 22.0;
        if v.wifi {
            for k in 0..3 {
                let rr = (3.5 + k as f32 * 3.5) * c.u;
                draw_arc(c.x(x + 6.0), c.y(19.0), 16, rr, 225.0, 1.6, 90.0, fg);
            }
            draw_circle(c.x(x + 6.0), c.y(18.5), 1.5 * c.u, fg);
        }
    }
}

fn nav_bar(app: &mut App, c: &Ctx, v: &PhoneView) {
    let y = H - NAV;
    let home = matches!(v.screen, Screen::Home);
    let light_app = home && !app.phone.app.is_empty() && !(app.phone.app == "galeria" && app.phone.sub.is_some()) && app.phone.app != "cofre";
    let col = if light_app { Color::new(0.1, 0.12, 0.16, 0.8) } else { Color::new(1.0, 1.0, 1.0, 0.85) };
    // gestos/botões: voltar, início
    let back = c.r(70.0, y + 6.0, 60.0, 32.0);
    let hm = c.r(150.0, y + 6.0, 60.0, 32.0);
    app.ui.text("◁", back.x + 22.0 * c.u, back.y + 22.0 * c.u, c.fs(16.0), F::Reg, col);
    draw_circle_lines(hm.center().x, hm.center().y, 8.0 * c.u, 2.0, col);
    let rec = c.r(230.0, y + 6.0, 60.0, 32.0);
    draw_rectangle_lines(rec.center().x - 7.0 * c.u, rec.center().y - 7.0 * c.u, 14.0 * c.u, 14.0 * c.u, 2.0, col);
    if app.ui.click_in(back) {
        if app.phone.sub.is_some() {
            app.phone.sub = None;
            app.phone.scroll = 0;
        } else if app.phone.emergency {
            app.phone.emergency = false;
        } else if home && !app.phone.app.is_empty() {
            app.phone.app.clear();
            app.phone.scroll = 0;
        }
    }
    if app.ui.click_in(hm) {
        app.phone.app.clear();
        app.phone.sub = None;
        app.phone.emergency = false;
        app.phone.scroll = 0;
    }
    let _ = app.ui.click_in(rec);
}

fn wallpaper(c: &Ctx) {
    let r = c.sr;
    gradient(r, hex(0x1b3b5f), hex(0x0d1b2a));
    // ondas abstratas
    for k in 0..5 {
        let y0 = r.y + r.h * (0.45 + k as f32 * 0.09);
        let col = Color::new(0.35 + k as f32 * 0.08, 0.55, 0.85, 0.08);
        for i in 0..36 {
            let x0 = r.x + r.w * i as f32 / 36.0;
            let x1 = r.x + r.w * (i + 1) as f32 / 36.0;
            let f = |x: f32| y0 + ((x - r.x) / r.w * 6.3 + k as f32).sin() * 22.0 * c.u;
            draw_triangle(vec2(x0, f(x0)), vec2(x1, f(x1)), vec2(x0, r.y + r.h), col);
            draw_triangle(vec2(x1, f(x1)), vec2(x1, r.y + r.h), vec2(x0, r.y + r.h), col);
        }
    }
}

fn setup(app: &mut App, c: &Ctx) {
    gradient(c.sr, hex(0xf4f6fa), hex(0xdfe6f0));
    let dark = hex(0x1d2733);
    app.ui.text("Olá", c.x(40.0), c.y(250.0), c.fs(46.0), F::Bold, dark);
    app.ui.text("Bem-vindo(a) ao seu novo aparelho", c.x(40.0), c.y(290.0), c.fs(15.0), F::Reg, dark);
    rrect(c.r(40.0, 330.0, 280.0, 44.0), 10.0 * c.u, WHITE);
    app.ui.text("Português (Brasil)  ▾", c.x(56.0), c.y(358.0), c.fs(14.0), F::Reg, dark);
    rrect(c.r(200.0, 640.0, 120.0, 44.0), 22.0 * c.u, hex(0x2f6fd6));
    app.ui.text("Iniciar", c.x(234.0), c.y(668.0), c.fs(15.0), F::Bold, WHITE);
    app.ui.text("Restaurado de fábrica: nenhum dado de usuário.", c.x(40.0), c.y(600.0), c.fs(11.0), F::Reg, hex(0x7a8696));
}

fn padlock(c: &Ctx, cx: f32, cy: f32, col: Color) {
    let u = c.u;
    draw_circle_lines(cx, cy - 6.0 * u, 7.0 * u, 2.4, col);
    draw_rectangle(cx - 10.0 * u, cy - 4.0 * u, 20.0 * u, 15.0 * u, col);
    draw_circle(cx, cy + 3.0 * u, 2.2 * u, hex(0x1b3b5f));
}

fn locked(app: &mut App, c: &Ctx, v: &PhoneView, afu: bool, acted: &mut Option<String>) {
    wallpaper(c);
    let white = WHITE;
    let dim = Color::new(1.0, 1.0, 1.0, 0.75);
    if app.phone.emergency {
        rrect(c.sr, 26.0 * c.u, Color::new(0.04, 0.05, 0.07, 1.0));
        app.ui.text("Emergência", c.x(24.0), c.y(80.0), c.fs(24.0), F::Bold, white);
        rrect(c.r(20.0, 110.0, 320.0, 150.0), 14.0 * c.u, hex(0x1f2630));
        app.ui.text("Informações de emergência", c.x(36.0), c.y(140.0), c.fs(14.0), F::Bold, white);
        app.ui.text("Proprietário: não informado", c.x(36.0), c.y(170.0), c.fs(12.0), F::Reg, dim);
        app.ui.text("Contatos de emergência: nenhum", c.x(36.0), c.y(192.0), c.fs(12.0), F::Reg, dim);
        app.ui.text("Informações médicas: nenhuma", c.x(36.0), c.y(214.0), c.fs(12.0), F::Reg, dim);
        app.ui.text("(visíveis sem desbloqueio)", c.x(36.0), c.y(240.0), c.fs(10.0), F::Reg, Color::new(1.0, 1.0, 1.0, 0.45));
        let b = c.r(20.0, 290.0, 320.0, 64.0);
        rrect(b, 14.0 * c.u, if app.ui.hover(b) { hex(0x2c3644) } else { hex(0x222a35) });
        app.ui.text("*#06#", c.x(40.0), c.y(330.0), c.fs(22.0), F::Bold, white);
        app.ui.text("código de serviço — exibe o IMEI", c.x(140.0), c.y(328.0), c.fs(11.5), F::Reg, dim);
        if app.ui.click_in(b) {
            *acted = Some(format!("tela {} imei", v.item.to_lowercase()));
        }
        app.ui.text("O discador de emergência aceita códigos de serviço", c.x(24.0), c.y(400.0), c.fs(10.5), F::Reg, Color::new(1.0, 1.0, 1.0, 0.5));
        app.ui.text("mesmo com a tela bloqueada, na maioria dos aparelhos.", c.x(24.0), c.y(416.0), c.fs(10.5), F::Reg, Color::new(1.0, 1.0, 1.0, 0.5));
        return;
    }
    padlock(c, c.x(W / 2.0), c.y(72.0), white);
    let tw = app.ui.measure(&v.time, c.fs(76.0), F::Reg);
    app.ui.text(&v.time, c.sr.x + (c.sr.w - tw) / 2.0, c.y(190.0), c.fs(76.0), F::Reg, white);
    let dw = app.ui.measure(&v.date, c.fs(15.0), F::Reg);
    app.ui.text(&v.date, c.sr.x + (c.sr.w - dw) / 2.0, c.y(222.0), c.fs(15.0), F::Reg, dim);
    // notificações: em AFU contadas, conteúdo oculto; em BFU, nada
    let card = c.r(20.0, 270.0, 320.0, if afu { 92.0 } else { 70.0 });
    rrect(card, 16.0 * c.u, Color::new(1.0, 1.0, 1.0, 0.14));
    if afu {
        let n = v.notifications;
        app.ui.text(if n == 1 { "1 notificação".to_string() } else { format!("{} notificações", n) }.as_str(), c.x(36.0), c.y(300.0), c.fs(14.0), F::Bold, white);
        app.ui.text("Conteúdo oculto até o desbloqueio", c.x(36.0), c.y(324.0), c.fs(12.0), F::Reg, dim);
        app.ui.text("Mensagens · Mensageiro", c.x(36.0), c.y(346.0), c.fs(11.0), F::Reg, Color::new(1.0, 1.0, 1.0, 0.55));
    } else {
        app.ui.text("Reiniciado", c.x(36.0), c.y(300.0), c.fs(14.0), F::Bold, white);
        app.ui.text("Algumas funções exigem o primeiro desbloqueio", c.x(36.0), c.y(324.0), c.fs(11.5), F::Reg, dim);
    }
    if v.alarms_disabled {
        app.ui.text("Alarmes: nenhum", c.x(36.0), c.y(400.0), c.fs(11.0), F::Reg, Color::new(1.0, 1.0, 1.0, 0.5));
    } else if let Some(a) = v.alarms.first() {
        draw_circle_lines(c.x(42.0), c.y(396.0), 6.0 * c.u, 1.5, dim);
        draw_line(c.x(42.0), c.y(396.0), c.x(42.0), c.y(392.0), 1.5, dim);
        draw_line(c.x(42.0), c.y(396.0), c.x(45.0), c.y(396.0), 1.5, dim);
        app.ui.text(&format!("Alarme {}", a), c.x(54.0), c.y(400.0), c.fs(12.0), F::Reg, dim);
    }
    let em = c.r(24.0, H - NAV - 52.0, 110.0, 34.0);
    rrect(em, 17.0 * c.u, Color::new(1.0, 1.0, 1.0, if app.ui.hover(em) { 0.22 } else { 0.12 }));
    app.ui.text("Emergência", em.x + 18.0 * c.u, em.y + 22.0 * c.u, c.fs(13.0), F::Reg, white);
    if app.ui.click_in(em) {
        app.phone.emergency = true;
    }
    // atalho da câmera (desenhado)
    let cb = c.r(W - 70.0, H - NAV - 52.0, 34.0, 34.0);
    draw_circle(cb.center().x, cb.center().y, 17.0 * c.u, Color::new(0.30, 0.38, 0.50, 1.0));
    draw_rectangle_lines(cb.x + 8.0 * c.u, cb.y + 11.0 * c.u, 18.0 * c.u, 13.0 * c.u, 1.6, dim);
    draw_circle_lines(cb.center().x, cb.y + 17.5 * c.u, 3.5 * c.u, 1.4, dim);
}

/// Ícone de aplicativo: quadrado arredondado com um desenho simples.
fn app_icon(app: &mut App, c: &Ctx, id: &str, r: Rect) {
    let (bg, glyph) = match id {
        "telefone" => (hex(0x34c759), "✆"),
        "mensagens" => (hex(0x2f80ed), "✉"),
        "contatos" => (hex(0xf2994a), "☺"),
        "galeria" => (hex(0xbb6bd9), "▣"),
        "arquivos" => (hex(0xf2c94c), "▤"),
        "relogio" => (hex(0x333a45), "◷"),
        "config" => (hex(0x8a94a6), "⚙"),
        "cofre" => (hex(0x27313e), "◆"),
        _ => (hex(0x555555), "?"),
    };
    rrect(r, r.w * 0.26, bg);
    draw_rectangle(r.x + r.w * 0.2, r.y + 2.0, r.w * 0.6, r.h * 0.18, Color::new(1.0, 1.0, 1.0, 0.07));
    let gs = c.fs(26.0);
    let gw = app.ui.measure(glyph, gs, F::Reg);
    app.ui.text(glyph, r.x + (r.w - gw) / 2.0, r.y + r.h * 0.7, gs, F::Reg, WHITE);
}

fn home(app: &mut App, c: &Ctx, v: &PhoneView) {
    wallpaper(c);
    let white = WHITE;
    // widget de relógio
    app.ui.text(&v.time, c.x(28.0), c.y(110.0), c.fs(52.0), F::Reg, white);
    app.ui.text(&format!("{} · {}", v.date, v.operator), c.x(30.0), c.y(136.0), c.fs(13.0), F::Reg, Color::new(1.0, 1.0, 1.0, 0.8));
    let apps: Vec<&(&str, &str)> = APPS.iter().filter(|(a, _)| *a != "cofre" || v.second_space.is_some()).collect();
    let cols = 4;
    let cell = 80.0;
    for (k, (aid, label)) in apps.iter().enumerate() {
        let (cx, cy) = ((k % cols) as f32, (k / cols) as f32);
        let r = c.r(20.0 + cx * cell + 10.0, 220.0 + cy * 104.0, 60.0, 60.0);
        let hov = app.ui.hover(r);
        app_icon(app, c, aid, r);
        if hov {
            rrect(r, r.w * 0.26, Color::new(1.0, 1.0, 1.0, 0.12));
        }
        let lab = clip_text(&app.ui, label, c.fs(11.0), F::Reg, 76.0 * c.u);
        let lw = app.ui.measure(&lab, c.fs(11.0), F::Reg);
        app.ui.text(&lab, r.center().x - lw / 2.0, r.y + r.h + 17.0 * c.u, c.fs(11.0), F::Reg, white);
        if *aid == "mensagens" && v.notifications > 0 {
            draw_circle(r.x + r.w - 4.0 * c.u, r.y + 4.0 * c.u, 10.0 * c.u, hex(0xff3b30));
            app.ui.text(&v.notifications.to_string(), r.x + r.w - 8.0 * c.u, r.y + 9.0 * c.u, c.fs(11.0), F::Bold, white);
        }
        if app.ui.click_in(r) {
            open_app(app, &v.item, aid);
        }
    }
    // dock
    rrect(c.r(16.0, H - NAV - 96.0, 328.0, 84.0), 26.0 * c.u, Color::new(1.0, 1.0, 1.0, 0.13));
    for (k, aid) in ["telefone", "mensagens", "galeria", "config"].iter().enumerate() {
        let r = c.r(32.0 + k as f32 * 80.0, H - NAV - 84.0, 60.0, 60.0);
        app_icon(app, c, aid, r);
        if app.ui.click_in(r) {
            open_app(app, &v.item, aid);
        }
    }
}

/// Abre um aplicativo: passa pelo motor (auditoria e regras).
fn open_app(app: &mut App, item: &str, aid: &str) {
    let out = app.run(&format!("tela {} {}", item.to_lowercase(), aid));
    if out.starts_with("ERRO") {
        app.toast(out.trim_start_matches("ERRO: "));
        return;
    }
    if out.contains("ATENÇÃO") {
        if let Some(l) = out.lines().find(|l| l.contains("ATENÇÃO")) {
            app.toast(l);
        }
    }
    app.phone.app = aid.to_string();
    app.phone.sub = None;
    app.phone.scroll = 0;
    app.phone.tab = 0;
}

/// Cabeçalho de aplicativo (fundo claro, título).
fn header(app: &mut App, c: &Ctx, title: &str, color: Color) {
    rrect(rect(c.sr.x, c.sr.y, c.sr.w, 60.0 * c.u), 26.0 * c.u, color);
    draw_rectangle(c.sr.x, c.sr.y + 30.0 * c.u, c.sr.w, 66.0 * c.u, color);
    app.ui.text(title, c.x(22.0), c.y(78.0), c.fs(26.0), F::Bold, WHITE);
}

/// Linha clicável em lista; retorna true se clicada.
fn row(app: &mut App, c: &Ctx, y: f32, h: f32) -> bool {
    let r = c.r(0.0, y, W, h);
    if app.ui.hover(r) {
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.05));
    }
    draw_line(c.x(72.0), r.y + r.h, c.x(W), r.y + r.h, 1.0, hex(0xe3e6ea));
    app.ui.click_in(r)
}

fn avatar(app: &mut App, c: &Ctx, cx: f32, cy: f32, name: &str) {
    let h = name.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let palette = [0x2f80ed, 0xeb5757, 0x27ae60, 0x9b51e0, 0xf2994a, 0x219653];
    draw_circle(c.x(cx), c.y(cy), 20.0 * c.u, hex(palette[(h % 6) as usize]));
    let ini: String = name.chars().find(|ch| ch.is_alphabetic()).map(|ch| ch.to_uppercase().collect()).unwrap_or_else(|| "#".into());
    let w = app.ui.measure(&ini, c.fs(16.0), F::Bold);
    app.ui.text(&ini, c.x(cx) - w / 2.0, c.y(cy + 6.0), c.fs(16.0), F::Bold, WHITE);
}

/// Rolagem de lista pela roda do mouse dentro da tela.
fn list_scroll(app: &mut App, c: &Ctx, total: usize, visible: usize) -> usize {
    let maxs = total.saturating_sub(visible);
    if app.ui.hover(c.sr) && app.ui.wheel != 0.0 {
        if app.ui.wheel < 0.0 {
            app.phone.scroll = (app.phone.scroll + 1).min(maxs);
        } else {
            app.phone.scroll = app.phone.scroll.saturating_sub(1);
        }
    }
    app.phone.scroll = app.phone.scroll.min(maxs);
    app.phone.scroll
}

fn app_screen(app: &mut App, c: &Ctx, v: &PhoneView, acted: &mut Option<String>) {
    let light = hex(0xf7f8fa);
    let dark = hex(0x1c2430);
    let mid = hex(0x6b7685);
    rrect(c.sr, 26.0 * c.u, light);
    let body_top = 100.0;
    let body_h = H - NAV - body_top;
    match app.phone.app.as_str() {
        "mensagens" => {
            if let Some(ti) = app.phone.sub.filter(|i| *i < v.threads.len()) {
                let t = &v.threads[ti];
                header(app, c, &clip_text(&app.ui, &t.name, c.fs(22.0), F::Bold, 300.0 * c.u), hex(0x2f80ed));
                app.ui.text(&t.app, c.x(24.0), c.y(94.0), c.fs(10.0), F::Reg, Color::new(1.0, 1.0, 1.0, 0.8));
                let mut y = body_top + 14.0;
                let start = list_scroll(app, c, t.msgs.len(), 6);
                for m in t.msgs.iter().skip(start) {
                    let lines = app.ui.wrap(&m.text, c.fs(13.0), F::Reg, 220.0 * c.u);
                    let bh = 22.0 + lines.len() as f32 * 18.0;
                    if y + bh + 18.0 > H - NAV - 8.0 {
                        break;
                    }
                    let maxw = lines.iter().map(|l| app.ui.measure(l, c.fs(13.0), F::Reg)).fold(0.0, f32::max) / c.u;
                    let bw = (maxw + 28.0).max(90.0);
                    let bx = if m.mine { W - 16.0 - bw } else { 16.0 };
                    rrect(c.r(bx, y, bw, bh), 14.0 * c.u, if m.mine { hex(0x2f80ed) } else { WHITE });
                    if !m.mine {
                        draw_rectangle_lines(c.x(bx), c.y(y), bw * c.u, bh * c.u, 0.6, hex(0xe0e3e8));
                    }
                    for (k, l) in lines.iter().enumerate() {
                        app.ui.text(l, c.x(bx + 14.0), c.y(y + 20.0 + k as f32 * 18.0), c.fs(13.0), F::Reg, if m.mine { WHITE } else { dark });
                    }
                    let meta = if m.time.is_empty() { m.from.clone() } else { format!("{} · {}", if m.mine { "Você" } else { &m.from }, m.time) };
                    let mw = app.ui.measure(&meta, c.fs(9.5), F::Reg) / c.u;
                    let mx = if m.mine { W - 18.0 - mw } else { bx + 4.0 };
                    app.ui.text(&meta, c.x(mx), c.y(y + bh + 13.0), c.fs(9.5), F::Reg, mid);
                    y += bh + 24.0;
                }
                app.ui.text(&format!("origem: {}", t.path), c.x(16.0), c.y(H - NAV - 8.0), c.fs(9.0), F::Mono, mid);
            } else {
                header(app, c, "Mensagens", hex(0x2f80ed));
                if v.threads.is_empty() {
                    app.ui.text("Nenhuma conversa", c.x(24.0), c.y(body_top + 40.0), c.fs(14.0), F::Reg, mid);
                }
                for (k, t) in v.threads.iter().enumerate() {
                    let y = body_top + k as f32 * 74.0;
                    if y + 74.0 > H - NAV {
                        break;
                    }
                    let clicked = row(app, c, y, 74.0);
                    avatar(app, c, 40.0, y + 37.0, &t.name);
                    app.ui.text(&clip_text(&app.ui, &t.name, c.fs(15.0), F::Bold, 200.0 * c.u), c.x(72.0), c.y(y + 30.0), c.fs(15.0), F::Bold, dark);
                    if let Some(last) = t.msgs.last() {
                        let prev = clip_text(&app.ui, &last.text, c.fs(12.5), F::Reg, 230.0 * c.u);
                        app.ui.text(&prev, c.x(72.0), c.y(y + 52.0), c.fs(12.5), if t.unread > 0 { F::Bold } else { F::Reg }, if t.unread > 0 { dark } else { mid });
                        if !last.time.is_empty() {
                            let hh = last.time.get(11..16).unwrap_or("");
                            app.ui.text(hh, c.x(W - 56.0), c.y(y + 30.0), c.fs(11.0), F::Reg, mid);
                        }
                    }
                    if t.unread > 0 {
                        draw_circle(c.x(W - 34.0), c.y(y + 48.0), 10.0 * c.u, hex(0x2f80ed));
                        app.ui.text(&t.unread.to_string(), c.x(W - 38.0), c.y(y + 53.0), c.fs(11.0), F::Bold, WHITE);
                    }
                    if clicked {
                        app.phone.sub = Some(k);
                        app.phone.scroll = 0;
                    }
                }
            }
        }
        "telefone" => {
            header(app, c, "Telefone", hex(0x34c759));
            // abas
            for (k, l) in ["Recentes", "Teclado"].iter().enumerate() {
                let r = c.r(20.0 + k as f32 * 160.0, body_top + 6.0, 150.0, 30.0);
                let on = app.phone.tab == k as u8;
                rrect(r, 15.0 * c.u, if on { hex(0xd9f5e1) } else { hex(0xeceff2) });
                let w = app.ui.measure(l, c.fs(13.0), F::Bold);
                app.ui.text(l, r.center().x - w / 2.0, r.y + 20.0 * c.u, c.fs(13.0), F::Bold, if on { hex(0x1e7a3c) } else { mid });
                if app.ui.click_in(r) {
                    app.phone.tab = k as u8;
                }
            }
            if app.phone.tab == 0 {
                for (k, call) in v.calls.iter().enumerate() {
                    let y = body_top + 46.0 + k as f32 * 70.0;
                    if y + 70.0 > H - NAV {
                        break;
                    }
                    row(app, c, y, 70.0);
                    let (arrow, col) = if call.incoming { ("↙", hex(0x2f80ed)) } else { ("↗", hex(0x27ae60)) };
                    app.ui.text(arrow, c.x(30.0), c.y(y + 44.0), c.fs(22.0), F::Bold, col);
                    let who = call.name.clone().unwrap_or_else(|| call.number.clone());
                    app.ui.text(&who, c.x(72.0), c.y(y + 28.0), c.fs(15.0), F::Bold, dark);
                    let det = format!("{} · {} · {}", if call.incoming { "recebida" } else { "efetuada" }, call.number, fmt_dur(call.duration_s));
                    app.ui.text(&clip_text(&app.ui, &det, c.fs(11.5), F::Reg, 270.0 * c.u), c.x(72.0), c.y(y + 48.0), c.fs(11.5), F::Reg, mid);
                    app.ui.text(&call.when, c.x(72.0), c.y(y + 63.0), c.fs(10.0), F::Reg, mid);
                }
            } else {
                app.ui.text("*#06#", c.x(110.0), c.y(body_top + 120.0), c.fs(34.0), F::Reg, dark);
                let b = c.r(60.0, body_top + 160.0, 240.0, 48.0);
                rrect(b, 24.0 * c.u, hex(0x34c759));
                app.ui.text("Exibir IMEI", b.x + 76.0 * c.u, b.y + 31.0 * c.u, c.fs(15.0), F::Bold, WHITE);
                if app.ui.click_in(b) {
                    *acted = Some(format!("tela {} imei", v.item.to_lowercase()));
                }
                app.ui.text("Chamadas não são realizadas: o aparelho deve", c.x(36.0), c.y(body_top + 250.0), c.fs(11.0), F::Reg, mid);
                app.ui.text("permanecer isolado de redes durante o exame.", c.x(36.0), c.y(body_top + 266.0), c.fs(11.0), F::Reg, mid);
            }
        }
        "contatos" => {
            header(app, c, "Contatos", hex(0xf2994a));
            app.ui.text(&format!("{} contato(s) · armazenados no SIM", v.contacts.len()), c.x(22.0), c.y(body_top + 24.0), c.fs(11.5), F::Reg, mid);
            for (k, (name, num)) in v.contacts.iter().enumerate() {
                let y = body_top + 36.0 + k as f32 * 64.0;
                if y + 64.0 > H - NAV {
                    break;
                }
                row(app, c, y, 64.0);
                avatar(app, c, 40.0, y + 32.0, name);
                app.ui.text(name, c.x(72.0), c.y(y + 28.0), c.fs(15.0), F::Bold, dark);
                app.ui.text(num, c.x(72.0), c.y(y + 48.0), c.fs(12.0), F::Reg, mid);
            }
        }
        "galeria" => {
            if let Some(pi) = app.phone.sub.filter(|i| *i < v.photos.len()) {
                let p = &v.photos[pi];
                rrect(c.sr, 26.0 * c.u, hex(0x000000));
                photo(c, c.r(0.0, 120.0, W, 270.0), p.seed);
                app.ui.text(&p.name, c.x(18.0), c.y(424.0), c.fs(15.0), F::Bold, WHITE);
                app.ui.text(&p.when, c.x(18.0), c.y(446.0), c.fs(12.0), F::Reg, Color::new(1.0, 1.0, 1.0, 0.7));
                app.ui.text("Detalhes", c.x(18.0), c.y(486.0), c.fs(13.0), F::Bold, WHITE);
                let mut y = 510.0;
                for (k, val) in &p.meta {
                    app.ui.text(&format!("{}: {}", k, val), c.x(18.0), c.y(y), c.fs(12.0), F::Mono, Color::new(1.0, 1.0, 1.0, 0.85));
                    y += 20.0;
                }
                app.ui.text(&p.path, c.x(18.0), c.y(y + 10.0), c.fs(9.5), F::Mono, Color::new(1.0, 1.0, 1.0, 0.5));
            } else {
                header(app, c, "Galeria", hex(0xbb6bd9));
                if v.photos.is_empty() {
                    app.ui.text("Nenhuma foto", c.x(24.0), c.y(body_top + 40.0), c.fs(14.0), F::Reg, mid);
                }
                for (k, p) in v.photos.iter().enumerate() {
                    let (cx, cy) = ((k % 3) as f32, (k / 3) as f32);
                    let r = c.r(4.0 + cx * 118.0, body_top + 4.0 + cy * 118.0, 114.0, 114.0);
                    photo(c, r, p.seed);
                    if app.ui.click_in(r) {
                        app.phone.sub = Some(k);
                    }
                }
            }
        }
        "arquivos" => {
            header(app, c, "Arquivos", hex(0xd4a72c));
            app.ui.text("Armazenamento interno", c.x(22.0), c.y(body_top + 24.0), c.fs(12.0), F::Bold, mid);
            let start = list_scroll(app, c, v.files.len(), ((body_h - 40.0) / 58.0) as usize);
            for (k, f) in v.files.iter().skip(start).enumerate() {
                let y = body_top + 36.0 + k as f32 * 58.0;
                if y + 58.0 > H - NAV {
                    break;
                }
                row(app, c, y, 58.0);
                rrect(c.r(22.0, y + 14.0, 32.0, 32.0), 6.0 * c.u, hex(0xf6e7b5));
                let name = f.path.rsplit('/').next().unwrap_or(&f.path);
                app.ui.text(&clip_text(&app.ui, name, c.fs(14.0), F::Bold, 270.0 * c.u), c.x(72.0), c.y(y + 26.0), c.fs(14.0), F::Bold, dark);
                let det = format!("{} · {} · {}", f.path.rsplit_once('/').map(|x| x.0).unwrap_or(""), human(f.size), f.when);
                app.ui.text(&clip_text(&app.ui, &det, c.fs(10.5), F::Reg, 270.0 * c.u), c.x(72.0), c.y(y + 45.0), c.fs(10.5), F::Reg, mid);
            }
            app.ui.text("Itens apagados não aparecem aqui (só na extração).", c.x(18.0), c.y(H - NAV - 10.0), c.fs(10.0), F::Reg, mid);
        }
        "relogio" => {
            header(app, c, "Relógio", hex(0x333a45));
            app.ui.text("Alarmes", c.x(22.0), c.y(body_top + 26.0), c.fs(13.0), F::Bold, mid);
            for (k, a) in v.alarms.iter().enumerate() {
                let y = body_top + 38.0 + k as f32 * 76.0;
                row(app, c, y, 76.0);
                let (hm, rest) = a.split_once(' ').unwrap_or((a, ""));
                let on = !v.alarms_disabled;
                app.ui.text(hm, c.x(22.0), c.y(y + 44.0), c.fs(30.0), F::Reg, if on { dark } else { mid });
                app.ui.text(rest.trim_matches(|ch| ch == '(' || ch == ')'), c.x(22.0), c.y(y + 64.0), c.fs(11.0), F::Reg, mid);
                let sw = c.r(W - 74.0, y + 26.0, 48.0, 26.0);
                rrect(sw, 13.0 * c.u, if on { hex(0x34c759) } else { hex(0xc9ced6) });
                draw_circle(if on { sw.x + sw.w - 13.0 * c.u } else { sw.x + 13.0 * c.u }, sw.center().y, 11.0 * c.u, WHITE);
                if app.ui.click_in(sw) && on {
                    *acted = Some(format!("alarmes {}", v.item.to_lowercase()));
                }
            }
            if !v.alarms_disabled {
                let b = c.r(40.0, H - NAV - 70.0, 280.0, 44.0);
                rrect(b, 22.0 * c.u, hex(0x333a45));
                app.ui.text("Desativar todos os alarmes", b.x + 46.0 * c.u, b.y + 28.0 * c.u, c.fs(13.5), F::Bold, WHITE);
                if app.ui.click_in(b) {
                    *acted = Some(format!("alarmes {}", v.item.to_lowercase()));
                }
            }
        }
        "config" => settings(app, c, v, acted),
        "cofre" => {
            rrect(c.sr, 26.0 * c.u, hex(0x111820));
            padlock(c, c.x(W / 2.0), c.y(250.0), WHITE);
            let t = v.second_space.clone().unwrap_or_else(|| "Espaço seguro".into());
            let tw = app.ui.measure(&t, c.fs(20.0), F::Bold);
            app.ui.text(&t, c.sr.x + (c.sr.w - tw) / 2.0, c.y(310.0), c.fs(20.0), F::Bold, WHITE);
            for (k, l) in ["Protegido por senha própria.", "O conteúdo não é exibido na tela inicial", "nem em buscas do aparelho."].iter().enumerate() {
                let w = app.ui.measure(l, c.fs(12.0), F::Reg);
                app.ui.text(l, c.sr.x + (c.sr.w - w) / 2.0, c.y(346.0 + k as f32 * 20.0), c.fs(12.0), F::Reg, Color::new(1.0, 1.0, 1.0, 0.7));
            }
        }
        _ => {}
    }
}

fn settings(app: &mut App, c: &Ctx, v: &PhoneView, acted: &mut Option<String>) {
    let dark = hex(0x1c2430);
    let mid = hex(0x6b7685);
    header(app, c, "Configurações", hex(0x5f6b7d));
    let page = app.phone.sub.unwrap_or(0);
    let body_top = 100.0;
    if page == 0 {
        let rows: Vec<(String, String, Option<bool>)> = vec![
            ("Modo avião".into(), if v.airplane { "Ativado".into() } else { "Desativado".into() }, Some(v.airplane)),
            ("Wi-Fi".into(), if v.wifi { "Conectado".into() } else { "Desligado".into() }, None),
            ("Bluetooth".into(), if v.bt { "Ligado".into() } else { "Desligado".into() }, None),
            ("Rede móvel".into(), v.operator.clone(), None),
            ("Bateria".into(), format!("{}%{}", v.battery, if v.charging { " · carregando" } else { "" }), None),
            ("Segurança".into(), "bloqueio de tela, smartlock, criptografia".into(), None),
            ("Chips (SIM)".into(), format!("{} chip(s){}", v.sims.len(), if v.esim.is_some() { " + eSIM" } else { "" }), None),
            ("Sobre o telefone".into(), format!("{} · {} {}", v.model, v.os, v.os_version), None),
        ];
        for (k, (t, d, tog)) in rows.iter().enumerate() {
            let y = body_top + k as f32 * 66.0;
            let clicked = row(app, c, y, 66.0);
            draw_circle(c.x(36.0), c.y(y + 33.0), 15.0 * c.u, hex(0xe7ebf0));
            app.ui.text(&t.chars().next().unwrap_or(' ').to_string(), c.x(31.0), c.y(y + 39.0), c.fs(14.0), F::Bold, mid);
            app.ui.text(t, c.x(68.0), c.y(y + 30.0), c.fs(15.0), F::Bold, dark);
            app.ui.text(&clip_text(&app.ui, d, c.fs(11.5), F::Reg, 220.0 * c.u), c.x(68.0), c.y(y + 50.0), c.fs(11.5), F::Reg, mid);
            if let Some(on) = tog {
                let sw = c.r(W - 74.0, y + 20.0, 48.0, 26.0);
                rrect(sw, 13.0 * c.u, if *on { hex(0x2f80ed) } else { hex(0xc9ced6) });
                draw_circle(if *on { sw.x + sw.w - 13.0 * c.u } else { sw.x + 13.0 * c.u }, sw.center().y, 11.0 * c.u, WHITE);
                if clicked && !*on {
                    *acted = Some(format!("airplane {}", v.item.to_lowercase()));
                }
            } else if clicked && k >= 5 {
                app.phone.sub = Some(k - 4);
            }
        }
        return;
    }
    let mut lines: Vec<(String, String)> = vec![];
    let title = match page {
        1 => {
            lines.push(("Bloqueio de tela".into(), if v.lock == "nenhum" { "Nenhum".into() } else { v.lock.to_uppercase() }));
            lines.push(("Smartlock".into(), v.smartlock.clone().map(|s| format!("ATIVO ({})", s)).unwrap_or_else(|| "Desativado".into())));
            lines.push(("Criptografia".into(), match v.encryption.as_str() { "FBE" => "Baseada em arquivos (FBE)".into(), "FDE" => "Disco inteiro (FDE)".into(), x => x.into() }));
            if let Some(ss) = &v.second_space {
                lines.push(("Espaço protegido".into(), format!("{} configurado", ss)));
            }
            "Segurança"
        }
        2 => {
            for (slot, op, inside) in &v.sims {
                lines.push((format!("SIM {}", slot), format!("{}{}", op, if *inside { "" } else { " (removido)" })));
            }
            if let Some(e) = &v.esim {
                lines.push(("eSIM".into(), e.clone()));
            }
            "Chips (SIM)"
        }
        _ => {
            lines.push(("Modelo".into(), v.model.clone()));
            lines.push(("Sistema".into(), format!("{} {}", v.os, v.os_version)));
            lines.push(("IMEI".into(), "toque para exibir (*#06#)".into()));
            "Sobre o telefone"
        }
    };
    app.ui.text(&format!("‹ {}", title), c.x(20.0), c.y(body_top + 30.0), c.fs(16.0), F::Bold, dark);
    for (k, (a, b)) in lines.iter().enumerate() {
        let y = body_top + 46.0 + k as f32 * 62.0;
        let clicked = row(app, c, y, 62.0);
        app.ui.text(a, c.x(24.0), c.y(y + 26.0), c.fs(14.0), F::Bold, dark);
        app.ui.text(&clip_text(&app.ui, b, c.fs(12.0), F::Reg, 310.0 * c.u), c.x(24.0), c.y(y + 46.0), c.fs(12.0), F::Reg, mid);
        if clicked && a == "IMEI" {
            *acted = Some(format!("tela {} imei", v.item.to_lowercase()));
        }
    }
}

/// Miniatura procedural de foto (paisagem com céu, sol, morros e água) a partir da semente.
fn photo(c: &Ctx, r: Rect, seed: u64) {
    let f = |k: u64| ((seed >> (k * 5)) & 31) as f32 / 31.0;
    let sky_t = Color::new(0.35 + f(1) * 0.3, 0.55 + f(2) * 0.2, 0.85, 1.0);
    let sky_b = Color::new(0.95, 0.80 + f(3) * 0.15, 0.62 + f(4) * 0.2, 1.0);
    gradient(rect(r.x, r.y, r.w, r.h * 0.62), sky_t, sky_b);
    draw_circle(r.x + r.w * (0.2 + f(5) * 0.6), r.y + r.h * (0.25 + f(6) * 0.2), r.w * 0.07, Color::new(1.0, 0.95, 0.8, 0.95));
    let hill = Color::new(0.20 + f(7) * 0.15, 0.38 + f(8) * 0.15, 0.25, 1.0);
    let n = 14;
    for i in 0..n {
        let x0 = r.x + r.w * i as f32 / n as f32;
        let x1 = r.x + r.w * (i + 1) as f32 / n as f32;
        let hgt = |x: f32| r.y + r.h * (0.5 + 0.08 * ((x - r.x) / r.w * 7.0 + f(9) * 6.0).sin());
        draw_triangle(vec2(x0, hgt(x0)), vec2(x1, hgt(x1)), vec2(x0, r.y + r.h * 0.7), hill);
        draw_triangle(vec2(x1, hgt(x1)), vec2(x1, r.y + r.h * 0.7), vec2(x0, r.y + r.h * 0.7), hill);
    }
    let water = if f(10) > 0.5 { Color::new(0.18, 0.42, 0.62, 1.0) } else { Color::new(0.62, 0.55, 0.40, 1.0) };
    gradient(rect(r.x, r.y + r.h * 0.7, r.w, r.h * 0.3), water, Color::new(water.r * 0.6, water.g * 0.6, water.b * 0.6, 1.0));
    let _ = c;
}

fn fmt_dur(s: u32) -> String {
    format!("{}min {:02}s", s / 60, s % 60)
}

fn human(n: u64) -> String {
    if n >= 1 << 20 {
        format!("{:.1} MB", n as f64 / 1048576.0)
    } else if n >= 1024 {
        format!("{:.0} KB", n as f64 / 1024.0)
    } else {
        format!("{} B", n)
    }
}

/// Painel lateral: o que o perito está fazendo e atalhos de documentação.
fn side(app: &mut App, v: &PhoneView, r: Rect, acted: &mut Option<String>) {
    if r.w < 160.0 {
        return;
    }
    let s = app.ui.scale;
    app.ui.frame(r, Color::new(0.07, 0.08, 0.10, 0.92), T.border);
    let pad = 12.0 * s;
    let mut y = r.y + pad + 14.0 * s;
    app.ui.text(&format!("{} — tela do aparelho", v.item), r.x + pad, y, 14.0, F::Bold, T.text);
    y += 22.0 * s;
    let estado = match v.screen {
        Screen::Off => "Tela apagada (desligado ou sem carga).",
        Screen::Setup => "Assistente de configuração: aparelho restaurado.",
        Screen::Locked { afu: true } => "Bloqueado — AFU (após o primeiro desbloqueio).",
        Screen::Locked { afu: false } => "Bloqueado — BFU (antes do primeiro desbloqueio).",
        Screen::Home => "Desbloqueado: o conteúdo é acessível pela tela.",
    };
    for l in app.ui.wrap(estado, 12.0, F::Reg, r.w - pad * 2.0) {
        app.ui.text(&l, r.x + pad, y, 12.0, F::Reg, T.dim);
        y += 17.0 * s;
    }
    y += 8.0 * s;
    let notes = [
        "Toda navegação na tela é registrada na auditoria (comando 'tela').",
        "Navegar no conteúdo altera o aparelho (ex.: mensagens lidas). No exame, faça-o depois da extração e documente com fotografias.",
        "Na apreensão, use a tela só para isolamento (modo avião), alarmes e IMEI.",
        "Não há tentativa de senha pela tela: o desbloqueio depende da senha documentada ou da extração.",
    ];
    for n in notes {
        for (k, l) in app.ui.wrap(n, 11.5, F::Reg, r.w - pad * 2.0 - 10.0 * s).iter().enumerate() {
            if k == 0 {
                app.ui.text("•", r.x + pad, y, 11.5, F::Reg, T.accent);
            }
            app.ui.text(l, r.x + pad + 10.0 * s, y, 11.5, F::Reg, T.text);
            y += 16.0 * s;
        }
        y += 6.0 * s;
    }
    y += 6.0 * s;
    let bw = r.w - pad * 2.0;
    if v.screen != Screen::Off && app.ui.button_c(rect(r.x + pad, y, bw, 32.0 * s), "Fotografar a tela", T.accent2, true) {
        *acted = Some(format!("tela {} foto", v.item.to_lowercase()));
    }
    y += 38.0 * s;
    if app.ui.button(rect(r.x + pad, y, bw, 32.0 * s), "Guardar o aparelho (Esc)") {
        app.phone.close();
    }
}
