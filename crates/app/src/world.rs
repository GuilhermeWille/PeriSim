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
    /// Quadro de provas na parede: (canto superior esquerdo útil, direita, baixo, tamanho, normal).
    pub board: Option<(Vec3, Vec3, Vec3, Vec2, Vec3)>,
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

pub fn build(r: &mut Renderer, sim: &Sim, selected: Option<&str>, hover: Option<&str>, bench: &crate::bench::Bench) -> World {
    let local = sim.case.setting == Setting::Local;
    let mut w = World {
        picks: vec![],
        colliders: vec![],
        bounds: (vec2(-3.25, -2.75), vec2(3.25, 2.75)),
        lights: [(vec3(-1.2, 2.6, -0.4), vec3(1.0, 0.96, 0.88)), (vec3(1.4, 2.6, 0.9), vec3(0.95, 0.93, 0.88)), (vec3(-3.3, 1.7, 0.6), vec3(0.35, 0.42, 0.55))],
        station_seat: (vec3(0.0, 0.0, 0.0), 0.0),
        start: (vec3(0.0, 0.0, 2.1), std::f32::consts::PI),
        board: None,
    };
    if assar_vista().is_some() {
        outside_far(r);
        return w;
    }
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
    // bloqueador de escrita: na bancada de exame (laboratório) ou na mesa da estação portátil (local)
    let tools_at = if local { vec3(1.92, 0.75, -1.72) } else { vec3(-0.9, 0.905, 0.0) };
    let wb_on = sim.hashes.keys().any(|k| k.starts_with("wb:"));
    tool_writeblocker(r, tools_at, wb_on);
    w.picks.push(Pick { id: "TOOL-WB".into(), center: tools_at + vec3(0.0, 0.04, 0.0), half: vec3(0.12, 0.05, 0.08), label: "Bloqueador de escrita".into() });
    let eq = &sim.equip;
    // mídias retiradas e cadeia montada, ao lado do bloqueador (cada mídia é um alvo "M:<id>")
    let media_at = |k: usize| if local { tools_at + vec3(0.0, 0.0, -0.25 - 0.15 * k as f32) } else { tools_at + vec3(0.22 + 0.2 * k as f32, 0.0, -0.12) };
    let mut k = 0usize;
    let mut media: Vec<&String> = eq.linked.keys().collect();
    media.extend(eq.removed.iter().filter(|m| !eq.linked.contains_key(*m)));
    for mid in media {
        let at = media_at(k);
        let mk = sim.case.media.iter().find(|m| &m.id == mid).map(|m| m.kind);
        match mk {
            Some(MediaKind::Hdd) => hdd(r, at + vec3(0.0, 0.013, 0.0), false),
            Some(MediaKind::Nvme) => nvme(r, at + vec3(0.0, 0.002, 0.0)),
            _ => {
                r.model("ssd_25", at, 0.0, &[]);
            }
        }
        if let Some(dest) = eq.linked.get(mid) {
            // cabo/adaptador instalado entre a mídia e o bloqueador (ou direto na estação)
            let cabo = match mk {
                Some(MediaKind::Nvme) => "adaptador_m2",
                Some(MediaKind::Sd) => "leitor_sd",
                Some(MediaKind::Usb) => "",
                _ => "cabo_sata",
            };
            if !cabo.is_empty() {
                let mid_pt = (at + tools_at) * 0.5;
                r.model(cabo, mid_pt, 0.4, &[]);
            }
            if dest == "estacao" {
                r.bx(at + vec3(0.0, 0.004, -0.35), vec3(0.006, 0.006, 0.7), plastic(rgb(200, 40, 40)));
            }
        }
        let sel = selected == Some(format!("M:{}", mid).as_str()) || hover == Some(format!("M:{}", mid).as_str());
        let half = vec3(0.07, 0.03, 0.09);
        if sel {
            outline(r, at + vec3(0.0, 0.03, 0.0), half, Color::new(0.6, 0.85, 1.0, 1.0));
        }
        w.picks.push(Pick { id: format!("M:{}", mid), center: at + vec3(0.0, 0.03, 0.0), half, label: format!("Mídia {} (retirada)", mid) });
        k += 1;
    }
    if eq.blocker_host {
        r.model("cabo_usb", tools_at + vec3(0.14, 0.0, 0.08), 0.0, &[]);
    }
    // bancada de ferramentas (a do laboratório fica encostada na parede da frente)
    let (bc, bl, bd) = crate::bench::bench_geom(local);
    if !local {
        table(r, vec3(bc.x, 0.0, bc.z), vec3(bl, bc.y, bd), m(Tex::Mat, rgb(120, 200, 180), 0.5, 0.1), metal(rgb(90, 92, 98)));
        w.colliders.push(col((bc.x - bl / 2.0, bc.z - bd / 2.0), (bc.x + bl / 2.0, bc.z + bd / 2.0)));
    }
    let hand_id = eq.hand.and_then(|t| pericia_core::equip::TOOLS.iter().find(|x| x.0 == t).map(|x| x.1));
    for id in crate::bench::items() {
        if Some(id) == hand_id {
            continue;
        }
        let Some((pos, yaw)) = bench.get(local, id) else { continue };
        let tf = crate::bench::transform(r, id, pos, yaw);
        let name = crate::bench::model_of(id);
        r.model_m(name, tf, &[]);
        let (lo, hi) = r.model_aabb(name, tf).unwrap_or((pos - Vec3::splat(0.04), pos + Vec3::splat(0.04)));
        // caixa de seleção com tamanho mínimo: peças minúsculas (ejetor, pendrive) também são miráveis
        let center = (lo + hi) * 0.5;
        let half = ((hi - lo) * 0.5).max(vec3(0.03, 0.025, 0.03));
        let pid = format!("T:{}", id);
        if hover == Some(pid.as_str()) {
            outline(r, center, half + Vec3::splat(0.008), Color::new(0.6, 0.85, 1.0, 1.0));
        }
        w.picks.push(Pick { id: pid, center, half, label: crate::bench::name_of(id).to_string() });
    }
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

/// Quadro de provas (cortiça lisa) na parede esquerda: o jogador fixa fotos e bilhetes.
fn board_wall(r: &mut Renderer, w: &mut World, z: f32, y: f32) {
    on_wall(r, "quadro_provas", 'E', z, y);
    let (bw, bh, b) = (1.2, 0.8, 0.025);
    let origin = vec3(-3.49 + 0.0035, y + bh - b, z + bw / 2.0 - b);
    w.board = Some((origin, vec3(0.0, 0.0, -1.0), vec3(0.0, -1.0, 0.0), vec2(bw - 2.0 * b, bh - 2.0 * b), vec3(1.0, 0.0, 0.0)));
    w.picks.push(Pick { id: "QUADRO".into(), center: vec3(-3.45, y + bh / 2.0, z), half: vec3(0.05, bh / 2.0, bw / 2.0), label: "Quadro de provas — fixar fotos e bilhetes".into() });
}

fn decor(r: &mut Renderer, local: bool, w: &mut World, sim: &Sim) {
    if local {
        on_wall(r, "pintura_parede", 'B', 0.0, 1.45);
        on_wall(r, "calendario", 'B', 1.45, 1.05);
        on_wall(r, "relogio_parede", 'B', -1.25, 1.85);
        board_wall(r, w, -1.6, 1.0);
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
        board_wall(r, w, -1.55, 1.0);
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

// ------------------------------------------------------------------ exterior (visto pela janela)

/// Cor externa já "iluminada" (sem as luzes da sala) e com névoa atmosférica pela distância.
fn sky_haze(c: [f32; 3], light: f32, dist: f32) -> Color {
    let hz = [0.78, 0.84, 0.90];
    let k = 1.0 - (-dist / 220.0).exp();
    let f = |a: f32, b: f32| (a * light) * (1.0 - k) + b * k;
    Color::new(f(c[0], hz[0]), f(c[1], hz[1]), f(c[2], hz[2]), 1.0)
}

/// Caixa externa com sombreamento fixo por face (sol da tarde vindo da direita) e névoa.
fn out_box(r: &mut Renderer, c: Vec3, size: Vec3, t: Tex, tile: f32, base: [f32; 3]) {
    let d = (c - vec3(-3.5, 1.5, 0.75)).length();
    let h = size * 0.5;
    let shade = |n: Vec3| -> f32 {
        let sun = vec3(0.45, 0.75, 0.48).normalize();
        0.62 + 0.38 * n.dot(sun).max(0.0) + if n.y > 0.5 { 0.05 } else { 0.0 }
    };
    let k = 1.0 / tile;
    let faces: [(Vec3, [Vec3; 4], f32, f32); 5] = [
        (vec3(1., 0., 0.), [c + vec3(h.x, h.y, h.z), c + vec3(h.x, h.y, -h.z), c + vec3(h.x, -h.y, -h.z), c + vec3(h.x, -h.y, h.z)], size.z, size.y),
        (vec3(0., 0., 1.), [c + vec3(-h.x, h.y, h.z), c + vec3(h.x, h.y, h.z), c + vec3(h.x, -h.y, h.z), c + vec3(-h.x, -h.y, h.z)], size.x, size.y),
        (vec3(0., 0., -1.), [c + vec3(h.x, h.y, -h.z), c + vec3(-h.x, h.y, -h.z), c + vec3(-h.x, -h.y, -h.z), c + vec3(h.x, -h.y, -h.z)], size.x, size.y),
        (vec3(0., 1., 0.), [c + vec3(-h.x, h.y, -h.z), c + vec3(h.x, h.y, -h.z), c + vec3(h.x, h.y, h.z), c + vec3(-h.x, h.y, h.z)], size.x, size.z),
        (vec3(-1., 0., 0.), [c + vec3(-h.x, h.y, -h.z), c + vec3(-h.x, h.y, h.z), c + vec3(-h.x, -h.y, h.z), c + vec3(-h.x, -h.y, -h.z)], size.z, size.y),
    ];
    for (n, p, a, b) in faces {
        let col = sky_haze(base, shade(n), d);
        r.quad(t, p, [vec2(0.0, 0.0), vec2(a * k, 0.0), vec2(a * k, b * k), vec2(0.0, b * k)], n, col, 0.0, true);
    }
}

/// Rua e prédios do outro lado, árvores, horizonte e céu: profundidade real pela janela.
/// A sala fica no 5º andar (piso externo a −14 m). Tudo emissivo (luz de dia já "assada").
/// Vista da janela: ponto de vista e campo de visão com que a paisagem foi pré-renderizada, e
/// distância do plano onde a imagem é exibida. Do interior da sala a imagem plana tem a mesma
/// perspectiva da cena 3D original (exata do ponto de captura, muito próxima de qualquer outro).
pub const VISTA_EYE: Vec3 = Vec3::new(-2.0, 1.55, 0.75);
pub const VISTA_FOVY_DEG: f32 = 78.0;
pub const VISTA_PLANO_X: f32 = -30.0;

/// Modo de pré-renderização da vista (PERICIA_ASSAR_VISTA=<arquivo.png>): só a paisagem.
pub fn assar_vista() -> Option<String> {
    std::env::var("PERICIA_ASSAR_VISTA").ok().filter(|v| !v.is_empty())
}

/// Fachada do nosso prédio, vão e vidro da janela: perto demais para ficar na imagem plana.
fn outside_near(r: &mut Renderer, wall: M) {
    let gy = -14.0;
    // fachada externa do nosso prédio e o vão da janela com espessura (peitoril, ombreiras, verga)
    let concreto = [0.80, 0.78, 0.74];
    out_box(r, vec3(-3.72, (gy + 0.9) / 2.0, 0.0), vec3(0.24, 0.9 - gy, 30.0), Tex::Ground, 2.0, concreto);
    out_box(r, vec3(-3.72, 2.5, 0.0), vec3(0.24, 0.8, 30.0), Tex::Ground, 2.0, concreto);
    out_box(r, vec3(-3.72, 1.5, -7.6), vec3(0.24, 1.2, 15.2), Tex::Ground, 2.0, concreto);
    out_box(r, vec3(-3.72, 1.5, 9.1), vec3(0.24, 1.2, 15.2), Tex::Ground, 2.0, concreto);
    r.bx(vec3(-3.62, 0.885, 0.75), vec3(0.04, 0.03, 1.5), wall);
    out_box(r, vec3(-3.9, 0.87, 0.75), vec3(0.16, 0.04, 1.62), Tex::Ground, 1.0, [0.9, 0.9, 0.88]); // peitoril externo
    // vidro (quase transparente) com reflexo leve
    r.boxr(vec3(-3.56, 1.5, 0.75), vec3(0.008, 1.2, 1.5), 0.0, M { t: Tex::Glass, c: Color::new(0.82, 0.9, 0.98, 0.16), tile: 1.5, spec: 1.0, emis: false });
}

/// A paisagem como uma imagem num plano distante (uma face em vez de milhares).
fn outside_flat(r: &mut Renderer) {
    let aspect = r.tex.get(&Tex::Vista).map(|t| t.width() / t.height().max(1.0)).filter(|a| a.is_finite() && *a > 0.2).unwrap_or(2.0);
    let l = VISTA_EYE.x - VISTA_PLANO_X;
    let hh = l * (VISTA_FOVY_DEG.to_radians() / 2.0).tan();
    let hw = hh * aspect;
    let (x, y, z) = (VISTA_PLANO_X, VISTA_EYE.y, VISTA_EYE.z);
    // olhando para −X, a direita da imagem fica em −Z; v = 0 no alto
    r.quad(Tex::Vista, [vec3(x, y + hh, z + hw), vec3(x, y + hh, z - hw), vec3(x, y - hh, z - hw), vec3(x, y - hh, z + hw)], [vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)], vec3(1.0, 0.0, 0.0), WHITE, 0.0, true);
    // borda: céu acima e chão abaixo, para olhares muito inclinados pela janela
    r.quad(Tex::Sky, [vec3(x, y + hh * 3.0, z + hw * 2.0), vec3(x, y + hh * 3.0, z - hw * 2.0), vec3(x, y + hh - 0.01, z - hw * 2.0), vec3(x, y + hh - 0.01, z + hw * 2.0)], [vec2(0.0, 0.0); 4], vec3(1.0, 0.0, 0.0), Color::new(0.38, 0.58, 0.86, 1.0), 0.0, true);
}

/// A paisagem 3D completa (usada só para pré-renderizar a imagem da vista).
fn outside_far(r: &mut Renderer) {
    let gy = -14.0;
    // chão: grama, calçadas e a rua
    out_box(r, vec3(-150.0, gy - 0.05, 0.0), vec3(300.0, 0.1, 600.0), Tex::Ground, 3.0, [0.36, 0.48, 0.30]);
    out_box(r, vec3(-11.0, gy + 0.06, 0.0), vec3(6.0, 0.12, 300.0), Tex::Ground, 1.0, [0.72, 0.71, 0.68]);
    out_box(r, vec3(-21.0, gy + 0.02, 0.0), vec3(14.0, 0.04, 300.0), Tex::Ground, 2.0, [0.26, 0.27, 0.29]);
    out_box(r, vec3(-31.0, gy + 0.06, 0.0), vec3(6.0, 0.12, 300.0), Tex::Ground, 1.0, [0.72, 0.71, 0.68]);
    for k in -12..12 {
        out_box(r, vec3(-21.0, gy + 0.045, k as f32 * 6.0), vec3(0.15, 0.01, 2.5), Tex::White, 1.0, [0.95, 0.92, 0.6]); // faixa
    }
    // carros estacionados (blocos simples, cores variadas)
    for (k, z) in [-22.0f32, -9.0, 4.0, 15.0, 31.0].iter().enumerate() {
        let c = [[0.7, 0.12, 0.1], [0.85, 0.85, 0.88], [0.15, 0.2, 0.35], [0.2, 0.2, 0.22], [0.75, 0.7, 0.55]][k];
        let x = if k % 2 == 0 { -15.2 } else { -26.8 };
        out_box(r, vec3(x, gy + 0.55, *z), vec3(1.8, 0.7, 4.2), Tex::White, 1.0, c);
        out_box(r, vec3(x, gy + 1.15, *z - 0.2), vec3(1.6, 0.55, 2.2), Tex::White, 1.0, [c[0] * 0.6, c[1] * 0.6, c[2] * 0.7]);
    }
    // árvores nas calçadas (copas na altura do 2º e 3º andar)
    let mut seed = 7u32;
    let mut rnd = || {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((seed >> 16) & 0x7fff) as f32 / 32767.0
    };
    for side in [-12.0f32, -30.5] {
        let mut z = -60.0;
        while z < 60.0 {
            let hgt = 5.5 + rnd() * 3.0;
            out_box(r, vec3(side, gy + hgt / 2.0, z), vec3(0.35, hgt, 0.35), Tex::Ground, 1.0, [0.38, 0.28, 0.2]);
            let cw = 3.0 + rnd() * 1.5;
            // copa em vários volumes desencontrados (menos "caixa")
            for j in 0..5 {
                let a = j as f32 * 1.3 + rnd();
                let off = vec3(a.cos() * cw * 0.3, (j % 3) as f32 * cw * 0.22, a.sin() * cw * 0.3);
                let sz = cw * (0.55 + rnd() * 0.3);
                let leaf = [0.22 + rnd() * 0.1, 0.40 + rnd() * 0.14, 0.20 + rnd() * 0.05];
                out_box(r, vec3(side, gy + hgt, z) + off, vec3(sz, sz * 0.75, sz), Tex::Ground, 1.0, leaf);
            }
            z += 9.0 + rnd() * 6.0;
        }
    }
    // prédios do outro lado da rua (frente para +X, de frente para a janela)
    let mut z = -70.0;
    while z < 70.0 {
        let wz = 10.0 + rnd() * 14.0;
        let hgt = 8.0 + rnd() * 20.0 + if rnd() > 0.82 { 22.0 } else { 0.0 };
        let dx = 14.0 + rnd() * 10.0;
        let tint = [0.86 + rnd() * 0.12, 0.84 + rnd() * 0.1, 0.80 + rnd() * 0.12];
        out_box(r, vec3(-36.0 - dx / 2.0 - rnd() * 4.0, gy + hgt / 2.0, z + wz / 2.0), vec3(dx, hgt, wz - 1.0), Tex::Facade, 6.0, tint);
        out_box(r, vec3(-36.0 - dx / 2.0, gy + hgt + 0.5, z + wz / 2.0), vec3(dx * 0.4, 1.0, wz * 0.3), Tex::Ground, 1.0, [0.7, 0.7, 0.7]); // casa de máquinas
        z += wz + rnd() * 3.0;
    }
    // horizonte distante, mais alto e enevoado
    let mut z = -260.0;
    while z < 260.0 {
        let wz = 20.0 + rnd() * 30.0;
        let hgt = 30.0 + rnd() * 90.0;
        let x = -110.0 - rnd() * 120.0;
        out_box(r, vec3(x, gy + hgt / 2.0, z), vec3(30.0, hgt, wz), Tex::Facade, 8.0, [0.85, 0.86, 0.9]);
        z += wz + 5.0 + rnd() * 25.0;
    }
    // céu: fundo, laterais e teto (não recebe névoa nem luz)
    let sky = |r: &mut Renderer, p: [Vec3; 4], n: Vec3| {
        r.quad(Tex::Sky, p, [vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(2.0, 1.0), vec2(0.0, 1.0)], n, WHITE, 0.0, true);
    };
    let (x0, x1, y0, y1, zz) = (-600.0, 0.0, gy - 5.0, 380.0, 600.0);
    sky(r, [vec3(x0, y1, zz), vec3(x0, y1, -zz), vec3(x0, y0, -zz), vec3(x0, y0, zz)], vec3(1.0, 0.0, 0.0));
    sky(r, [vec3(x1, y1, -zz), vec3(x0, y1, -zz), vec3(x0, y0, -zz), vec3(x1, y0, -zz)], vec3(0.0, 0.0, 1.0));
    sky(r, [vec3(x0, y1, zz), vec3(x1, y1, zz), vec3(x1, y0, zz), vec3(x0, y0, zz)], vec3(0.0, 0.0, -1.0));
    r.quad(Tex::Sky, [vec3(x0, y1, -zz), vec3(x1, y1, -zz), vec3(x1, y1, zz), vec3(x0, y1, zz)], [vec2(0.0, 0.0); 4], vec3(0.0, -1.0, 0.0), Color::new(0.45, 0.62, 0.88, 1.0), 0.0, true);
}

// ------------------------------------------------------------------ sala

fn room(r: &mut Renderer, local: bool) {
    let (w, d, h) = (7.0, 6.0, 2.8);
    let floor = if local { m(Tex::WoodFloor, rgb(255, 255, 255), 1.6, 0.25) } else { m(Tex::Vinyl, rgb(255, 255, 255), 1.2, 0.35) };
    r.bx(vec3(0.0, -0.05, 0.0), vec3(w, 0.1, d), floor);
    r.bx(vec3(0.0, h + 0.05, 0.0), vec3(w, 0.1, d), m(Tex::Ceiling, rgb(255, 255, 255), 1.2, 0.0));
    let wall = if local { m(Tex::Wall, rgb(236, 226, 205), 2.0, 0.05) } else { m(Tex::Wall, rgb(222, 228, 232), 2.0, 0.05) };
    // paredes (com abertura de janela na esquerda e porta na frente)
    r.bx(vec3(0.0, h / 2.0, -d / 2.0 - 0.05), vec3(w + 0.2, h, 0.1), wall);
    r.bx(vec3(w / 2.0 + 0.05, h / 2.0, 0.0), vec3(0.1, h, d + 0.2), wall);
    // parede esquerda em partes, deixando a janela (z 0..1.4, y 0.9..2.1)
    r.bx(vec3(-w / 2.0 - 0.05, h / 2.0, -1.55), vec3(0.1, h, 3.1), wall);
    r.bx(vec3(-w / 2.0 - 0.05, h / 2.0, 2.3), vec3(0.1, h, 1.6), wall);
    r.bx(vec3(-w / 2.0 - 0.05, 0.45, 0.75), vec3(0.1, 0.9, 1.5), wall);
    r.bx(vec3(-w / 2.0 - 0.05, 2.45, 0.75), vec3(0.1, 0.7, 1.5), wall);
    // janela: vidro emissivo (luz do dia), caixilho e peitoril
    outside_near(r, wall);
    outside_flat(r);
    let frame = metal(rgb(225, 228, 232));
    r.bx(vec3(-w / 2.0 - 0.04, 1.5, 0.75), vec3(0.05, 1.24, 0.05), frame);
    r.bx(vec3(-w / 2.0 - 0.04, 1.5, 0.75), vec3(0.05, 0.05, 1.54), frame);
    r.bx(vec3(-w / 2.0 + 0.02, 0.89, 0.75), vec3(0.16, 0.04, 1.6), plain(rgb(235, 235, 232)));
    // parede da frente com porta
    r.bx(vec3(-1.05, h / 2.0, d / 2.0 + 0.05), vec3(5.1, h, 0.1), wall);
    r.bx(vec3(3.05, h / 2.0, d / 2.0 + 0.05), vec3(1.1, h, 0.1), wall);
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
    // mesa de apoio do perito = bancada de ferramentas do local
    let (bc, bl, bd) = crate::bench::bench_geom(true);
    table(r, vec3(bc.x, 0.0, bc.z), vec3(bl, bc.y, bd), metal(rgb(150, 154, 160)), metal(rgb(90, 92, 98)));
    w.colliders.push(col((bc.x - bl / 2.0, bc.z - bd / 2.0), (bc.x + bl / 2.0, bc.z + bd / 2.0)));
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

#[allow(dead_code)]
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
