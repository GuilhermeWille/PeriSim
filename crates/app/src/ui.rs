//! Kit mínimo de interface imediata (botões, campos de texto, áreas roláveis) e tema.
//! Escrito sobre macroquad para não depender de bibliotecas de widgets pesadas.

use macroquad::prelude::*;

pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub panel2: Color,
    pub border: Color,
    pub text: Color,
    pub dim: Color,
    pub accent: Color,
    pub accent2: Color,
    pub ok: Color,
    pub warn: Color,
    pub bad: Color,
    pub term_bg: Color,
    pub term_fg: Color,
    pub highlight: Color,
}

pub const T: Theme = Theme {
    bg: Color::new(0.075, 0.086, 0.11, 1.0),
    panel: Color::new(0.11, 0.125, 0.16, 1.0),
    panel2: Color::new(0.15, 0.17, 0.215, 1.0),
    border: Color::new(0.24, 0.27, 0.33, 1.0),
    text: Color::new(0.90, 0.92, 0.95, 1.0),
    dim: Color::new(0.60, 0.64, 0.71, 1.0),
    accent: Color::new(0.25, 0.62, 0.95, 1.0),
    accent2: Color::new(0.18, 0.42, 0.68, 1.0),
    ok: Color::new(0.35, 0.80, 0.50, 1.0),
    warn: Color::new(0.98, 0.75, 0.25, 1.0),
    bad: Color::new(0.95, 0.40, 0.38, 1.0),
    term_bg: Color::new(0.04, 0.05, 0.06, 1.0),
    term_fg: Color::new(0.72, 0.92, 0.75, 1.0),
    highlight: Color::new(1.0, 0.86, 0.25, 1.0),
};

pub struct Fonts {
    pub reg: Font,
    pub bold: Font,
    pub mono: Font,
}

impl Fonts {
    pub fn load() -> Fonts {
        Fonts {
            reg: load_ttf_font_from_bytes(include_bytes!("../assets/DejaVuSans.ttf")).unwrap(),
            bold: load_ttf_font_from_bytes(include_bytes!("../assets/DejaVuSans-Bold.ttf")).unwrap(),
            mono: load_ttf_font_from_bytes(include_bytes!("../assets/DejaVuSansMono.ttf")).unwrap(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum F {
    Reg,
    Bold,
    Mono,
}

pub struct Ui {
    pub fonts: Fonts,
    pub scale: f32,
    pub mouse: Vec2,
    pub clicked: bool,
    pub click_consumed: bool,
    pub wheel: f32,
    pub focus: Option<String>,
    pub chars: Vec<char>,
    pub key_enter: bool,
    pub key_back: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_tab: bool,
    pub key_esc: bool,
    pub ctrl: bool,
    pub paste: Option<String>,
    pub tooltip: Option<String>,
    pub any_input: bool,
    back_repeat: f32,
    pub mono_w: f32,
}

impl Ui {
    pub fn new(fonts: Fonts) -> Ui {
        Ui {
            fonts,
            scale: 1.0,
            mouse: Vec2::ZERO,
            clicked: false,
            click_consumed: false,
            wheel: 0.0,
            focus: None,
            chars: vec![],
            key_enter: false,
            key_back: false,
            key_up: false,
            key_down: false,
            key_tab: false,
            key_esc: false,
            ctrl: false,
            paste: None,
            tooltip: None,
            any_input: false,
            back_repeat: 0.0,
            mono_w: 8.0,
        }
    }

    pub fn begin(&mut self, user_scale: f32) {
        let base = (screen_width() / 1280.0).min(screen_height() / 720.0).clamp(0.75, 2.2);
        self.scale = base * user_scale;
        let (mx, my) = mouse_position();
        let m = vec2(mx, my);
        self.any_input = m != self.mouse;
        self.mouse = m;
        self.clicked = is_mouse_button_pressed(MouseButton::Left);
        self.click_consumed = false;
        self.wheel = mouse_wheel().1;
        self.chars.clear();
        while let Some(c) = get_char_pressed() {
            if !c.is_control() {
                self.chars.push(c);
            }
        }
        self.ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) || is_key_down(KeyCode::LeftSuper);
        self.key_enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter);
        // backspace com repetição
        self.key_back = false;
        if is_key_pressed(KeyCode::Backspace) {
            self.key_back = true;
            self.back_repeat = 0.4;
        } else if is_key_down(KeyCode::Backspace) {
            self.back_repeat -= get_frame_time();
            if self.back_repeat <= 0.0 {
                self.key_back = true;
                self.back_repeat = 0.04;
            }
        }
        self.key_up = is_key_pressed(KeyCode::Up);
        self.key_down = is_key_pressed(KeyCode::Down);
        self.key_tab = is_key_pressed(KeyCode::Tab);
        self.key_esc = is_key_pressed(KeyCode::Escape);
        self.paste = None;
        if self.ctrl && is_key_pressed(KeyCode::V) {
            self.paste = miniquad::window::clipboard_get();
        }
        if self.clicked || self.wheel != 0.0 || !self.chars.is_empty() || self.key_enter || self.key_back || self.key_up || self.key_down || self.key_esc {
            self.any_input = true;
        }
        self.chars.retain(|_| true);
        self.tooltip = None;
        self.mono_w = measure_text("M", Some(&self.fonts.mono), self.fs(13.0) as u16, 1.0).width;
    }

    pub fn s(&self, v: f32) -> f32 {
        v * self.scale
    }
    pub fn fs(&self, v: f32) -> f32 {
        (v * self.scale).round().max(8.0)
    }

    fn font(&self, f: F) -> &Font {
        match f {
            F::Reg => &self.fonts.reg,
            F::Bold => &self.fonts.bold,
            F::Mono => &self.fonts.mono,
        }
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, f: F, color: Color) {
        draw_text_ex(s, x.round(), y.round(), TextParams { font: Some(self.font(f)), font_size: self.fs(size) as u16, color, ..Default::default() });
    }

    pub fn measure(&self, s: &str, size: f32, f: F) -> f32 {
        measure_text(s, Some(self.font(f)), self.fs(size) as u16, 1.0).width
    }

    pub fn hover(&self, r: Rect) -> bool {
        r.contains(self.mouse)
    }

    /// Clique dentro do retângulo (consome o clique para elementos de trás).
    pub fn click_in(&mut self, r: Rect) -> bool {
        if self.clicked && !self.click_consumed && r.contains(self.mouse) {
            self.click_consumed = true;
            true
        } else {
            false
        }
    }

    pub fn panel(&self, r: Rect, c: Color) {
        draw_rectangle(r.x, r.y, r.w, r.h, c);
    }

    pub fn frame(&self, r: Rect, fill: Color, border: Color) {
        draw_rectangle(r.x, r.y, r.w, r.h, fill);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, border);
    }

    pub fn button(&mut self, r: Rect, label: &str) -> bool {
        self.button_c(r, label, T.panel2, true)
    }

    pub fn button_c(&mut self, r: Rect, label: &str, base: Color, enabled: bool) -> bool {
        let hov = enabled && self.hover(r);
        let fill = if !enabled {
            Color::new(base.r * 0.6, base.g * 0.6, base.b * 0.6, 1.0)
        } else if hov {
            Color::new((base.r + 0.08).min(1.0), (base.g + 0.08).min(1.0), (base.b + 0.1).min(1.0), 1.0)
        } else {
            base
        };
        self.frame(r, fill, if hov { T.accent } else { T.border });
        let size = 13.0;
        let mut l = label.to_string();
        while self.measure(&l, size, F::Reg) > r.w - self.s(10.0) && l.chars().count() > 3 {
            l.pop();
        }
        let w = self.measure(&l, size, F::Reg);
        self.text(&l, r.x + (r.w - w) / 2.0, r.y + r.h / 2.0 + self.s(5.0), size, F::Reg, if enabled { T.text } else { T.dim });
        enabled && self.click_in(r)
    }

    pub fn tab(&mut self, r: Rect, label: &str, active: bool) -> bool {
        let fill = if active { T.accent2 } else if self.hover(r) { T.panel2 } else { T.panel };
        self.frame(r, fill, if active { T.accent } else { T.border });
        let w = self.measure(label, 13.0, F::Bold);
        self.text(label, r.x + (r.w - w) / 2.0, r.y + r.h / 2.0 + self.s(5.0), 13.0, F::Bold, T.text);
        self.click_in(r)
    }

    /// Campo de texto de uma linha. Retorna true quando Enter é pressionado com foco.
    pub fn input(&mut self, id: &str, r: Rect, buf: &mut String, placeholder: &str) -> bool {
        let focused = self.focus.as_deref() == Some(id);
        if self.click_in(r) {
            self.focus = Some(id.to_string());
        }
        self.frame(r, T.term_bg, if focused { T.accent } else { T.border });
        let mut enter = false;
        if focused {
            for c in &self.chars {
                buf.push(*c);
            }
            if let Some(p) = &self.paste {
                buf.push_str(&p.replace('\n', " "));
            }
            if self.key_back {
                buf.pop();
            }
            if self.key_enter {
                enter = true;
            }
        }
        let size = 13.0;
        let pad = self.s(6.0);
        let maxw = r.w - pad * 2.0;
        // mostra o final do texto se não couber
        let mut shown: String = buf.clone();
        while self.measure(&shown, size, F::Mono) > maxw && !shown.is_empty() {
            shown.remove(0);
        }
        if buf.is_empty() && !focused {
            self.text(placeholder, r.x + pad, r.y + r.h / 2.0 + self.s(5.0), size, F::Reg, T.dim);
        } else {
            self.text(&shown, r.x + pad, r.y + r.h / 2.0 + self.s(5.0), size, F::Mono, T.text);
        }
        if focused && (get_time() * 2.0) as i64 % 2 == 0 {
            let cx = r.x + pad + self.measure(&shown, size, F::Mono) + 1.0;
            draw_line(cx, r.y + self.s(5.0), cx, r.y + r.h - self.s(5.0), 1.5, T.accent);
        }
        enter
    }

    /// Área de texto multilinha simples (cursor no final; Enter = nova linha).
    pub fn text_area(&mut self, id: &str, r: Rect, buf: &mut String, scroll: &mut f32) {
        let focused = self.focus.as_deref() == Some(id);
        if self.click_in(r) {
            self.focus = Some(id.to_string());
        }
        self.frame(r, T.term_bg, if focused { T.accent } else { T.border });
        if focused {
            for c in &self.chars {
                buf.push(*c);
            }
            if let Some(p) = &self.paste {
                buf.push_str(p);
            }
            if self.key_back {
                buf.pop();
            }
            if self.key_enter {
                buf.push('\n');
            }
        }
        let size = 12.5;
        let pad = self.s(6.0);
        let lines = self.wrap(buf, size, F::Reg, r.w - pad * 2.0);
        let lh = self.s(17.0);
        let visible = ((r.h - pad * 2.0) / lh).floor().max(1.0) as usize;
        let total = lines.len();
        if self.hover(r) && self.wheel != 0.0 {
            *scroll = (*scroll - self.wheel.signum() * 2.0).max(0.0);
        }
        if focused && (self.key_enter || !self.chars.is_empty() || self.paste.is_some()) {
            *scroll = total.saturating_sub(visible) as f32;
        }
        let maxs = total.saturating_sub(visible) as f32;
        *scroll = scroll.min(maxs);
        let start = *scroll as usize;
        for (i, l) in lines.iter().skip(start).take(visible).enumerate() {
            self.text(l, r.x + pad, r.y + pad + lh * (i as f32 + 0.8), size, F::Reg, T.text);
        }
        if focused && (get_time() * 2.0) as i64 % 2 == 0 {
            let last = lines.last().cloned().unwrap_or_default();
            let row = (total.saturating_sub(1)).saturating_sub(start).min(visible - 1);
            let cx = r.x + pad + self.measure(&last, size, F::Reg) + 1.0;
            let cy = r.y + pad + lh * row as f32;
            draw_line(cx, cy + self.s(3.0), cx, cy + lh, 1.5, T.accent);
        }
    }

    pub fn wrap(&self, text: &str, size: f32, f: F, maxw: f32) -> Vec<String> {
        let mut out = vec![];
        let charw = if f == F::Mono { Some(measure_text("M", Some(self.font(f)), self.fs(size) as u16, 1.0).width) } else { None };
        let width = |s: &str| -> f32 {
            match charw {
                Some(cw) => s.chars().count() as f32 * cw,
                None => self.measure(s, size, f),
            }
        };
        for para in text.split('\n') {
            if para.is_empty() {
                out.push(String::new());
                continue;
            }
            if width(para) <= maxw {
                out.push(para.to_string());
                continue;
            }
            let mut cur = String::new();
            for word in para.split(' ') {
                let cand = if cur.is_empty() { word.to_string() } else { format!("{} {}", cur, word) };
                if width(&cand) > maxw && !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    cur = word.to_string();
                } else {
                    cur = cand;
                }
                while width(&cur) > maxw && cur.chars().count() > 1 {
                    let n = match charw {
                        Some(cw) => ((maxw / cw).floor() as usize).max(1),
                        None => (cur.chars().count() * 3 / 4).max(1),
                    };
                    let head: String = cur.chars().take(n).collect();
                    let tail: String = cur.chars().skip(n).collect();
                    out.push(head);
                    cur = tail;
                }
            }
            out.push(cur);
        }
        out
    }

    /// Texto rolável (somente leitura). Linhas que começam com "!" ficam em destaque.
    pub fn scroll_text(&mut self, r: Rect, text: &str, scroll: &mut f32, size: f32, f: F, color: Color, stick_bottom: bool) {
        self.frame(r, T.term_bg, T.border);
        let pad = self.s(8.0);
        let lines = self.wrap(text, size, f, r.w - pad * 2.0 - self.s(8.0));
        let lh = self.s(size * 1.38);
        let visible = ((r.h - pad * 2.0) / lh).floor().max(1.0) as usize;
        let maxs = lines.len().saturating_sub(visible) as f32;
        if self.hover(r) && self.wheel != 0.0 {
            *scroll = (*scroll - self.wheel.signum() * 3.0).clamp(0.0, maxs);
        }
        if stick_bottom && *scroll < 0.0 {
            *scroll = maxs;
        }
        *scroll = scroll.clamp(0.0, maxs);
        let start = *scroll as usize;
        for (i, l) in lines.iter().skip(start).take(visible).enumerate() {
            let (col, txt) = line_style(l, color);
            self.text(txt, r.x + pad, r.y + pad + lh * (i as f32 + 0.8), size, f, col);
        }
        if maxs > 0.0 {
            let bar_h = (r.h * visible as f32 / lines.len() as f32).max(self.s(20.0));
            let y = r.y + (r.h - bar_h) * (*scroll / maxs);
            draw_rectangle(r.x + r.w - self.s(5.0), y, self.s(3.0), bar_h, T.border);
        }
    }
}

/// Coloração de linhas por prefixo/conteúdo (comandos, alertas, sucesso).
pub fn line_style(l: &str, default: Color) -> (Color, &str) {
    if let Some(rest) = l.strip_prefix("> ") {
        let _ = rest;
        return (T.accent, l);
    }
    let up = l.to_uppercase();
    if up.contains("ATENÇÃO") || up.contains("AVISO") || up.contains("DIVERG") || up.contains("REPROVADO") {
        return (T.warn, l);
    }
    if up.starts_with("ERRO") || up.contains("NÃO FOI POSSÍVEL") {
        return (T.bad, l);
    }
    if up.contains("APROVADO") || up.contains("CONFERE") || up.contains("CONCLUÍDA") || up.contains("RECUPERADO") {
        return (T.ok, l);
    }
    (default, l)
}

pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}
