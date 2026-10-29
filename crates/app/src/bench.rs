//! Bancada de ferramentas: cada ferramenta é um objeto solto na cena, pego com E e largado
//! com G onde o perito mirar (bancada, mesa de exame, mesa da estação). A arrumação é do
//! jogador e fica gravada por tipo de cenário (laboratório / local de busca), valendo para
//! todos os casos.

use crate::render::Renderer;
use crate::save;
use macroquad::prelude::*;
use pericia_core::equip::{Tool, TOOLS};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Itens da bancada além das ferramentas de mão.
pub const LUVAS: &str = "luvas";
pub const PULSEIRA: &str = "pulseira";

/// Superfície horizontal onde algo pode ser apoiado: retângulo no plano XZ e altura do tampo.
#[derive(Clone, Copy)]
pub struct Surface {
    pub min: Vec2,
    pub max: Vec2,
    pub y: f32,
}

/// Bancada de ferramentas de cada cenário: (centro do tampo, comprimento ao longo de X,
/// profundidade, altura). No laboratório fica encostada na parede da frente; no local de
/// busca é a mesa de apoio do perito.
pub fn bench_geom(local: bool) -> (Vec3, f32, f32) {
    if local {
        (vec3(2.5, 0.72, 0.95), 1.8, 0.6)
    } else {
        (vec3(-1.25, 0.90, 2.42), 1.8, 0.6)
    }
}

/// Onde se pode largar uma ferramenta.
pub fn surfaces(local: bool) -> Vec<Surface> {
    let (c, l, d) = bench_geom(local);
    let mut v = vec![Surface { min: vec2(c.x - l / 2.0 + 0.03, c.z - d / 2.0 + 0.03), max: vec2(c.x + l / 2.0 - 0.03, c.z + d / 2.0 - 0.03), y: c.y }];
    if local {
        v.push(Surface { min: vec2(-0.85, -1.4), max: vec2(0.85, -0.7), y: 0.765 }); // mesa do investigado
        v.push(Surface { min: vec2(1.75, -2.12), max: vec2(3.15, -1.48), y: 0.75 }); // mesa da estação
    } else {
        v.push(Surface { min: vec2(-1.25, -0.95), max: vec2(1.25, 0.05), y: 0.905 }); // bancada de exame
        v.push(Surface { min: vec2(-2.3, -2.82), max: vec2(-0.9, -2.18), y: 0.75 }); // mesa da estação
    }
    v
}

/// Ponto de apoio sob a mira (até `reach` metros), no tampo mais próximo.
pub fn place_ray(o: Vec3, d: Vec3, local: bool, reach: f32) -> Option<Vec3> {
    if d.y >= -1e-4 {
        return None;
    }
    let mut best: Option<(f32, Vec3)> = None;
    for s in surfaces(local) {
        let t = (s.y - o.y) / d.y;
        if t <= 0.0 || t > reach {
            continue;
        }
        let p = o + d * t;
        if p.x >= s.min.x && p.x <= s.max.x && p.z >= s.min.y && p.z <= s.max.y && best.map(|b| t < b.0).unwrap_or(true) {
            best = Some((t, vec3(p.x, s.y, p.z)));
        }
    }
    best.map(|b| b.1)
}

/// Todos os itens da bancada (ids estáveis, usados no arquivo e nas seleções "T:<id>").
pub fn items() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = TOOLS.iter().map(|t| t.1).collect();
    v.push(LUVAS);
    v.push(PULSEIRA);
    v
}

pub fn tool_of(id: &str) -> Option<Tool> {
    TOOLS.iter().find(|t| t.1 == id).map(|t| t.0)
}

pub fn name_of(id: &str) -> &'static str {
    match id {
        LUVAS => "Caixa de luvas de nitrilo",
        PULSEIRA => "Pulseira antiestática",
        _ => TOOLS.iter().find(|t| t.1 == id).map(|t| t.2).unwrap_or("ferramenta"),
    }
}

pub fn model_of(id: &str) -> &'static str {
    match id {
        LUVAS => "caixa_luvas",
        PULSEIRA => "pulseira_esd",
        _ => tool_of(id).map(crate::maleta::icon_name).unwrap_or("caixa_luvas"),
    }
}

/// Postura de repouso: modelos feitos "em pé" (eixo da pega vertical) deitam na bancada.
pub fn rest(id: &str) -> Mat4 {
    match id {
        "chave" | "lanterna" => Mat4::from_rotation_z(-std::f32::consts::FRAC_PI_2),
        _ => Mat4::IDENTITY,
    }
}

/// Transformação completa de um item apoiado em `pos` (no tampo) com giro `yaw`.
pub fn transform(r: &Renderer, id: &str, pos: Vec3, yaw: f32) -> Mat4 {
    let rot = Mat4::from_rotation_y(yaw) * rest(id);
    let lift = r.model_aabb(model_of(id), rot).map(|(lo, _)| -lo.y).unwrap_or(0.0);
    Mat4::from_translation(pos + vec3(0.0, lift, 0.0)) * rot
}

#[derive(Default)]
pub struct Bench {
    /// Posições por cenário: id → (ponto no tampo, giro).
    lab: HashMap<String, (Vec3, f32)>,
    local: HashMap<String, (Vec3, f32)>,
    loaded: bool,
}

fn path() -> std::path::PathBuf {
    save::data_dir().join("saves").join("bancada.json")
}

impl Bench {
    fn map(&mut self, local: bool) -> &mut HashMap<String, (Vec3, f32)> {
        if local {
            &mut self.local
        } else {
            &mut self.lab
        }
    }

    fn load(&mut self) {
        self.loaded = true;
        let Ok(s) = std::fs::read_to_string(path()) else { return };
        let Ok(v) = serde_json::from_str::<Value>(&s) else { return };
        for (key, local) in [("lab", false), ("local", true)] {
            if let Some(o) = v[key].as_object() {
                for (id, p) in o {
                    let a: Vec<f32> = p.as_array().map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect()).unwrap_or_default();
                    if a.len() == 4 && items().contains(&id.as_str()) {
                        self.map(local).insert(id.clone(), (vec3(a[0], a[1], a[2]), a[3]));
                    }
                }
            }
        }
    }

    pub fn persist(&self) {
        let enc = |m: &HashMap<String, (Vec3, f32)>| -> Value {
            let mut o = serde_json::Map::new();
            for (k, (p, y)) in m {
                o.insert(k.clone(), json!([p.x, p.y, p.z, y]));
            }
            Value::Object(o)
        };
        let _ = std::fs::create_dir_all(save::data_dir().join("saves"));
        let _ = save::write_atomic(&path(), json!({"formato": 1, "lab": enc(&self.lab), "local": enc(&self.local)}).to_string().as_bytes());
    }

    /// Garante uma posição para cada item; o que faltar é arrumado em fileiras na bancada.
    pub fn ensure(&mut self, r: &Renderer, local: bool) {
        if !self.loaded {
            self.load();
        }
        if items().iter().all(|id| self.map(local).contains_key(*id)) {
            return;
        }
        let layout = default_layout(r, local);
        let m = self.map(local);
        for (id, p) in layout {
            m.entry(id.to_string()).or_insert(p);
        }
        self.persist();
    }

    pub fn get(&self, local: bool, id: &str) -> Option<(Vec3, f32)> {
        if local { self.local.get(id) } else { self.lab.get(id) }.copied()
    }

    pub fn set(&mut self, local: bool, id: &str, pos: Vec3, yaw: f32) {
        self.map(local).insert(id.to_string(), (pos, yaw));
        self.persist();
    }

    /// Volta tudo para a arrumação original da bancada.
    pub fn reset(&mut self, r: &Renderer, local: bool) {
        self.map(local).clear();
        self.ensure(r, local);
    }
}

/// Arrumação original: três fileiras (fundo: itens grandes e planos; meio: câmera, cabos e
/// bateria; frente: ferramentas pequenas), espaçadas pela largura real de cada modelo.
pub fn default_layout(r: &Renderer, local: bool) -> Vec<(&'static str, (Vec3, f32))> {
    let (c, l, d) = bench_geom(local);
    // (itens, distância do centro do tampo em direção à parede, em metros, para 0,6 m de fundo)
    let rows: [(&[&'static str], f32); 3] = [
        (&["oficio", "saco", "faraday", "etiquetas", LUVAS, "fita-lacre", "placas"], 0.145),
        (&["camera", "bateria", "cabo-sata", "cabo-usb", "cabo-usbc", "cabo-lightning", "aluminio", PULSEIRA, "lanterna"], -0.08),
        (&["chave", "tesoura", "ejetor", "pendrive-coleta", "adaptador-m2", "leitor-sd"], -0.225),
    ];
    // compridos de lado (o lado maior acompanha a bancada)
    let extra = |id: &str| if matches!(id, "oficio" | "saco" | "tesoura") { std::f32::consts::FRAC_PI_2 } else { 0.0 };
    // o jogador fica do lado de z menor (laboratório, bancada na parede da frente) ou maior (local)
    let toward_wall = if local { -1.0 } else { 1.0 };
    let base_yaw = if local { 0.0 } else { std::f32::consts::PI };
    let mut out = vec![];
    for (ids, v) in rows {
        let widths: Vec<f32> = ids
            .iter()
            .map(|id| {
                let m = Mat4::from_rotation_y(base_yaw + extra(id)) * rest(id);
                r.model_aabb(model_of(id), m).map(|(lo, hi)| hi.x - lo.x).unwrap_or(0.12)
            })
            .collect();
        let total: f32 = widths.iter().sum();
        let gap = ((l - 0.08 - total) / (ids.len() as f32 + 1.0)).max(0.01);
        let mut x = c.x - l / 2.0 + 0.04 + gap;
        for (id, w) in ids.iter().zip(widths) {
            let pz = c.z + toward_wall * v * (d / 0.6);
            out.push((*id, (vec3(x + w / 2.0, c.y, pz), base_yaw + extra(id))));
            x += w + gap;
        }
    }
    out
}
