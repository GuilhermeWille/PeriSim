//! Cena 3D leve: malhas procedurais low-poly com iluminação calculada na CPU por face
//! (compatível com OpenGL 2.1 / GLES 2, sem shaders próprios), câmera orbital,
//! seleção por raio contra caixas e modelos dos equipamentos e da bancada.

use macroquad::prelude::*;
use macroquad::models::Vertex;
use pericia_core::engine::Sim;
use pericia_core::model::*;

const LIGHT: Vec3 = Vec3::new(0.45, 0.85, 0.35);

pub struct Batch {
    verts: Vec<Vertex>,
    idx: Vec<u16>,
}

impl Batch {
    pub fn new() -> Batch {
        Batch { verts: Vec::with_capacity(4096), idx: Vec::with_capacity(8192) }
    }
    fn flush(&mut self) {
        if self.idx.is_empty() {
            return;
        }
        let mesh = Mesh { vertices: std::mem::take(&mut self.verts), indices: std::mem::take(&mut self.idx), texture: None };
        draw_mesh(&mesh);
    }
    fn quad(&mut self, p: [Vec3; 4], n: Vec3, c: Color) {
        if self.verts.len() > 60000 {
            self.flush();
        }
        let l = 0.42 + 0.58 * n.normalize().dot(LIGHT.normalize()).max(0.0);
        let col = [(c.r * l * 255.0).min(255.0) as u8, (c.g * l * 255.0).min(255.0) as u8, (c.b * l * 255.0).min(255.0) as u8, (c.a * 255.0) as u8];
        let b = self.verts.len() as u16;
        for v in p {
            self.verts.push(Vertex { position: v, uv: Vec2::ZERO, color: col, normal: Vec4::ZERO });
        }
        self.idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    /// Caixa alinhada aos eixos, com rotação opcional em Y (rad) em torno do centro.
    pub fn boxr(&mut self, c: Vec3, size: Vec3, rot_y: f32, col: Color) {
        let h = size * 0.5;
        let (s, co) = rot_y.sin_cos();
        let tr = |x: f32, y: f32, z: f32| -> Vec3 { vec3(c.x + x * co + z * s, c.y + y, c.z - x * s + z * co) };
        let rn = |n: Vec3| -> Vec3 { vec3(n.x * co + n.z * s, n.y, -n.x * s + n.z * co) };
        let (x, y, z) = (h.x, h.y, h.z);
        self.quad([tr(-x, y, -z), tr(x, y, -z), tr(x, y, z), tr(-x, y, z)], vec3(0., 1., 0.), col);
        self.quad([tr(-x, -y, z), tr(x, -y, z), tr(x, -y, -z), tr(-x, -y, -z)], vec3(0., -1., 0.), col);
        self.quad([tr(-x, -y, z), tr(-x, y, z), tr(x, y, z), tr(x, -y, z)], rn(vec3(0., 0., 1.)), col);
        self.quad([tr(x, -y, -z), tr(x, y, -z), tr(-x, y, -z), tr(-x, -y, -z)], rn(vec3(0., 0., -1.)), col);
        self.quad([tr(x, -y, z), tr(x, y, z), tr(x, y, -z), tr(x, -y, -z)], rn(vec3(1., 0., 0.)), col);
        self.quad([tr(-x, -y, -z), tr(-x, y, -z), tr(-x, y, z), tr(-x, -y, z)], rn(vec3(-1., 0., 0.)), col);
    }
    pub fn bx(&mut self, c: Vec3, size: Vec3, col: Color) {
        self.boxr(c, size, 0.0, col);
    }
    /// Cilindro vertical (eixo Y).
    pub fn cyl(&mut self, c: Vec3, r: f32, h: f32, col: Color) {
        let n = 14;
        for i in 0..n {
            let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
            let p0 = vec3(a0.cos() * r, 0.0, a0.sin() * r);
            let p1 = vec3(a1.cos() * r, 0.0, a1.sin() * r);
            let top = vec3(0.0, h / 2.0, 0.0);
            let nrm = (p0 + p1).normalize();
            self.quad([c + p0 - top, c + p0 + top, c + p1 + top, c + p1 - top], nrm, col);
            self.quad([c + top, c + top, c + p1 + top, c + p0 + top], vec3(0., 1., 0.), col);
        }
    }
    /// Plano inclinado (tela de notebook): retângulo girado em X.
    pub fn slab_tilt(&mut self, base: Vec3, w: f32, h: f32, d: f32, tilt: f32, col: Color, face: Color) {
        let (s, c) = tilt.sin_cos();
        let up = vec3(0.0, c, -s);
        let back = vec3(0.0, s, c) * -1.0;
        let x = vec3(w / 2.0, 0.0, 0.0);
        let p = |a: Vec3| base + a;
        let top = up * h;
        let th = back * d;
        // face frontal (tela)
        self.quad([p(-x), p(x), p(x + top), p(-x + top)], vec3(0.0, s, c), face);
        self.quad([p(x + th), p(-x + th), p(-x + th + top), p(x + th + top)], -vec3(0.0, s, c), col);
        self.quad([p(-x + top), p(x + top), p(x + top + th), p(-x + top + th)], up, col);
        self.quad([p(x), p(x + th), p(x + th + top), p(x + top)], vec3(1., 0., 0.), col);
        self.quad([p(-x + th), p(-x), p(-x + top), p(-x + th + top)], vec3(-1., 0., 0.), col);
    }
    pub fn finish(&mut self) {
        self.flush();
    }
}

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

// ------------------------------------------------------------------ câmera e seleção

pub struct OrbitCam {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub target: Vec3,
    pub goal_target: Vec3,
    pub goal_dist: f32,
}

impl OrbitCam {
    pub fn new(target: Vec3, dist: f32) -> OrbitCam {
        OrbitCam { yaw: -0.6, pitch: 0.55, dist, target, goal_target: target, goal_dist: dist }
    }
    pub fn camera(&self) -> Camera3D {
        let pos = self.target + vec3(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos()) * self.dist;
        Camera3D { position: pos, target: self.target, up: vec3(0., 1., 0.), fovy: 0.85, ..Default::default() }
    }
    /// Retorna true se a câmera ainda está em movimento (animação de foco).
    pub fn update(&mut self, viewport: Rect, mouse: Vec2, wheel: f32, allow: bool) -> bool {
        if allow && viewport.contains(mouse) {
            if is_mouse_button_down(MouseButton::Right) || is_mouse_button_down(MouseButton::Middle) {
                let d = mouse_delta_position();
                self.yaw += d.x * 2.6;
                self.pitch = (self.pitch - d.y * 2.0).clamp(0.08, 1.45);
            }
            if wheel != 0.0 {
                self.goal_dist = (self.goal_dist * if wheel > 0.0 { 0.88 } else { 1.13 }).clamp(0.6, 14.0);
            }
        } else {
            let _ = mouse_delta_position();
        }
        if is_key_down(KeyCode::Left) && allow {
            self.yaw -= 0.03;
        }
        if is_key_down(KeyCode::Right) && allow {
            self.yaw += 0.03;
        }
        let k = 0.18;
        let moving = (self.target - self.goal_target).length() > 0.002 || (self.dist - self.goal_dist).abs() > 0.002;
        self.target = self.target.lerp(self.goal_target, k);
        self.dist += (self.goal_dist - self.dist) * k;
        moving
    }
}

/// Raio no espaço do mundo a partir de um ponto da tela, para um viewport.
pub fn ray(cam: &Camera3D, viewport: Rect, p: Vec2) -> (Vec3, Vec3) {
    let ndc = vec2((p.x - viewport.x) / viewport.w * 2.0 - 1.0, 1.0 - (p.y - viewport.y) / viewport.h * 2.0);
    let aspect = viewport.w / viewport.h;
    let m = Mat4::perspective_rh_gl(cam.fovy, aspect, cam.z_near, cam.z_far) * Mat4::look_at_rh(cam.position, cam.target, cam.up);
    let inv = m.inverse();
    let a = inv.project_point3(vec3(ndc.x, ndc.y, -1.0));
    let b = inv.project_point3(vec3(ndc.x, ndc.y, 1.0));
    (a, (b - a).normalize())
}

pub fn ray_aabb(o: Vec3, d: Vec3, c: Vec3, half: Vec3) -> Option<f32> {
    let mn = c - half;
    let mx = c + half;
    let mut t0 = f32::MIN;
    let mut t1 = f32::MAX;
    for i in 0..3 {
        let (oi, di, a, b) = (o[i], d[i], mn[i], mx[i]);
        if di.abs() < 1e-6 {
            if oi < a || oi > b {
                return None;
            }
        } else {
            let (mut ta, mut tb) = ((a - oi) / di, (b - oi) / di);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            if t0 > t1 {
                return None;
            }
        }
    }
    Some(t0.max(0.0))
}

pub fn project(cam: &Camera3D, viewport: Rect, p: Vec3) -> Option<Vec2> {
    let aspect = viewport.w / viewport.h;
    let m = Mat4::perspective_rh_gl(cam.fovy, aspect, cam.z_near, cam.z_far) * Mat4::look_at_rh(cam.position, cam.target, cam.up);
    let v = m * p.extend(1.0);
    if v.w <= 0.0 {
        return None;
    }
    let n = v.truncate() / v.w;
    Some(vec2(viewport.x + (n.x + 1.0) * 0.5 * viewport.w, viewport.y + (1.0 - n.y) * 0.5 * viewport.h))
}

// ------------------------------------------------------------------ objetos clicáveis

#[derive(Clone)]
pub struct Pick {
    pub id: String,
    pub center: Vec3,
    pub half: Vec3,
    pub label: String,
}

/// Monta a cena da Área de Hardware. Retorna os objetos clicáveis.
pub fn build_hardware(b: &mut Batch, sim: &Sim, selected: Option<&str>) -> Vec<Pick> {
    let mut picks = vec![];
    let local = sim.case.setting == Setting::Local;
    room(b, local);
    let visible: Vec<&Item> = sim.case.items.iter().filter(|i| !i.hidden || i.flag("descoberto")).collect();
    // posições ao longo da bancada/mesa
    let n = visible.len().max(1) as f32;
    for (k, it) in visible.iter().enumerate() {
        let x = -1.1 + (k as f32 + 0.5) * (2.2 / n);
        let mut base = vec3(x, 0.80, 0.0);
        if local && it.hidden {
            base = vec3(1.55, 0.42, 0.55); // gaveta aberta
        }
        if local && it.kind.is_computer() && !it.power_on && it.id != "ITEM-01" {
            base = vec3(x, 0.0, 0.15); // sob a mesa
        }
        let half = draw_item(b, sim, it, base);
        contact_shadow(b, base, half);
        let center = base + vec3(0.0, half.y, 0.0);
        if selected == Some(it.id.as_str()) {
            outline(b, center, half + Vec3::splat(0.02));
        }
        picks.push(Pick { id: it.id.clone(), center, half, label: format!("{} · {}", it.id, short_label(it)) });
    }
    // ferramentas sobre a bancada (clicáveis apenas para informação)
    let wb_on = sim.hashes.keys().any(|k| k.starts_with("wb:"));
    tool_writeblocker(b, vec3(-1.35, 0.80, 0.42), wb_on);
    picks.push(Pick { id: "TOOL-WB".into(), center: vec3(-1.35, 0.85, 0.42), half: vec3(0.12, 0.05, 0.08), label: "Bloqueador de escrita".into() });
    tool_camera(b, vec3(-0.75, 0.80, 0.45));
    picks.push(Pick { id: "TOOL-CAM".into(), center: vec3(-0.75, 0.86, 0.45), half: vec3(0.09, 0.06, 0.06), label: "Câmera fotográfica".into() });
    tool_bags(b, vec3(0.55, 0.80, 0.45));
    picks.push(Pick { id: "TOOL-BAG".into(), center: vec3(0.6, 0.82, 0.45), half: vec3(0.2, 0.03, 0.1), label: "Embalagens / Faraday bag / lacres".into() });
    // estação pericial ao fundo
    workstation(b, vec3(0.0, 0.80, -1.6), false);
    picks.push(Pick { id: "STATION".into(), center: vec3(0.0, 1.15, -1.6), half: vec3(0.45, 0.35, 0.3), label: "Estação Pericial (clique para usar)".into() });
    picks
}

fn short_label(it: &Item) -> String {
    let mut s = it.kind.label().to_string();
    if it.kind.is_computer() {
        s.push_str(if it.power_on { " (ligado)" } else { " (desligado)" });
    }
    if let Some(p) = &it.phone {
        if p.wiped {
            s.push_str(" (APAGADO)");
        }
    }
    s
}

fn outline(b: &mut Batch, c: Vec3, h: Vec3) {
    let col = Color::new(1.0, 0.85, 0.2, 1.0);
    let t = 0.008;
    for sx in [-1.0, 1.0] {
        for sz in [-1.0, 1.0] {
            b.bx(c + vec3(sx * h.x, 0.0, sz * h.z), vec3(t, h.y * 2.0, t), col);
        }
        for sy in [-1.0, 1.0] {
            b.bx(c + vec3(sx * h.x, sy * h.y, 0.0), vec3(t, t, h.z * 2.0), col);
            b.bx(c + vec3(0.0, sy * h.y, sx * h.z), vec3(h.x * 2.0, t, t), col);
        }
    }
}

/// Sombra de contato aproximada (camadas escuras finas sobre a superfície de apoio).
fn contact_shadow(b: &mut Batch, base: Vec3, half: Vec3) {
    for (k, grow) in [(0, 1.18f32), (1, 1.06)] {
        let a = if k == 0 { 0.18 } else { 0.30 };
        b.bx(base + vec3(0.0, 0.0012 + k as f32 * 0.0006, 0.0), vec3(half.x * 2.0 * grow, 0.001, half.z * 2.0 * grow), Color::new(0.0, 0.0, 0.0, a));
    }
}

fn room(b: &mut Batch, local: bool) {
    // piso e paredes
    let floor = if local { rgb(120, 98, 78) } else { rgb(70, 76, 86) };
    b.bx(vec3(0.0, -0.02, -0.4), vec3(7.0, 0.04, 5.5), floor);
    let wall = if local { rgb(196, 186, 168) } else { rgb(150, 160, 172) };
    b.bx(vec3(0.0, 1.5, -3.1), vec3(7.0, 3.0, 0.05), wall);
    b.bx(vec3(-3.5, 1.5, -0.4), vec3(0.05, 3.0, 5.5), Color::new(wall.r * 0.92, wall.g * 0.92, wall.b * 0.92, 1.0));
    // rodapé
    b.bx(vec3(0.0, 0.05, -3.07), vec3(7.0, 0.1, 0.02), Color::new(wall.r * 0.7, wall.g * 0.7, wall.b * 0.7, 1.0));
    // janela com luz difusa na parede lateral
    b.bx(vec3(-3.47, 1.65, -0.9), vec3(0.02, 1.1, 1.5), rgb(200, 222, 240));
    b.bx(vec3(-3.46, 1.65, -0.9), vec3(0.02, 1.16, 0.04), rgb(235, 235, 238));
    b.bx(vec3(-3.46, 1.65, -0.9), vec3(0.02, 0.04, 1.56), rgb(235, 235, 238));
    // luminária no teto
    b.bx(vec3(0.0, 2.95, -0.2), vec3(1.2, 0.04, 0.25), rgb(240, 240, 236));
    b.bx(vec3(0.0, 2.93, -0.2), vec3(1.1, 0.01, 0.2), rgb(255, 252, 230));
    if local {
        // mesa de escritório, cadeira, gaveteiro, estante
        let wood = rgb(140, 100, 64);
        b.bx(vec3(0.0, 0.77, 0.0), vec3(2.8, 0.05, 1.0), wood);
        for (x, z) in [(-1.3, -0.42), (1.3, -0.42), (-1.3, 0.42), (1.3, 0.42)] {
            b.bx(vec3(x, 0.38, z), vec3(0.06, 0.76, 0.06), rgb(90, 64, 40));
        }
        b.bx(vec3(1.55, 0.35, 0.3), vec3(0.45, 0.7, 0.55), rgb(118, 84, 54));
        b.bx(vec3(1.55, 0.42, 0.6), vec3(0.40, 0.14, 0.12), rgb(100, 72, 46));
        b.bx(vec3(-2.6, 1.0, -2.9), vec3(1.2, 2.0, 0.35), rgb(110, 80, 50));
        for i in 0..4 {
            b.bx(vec3(-2.6, 0.3 + i as f32 * 0.5, -2.75), vec3(1.15, 0.03, 0.3), rgb(130, 95, 60));
        }
        b.bx(vec3(0.0, 0.45, 0.85), vec3(0.5, 0.06, 0.5), rgb(40, 40, 46));
        b.bx(vec3(0.0, 0.75, 1.08), vec3(0.5, 0.6, 0.06), rgb(40, 40, 46));
        // roteador na estante (conexão de rede do local)
        b.bx(vec3(-2.5, 1.36, -2.75), vec3(0.3, 0.05, 0.18), rgb(30, 30, 34));
    } else {
        // bancada de laboratório com tampo antiestático
        b.bx(vec3(0.0, 0.77, 0.0), vec3(3.2, 0.05, 1.3), rgb(54, 96, 84));
        b.bx(vec3(0.0, 0.40, 0.0), vec3(3.1, 0.72, 1.2), rgb(190, 194, 198));
        b.bx(vec3(0.0, 0.40, 0.61), vec3(3.0, 0.62, 0.01), rgb(170, 174, 178));
        // armário de evidências
        b.bx(vec3(2.6, 1.0, -2.7), vec3(0.9, 2.0, 0.6), rgb(120, 126, 134));
        b.bx(vec3(2.6, 1.0, -2.39), vec3(0.02, 1.9, 0.01), rgb(60, 60, 66));
    }
}

pub fn workstation(b: &mut Batch, at: Vec3, big: bool) {
    let s = if big { 1.0 } else { 0.8 };
    // mesa
    b.bx(at + vec3(0.0, -0.03, 0.0) * s, vec3(1.4, 0.05, 0.7) * s, rgb(70, 74, 82));
    if !big {
        for (x, z) in [(-0.65, -0.3), (0.65, -0.3), (-0.65, 0.3), (0.65, 0.3)] {
            b.bx(at + vec3(x * s, -0.42, z * s), vec3(0.04, 0.76, 0.04), rgb(50, 52, 58));
        }
    }
    // monitor
    b.bx(at + vec3(0.0, 0.02, -0.15) * s, vec3(0.22, 0.03, 0.15) * s, rgb(30, 30, 34));
    b.bx(at + vec3(0.0, 0.14, -0.18) * s, vec3(0.04, 0.22, 0.03) * s, rgb(30, 30, 34));
    b.bx(at + vec3(0.0, 0.38, -0.16) * s, vec3(0.78, 0.46, 0.04) * s, rgb(24, 24, 28));
    b.bx(at + vec3(0.0, 0.385, -0.137) * s, vec3(0.72, 0.40, 0.005) * s, rgb(40, 110, 180));
    // gabinete da estação
    b.bx(at + vec3(0.55, 0.25, -0.05) * s, vec3(0.2, 0.5, 0.45) * s, rgb(44, 48, 56));
    b.bx(at + vec3(0.55, 0.42, 0.176) * s, vec3(0.02, 0.02, 0.005) * s, rgb(60, 220, 120));
    // teclado e mouse
    b.bx(at + vec3(-0.05, 0.01, 0.15) * s, vec3(0.45, 0.02, 0.14) * s, rgb(36, 36, 40));
    b.bx(at + vec3(0.32, 0.01, 0.16) * s, vec3(0.06, 0.02, 0.1) * s, rgb(36, 36, 40));
    // bloqueador de escrita conectado à estação
    b.bx(at + vec3(-0.5, 0.03, 0.05) * s, vec3(0.16, 0.05, 0.1) * s, rgb(200, 70, 40));
}

fn tool_writeblocker(b: &mut Batch, at: Vec3, on: bool) {
    b.bx(at + vec3(0.0, 0.04, 0.0), vec3(0.22, 0.07, 0.14), rgb(205, 72, 40));
    b.bx(at + vec3(-0.06, 0.08, 0.071), vec3(0.02, 0.012, 0.004), if on { rgb(60, 240, 110) } else { rgb(40, 60, 40) });
    b.bx(at + vec3(-0.02, 0.08, 0.071), vec3(0.02, 0.012, 0.004), if on { rgb(250, 190, 40) } else { rgb(60, 50, 30) });
    b.bx(at + vec3(0.07, 0.05, 0.072), vec3(0.05, 0.02, 0.004), rgb(30, 30, 30));
}

fn tool_camera(b: &mut Batch, at: Vec3) {
    b.bx(at + vec3(0.0, 0.05, 0.0), vec3(0.16, 0.10, 0.08), rgb(28, 28, 32));
    b.cyl(at + vec3(0.0, 0.05, 0.06), 0.035, 0.05, rgb(50, 50, 56));
}

fn tool_bags(b: &mut Batch, at: Vec3) {
    b.boxr(at + vec3(-0.1, 0.01, 0.0), vec3(0.22, 0.012, 0.15), 0.2, rgb(150, 160, 170)); // antiestática
    b.boxr(at + vec3(0.14, 0.015, 0.02), vec3(0.18, 0.02, 0.13), -0.3, rgb(40, 42, 46)); // Faraday
    b.cyl(at + vec3(0.33, 0.03, -0.03), 0.045, 0.06, rgb(220, 60, 50)); // fita lacre
}

/// Desenha um item e retorna a meia-extensão da sua caixa de seleção.
fn draw_item(b: &mut Batch, sim: &Sim, it: &Item, base: Vec3) -> Vec3 {
    match it.kind {
        ItemKind::Desktop | ItemKind::Servidor => tower(b, sim, it, base),
        ItemKind::Notebook => notebook(b, it, base),
        ItemKind::Celular | ItemKind::Tablet => phone(b, it, base),
        ItemKind::Hdd => {
            hdd(b, base + vec3(0.0, 0.015, 0.0), it.seal.is_some() && it.seal.as_ref().unwrap().intact);
            vec3(0.1, 0.03, 0.15)
        }
        ItemKind::Ssd => {
            b.bx(base + vec3(0.0, 0.006, 0.0), vec3(0.1, 0.012, 0.07), rgb(40, 44, 52));
            vec3(0.06, 0.02, 0.05)
        }
        ItemKind::Nvme => {
            nvme(b, base + vec3(0.0, 0.004, 0.0));
            vec3(0.06, 0.02, 0.03)
        }
        ItemKind::Pendrive => {
            b.bx(base + vec3(0.0, 0.01, 0.0), vec3(0.06, 0.02, 0.022), rgb(70, 120, 200));
            b.bx(base + vec3(0.04, 0.01, 0.0), vec3(0.02, 0.01, 0.014), rgb(190, 190, 196));
            if it.physical.camouflaged.is_some() {
                b.cyl(base + vec3(-0.05, 0.01, 0.0), 0.015, 0.006, rgb(200, 180, 60));
            }
            vec3(0.06, 0.03, 0.04)
        }
        ItemKind::CartaoSd => {
            b.bx(base + vec3(0.0, 0.002, 0.0), vec3(0.015, 0.003, 0.011), rgb(30, 30, 40));
            vec3(0.03, 0.02, 0.03)
        }
    }
}

fn seal_strip(b: &mut Batch, c: Vec3, size: Vec3, intact: bool) {
    let col = if intact { rgb(230, 60, 50) } else { rgb(150, 80, 70) };
    b.bx(c, size, col);
}

fn hdd(b: &mut Batch, at: Vec3, sealed: bool) {
    b.bx(at, vec3(0.10, 0.026, 0.147), rgb(150, 156, 164));
    b.bx(at + vec3(0.0, 0.014, 0.01), vec3(0.08, 0.002, 0.09), rgb(220, 222, 226));
    b.bx(at + vec3(0.0, 0.0, -0.075), vec3(0.07, 0.01, 0.004), rgb(30, 30, 30));
    if sealed {
        seal_strip(b, at + vec3(0.0, 0.016, 0.0), vec3(0.105, 0.003, 0.02), true);
    }
}

fn nvme(b: &mut Batch, at: Vec3) {
    b.bx(at, vec3(0.08, 0.004, 0.022), rgb(20, 90, 50));
    b.bx(at + vec3(-0.01, 0.003, 0.0), vec3(0.025, 0.003, 0.016), rgb(30, 30, 34));
    b.bx(at + vec3(0.02, 0.003, 0.0), vec3(0.02, 0.003, 0.016), rgb(30, 30, 34));
}

fn tower(b: &mut Batch, _sim: &Sim, it: &Item, base: Vec3) -> Vec3 {
    let w = 0.20;
    let h = 0.44;
    let d = 0.45;
    let c = base + vec3(0.0, h / 2.0, 0.0);
    let body = if it.is_server { rgb(36, 38, 44) } else { rgb(58, 62, 72) };
    let side_removed = it.components.iter().any(|c| c.kind == "tampa" && c.removed) || it.flag("aberto");
    // chassi: base, topo, traseira, lado direito; lado esquerdo = tampa removível
    b.bx(c + vec3(0.0, -h / 2.0 + 0.01, 0.0), vec3(w, 0.02, d), body);
    b.bx(c + vec3(0.0, h / 2.0 - 0.01, 0.0), vec3(w, 0.02, d), body);
    b.bx(c + vec3(0.0, 0.0, -d / 2.0 + 0.01), vec3(w, h, 0.02), body);
    b.bx(c + vec3(w / 2.0 - 0.005, 0.0, 0.0), vec3(0.01, h, d), body);
    // frente
    b.bx(c + vec3(0.0, 0.0, d / 2.0 - 0.01), vec3(w, h, 0.02), rgb(34, 36, 42));
    for k in 0..2 {
        b.bx(c + vec3(0.0, h / 2.0 - 0.07 - k as f32 * 0.045, d / 2.0 + 0.001), vec3(w * 0.8, 0.035, 0.003), rgb(48, 50, 58));
    }
    for k in 0..6 {
        b.bx(c + vec3(0.0, -0.05 - k as f32 * 0.022, d / 2.0 + 0.001), vec3(w * 0.7, 0.006, 0.003), rgb(22, 23, 27));
    }
    b.bx(c + vec3(-0.04, h / 2.0 - 0.05, d / 2.0 + 0.003), vec3(0.018, 0.018, 0.004), rgb(80, 84, 94));
    let led = if it.power_on { rgb(70, 240, 120) } else { rgb(40, 50, 40) };
    b.bx(c + vec3(0.04, h / 2.0 - 0.05, d / 2.0 + 0.002), vec3(0.015, 0.015, 0.004), led);
    if !side_removed {
        b.bx(c + vec3(-w / 2.0 + 0.005, 0.0, 0.0), vec3(0.01, h, d), Color::new(body.r * 1.08, body.g * 1.08, body.b * 1.08, 1.0));
    } else {
        // interior visível: placa-mãe na lateral direita interna
        b.bx(c + vec3(w / 2.0 - 0.02, 0.03, -0.02), vec3(0.01, 0.34, 0.32), rgb(30, 80, 50));
        // memórias
        for i in 0..2 {
            b.bx(c + vec3(w / 2.0 - 0.04, 0.12, 0.02 + i as f32 * 0.02), vec3(0.03, 0.12, 0.006), rgb(20, 20, 24));
        }
        for comp in &it.components {
            if comp.removed {
                continue;
            }
            match comp.kind.as_str() {
                "gpu" => b.bx(c + vec3(w / 2.0 - 0.06, -0.06, -0.03), vec3(0.08, 0.025, 0.26), rgb(40, 40, 46)),
                "dissipador" => {
                    b.bx(c + vec3(w / 2.0 - 0.06, 0.10, -0.08), vec3(0.07, 0.09, 0.09), rgb(170, 175, 182));
                    b.cyl(c + vec3(w / 2.0 - 0.06, 0.10, -0.03), 0.035, 0.012, rgb(40, 40, 44));
                }
                _ => {}
            }
        }
        // slots com mídias (inclusive o oculto, visível apenas se o componente foi removido)
        let mut k = 0.0;
        for sl in &it.slots {
            if sl.media_id.is_none() || sl.hidden_by.is_some() {
                continue;
            }
            if sl.iface == "NVMe" {
                let p = if sl.label.contains("oculto") { c + vec3(w / 2.0 - 0.03, 0.04, -0.08) } else { c + vec3(w / 2.0 - 0.03, -0.01, 0.02) };
                b.bx(p, vec3(0.006, 0.022, 0.08), rgb(20, 120, 60));
            } else {
                b.bx(c + vec3(0.0, -h / 2.0 + 0.05 + k, d / 2.0 - 0.1), vec3(0.10, 0.026, 0.147), rgb(150, 156, 164));
                k += 0.04;
            }
        }
    }
    if let Some(seal) = &it.seal {
        seal_strip(b, c + vec3(-w / 2.0 - 0.003, 0.0, 0.0), vec3(0.003, h * 0.9, 0.03), seal.intact);
    }
    // monitor ligado no local
    if it.power_on && it.live.is_some() {
        let mc = base + vec3(-0.35, 0.0, -0.1);
        b.bx(mc + vec3(0.0, 0.01, 0.0), vec3(0.18, 0.02, 0.12), rgb(30, 30, 34));
        b.bx(mc + vec3(0.0, 0.12, -0.02), vec3(0.03, 0.2, 0.02), rgb(30, 30, 34));
        b.bx(mc + vec3(0.0, 0.32, 0.0), vec3(0.56, 0.34, 0.03), rgb(24, 24, 28));
        let mounted = it.live.as_ref().map(|l| l.mounted_encrypted.is_some()).unwrap_or(false);
        b.bx(mc + vec3(0.0, 0.32, 0.017), vec3(0.52, 0.30, 0.004), if mounted { rgb(60, 130, 200) } else { rgb(30, 40, 60) });
        let net = it.live.as_ref().map(|l| l.network_connected).unwrap_or(false);
        if net {
            // cabo de rede até a parede
            b.bx(base + vec3(0.0, 0.05, -0.5), vec3(0.008, 0.008, 0.6), rgb(60, 120, 220));
        }
    }
    vec3(w / 2.0 + 0.02, h / 2.0, d / 2.0)
}

fn notebook(b: &mut Batch, it: &Item, base: Vec3) -> Vec3 {
    let open = it.flag("aberto");
    b.bx(base + vec3(0.0, 0.012, 0.0), vec3(0.34, 0.022, 0.24), rgb(70, 74, 82));
    b.bx(base + vec3(0.0, 0.0235, 0.03), vec3(0.28, 0.002, 0.1), rgb(34, 34, 38));
    let scr = if it.power_on { rgb(50, 110, 170) } else { rgb(18, 18, 22) };
    b.slab_tilt(base + vec3(0.0, 0.023, -0.118), 0.34, 0.22, 0.01, 0.25, rgb(70, 74, 82), scr);
    if open {
        // tampa inferior removida: NVMe visível ao lado
        nvme(b, base + vec3(0.24, 0.004, 0.05));
    }
    if let Some(seal) = &it.seal {
        seal_strip(b, base + vec3(0.17, 0.012, 0.0), vec3(0.004, 0.024, 0.2), seal.intact);
    }
    vec3(0.18, 0.12, 0.13)
}

fn phone(b: &mut Batch, it: &Item, base: Vec3) -> Vec3 {
    let ph = it.phone.as_ref();
    let tablet = it.kind == ItemKind::Tablet;
    let (w, d) = if tablet { (0.17, 0.25) } else { (0.075, 0.155) };
    let c = base + vec3(0.0, 0.006, 0.0);
    let rf = ph.map(|p| p.rf).unwrap_or(Rf::Nenhum);
    // corpo
    b.bx(c, vec3(w, 0.009, d), rgb(30, 32, 38));
    let screen = match ph {
        Some(p) if p.wiped => rgb(240, 240, 240),
        Some(p) if p.boot == Boot::Desligado => rgb(10, 10, 12),
        Some(p) if p.boot == Boot::Desbloqueado => rgb(80, 150, 220),
        Some(_) => rgb(28, 40, 70),
        None => rgb(10, 10, 12),
    };
    b.bx(c + vec3(0.0, 0.0047, 0.0), vec3(w * 0.9, 0.0008, d * 0.92), screen);
    // moldura, alto-falante, câmera frontal e botões laterais
    b.bx(c + vec3(0.0, 0.0049, -d * 0.475), vec3(w * 0.25, 0.0006, 0.004), rgb(50, 52, 58));
    b.bx(c + vec3(w * 0.2, 0.0049, -d * 0.475), vec3(0.006, 0.0006, 0.006), rgb(8, 8, 10));
    b.bx(c + vec3(w / 2.0 + 0.002, 0.0, -d * 0.18), vec3(0.003, 0.005, 0.03), rgb(60, 62, 70));
    b.bx(c + vec3(-w / 2.0 - 0.002, 0.0, -d * 0.22), vec3(0.003, 0.005, 0.045), rgb(60, 62, 70));
    if ph.map(|p| p.screen_broken).unwrap_or(false) || it.physical.damage.iter().any(|x| x.contains("tela")) {
        for k in 0..4 {
            b.boxr(c + vec3(-0.01 + k as f32 * 0.006, 0.0053, 0.02 - k as f32 * 0.01), vec3(0.002, 0.0006, 0.07), 0.4 + k as f32 * 0.5, rgb(230, 230, 230));
        }
    }
    if ph.map(|p| p.front_camera_covered).unwrap_or(false) || it.flag("etiqueta_camera") {
        b.bx(c + vec3(0.0, 0.0056, -d * 0.43), vec3(0.016, 0.0008, 0.01), rgb(250, 230, 120));
    }
    // bandeja do SIM (projeção lateral quando removido)
    if let Some(p) = ph {
        let removed = p.sims.iter().any(|s| s.place != SimWhere::Slot);
        if removed {
            b.bx(c + vec3(w / 2.0 + 0.012, 0.0, 0.02), vec3(0.02, 0.002, 0.03), rgb(190, 190, 196));
            b.bx(base + vec3(w / 2.0 + 0.05, 0.002, 0.02), vec3(0.015, 0.002, 0.012), rgb(220, 180, 60));
        }
        if p.external_battery || p.charging {
            b.bx(base + vec3(-w / 2.0 - 0.07, 0.012, 0.0), vec3(0.06, 0.022, 0.11), rgb(230, 230, 236));
            b.bx(base + vec3(-w / 2.0 - 0.02, 0.006, 0.05), vec3(0.06, 0.004, 0.004), rgb(30, 30, 30));
        }
    }
    if it.physical.wet {
        b.bx(base + vec3(0.0, 0.0005, 0.0), vec3(w * 1.6, 0.001, d * 1.3), Color::new(0.4, 0.6, 0.9, 0.9));
    }
    // envoltório de isolamento (desenhado por último, semitransparente não suportado: usa malha aberta)
    match rf {
        Rf::FaradayBag => {
            b.bx(c + vec3(0.0, 0.012, 0.0), vec3(w * 1.5, 0.02, d * 1.25), rgb(40, 42, 46));
            b.bx(c + vec3(0.0, 0.023, d * 0.45), vec3(w * 1.5, 0.004, 0.02), rgb(70, 72, 78));
        }
        Rf::Aluminio3 | Rf::AluminioInsuficiente => {
            let col = if rf == Rf::Aluminio3 { rgb(200, 204, 210) } else { rgb(170, 174, 180) };
            b.bx(c + vec3(0.0, 0.01, 0.0), vec3(w * 1.15, 0.016, if rf == Rf::Aluminio3 { d * 1.08 } else { d * 0.7 }), col);
        }
        _ => {}
    }
    if let Some(seal) = &it.seal {
        seal_strip(b, c + vec3(0.0, 0.03, 0.0), vec3(w * 1.6, 0.002, 0.012), seal.intact);
    }
    if it.flag("embalado") {
        b.bx(c + vec3(0.0, 0.02, 0.0), vec3(w * 1.8, 0.03, d * 1.4), Color::new(0.75, 0.78, 0.82, 1.0));
    }
    vec3(w * 0.9, 0.04, d * 0.7)
}
