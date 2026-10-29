//! Mundo 3D em primeira pessoa: sala recriada em escala real (laboratório ou local de
//! busca), mobília com colisores, estação pericial dentro da sala, equipamentos com
//! materiais texturizados e objetos interativos.

use crate::render::*;
use macroquad::prelude::*;
use pericia_core::engine::Sim;
use pericia_core::model::*;
use pericia_core::model::Vestigio;

pub const EYE: f32 = 1.62;

#[derive(Clone)]
pub struct Pick {
    pub id: String,
    pub center: Vec3,
    pub half: Vec3,
    pub label: String,
}

pub struct World {
    pub picks: Vec<Pick>,
    /// Retângulos de colisão no plano XZ: (mín, máx).
    pub colliders: Vec<(Vec2, Vec2)>,
    pub bounds: (Vec2, Vec2),
    pub lights: [(Vec3, Vec3); 3],
    /// Onde o perito fica ao sentar na estação (posição, yaw).
    pub station_seat: (Vec3, f32),
    pub start: (Vec3, f32),
}

fn col(min: (f32, f32), max: (f32, f32)) -> (Vec2, Vec2) {
    (vec2(min.0, min.1), vec2(max.0, max.1))
}

// ------------------------------------------------------------------ materiais da cena

fn plastic(c: Color) -> M {
    m(Tex::Plastic, c, 0.3, 0.35)
}
fn metal(c: Color) -> M {
    m(Tex::Metal, c, 0.4, 0.9)
}
fn plain(c: Color) -> M {
    m(Tex::White, c, 1.0, 0.2)
}
fn glow(c: Color) -> M {
    emis(Tex::White, c)
}

pub fn build(r: &mut Renderer, sim: &Sim, selected: Option<&str>, hover: Option<&str>) -> World {
    let local = sim.case.setting == Setting::Local;
    let mut w = World {
        picks: vec![],
        colliders: vec![],
        bounds: (vec2(-3.25, -2.75), vec2(3.25, 2.75)),
        lights: [(vec3(-1.2, 2.6, -0.4), vec3(1.0, 0.96, 0.88)), (vec3(1.4, 2.6, 0.9), vec3(0.95, 0.93, 0.88)), (vec3(-3.3, 1.7, 0.6), vec3(0.35, 0.42, 0.55))],
        station_seat: (vec3(0.0, 0.0, 0.0), 0.0),
        start: (vec3(0.0, 0.0, 2.1), std::f32::consts::PI),
    };
    room(r, local);
    // pontos de apoio para os itens
    let visible: Vec<&Item> = sim.case.items.iter().filter(|i| !i.hidden || i.flag("descoberto")).collect();
    let (surface_y, span_x, base_z) = if local { (0.765, 1.5, -1.05) } else { (0.905, 2.0, -0.45) };
    if local {
        office(r, &mut w);
    } else {
        lab(r, &mut w);
    }
    let n = visible.len().max(1) as f32;
    let mut item_spots: Vec<(String, Vec3, Vec3, Vec3)> = vec![];
    for (k, it) in visible.iter().enumerate() {
        let x = -span_x / 2.0 + (k as f32 + 0.5) * (span_x / n);
        let mut base = vec3(x, surface_y, base_z);
        if local && it.hidden {
            base = vec3(1.45, 0.50, -0.70); // dentro da gaveta aberta
        }
        if local && it.kind.is_computer() && !it.power_on && it.id != "ITEM-01" {
            base = vec3(0.55, 0.0, -1.15); // gabinete no piso, sob a mesa
        }
        let half = draw_item(r, sim, it, base);
        r.shadow(base, half);
        let center = base + vec3(0.0, half.y, 0.0);
        let sel = selected == Some(it.id.as_str());
        let hov = hover == Some(it.id.as_str());
        if sel || hov {
            outline(r, center, half + Vec3::splat(0.015), if sel { Color::new(1.0, 0.85, 0.2, 1.0) } else { Color::new(0.6, 0.85, 1.0, 1.0) });
        }
        w.picks.push(Pick { id: it.id.clone(), center, half, label: format!("{} · {}", it.id, short_label(it)) });
        item_spots.push((it.id.clone(), base, center, half));
    }
    decor(r, local, &mut w, sim);
    vestigios(r, sim, &mut w, &item_spots, local, selected, hover);
    // ferramentas do perito (na bancada ou na mesa de apoio)
    let tools_at = if local { vec3(2.1, 0.72, 0.95) } else { vec3(-0.9, 0.905, 0.0) };
    let wb_on = sim.hashes.keys().any(|k| k.starts_with("wb:"));
    tool_writeblocker(r, tools_at, wb_on);
    w.picks.push(Pick { id: "TOOL-WB".into(), center: tools_at + vec3(0.0, 0.04, 0.0), half: vec3(0.12, 0.05, 0.08), label: "Bloqueador de escrita".into() });
    let eq = &sim.equip;
    // cadeia montada: cabo SATA/adaptador junto ao bloqueador, mídia retirada ao lado, cabo USB para a estação
    let mut k = 0.0;
    for (mid, dest) in &eq.linked {
        let at = tools_at + vec3(-0.12 - k, 0.0, 0.16);
        let mk = sim.case.media.iter().find(|m| &m.id == mid).map(|m| m.kind);
        match mk {
            Some(MediaKind::Hdd) => hdd(r, at + vec3(-0.08, 0.013, 0.0), false),
            Some(MediaKind::Nvme) => nvme(r, at + vec3(-0.08, 0.002, 0.0)),
            _ => {
                r.model("ssd_25", at + vec3(-0.08, 0.0, 0.0), 0.0, &[]);
            }
        }
        let cabo = match mk {
            Some(MediaKind::Nvme) => "adaptador_m2",
            Some(MediaKind::Sd) => "leitor_sd",
            Some(MediaKind::Usb) => "",
            _ => "cabo_sata",
        };
        if !cabo.is_empty() {
            r.model(cabo, at + vec3(0.02, 0.0, -0.02), 0.4, &[]);
        }
        if dest == "estacao" {
            r.bx(at + vec3(0.0, 0.004, -0.35), vec3(0.006, 0.006, 0.7), plastic(rgb(200, 40, 40)));
        }
        k += 0.22;
    }
    for mid in eq.removed.iter().filter(|m| !eq.linked.contains_key(*m)) {
        let at = tools_at + vec3(0.12 + k, 0.0, 0.18);
        match sim.case.media.iter().find(|m| &m.id == mid).map(|m| m.kind) {
            Some(MediaKind::Nvme) => nvme(r, at + vec3(0.0, 0.002, 0.0)),
            Some(MediaKind::Ssd) => {
                r.model("ssd_25", at, 0.0, &[]);
            }
            _ => hdd(r, at + vec3(0.0, 0.013, 0.0), false),
        }
        k += 0.16;
    }
    if eq.blocker_host {
        r.model("cabo_usb", tools_at + vec3(0.14, 0.0, -0.08), 0.0, &[]);
    }
    if eq.hand != Some(pericia_core::equip::Tool::Camera) {
        tool_camera(r, tools_at + vec3(0.35, 0.0, 0.05));
        w.picks.push(Pick { id: "TOOL-CAM".into(), center: tools_at + vec3(0.35, 0.06, 0.05), half: vec3(0.09, 0.06, 0.06), label: "Câmera fotográfica — clique para pegar".into() });
    }
    // maleta pericial (todas as ferramentas, luvas e pulseira)
    let (maleta_at, maleta_rot) = if local { (vec3(2.82, 0.72, 0.95), -std::f32::consts::FRAC_PI_2) } else { (vec3(2.0, 0.72, 1.0), -std::f32::consts::FRAC_PI_2) };
    if !local {
        table(r, vec3(2.0, 0.0, 1.0), vec3(1.0, 0.72, 0.6), metal(rgb(150, 154, 160)), metal(rgb(90, 92, 98)));
        w.colliders.push(col((1.5, 0.7), (2.5, 1.3)));
    }
    if !r.model("maleta_pericial", maleta_at, maleta_rot, &[]) {
        r.bx(maleta_at + vec3(0.0, 0.06, 0.0), vec3(0.4, 0.12, 0.55), plastic(rgb(40, 42, 46)));
    }
    let side = if local { vec3(-0.45, 0.0, -0.18) } else { vec3(-0.42, 0.0, -0.18) };
    r.model("caixa_luvas", maleta_at + side, 0.3, &[]);
    r.model("pulseira_esd", maleta_at + side + vec3(0.02, 0.0, 0.3), 0.0, &[]);
    w.picks.push(Pick { id: "MALETA".into(), center: maleta_at + vec3(0.0, 0.15, 0.0), half: vec3(0.3, 0.2, 0.32), label: "Maleta pericial — ferramentas, luvas e pulseira".into() });
    w
}

// ------------------------------------------------------------------ decoração

/// Peça de parede: `wall` = 'B' fundo (z−), 'F' frente (z+), 'E' esquerda (x−), 'D' direita (x+).
fn on_wall(r: &mut Renderer, name: &str, wall: char, pos: f32, y: f32) {
    let (at, rot) = match wall {
        'B' => (vec3(pos, y, -2.99), 0.0),
        'F' => (vec3(pos, y, 2.99), std::f32::consts::PI),
        'E' => (vec3(-3.49, y, pos), std::f32::consts::FRAC_PI_2),
        _ => (vec3(3.49, y, pos), -std::f32::consts::FRAC_PI_2),
    };
    r.model(name, at, rot, &[]);
}

fn decor(r: &mut Renderer, local: bool, w: &mut World, sim: &Sim) {
    if local {
        on_wall(r, "pintura_parede", 'B', 0.0, 1.45);
        on_wall(r, "calendario", 'B', 1.45, 1.05);
        on_wall(r, "relogio_parede", 'B', -1.25, 1.85);
        on_wall(r, "quadro_cortica", 'E', -1.6, 1.0);
        // objetos pessoais na mesa do investigado
        r.model("porta_retrato", vec3(0.72, 0.765, -1.33), 0.25, &[]);
        r.model("cinzeiro", vec3(-0.72, 0.765, -1.3), 0.0, &[]);
        if sim.case.vestigios.iter().any(|v| v.kind == "copo") {
            r.model("copo_cafe", vec3(0.42, 0.765, -1.32), 0.0, &[]);
        }
        r.model("lixeira", vec3(1.05, 0.0, -0.35), 0.0, &[]);
        w.colliders.push(col((0.88, -0.52), (1.22, -0.18)));
    } else {
        on_wall(r, "placa_lab", 'B', 0.62, 2.22);
        on_wall(r, "quadro_branco", 'B', 0.65, 1.0);
        on_wall(r, "cartaz_pop", 'B', -0.45, 0.95);
        on_wall(r, "quadro_cortica", 'E', -1.55, 1.0);
        on_wall(r, "cartaz_epi", 'F', 0.6, 1.0);
        on_wall(r, "relogio_parede", 'F', -1.2, 1.95);
        r.model("lixeira", vec3(1.55, 0.0, -1.35), 0.0, &[]);
        w.colliders.push(col((1.38, -1.52), (1.72, -1.18)));
    }
    r.model("extintor", vec3(2.82, 0.0, 2.84), std::f32::consts::PI, &[]);
    w.colliders.push(col((2.7, 2.7), (2.95, 2.95)));
}

// ------------------------------------------------------------------ vestígios

/// Mancha (decalque horizontal com a textura de sangue seco).
fn decal(r: &mut Renderer, c: Vec3, size: f32, rot: f32, color: Color) {
    let (s, co) = rot.sin_cos();
    let h = size / 2.0;
    let p = |x: f32, z: f32| c + vec3(x * co + z * s, 0.0, -x * s + z * co);
    let uv = [vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)];
    r.quad(Tex::Blood, [p(-h, -h), p(h, -h), p(h, h), p(-h, h)], uv, vec3(0.0, 1.0, 0.0), color, 0.3, false);
}

/// Fio de cabelo: segmentos finos numa curva suave.
fn hair(r: &mut Renderer, start: Vec3, len: f32, seed: f32) {
    let n = 14;
    let mut p = start;
    let mut a = seed;
    let seg = len / n as f32;
    for k in 0..n {
        a += (seed * 3.1 + k as f32 * 0.9).sin() * 0.35;
        let q = p + vec3(a.cos() * seg, 0.0, a.sin() * seg);
        r.boxr((p + q) / 2.0, vec3(seg * 1.1, 0.0012, 0.0016), -a, plain(rgb(38, 26, 20)));
        p = q;
    }
}

fn vestigio_pos(v: &Vestigio, spots: &[(String, Vec3, Vec3, Vec3)], local: bool) -> Vec3 {
    match v.spot.as_str() {
        "item" => {
            if let Some((_, _, c, h)) = spots.iter().find(|s| Some(&s.0) == v.on_item.as_ref()) {
                let dx = if v.kind == "cabelo" { -0.03 } else { 0.03 };
                return *c + vec3(dx, h.y + 0.003, 0.02);
            }
            vec3(0.0, 0.91, -0.2)
        }
        "piso_mesa" => {
            if v.kind == "pegada" {
                vec3(0.62, 0.009, -0.38)
            } else {
                vec3(0.38, 0.009, -0.55)
            }
        }
        "teclado" => vec3(-0.18, 0.771, -0.78),
        "cinzeiro" => vec3(-0.72, 0.80, -1.3),
        "mesa" => vec3(0.42, 0.87, -1.32),
        _ => if local { vec3(0.0, 0.77, -0.9) } else { vec3(0.0, 0.91, -0.2) },
    }
}

fn vestigios(r: &mut Renderer, sim: &Sim, w: &mut World, spots: &[(String, Vec3, Vec3, Vec3)], local: bool, selected: Option<&str>, hover: Option<&str>) {
    let dried = Color::new(0.62, 0.30, 0.27, 0.99);
    for (k, v) in sim.case.vestigios.iter().enumerate() {
        let p = vestigio_pos(v, spots, local);
        let seed = k as f32 * 1.7 + 0.4;
        let half = match v.kind.as_str() {
            "sangue" => {
                if v.spot == "item" {
                    decal(r, p, 0.07, seed, dried);
                    vec3(0.05, 0.02, 0.05)
                } else {
                    // gotas espaçadas, como num rastro
                    decal(r, p, 0.34, seed, dried);
                    for (j, (dx, dz, s)) in [(0.32, -0.12, 0.12), (0.52, -0.2, 0.09), (-0.28, 0.1, 0.1)].iter().enumerate() {
                        decal(r, p + vec3(*dx, 0.0005 * j as f32, *dz), *s, seed + j as f32, dried);
                    }
                    vec3(0.3, 0.03, 0.2)
                }
            }
            "pegada" => {
                for j in 0..3 {
                    r.boxr(p + vec3(0.0, 0.0005, -0.06 + j as f32 * 0.05), vec3(0.08, 0.001, 0.012), 0.3, M { t: Tex::Blood, c: Color::new(0.35, 0.18, 0.16, 0.99), tile: 1.0, spec: 0.1, emis: false });
                }
                vec3(0.08, 0.03, 0.1)
            }
            "cabelo" => {
                hair(r, p, if v.spot == "item" { 0.12 } else { 0.2 }, seed);
                vec3(0.08, 0.02, 0.06)
            }
            _ => vec3(0.07, 0.06, 0.07),
        };
        if let Some(n) = sim.equip.markers.get(&v.id) {
            let mp = if v.spot == "item" { spots.iter().find(|s| Some(&s.0) == v.on_item.as_ref()).map(|s| s.1 + vec3(0.18 + 0.05 * (*n as f32 % 2.0), 0.0, 0.12)).unwrap_or(p) } else { vec3(p.x + 0.2, if p.y < 0.1 { 0.0 } else { p.y - 0.006 }, p.z + 0.18) };
            r.model("placa_vestigio", mp, 0.0, &[]);
        }
        let center = p + vec3(0.0, half.y * 0.5, 0.0);
        let sel = selected == Some(v.id.as_str());
        if sel || hover == Some(v.id.as_str()) {
            outline(r, center, half + Vec3::splat(0.01), if sel { Color::new(1.0, 0.85, 0.2, 1.0) } else { Color::new(0.6, 0.85, 1.0, 1.0) });
        }
        w.picks.push(Pick { id: v.id.clone(), center, half, label: format!("{} · {}", v.id, v.label) });
    }
}


pub fn short_label(it: &Item) -> String {
    let mut s = it.kind.label().to_string();
    if it.kind.is_computer() {
        s.push_str(if it.power_on { " (ligado)" } else { " (desligado)" });
    }
    if let Some(p) = &it.phone {
        if p.wiped {
            s.push_str(" (APAGADO)");
        } else {
            s.push_str(&format!(" · {}", match p.boot {
                Boot::Desligado => "desligado",
                Boot::Bfu => "BFU",
                Boot::Afu => "AFU",
                Boot::Desbloqueado => "desbloqueado",
            }));
        }
    }
    s
}

fn outline(r: &mut Renderer, c: Vec3, h: Vec3, color: Color) {
    let t = 0.006;
    let mt = glow(color);
    for sx in [-1.0, 1.0] {
        for sz in [-1.0, 1.0] {
            r.bx(c + vec3(sx * h.x, 0.0, sz * h.z), vec3(t, h.y * 2.0, t), mt);
        }
        for sy in [-1.0, 1.0] {
            r.bx(c + vec3(sx * h.x, sy * h.y, 0.0), vec3(t, t, h.z * 2.0), mt);
            r.bx(c + vec3(0.0, sy * h.y, sx * h.z), vec3(h.x * 2.0, t, t), mt);
        }
    }
}

// ------------------------------------------------------------------ sala

fn room(r: &mut Renderer, local: bool) {
    let (w, d, h) = (7.0, 6.0, 2.8);
    let floor = if local { m(Tex::WoodFloor, rgb(255, 255, 255), 1.6, 0.25) } else { m(Tex::Vinyl, rgb(255, 255, 255), 1.2, 0.35) };
    r.bx(vec3(0.0, -0.05, 0.0), vec3(w, 0.1, d), floor);
    r.bx(vec3(0.0, h + 0.05, 0.0), vec3(w, 0.1, d), m(Tex::Ceiling, rgb(255, 255, 255), 1.2, 0.0));
    let wall = if local { m(Tex::Wall, rgb(236, 226, 205), 2.0, 0.05) } else { m(Tex::Wall, rgb(222, 228, 232), 2.0, 0.05) };
    // paredes (com abertura de janela na esquerda e porta na frente)
    r.bx(vec3(0.0, h / 2.0, -d / 2.0 - 0.05), vec3(w, h, 0.1), wall);
    r.bx(vec3(w / 2.0 + 0.05, h / 2.0, 0.0), vec3(0.1, h, d), wall);
    // parede esquerda em partes, deixando a janela (z 0..1.4, y 0.9..2.1)
    r.bx(vec3(-w / 2.0 - 0.05, h / 2.0, -1.75), vec3(0.1, h, 2.5), wall);
    r.bx(vec3(-w / 2.0 - 0.05, h / 2.0, 2.25), vec3(0.1, h, 1.5), wall);
    r.bx(vec3(-w / 2.0 - 0.05, 0.45, 0.75), vec3(0.1, 0.9, 1.5), wall);
    r.bx(vec3(-w / 2.0 - 0.05, 2.45, 0.75), vec3(0.1, 0.7, 1.5), wall);
    // janela: vidro emissivo (luz do dia), caixilho e peitoril
    r.boxr(vec3(-w / 2.0 - 0.09, 1.5, 0.75), vec3(0.02, 1.2, 1.5), 0.0, M { t: Tex::Outside, c: WHITE, tile: 1.5, spec: 0.0, emis: true });
    let frame = metal(rgb(225, 228, 232));
    r.bx(vec3(-w / 2.0 - 0.04, 1.5, 0.75), vec3(0.05, 1.24, 0.05), frame);
    r.bx(vec3(-w / 2.0 - 0.04, 1.5, 0.75), vec3(0.05, 0.05, 1.54), frame);
    r.bx(vec3(-w / 2.0 + 0.02, 0.89, 0.75), vec3(0.16, 0.04, 1.6), plain(rgb(235, 235, 232)));
    // parede da frente com porta
    r.bx(vec3(-1.0, h / 2.0, d / 2.0 + 0.05), vec3(5.0, h, 0.1), wall);
    r.bx(vec3(3.0, h / 2.0, d / 2.0 + 0.05), vec3(1.0, h, 0.1), wall);
    r.bx(vec3(2.0, 2.45, d / 2.0 + 0.05), vec3(1.0, 0.7, 0.1), wall);
    r.bx(vec3(2.0, 1.05, d / 2.0 + 0.03), vec3(0.92, 2.1, 0.05), m(Tex::Wood, rgb(200, 190, 175), 0.8, 0.3));
    r.bx(vec3(1.66, 1.0, d / 2.0 - 0.0), vec3(0.12, 0.03, 0.04), metal(rgb(200, 200, 205)));
    // rodapés
    let base = plain(if local { rgb(150, 120, 90) } else { rgb(90, 96, 104) });
    r.bx(vec3(0.0, 0.05, -d / 2.0 + 0.01), vec3(w, 0.1, 0.02), base);
    r.bx(vec3(w / 2.0 - 0.01, 0.05, 0.0), vec3(0.02, 0.1, d), base);
    // luminárias embutidas (emissivas)
    for (x, z) in [(-1.2, -0.4), (1.4, 0.9)] {
        r.bx(vec3(x, h - 0.01, z), vec3(1.2, 0.02, 0.6), metal(rgb(230, 230, 232)));
        r.bx(vec3(x, h - 0.025, z), vec3(1.1, 0.01, 0.5), glow(rgb(255, 250, 236)));
    }
    // tomada e interruptor
    r.bx(vec3(1.2, 0.3, -d / 2.0 + 0.005), vec3(0.08, 0.12, 0.01), plain(rgb(240, 240, 240)));
    r.bx(vec3(2.65, 1.2, d / 2.0 - 0.005), vec3(0.08, 0.12, 0.01), plain(rgb(240, 240, 240)));
}

fn table(r: &mut Renderer, c: Vec3, size: Vec3, top: M, legs: M) {
    // c = centro no piso; size = (largura, altura, profundidade)
    let ty = size.y;
    r.bx(c + vec3(0.0, ty - 0.02, 0.0), vec3(size.x, 0.04, size.z), top);
    for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        r.bx(c + vec3(sx * (size.x / 2.0 - 0.05), (ty - 0.04) / 2.0, sz * (size.z / 2.0 - 0.05)), vec3(0.045, ty - 0.04, 0.045), legs);
    }
}

fn chair(r: &mut Renderer, c: Vec3, rot: f32) {
    if r.model("cadeira_escritorio", c, rot, &[]) {
        return;
    }
    let fabric = m(Tex::Fabric, rgb(255, 255, 255), 0.25, 0.05);
    let dark = metal(rgb(60, 62, 68));
    r.cyl(c + vec3(0.0, 0.05, 0.0), 0.28, 0.04, dark);
    r.cyl(c + vec3(0.0, 0.25, 0.0), 0.03, 0.4, dark);
    r.boxr(c + vec3(0.0, 0.47, 0.0), vec3(0.48, 0.08, 0.46), rot, fabric);
    let (s, co) = rot.sin_cos();
    let back = vec3(0.0 * co + 0.24 * s, 0.0, -0.0 * s + 0.24 * co);
    r.boxr(c + back + vec3(0.0, 0.8, 0.0), vec3(0.46, 0.55, 0.07), rot, fabric);
}

fn monitor(r: &mut Renderer, at: Vec3, screen: M, scale: f32) {
    if r.model_s("monitor", at, 0.0, scale, &[("tela", screen)]) {
        return;
    }
    let s = scale;
    let dark = plastic(rgb(40, 42, 46));
    r.bx(at + vec3(0.0, 0.012, 0.0) * s, vec3(0.24, 0.024, 0.16) * s, dark);
    r.bx(at + vec3(0.0, 0.14, -0.03) * s, vec3(0.05, 0.24, 0.03) * s, dark);
    r.bx(at + vec3(0.0, 0.36, -0.01) * s, vec3(0.62, 0.38, 0.03) * s, dark);
    let z = at.z + (-0.01 + 0.0152) * s;
    let (hx, y0, y1) = (0.295 * s, at.y + 0.185 * s, at.y + 0.535 * s);
    r.panel(vec3(at.x - hx, y1, z), vec3(at.x + hx, y1, z), vec3(at.x + hx, y0, z), vec3(at.x - hx, y0, z), vec3(0.0, 0.0, 1.0), screen, false);
}

/// Estação pericial completa: mesa, monitor com a tela capturada, gabinete, teclado, mouse.
pub fn workstation(r: &mut Renderer, c: Vec3, w: &mut World) {
    if !r.model("mesa_estacao", c, 0.0, &[]) {
        table(r, c, vec3(1.5, 0.75, 0.75), m(Tex::Wood, rgb(230, 220, 205), 0.9, 0.35), metal(rgb(70, 72, 78)));
    }
    let top = c.y + 0.75;
    monitor(r, vec3(c.x, top, c.z - 0.18), emis(Tex::Desktop, WHITE), 1.15);
    if r.model("gabinete_pc", vec3(c.x + 0.58, top, c.z - 0.05), 0.0, &[]) {
        r.bx(vec3(c.x + 0.63, top + 0.415, c.z + 0.183), vec3(0.01, 0.01, 0.003), glow(rgb(60, 220, 120)));
    } else {
        r.bx(vec3(c.x + 0.58, top + 0.23, c.z - 0.05), vec3(0.2, 0.46, 0.44), plastic(rgb(48, 50, 56)));
        r.bx(vec3(c.x + 0.58, top + 0.23, c.z + 0.171), vec3(0.17, 0.42, 0.003), m(Tex::Vent, WHITE, 0.08, 0.2));
        r.bx(vec3(c.x + 0.52, top + 0.42, c.z + 0.174), vec3(0.012, 0.012, 0.003), glow(rgb(60, 220, 120)));
    }
    if !r.model("teclado", vec3(c.x - 0.02, top, c.z + 0.17), 0.0, &[]) {
        r.bx(vec3(c.x - 0.02, top + 0.012, c.z + 0.17), vec3(0.44, 0.022, 0.14), m(Tex::Keyboard, WHITE, 0.07, 0.2));
    }
    if !r.model("mouse", vec3(c.x + 0.32, top, c.z + 0.18), 0.0, &[]) {
        r.bx(vec3(c.x + 0.32, top + 0.012, c.z + 0.18), vec3(0.06, 0.024, 0.1), plastic(rgb(36, 36, 40)));
    }
    // bloqueador de escrita da estação
    r.bx(vec3(c.x - 0.55, top + 0.03, c.z + 0.05), vec3(0.16, 0.05, 0.1), plastic(rgb(200, 70, 40)));
    chair(r, vec3(c.x, 0.0, c.z + 0.7), 0.0);
    w.colliders.push(col((c.x - 0.78, c.z - 0.4), (c.x + 0.78, c.z + 0.4)));
    w.picks.push(Pick { id: "STATION".into(), center: vec3(c.x, top + 0.36 * 1.15, c.z - 0.18), half: vec3(0.38, 0.24, 0.06), label: "Estação Pericial — clique para sentar e usar".into() });
    w.station_seat = (vec3(c.x, 0.0, c.z + 0.85), std::f32::consts::PI);
}

fn lab(r: &mut Renderer, w: &mut World) {
    // bancada central com tampo antiestático e gaveteiros
    let bench_c = vec3(0.0, 0.0, -0.45);
    let bench_model = r.model("bancada_lab", bench_c, 0.0, &[]);
    if !bench_model {
        r.bx(bench_c + vec3(0.0, 0.885, 0.0), vec3(2.6, 0.04, 1.1), m(Tex::Mat, WHITE, 0.5, 0.1));
        r.bx(bench_c + vec3(0.0, 0.84, 0.0), vec3(2.64, 0.05, 1.14), metal(rgb(170, 176, 182)));
    }
    for sx in [-1.0, 1.0] {
        if bench_model {
            break;
        }
        r.bx(bench_c + vec3(sx * 0.95, 0.41, 0.0), vec3(0.6, 0.82, 1.0), metal(rgb(196, 200, 206)));
        for k in 0..3 {
            r.bx(bench_c + vec3(sx * 0.95, 0.15 + k as f32 * 0.26, 0.505), vec3(0.56, 0.22, 0.01), metal(rgb(214, 218, 222)));
            r.bx(bench_c + vec3(sx * 0.95, 0.22 + k as f32 * 0.26, 0.515), vec3(0.18, 0.02, 0.02), metal(rgb(120, 124, 130)));
        }
    }
    // lâmpada articulada da bancada
    let lamp_model = r.model("luminaria_bancada", bench_c + vec3(1.1, 0.905, -0.4), 0.0, &[]);
    if !lamp_model {
    r.cyl(bench_c + vec3(1.1, 0.92, -0.4), 0.07, 0.02, metal(rgb(60, 62, 68)));
    r.bx(bench_c + vec3(1.1, 1.15, -0.4), vec3(0.02, 0.46, 0.02), metal(rgb(60, 62, 68)));
    r.bx(bench_c + vec3(0.95, 1.37, -0.3), vec3(0.3, 0.02, 0.02), metal(rgb(60, 62, 68)));
    r.bx(bench_c + vec3(0.8, 1.33, -0.25), vec3(0.14, 0.06, 0.1), metal(rgb(60, 62, 68)));
    r.bx(bench_c + vec3(0.8, 1.298, -0.25), vec3(0.12, 0.005, 0.08), glow(rgb(255, 248, 225)));
    }
    w.colliders.push(col((-1.32, -1.0), (1.32, 0.1)));
    // estação pericial junto à parede do fundo
    workstation(r, vec3(-1.6, 0.0, -2.5), w);
    // armário de evidências e estante
    if r.model("armario_aco", vec3(2.6, 0.0, -2.65), 0.0, &[]) {
        r.bx(vec3(2.6, 1.6, -2.325), vec3(0.3, 0.12, 0.005), m(Tex::Label, WHITE, 0.3, 0.0));
    } else {
        r.bx(vec3(2.6, 1.0, -2.65), vec3(1.0, 2.0, 0.6), metal(rgb(150, 156, 164)));
        r.bx(vec3(2.6, 1.0, -2.345), vec3(0.01, 1.9, 0.005), plain(rgb(40, 40, 44)));
        for sx in [-1.0, 1.0] {
            r.bx(vec3(2.6 + sx * 0.06, 1.05, -2.33), vec3(0.02, 0.18, 0.03), metal(rgb(90, 92, 98)));
        }
        r.bx(vec3(2.6, 1.6, -2.34), vec3(0.3, 0.12, 0.005), m(Tex::Label, WHITE, 0.3, 0.0));
    }
    w.colliders.push(col((2.1, -2.95), (3.1, -2.35)));
    shelf(r, vec3(3.05, 0.0, 0.2), w);
}

fn shelf(r: &mut Renderer, c: Vec3, w: &mut World) {
    if !r.model("estante_aco", c, -std::f32::consts::FRAC_PI_2, &[]) {
        let mt = metal(rgb(120, 126, 134));
        for k in 0..5 {
            r.bx(c + vec3(0.0, 0.1 + k as f32 * 0.45, 0.0), vec3(0.4, 0.03, 1.4), mt);
        }
        for sz in [-1.0, 1.0] {
            r.bx(c + vec3(0.0, 1.0, sz * 0.69), vec3(0.4, 2.0, 0.03), mt);
        }
    }
    // caixas de evidência
    for k in 0..4 {
        let z = -0.45 + k as f32 * 0.32;
        r.bx(c + vec3(0.0, 0.25 + (k % 2) as f32 * 0.45, z), vec3(0.34, 0.26, 0.28), m(Tex::Cardboard, WHITE, 0.4, 0.05));
        r.bx(c + vec3(-0.171, 0.28 + (k % 2) as f32 * 0.45, z), vec3(0.003, 0.08, 0.14), m(Tex::Label, WHITE, 0.2, 0.0));
    }
    w.colliders.push(col((c.x - 0.25, c.z - 0.72), (c.x + 0.25, c.z + 0.72)));
}

fn office(r: &mut Renderer, w: &mut World) {
    // tapete, mesa de trabalho do investigado, cadeira, gaveteiro, estante, planta
    r.bx(vec3(0.0, 0.003, -0.4), vec3(2.6, 0.006, 1.8), m(Tex::Rug, WHITE, 2.6, 0.0));
    if !r.model("mesa_escritorio", vec3(0.0, 0.0, -1.05), 0.0, &[]) {
        table(r, vec3(0.0, 0.0, -1.05), vec3(1.8, 0.76, 0.8), m(Tex::Wood, WHITE, 0.9, 0.4), m(Tex::Wood, rgb(160, 140, 120), 0.6, 0.2));
    }
    w.colliders.push(col((-0.95, -1.5), (0.95, -0.6)));
    chair(r, vec3(0.0, 0.0, -0.35), 0.0);
    // gaveteiro com gaveta aberta (onde pode haver mídia camuflada)
    let gv = vec3(1.45, 0.0, -1.0);
    if !r.model("gaveteiro", gv, 0.0, &[]) {
        r.bx(gv + vec3(0.0, 0.35, 0.0), vec3(0.45, 0.7, 0.55), m(Tex::Wood, rgb(210, 190, 170), 0.6, 0.3));
        r.bx(gv + vec3(0.0, 0.48, 0.32), vec3(0.4, 0.13, 0.1), m(Tex::Wood, rgb(200, 180, 160), 0.6, 0.3));
    }
    w.colliders.push(col((1.2, -1.3), (1.7, -0.6)));
    // estante com livros
    let est = vec3(-2.6, 0.0, -2.75);
    let est_model = r.model("estante_livros", est, 0.0, &[]);
    for k in 0..5 {
        if est_model {
            break;
        }
        r.bx(est + vec3(0.0, 0.05 + k as f32 * 0.45, 0.0), vec3(1.3, 0.03, 0.35), m(Tex::Wood, WHITE, 0.9, 0.3));
    }
    for sx in [-1.0, 1.0] {
        if est_model {
            break;
        }
        r.bx(est + vec3(sx * 0.64, 0.95, 0.0), vec3(0.03, 1.9, 0.35), m(Tex::Wood, WHITE, 0.9, 0.3));
    }
    for k in 0..3 {
        if est_model {
            break;
        }
        for b in 0..9 {
            let hgt = 0.22 + ((b * 7 + k * 3) % 5) as f32 * 0.02;
            let colr = [rgb(150, 40, 40), rgb(40, 70, 130), rgb(60, 110, 60), rgb(180, 150, 60), rgb(90, 60, 110)][(b + k) % 5];
            r.bx(est + vec3(-0.55 + b as f32 * 0.12, 0.08 + k as f32 * 0.45 + hgt / 2.0, 0.0), vec3(0.08, hgt, 0.24), plain(colr));
        }
    }
    // roteador na estante (a rede do local)
    r.bx(est + vec3(0.3, 1.43, 0.0), vec3(0.28, 0.05, 0.18), plastic(rgb(30, 30, 34)));
    for k in 0..3 {
        r.bx(est + vec3(0.2 + k as f32 * 0.04, 1.456, 0.091), vec3(0.008, 0.006, 0.002), glow(rgb(80, 230, 120)));
    }
    w.colliders.push(col((-3.3, -2.95), (-1.9, -2.55)));
    // planta
    let pl = vec3(-2.9, 0.0, 2.3);
    let pl_model = r.model("planta", pl, 0.0, &[]);
    if !pl_model {
        r.cyl(pl + vec3(0.0, 0.2, 0.0), 0.2, 0.4, m(Tex::Concrete, rgb(180, 120, 90), 0.4, 0.1));
    }
    for k in 0..7 {
        if pl_model {
            break;
        }
        let a = k as f32 * 0.9;
        r.boxr(pl + vec3(a.cos() * 0.12, 0.6 + (k % 3) as f32 * 0.1, a.sin() * 0.12), vec3(0.08, 0.35, 0.02), a, m(Tex::Leaf, WHITE, 0.3, 0.2));
    }
    w.colliders.push(col((-3.15, 2.05), (-2.65, 2.55)));
    // mesa de apoio do perito com a estação portátil
    workstation(r, vec3(2.45, 0.0, -1.8), w);
    table(r, vec3(2.55, 0.0, 0.95), vec3(1.2, 0.72, 0.6), metal(rgb(150, 154, 160)), metal(rgb(90, 92, 98)));
    w.colliders.push(col((1.9, 0.6), (3.2, 1.3)));
}

// ------------------------------------------------------------------ ferramentas

fn tool_writeblocker(r: &mut Renderer, at: Vec3, on: bool) {
    if r.model("bloqueador_escrita", at, 0.0, &[]) {
        // LEDs de estado (energia / atividade) por cima do modelo
        r.bx(at + vec3(-0.06, 0.071, 0.06), vec3(0.016, 0.003, 0.01), if on { glow(rgb(60, 240, 110)) } else { plain(rgb(40, 60, 40)) });
        r.bx(at + vec3(-0.03, 0.071, 0.06), vec3(0.016, 0.003, 0.01), if on { glow(rgb(250, 190, 40)) } else { plain(rgb(60, 50, 30)) });
        return;
    }
    r.bx(at + vec3(0.0, 0.035, 0.0), vec3(0.22, 0.07, 0.14), plastic(rgb(205, 72, 40)));
    r.bx(at + vec3(-0.06, 0.071, 0.06), vec3(0.018, 0.004, 0.012), if on { glow(rgb(60, 240, 110)) } else { plain(rgb(40, 60, 40)) });
    r.bx(at + vec3(-0.03, 0.071, 0.06), vec3(0.018, 0.004, 0.012), if on { glow(rgb(250, 190, 40)) } else { plain(rgb(60, 50, 30)) });
    r.bx(at + vec3(0.04, 0.071, 0.0), vec3(0.08, 0.002, 0.05), m(Tex::Label, WHITE, 0.08, 0.0));
}

fn tool_camera(r: &mut Renderer, at: Vec3) {
    if r.model("camera_foto", at, 0.0, &[]) {
        return;
    }
    r.bx(at + vec3(0.0, 0.05, 0.0), vec3(0.15, 0.10, 0.08), plastic(rgb(30, 30, 34)));
    r.bx(at + vec3(-0.04, 0.105, 0.0), vec3(0.05, 0.02, 0.05), plastic(rgb(30, 30, 34)));
    r.boxr(at + vec3(0.0, 0.05, 0.06), vec3(0.07, 0.07, 0.05), 0.0, metal(rgb(50, 50, 56)));
    r.bx(at + vec3(0.0, 0.05, 0.086), vec3(0.05, 0.05, 0.002), glow(rgb(20, 30, 50)));
}


// ------------------------------------------------------------------ equipamentos

fn draw_item(r: &mut Renderer, sim: &Sim, it: &Item, base: Vec3) -> Vec3 {
    match it.kind {
        ItemKind::Desktop | ItemKind::Servidor => tower(r, sim, it, base),
        ItemKind::Notebook => notebook(r, it, base),
        ItemKind::Celular | ItemKind::Tablet => phone(r, it, base),
        ItemKind::Hdd => {
            hdd(r, base + vec3(0.0, 0.013, 0.0), it.seal.as_ref().map(|s| s.intact).unwrap_or(false));
            vec3(0.06, 0.03, 0.08)
        }
        ItemKind::Ssd => {
            if !r.model("ssd_25", base, 0.0, &[]) {
                r.bx(base + vec3(0.0, 0.0035, 0.0), vec3(0.07, 0.007, 0.1), metal(rgb(60, 64, 72)));
                r.bx(base + vec3(0.0, 0.0072, 0.0), vec3(0.06, 0.0005, 0.07), m(Tex::Label, WHITE, 0.07, 0.0));
            }
            vec3(0.05, 0.02, 0.06)
        }
        ItemKind::Nvme => {
            nvme(r, base + vec3(0.0, 0.002, 0.0));
            vec3(0.05, 0.02, 0.03)
        }
        ItemKind::Pendrive => {
            if !r.model("pendrive", base, 0.0, &[]) {
                r.bx(base + vec3(0.0, 0.006, 0.0), vec3(0.055, 0.012, 0.02), plastic(rgb(70, 120, 200)));
                r.bx(base + vec3(0.036, 0.006, 0.0), vec3(0.018, 0.0045, 0.012), metal(rgb(200, 200, 206)));
            }
            if it.physical.camouflaged.is_some() {
                r.cyl(base + vec3(-0.045, 0.006, 0.0), 0.014, 0.01, plastic(rgb(200, 180, 60)));
            }
            vec3(0.06, 0.02, 0.03)
        }
        ItemKind::CartaoSd => {
            r.bx(base + vec3(0.0, 0.0006, 0.0), vec3(0.011, 0.0012, 0.015), plastic(rgb(30, 30, 40)));
            vec3(0.02, 0.01, 0.02)
        }
    }
}

fn seal(r: &mut Renderer, c: Vec3, size: Vec3, intact: bool) {
    r.bx(c, size, plain(if intact { rgb(220, 50, 45) } else { rgb(140, 80, 70) }));
}

fn hdd(r: &mut Renderer, at: Vec3, sealed: bool) {
    if r.model("hd_35", at - vec3(0.0, 0.013, 0.0), 0.0, &[]) {
        if sealed {
            seal(r, at + vec3(0.0, 0.0142, -0.02), vec3(0.11, 0.0006, 0.02), true);
        }
        return;
    }
    r.bx(at, vec3(0.102, 0.026, 0.147), metal(rgb(170, 175, 182)));
    r.bx(at + vec3(0.0, 0.0132, 0.012), vec3(0.085, 0.0005, 0.095), m(Tex::Label, WHITE, 0.09, 0.0));
    r.bx(at + vec3(0.0, -0.002, -0.074), vec3(0.07, 0.012, 0.003), plain(rgb(25, 25, 25)));
    for k in 0..4 {
        r.cyl(at + vec3(-0.04 + (k % 2) as f32 * 0.08, 0.0135, -0.06 + (k / 2) as f32 * 0.12), 0.003, 0.001, metal(rgb(110, 110, 116)));
    }
    if sealed {
        seal(r, at + vec3(0.0, 0.0142, -0.02), vec3(0.11, 0.0006, 0.02), true);
    }
}

fn nvme(r: &mut Renderer, at: Vec3) {
    r.bx(at, vec3(0.08, 0.0016, 0.022), plain(rgb(20, 90, 50)));
    r.bx(at + vec3(-0.01, 0.0018, 0.0), vec3(0.025, 0.002, 0.016), plastic(rgb(30, 30, 34)));
    r.bx(at + vec3(0.02, 0.0018, 0.0), vec3(0.02, 0.002, 0.016), plastic(rgb(30, 30, 34)));
    r.bx(at + vec3(-0.037, 0.0, 0.0), vec3(0.006, 0.0018, 0.02), metal(rgb(210, 180, 80)));
}

fn tower(r: &mut Renderer, sim: &Sim, it: &Item, base: Vec3) -> Vec3 {
    let (w, h, d) = (0.21, 0.45, 0.46);
    let c = base + vec3(0.0, h / 2.0, 0.0);
    let body = plastic(if it.is_server { rgb(40, 42, 48) } else { rgb(62, 66, 76) });
    let open = it.components.iter().any(|c| c.kind == "tampa" && c.removed) || it.flag("aberto");
    if !open && r.model("gabinete_pc", base, 0.0, &[]) {
        r.bx(c + vec3(0.05, h / 2.0 - 0.035, d / 2.0 + 0.004), vec3(0.01, 0.01, 0.003), if it.power_on { glow(rgb(70, 240, 120)) } else { plain(rgb(40, 50, 40)) });
        if let Some(s) = &it.seal {
            seal(r, c + vec3(-w / 2.0 - 0.003, 0.0, 0.0), vec3(0.002, h * 0.9, 0.03), s.intact);
        }
        tower_live(r, it, base);
        return vec3(w / 2.0 + 0.02, h / 2.0, d / 2.0);
    }
    r.bx(c + vec3(0.0, -h / 2.0 + 0.01, 0.0), vec3(w, 0.02, d), body);
    r.bx(c + vec3(0.0, h / 2.0 - 0.01, 0.0), vec3(w, 0.02, d), body);
    r.bx(c + vec3(0.0, 0.0, -d / 2.0 + 0.01), vec3(w, h, 0.02), body);
    r.bx(c + vec3(w / 2.0 - 0.005, 0.0, 0.0), vec3(0.01, h, d), body);
    // frente com grade de ventilação, baias e botão
    r.bx(c + vec3(0.0, 0.0, d / 2.0 - 0.01), vec3(w, h, 0.02), plastic(rgb(34, 36, 42)));
    r.bx(c + vec3(0.0, -0.06, d / 2.0 + 0.001), vec3(w * 0.8, h * 0.55, 0.002), m(Tex::Vent, WHITE, 0.08, 0.2));
    for k in 0..2 {
        r.bx(c + vec3(0.0, h / 2.0 - 0.07 - k as f32 * 0.045, d / 2.0 + 0.001), vec3(w * 0.8, 0.035, 0.003), plastic(rgb(50, 52, 60)));
    }
    r.bx(c + vec3(-0.05, h / 2.0 - 0.035, d / 2.0 + 0.003), vec3(0.016, 0.016, 0.004), metal(rgb(90, 94, 104)));
    r.bx(c + vec3(0.05, h / 2.0 - 0.035, d / 2.0 + 0.003), vec3(0.01, 0.01, 0.003), if it.power_on { glow(rgb(70, 240, 120)) } else { plain(rgb(40, 50, 40)) });
    if !open {
        r.bx(c + vec3(-w / 2.0 + 0.005, 0.0, 0.0), vec3(0.01, h, d), plastic(rgb(70, 74, 84)));
    } else {
        r.bx(c + vec3(w / 2.0 - 0.02, 0.03, -0.02), vec3(0.01, 0.34, 0.32), plain(rgb(28, 82, 50)));
        for k in 0..2 {
            r.bx(c + vec3(w / 2.0 - 0.04, 0.12, 0.02 + k as f32 * 0.02), vec3(0.03, 0.12, 0.006), plain(rgb(20, 20, 24)));
        }
        for comp in &it.components {
            if comp.removed {
                continue;
            }
            match comp.kind.as_str() {
                "gpu" => r.bx(c + vec3(w / 2.0 - 0.06, -0.06, -0.03), vec3(0.08, 0.025, 0.26), plastic(rgb(40, 40, 46))),
                "dissipador" => {
                    r.bx(c + vec3(w / 2.0 - 0.06, 0.10, -0.08), vec3(0.07, 0.09, 0.09), metal(rgb(185, 190, 196)));
                    r.cyl(c + vec3(w / 2.0 - 0.06, 0.10, -0.03), 0.035, 0.012, plastic(rgb(40, 40, 44)));
                }
                _ => {}
            }
        }
        let mut k = 0.0;
        for sl in &it.slots {
            if sl.media_id.is_none() || sl.hidden_by.is_some() || sl.media_id.as_ref().map(|m| sim.equip.removed.contains(m)).unwrap_or(false) {
                continue;
            }
            if sl.iface == "NVMe" {
                let p = if sl.label.contains("oculto") { c + vec3(w / 2.0 - 0.03, 0.04, -0.08) } else { c + vec3(w / 2.0 - 0.03, -0.01, 0.02) };
                r.bx(p, vec3(0.004, 0.022, 0.08), plain(rgb(20, 120, 60)));
            } else {
                hdd(r, c + vec3(0.0, -h / 2.0 + 0.05 + k, d / 2.0 - 0.1), false);
                k += 0.04;
            }
        }
    }
    if let Some(s) = &it.seal {
        seal(r, c + vec3(-w / 2.0 - 0.002, 0.0, 0.0), vec3(0.002, h * 0.9, 0.03), s.intact);
    }
    tower_live(r, it, base);
    vec3(w / 2.0 + 0.02, h / 2.0, d / 2.0)
}

/// Monitor, teclado e cabo de rede do equipamento ligado no local.
fn tower_live(r: &mut Renderer, it: &Item, base: Vec3) {
    if it.power_on && it.live.is_some() {
        let mounted = it.live.as_ref().map(|l| l.mounted_encrypted.is_some()).unwrap_or(false);
        let scr = glow(if mounted { rgb(70, 140, 210) } else { rgb(30, 40, 60) });
        monitor(r, base + vec3(-0.42, 0.0, -0.05), scr, 1.0);
        if !r.model("teclado", base + vec3(-0.15, 0.0, 0.25), 0.0, &[]) {
            r.bx(base + vec3(-0.15, 0.011, 0.25), vec3(0.42, 0.022, 0.14), m(Tex::Keyboard, WHITE, 0.07, 0.2));
        }
        if it.live.as_ref().map(|l| l.network_connected).unwrap_or(false) {
            r.bx(base + vec3(0.0, 0.005, -0.55), vec3(0.008, 0.008, 0.6), plastic(rgb(60, 120, 220)));
        }
    }
}

fn notebook(r: &mut Renderer, it: &Item, base: Vec3) -> Vec3 {
    let body = metal(rgb(110, 114, 122));
    let face = if it.power_on { glow(rgb(60, 120, 180)) } else { plain(rgb(14, 14, 18)) };
    if !r.model("notebook", base, 0.0, &[("tela", face)]) {
        r.bx(base + vec3(0.0, 0.011, 0.0), vec3(0.34, 0.02, 0.24), body);
        r.bx(base + vec3(0.0, 0.0215, 0.02), vec3(0.28, 0.001, 0.11), m(Tex::Keyboard, WHITE, 0.06, 0.2));
        r.bx(base + vec3(0.0, 0.0215, 0.09), vec3(0.1, 0.001, 0.05), plastic(rgb(90, 94, 100)));
        r.slab_tilt(base + vec3(0.0, 0.021, -0.118), 0.34, 0.22, 0.008, 0.25, body, face);
    }
    if it.flag("aberto") {
        nvme(r, base + vec3(0.25, 0.002, 0.05));
    }
    if let Some(s) = &it.seal {
        seal(r, base + vec3(0.171, 0.011, 0.0), vec3(0.003, 0.022, 0.2), s.intact);
    }
    vec3(0.18, 0.12, 0.13)
}

fn phone(r: &mut Renderer, it: &Item, base: Vec3) -> Vec3 {
    let ph = it.phone.as_ref();
    let tablet = it.kind == ItemKind::Tablet;
    let (w, d) = if tablet { (0.17, 0.25) } else { (0.072, 0.152) };
    let c = base + vec3(0.0, 0.0045, 0.0);
    let rf = ph.map(|p| p.rf).unwrap_or(Rf::Nenhum);
    let scr = match ph {
        Some(p) if p.wiped => glow(rgb(235, 238, 245)),
        Some(p) if p.boot == Boot::Desligado => plain(rgb(8, 8, 10)),
        Some(p) if p.boot == Boot::Desbloqueado => glow(rgb(70, 140, 210)),
        Some(_) => glow(rgb(22, 34, 62)),
        None => plain(rgb(8, 8, 10)),
    };
    if tablet || !r.model("celular", base, 0.0, &[("tela", scr)]) {
        r.bx(c, vec3(w, 0.008, d), metal(rgb(40, 42, 48)));
        r.bx(c + vec3(0.0, 0.0, 0.0), vec3(w + 0.002, 0.006, d - 0.004), plastic(rgb(30, 32, 36)));
        r.bx(c + vec3(0.0, 0.0041, 0.0), vec3(w * 0.92, 0.0004, d * 0.94), scr);
        r.bx(c + vec3(w / 2.0 + 0.0015, 0.0, -d * 0.18), vec3(0.002, 0.004, 0.03), metal(rgb(70, 72, 80)));
    }
    r.bx(c + vec3(0.0, 0.0045, -d * 0.44), vec3(0.006, 0.0004, 0.006), plain(rgb(5, 5, 6)));
    if ph.map(|p| p.screen_broken).unwrap_or(false) || it.physical.damage.iter().any(|x| x.contains("tela")) {
        for k in 0..5 {
            r.boxr(c + vec3(-0.012 + k as f32 * 0.006, 0.0046, 0.015 - k as f32 * 0.012), vec3(0.0012, 0.0003, 0.06), 0.35 + k as f32 * 0.55, glow(rgb(220, 225, 230)));
        }
    }
    if ph.map(|p| p.front_camera_covered).unwrap_or(false) {
        r.bx(c + vec3(0.0, 0.0049, -d * 0.44), vec3(0.016, 0.0004, 0.012), plain(rgb(250, 230, 120)));
    }
    if let Some(p) = ph {
        if p.sims.iter().any(|s| s.place != SimWhere::Slot) {
            r.bx(c + vec3(w / 2.0 + 0.012, -0.001, 0.02), vec3(0.02, 0.0015, 0.03), metal(rgb(200, 200, 206)));
            r.bx(base + vec3(w / 2.0 + 0.05, 0.001, 0.02), vec3(0.015, 0.0008, 0.012), metal(rgb(220, 180, 60)));
        }
        if p.external_battery || p.charging {
            r.bx(base + vec3(-w / 2.0 - 0.07, 0.011, 0.0), vec3(0.065, 0.022, 0.12), plastic(rgb(232, 232, 238)));
            r.bx(base + vec3(-w / 2.0 - 0.04, 0.0235, -0.045), vec3(0.01, 0.002, 0.004), glow(rgb(80, 220, 120)));
            r.bx(base + vec3(-w / 2.0 - 0.02, 0.004, 0.05), vec3(0.05, 0.004, 0.004), plain(rgb(25, 25, 25)));
        }
    }
    if it.physical.wet {
        r.bx(base + vec3(0.0, 0.0004, 0.0), vec3(w * 1.7, 0.0006, d * 1.35), M { t: Tex::Glass, c: Color::new(0.5, 0.65, 0.9, 0.6), tile: 0.2, spec: 1.0, emis: false });
    }
    match rf {
        Rf::FaradayBag => {
            r.bx(c + vec3(0.0, 0.009, 0.0), vec3(w * 1.6, 0.02, d * 1.3), m(Tex::Fabric, rgb(70, 70, 74), 0.08, 0.15));
            r.bx(c + vec3(0.0, 0.0195, d * 0.5), vec3(w * 1.6, 0.003, 0.025), plastic(rgb(30, 30, 34)));
        }
        Rf::Aluminio3 | Rf::AluminioInsuficiente => {
            let cover = if rf == Rf::Aluminio3 { d * 1.1 } else { d * 0.7 };
            r.bx(c + vec3(0.0, 0.007, 0.0), vec3(w * 1.15, 0.016, cover), metal(rgb(215, 218, 224)));
        }
        _ => {}
    }
    if let Some(s) = &it.seal {
        seal(r, c + vec3(0.0, 0.03, 0.0), vec3(w * 1.7, 0.001, 0.012), s.intact);
    }
    if it.flag("embalado") {
        r.bx(c + vec3(0.0, 0.016, 0.0), vec3(w * 1.9, 0.03, d * 1.45), M { t: Tex::Glass, c: Color::new(0.85, 0.88, 0.92, 0.55), tile: 0.2, spec: 0.8, emis: false });
    }
    vec3(w * 0.9, 0.04, d * 0.7)
}

// ------------------------------------------------------------------ câmera e seleção

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

pub fn view_proj(cam: &Camera3D, aspect: f32) -> Mat4 {
    Mat4::perspective_rh_gl(cam.fovy, aspect, cam.z_near, cam.z_far) * Mat4::look_at_rh(cam.position, cam.target, cam.up)
}

pub fn project(cam: &Camera3D, viewport: Rect, p: Vec3) -> Option<Vec2> {
    let v = view_proj(cam, viewport.w / viewport.h) * p.extend(1.0);
    if v.w <= 0.0 {
        return None;
    }
    let n = v.truncate() / v.w;
    Some(vec2(viewport.x + (n.x + 1.0) * 0.5 * viewport.w, viewport.y + (1.0 - n.y) * 0.5 * viewport.h))
}

pub fn ray(cam: &Camera3D, viewport: Rect, p: Vec2) -> (Vec3, Vec3) {
    let ndc = vec2((p.x - viewport.x) / viewport.w * 2.0 - 1.0, 1.0 - (p.y - viewport.y) / viewport.h * 2.0);
    let inv = view_proj(cam, viewport.w / viewport.h).inverse();
    let a = inv.project_point3(vec3(ndc.x, ndc.y, -1.0));
    let b = inv.project_point3(vec3(ndc.x, ndc.y, 1.0));
    (a, (b - a).normalize())
}
