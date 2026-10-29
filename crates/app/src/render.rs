//! Renderizador 3D: texturas procedurais (geradas na CPU, ladrilháveis, com mipmaps),
//! shader com iluminação por pixel (luzes pontuais do teto, luz da janela, ambiente de
//! hemisfério, especular Blinn-Phong, névoa e mapeamento de tons) e lotes por textura.
//! Compatível com OpenGL 2.1 / GLES 2 (GLSL 100).

use macroquad::models::Vertex;
use macroquad::prelude::*;
use miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation, MipmapFilterMode, PipelineParams, TextureFormat, TextureKind, TextureParams, TextureWrap, UniformDesc, UniformType};

// ------------------------------------------------------------------ texturas

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tex {
    White,
    Wood,
    WoodFloor,
    Vinyl,
    Wall,
    Ceiling,
    Metal,
    Plastic,
    Fabric,
    Mat,
    Vent,
    Keyboard,
    Label,
    Rug,
    Leaf,
    Cardboard,
    Screen,
    Desktop,
    Concrete,
    Glass,
    Blob,
    Outside,
    // imagens de decoração (assets/textures, geradas por tools/texturas) e vestígios
    Cork,
    Whiteboard,
    PosterPop,
    PosterEpi,
    Painting,
    Calendar,
    SignLab,
    ClockFace,
    Document,
    Blood,
}

impl Tex {
    /// Nome do arquivo que pode substituir a textura procedural (assets/textures/<nome>.png|.tga).
    pub fn file_name(self) -> &'static str {
        match self {
            Tex::White => "branco",
            Tex::Wood => "madeira",
            Tex::WoodFloor => "piso_madeira",
            Tex::Vinyl => "piso_vinilico",
            Tex::Wall => "parede",
            Tex::Ceiling => "forro",
            Tex::Metal => "metal",
            Tex::Plastic => "plastico",
            Tex::Fabric => "tecido",
            Tex::Mat => "manta_antiestatica",
            Tex::Vent => "grade_ventilacao",
            Tex::Keyboard => "teclado",
            Tex::Label => "etiqueta",
            Tex::Rug => "tapete",
            Tex::Leaf => "folhagem",
            Tex::Cardboard => "papelao",
            Tex::Screen => "tela",
            Tex::Desktop => "tela_desktop",
            Tex::Concrete => "concreto",
            Tex::Glass => "vidro",
            Tex::Blob => "sombra_contato",
            Tex::Outside => "vista_janela",
            Tex::Cork => "quadro_cortica",
            Tex::Whiteboard => "quadro_branco",
            Tex::PosterPop => "cartaz_pop",
            Tex::PosterEpi => "cartaz_epi",
            Tex::Painting => "pintura",
            Tex::Calendar => "calendario",
            Tex::SignLab => "placa_lab",
            Tex::ClockFace => "relogio",
            Tex::Document => "documento",
            Tex::Blood => "mancha",
        }
    }
    /// Imagem aplicada com UV 0..1 (quadros, cartazes, mostradores): não repete.
    pub fn is_image(self) -> bool {
        matches!(self, Tex::Cork | Tex::Whiteboard | Tex::PosterPop | Tex::PosterEpi | Tex::Painting | Tex::Calendar | Tex::SignLab | Tex::ClockFace | Tex::Document | Tex::Blood)
    }
}

pub const ALL_TEX: [Tex; 31] = [
    Tex::White, Tex::Wood, Tex::WoodFloor, Tex::Vinyl, Tex::Wall, Tex::Ceiling, Tex::Metal, Tex::Plastic, Tex::Fabric, Tex::Mat, Tex::Vent, Tex::Keyboard, Tex::Label, Tex::Rug,
    Tex::Leaf, Tex::Cardboard, Tex::Screen, Tex::Concrete, Tex::Glass, Tex::Blob, Tex::Outside, Tex::Cork, Tex::Whiteboard, Tex::PosterPop, Tex::PosterEpi,
    Tex::Painting, Tex::Calendar, Tex::SignLab, Tex::ClockFace, Tex::Document, Tex::Blood,
];

fn hash(x: i32, y: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ s.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

/// Ruído de valor ladrilhável (período p células).
fn vnoise(x: f32, y: f32, p: i32, s: u32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let sm = |t: f32| t * t * (3.0 - 2.0 * t);
    let g = |a: i32, b: i32| hash(a.rem_euclid(p), b.rem_euclid(p), s);
    let a = g(xi, yi);
    let b = g(xi + 1, yi);
    let c = g(xi, yi + 1);
    let d = g(xi + 1, yi + 1);
    let (u, v) = (sm(fx), sm(fy));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

fn fbm(u: f32, v: f32, base: i32, oct: u32, s: u32) -> f32 {
    let mut f = 0.0;
    let mut amp = 0.5;
    let mut p = base;
    for o in 0..oct {
        f += amp * vnoise(u * p as f32, v * p as f32, p, s + o);
        amp *= 0.5;
        p *= 2;
    }
    f
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Gera os pixels (RGBA8) de uma textura 256×256 ladrilhável.
pub fn gen(t: Tex) -> Vec<u8> {
    let n = 256usize;
    let mut px = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (x as f32 / n as f32, y as f32 / n as f32);
            let c: [f32; 3] = match t {
                Tex::White => [1.0, 1.0, 1.0],
                Tex::Wood => {
                    // veios longitudinais (ao longo de u) com nós suaves
                    let warp = fbm(u * 0.5, v * 2.0, 2, 3, 11) * 0.35;
                    let grain = ((v + warp) * 70.0).sin() * 0.5 + 0.5;
                    let fine = fbm(u * 0.25, v * 16.0, 4, 3, 12);
                    let k = grain.powf(2.5) * 0.35 + fine * 0.45;
                    mix([0.40, 0.25, 0.14], [0.68, 0.48, 0.30], k)
                }
                Tex::WoodFloor => {
                    let row = (v * 6.0).floor();
                    let off = hash(row as i32, 0, 3) * 0.7;
                    let col = ((u + off) * 2.0).fract();
                    let seam = if (v * 6.0).fract() < 0.03 || col < 0.012 { 0.55 } else { 1.0 };
                    let tone = hash(row as i32, ((u + off) * 2.0).floor() as i32, 4);
                    let grain = fbm(u * 4.0, v * 24.0, 4, 3, 5) * 0.4;
                    let base = mix([0.45, 0.30, 0.18], [0.62, 0.44, 0.27], tone * 0.6 + grain);
                    [base[0] * seam, base[1] * seam, base[2] * seam]
                }
                Tex::Vinyl => {
                    let gx = (u * 4.0).fract();
                    let gy = (v * 4.0).fract();
                    let seam = gx < 0.008 || gy < 0.008;
                    let tile = hash((u * 4.0) as i32, (v * 4.0) as i32, 7) * 0.05;
                    let speck = if hash(x as i32, y as i32, 8) > 0.985 { -0.18 } else { 0.0 };
                    let k = 0.78 + tile + speck + fbm(u, v, 8, 3, 9) * 0.06;
                    if seam { [0.55, 0.56, 0.58] } else { [k * 0.93, k * 0.95, k * 0.97] }
                }
                Tex::Wall => {
                    let k = 0.86 + (fbm(u, v, 16, 4, 21) - 0.5) * 0.06;
                    [k, k * 0.985, k * 0.95]
                }
                Tex::Ceiling => {
                    let gx = (u * 2.0).fract();
                    let gy = (v * 2.0).fract();
                    if gx < 0.015 || gy < 0.015 {
                        [0.72, 0.72, 0.72]
                    } else {
                        let dot = if hash(x as i32 / 2, y as i32 / 2, 31) > 0.93 { 0.8 } else { 0.95 };
                        [dot, dot, dot * 0.98]
                    }
                }
                Tex::Metal => {
                    let streak = fbm(u * 0.5, v * 32.0, 2, 4, 41) * 0.22;
                    let k = 0.62 + streak;
                    [k, k * 1.01, k * 1.04]
                }
                Tex::Plastic => {
                    let k = 0.30 + fbm(u, v, 32, 3, 51) * 0.05;
                    [k, k, k * 1.05]
                }
                Tex::Fabric => {
                    let w = ((x % 4) as f32 - 1.5).abs() * 0.04 + ((y % 4) as f32 - 1.5).abs() * 0.04;
                    let k = 0.22 + w + fbm(u, v, 16, 2, 61) * 0.08;
                    [k * 0.8, k * 0.9, k * 1.3]
                }
                Tex::Mat => {
                    let dot = (x % 16 < 2) && (y % 16 < 2);
                    let k = fbm(u, v, 8, 3, 71) * 0.05;
                    if dot { [0.20, 0.42, 0.34] } else { [0.24 + k, 0.50 + k, 0.42 + k] }
                }
                Tex::Vent => {
                    let cx = (x % 16) as f32 - 7.5;
                    let cy = (y % 16) as f32 - 7.5;
                    if cx * cx + cy * cy < 22.0 { [0.03, 0.03, 0.035] } else { [0.20, 0.21, 0.23] }
                }
                Tex::Keyboard => {
                    let kx = (x % 32) as i32;
                    let ky = (y % 32) as i32;
                    if kx < 3 || ky < 3 || kx > 29 || ky > 29 { [0.05, 0.05, 0.06] } else if ky < 6 { [0.24, 0.24, 0.27] } else { [0.17, 0.17, 0.19] }
                }
                Tex::Label => {
                    let stripe = (16..44).contains(&y) && (20..236).contains(&x) && hash(x as i32 / 3, 0, 81) > 0.45;
                    let line = (60..240).contains(&y) && (y / 14) % 2 == 0 && (20..200).contains(&x) && hash(x as i32 / 7, y as i32 / 14, 82) > 0.25;
                    if stripe { [0.08, 0.08, 0.08] } else if line { [0.45, 0.47, 0.50] } else { [0.93, 0.93, 0.90] }
                }
                Tex::Rug => {
                    let b = (u - 0.5).abs().max((v - 0.5).abs());
                    let ring = (b * 10.0).floor() as i32 % 2 == 0;
                    let k = fbm(u, v, 32, 2, 91) * 0.08;
                    if b > 0.46 { [0.62 + k, 0.52 + k, 0.30] } else if ring { [0.48 + k, 0.12, 0.12] } else { [0.36 + k, 0.08, 0.10] }
                }
                Tex::Leaf => {
                    let k = fbm(u, v, 8, 4, 101);
                    mix([0.10, 0.28, 0.10], [0.25, 0.52, 0.20], k)
                }
                Tex::Cardboard => {
                    let k = fbm(u, v * 6.0, 4, 3, 111) * 0.12;
                    [0.62 + k, 0.46 + k, 0.28 + k * 0.5]
                }
                Tex::Screen | Tex::Desktop => {
                    let k = v * 0.5;
                    [0.08 + k * 0.1, 0.20 + k * 0.2, 0.32 + k * 0.25]
                }
                Tex::Concrete => {
                    let k = 0.5 + (fbm(u, v, 8, 5, 121) - 0.5) * 0.3;
                    [k, k, k * 0.98]
                }
                Tex::Glass => [0.70, 0.82, 0.92],
                Tex::Blob => [0.0, 0.0, 0.0],
                Tex::Outside => {
                    // céu com nuvens, prédios ao longe e copas de árvores (vista da janela)
                    let yy = 1.0 - v; // 0 = base da janela
                    let sky = mix([0.62, 0.76, 0.90], [0.36, 0.56, 0.82], yy);
                    let cloud = (fbm(u * 1.5, v * 3.0, 4, 4, 131) - 0.5).max(0.0) * 2.2 * yy;
                    let mut c = mix(sky, [0.95, 0.96, 0.97], cloud.min(1.0));
                    let bx = (u * 9.0).floor();
                    let bh = 0.25 + hash(bx as i32, 0, 141) * 0.35;
                    if yy < bh {
                        let shade = 0.55 + hash(bx as i32, 1, 142) * 0.2;
                        let win = ((u * 9.0).fract() * 6.0).fract() < 0.45 && ((yy * 40.0).fract()) < 0.5;
                        c = if win { [shade * 0.75, shade * 0.85, shade * 0.95] } else { [shade, shade * 0.98, shade * 0.95] };
                    }
                    let tree = 0.16 + fbm(u * 4.0, 0.3, 8, 3, 151) * 0.12;
                    if yy < tree {
                        let k = fbm(u * 6.0, yy * 6.0, 8, 3, 152);
                        c = mix([0.12, 0.28, 0.12], [0.26, 0.46, 0.20], k);
                    }
                    c
                }
                // sem o arquivo de imagem: tons neutros (o jogo continua coerente)
                Tex::Cork => mix([0.55, 0.38, 0.22], [0.70, 0.50, 0.30], fbm(u, v, 32, 3, 161)),
                Tex::Whiteboard | Tex::Document => [0.96, 0.96, 0.95],
                Tex::PosterPop => if v < 0.14 { [0.09, 0.2, 0.38] } else { [0.95, 0.96, 0.97] },
                Tex::PosterEpi => [1.0, 0.84, 0.16],
                Tex::Painting => mix([0.3, 0.32, 0.55], [0.85, 0.6, 0.4], v),
                Tex::Calendar | Tex::ClockFace => [0.98, 0.98, 0.97],
                Tex::SignLab => [0.09, 0.2, 0.38],
                Tex::Blood => [0.30, 0.06, 0.05],
            };
            let i = (y * n + x) * 4;
            px[i] = (c[0].clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 1] = (c[1].clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 2] = (c[2].clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 3] = if t == Tex::Blood {
                let d = ((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)).sqrt() * 2.0;
                if d < 0.55 + 0.1 * (fbm(u, v, 8, 2, 171) - 0.5) { 230 } else { 0 }
            } else if t == Tex::Blob {
                let d = ((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)).sqrt() * 2.0;
                ((1.0 - d).clamp(0.0, 1.0).powf(1.6) * 255.0) as u8
            } else {
                255
            };
        }
    }
    px
}

pub fn make_texture(bytes: &[u8], w: u32, h: u32, repeat: bool) -> Texture2D {
    unsafe {
        let gl = get_internal_gl();
        let id = gl.quad_context.new_texture_from_data_and_format(
            bytes,
            TextureParams {
                kind: TextureKind::Texture2D,
                format: TextureFormat::RGBA8,
                wrap: if repeat { TextureWrap::Repeat } else { TextureWrap::Clamp },
                min_filter: miniquad::FilterMode::Linear,
                mag_filter: miniquad::FilterMode::Linear,
                mipmap_filter: MipmapFilterMode::Linear,
                width: w,
                height: h,
                allocate_mipmaps: true,
                sample_count: 1,
            },
        );
        gl.quad_context.texture_generate_mipmaps(id);
        Texture2D::from_miniquad_texture(id)
    }
}

// ------------------------------------------------------------------ material

const VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
varying vec2 uv;
varying vec4 color;
varying vec3 wpos;
varying vec4 nrm;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    vec4 w = Model * vec4(position, 1.0);
    wpos = w.xyz;
    gl_Position = Projection * w;
    color = color0 / 255.0;
    uv = texcoord;
    nrm = normal;
}"#;

const FS: &str = r#"#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
varying vec3 wpos;
varying vec4 nrm;
uniform sampler2D Texture;
uniform vec3 CamPos;
uniform vec3 L0Pos;
uniform vec3 L0Col;
uniform vec3 L1Pos;
uniform vec3 L1Col;
uniform vec3 L2Pos;
uniform vec3 L2Col;
uniform vec3 SunDir;
uniform vec3 SunCol;
uniform vec3 SkyCol;
uniform vec3 GroundCol;
uniform vec3 FogCol;
uniform float FogDen;
uniform mat4 LightVP;
uniform sampler2D ShadowMap;

float unpack(vec4 c) {
    return dot(c, vec4(1.0, 1.0 / 255.0, 1.0 / 65025.0, 1.0 / 16581375.0));
}

float shadow_at(vec3 n) {
    vec4 lp = LightVP * vec4(wpos, 1.0);
    vec3 p = lp.xyz / lp.w * 0.5 + 0.5;
    if (p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0 || p.z > 1.0) {
        return 1.0;
    }
    float bias = 0.0018 + 0.004 * (1.0 - abs(n.y));
    float s = 0.0;
    float t = 1.6 / 1024.0;
    s += (p.z - bias > unpack(texture2D(ShadowMap, p.xy))) ? 0.0 : 1.0;
    s += (p.z - bias > unpack(texture2D(ShadowMap, p.xy + vec2(t, 0.0)))) ? 0.0 : 1.0;
    s += (p.z - bias > unpack(texture2D(ShadowMap, p.xy - vec2(t, 0.0)))) ? 0.0 : 1.0;
    s += (p.z - bias > unpack(texture2D(ShadowMap, p.xy + vec2(0.0, t)))) ? 0.0 : 1.0;
    s += (p.z - bias > unpack(texture2D(ShadowMap, p.xy - vec2(0.0, t)))) ? 0.0 : 1.0;
    return s / 5.0;
}

vec3 point_light(vec3 lp, vec3 lc, vec3 n, vec3 v, float spec) {
    vec3 d = lp - wpos;
    float dist = length(d);
    vec3 l = d / max(dist, 0.001);
    float att = 1.0 / (1.0 + 0.22 * dist * dist);
    float diff = max(dot(n, l), 0.0);
    vec3 h = normalize(l + v);
    float s = pow(max(dot(n, h), 0.0), 40.0) * spec;
    return lc * (diff + s) * att;
}

void main() {
    vec4 base = texture2D(Texture, uv) * color;
    if (nrm.w > 1.5) {
        gl_FragColor = base;
        return;
    }
    vec3 n = normalize(nrm.xyz);
    vec3 v = normalize(CamPos - wpos);
    float spec = nrm.w;
    vec3 amb = mix(GroundCol, SkyCol, n.y * 0.5 + 0.5);
    vec3 sun = SunCol * max(dot(n, -SunDir), 0.0);
    float sh = n.y < -0.5 ? 1.0 : mix(0.38, 1.0, shadow_at(n));
    vec3 lit = amb + (sun + point_light(L0Pos, L0Col, n, v, spec) + point_light(L1Pos, L1Col, n, v, spec)) * sh + point_light(L2Pos, L2Col, n, v, spec);
    vec3 c = base.rgb * lit;
    c = c / (c + vec3(0.9)) * 1.9;
    float f = 1.0 - exp(-FogDen * length(CamPos - wpos));
    c = mix(c, FogCol, clamp(f, 0.0, 1.0));
    gl_FragColor = vec4(c, base.a);
}"#;

const DEPTH_FS: &str = r#"#version 100
precision highp float;
varying vec2 uv;
varying vec4 color;
varying vec3 wpos;
varying vec4 nrm;
void main() {
    float d = gl_FragCoord.z;
    vec4 enc = vec4(1.0, 255.0, 65025.0, 16581375.0) * d;
    enc = fract(enc);
    enc -= enc.yzww * vec4(1.0 / 255.0, 1.0 / 255.0, 1.0 / 255.0, 0.0);
    gl_FragColor = enc;
}"#;

pub struct Lights {
    pub cam: Vec3,
    pub l: [(Vec3, Vec3); 3],
    pub sun_dir: Vec3,
    pub sun: Vec3,
    pub sky: Vec3,
    pub ground: Vec3,
    pub fog: Vec3,
    pub fog_den: f32,
}

pub const SHADOW_RES: u32 = 1024;
/// Limites de um lote (malha por textura) — iguais à capacidade configurada no macroquad.
pub const MAX_BATCH_VERTS: usize = 32768;
pub const MAX_BATCH_INDICES: usize = 49152;

/// Parte de um modelo com um único material.
pub struct ModelPart {
    /// Nome do material no MTL (permite substituir no momento de desenhar, ex.: "tela").
    pub material: String,
    pub mt: M,
    /// Pedaços prontos para o lote: vértices (posição, normal, uv) e índices locais de 16 bits.
    /// Montados uma vez no carregamento — por quadro só se transformam os vértices.
    pub chunks: Vec<(Vec<(Vec3, Vec3, Vec2)>, Vec<u16>)>,
}

/// Modelo 3D carregado de assets/models (OBJ/MTL), em metros, origem no piso, frente para +Z.
pub struct Model {
    pub parts: Vec<ModelPart>,
    pub half: Vec3,
    pub tris: usize,
}

/// Material do modelo → textura do jogo, pelo prefixo do nome (`madeira_escura` → madeira).
/// Devolve (textura, metros por repetição, brilho especular padrão, emissivo).
pub fn material_tex(name: &str) -> (Tex, f32, f32, bool) {
    let base = name.split(|c| c == '_' || c == '.').next().unwrap_or("").to_lowercase();
    match base.as_str() {
        "madeira" => (Tex::Wood, 0.9, 0.35, false),
        "metal" => (Tex::Metal, 0.5, 0.5, false),
        "plastico" | "borracha" => (Tex::Plastic, 0.4, 0.25, false),
        "tecido" => (Tex::Fabric, 0.25, 0.05, false),
        "manta" => (Tex::Mat, 0.5, 0.1, false),
        "grade" => (Tex::Vent, 0.08, 0.2, false),
        "teclado" => (Tex::Keyboard, 0.07, 0.2, false),
        "etiqueta" => (Tex::Label, 0.3, 0.0, false),
        "papelao" => (Tex::Cardboard, 0.4, 0.05, false),
        "folhagem" => (Tex::Leaf, 0.3, 0.2, false),
        "concreto" | "ceramica" => (Tex::Concrete, 0.4, 0.1, false),
        "vidro" => (Tex::Glass, 1.0, 0.8, false),
        "tela" => (Tex::Screen, 1.0, 0.6, false),
        "luz" => (Tex::White, 1.0, 0.0, true),
        "cortica" => (Tex::Cork, 1.0, 0.05, false),
        "quadrobranco" => (Tex::Whiteboard, 1.0, 0.45, false),
        "cartazpop" => (Tex::PosterPop, 1.0, 0.25, false),
        "cartazepi" => (Tex::PosterEpi, 1.0, 0.25, false),
        "pintura" => (Tex::Painting, 1.0, 0.2, false),
        "calendario" => (Tex::Calendar, 1.0, 0.1, false),
        "placalab" => (Tex::SignLab, 1.0, 0.3, false),
        "relogio" => (Tex::ClockFace, 1.0, 0.5, false),
        "documento" => (Tex::Document, 1.0, 0.05, false),
        _ => (Tex::White, 1.0, 0.2, false),
    }
}

pub struct Renderer {
    pub tex: std::collections::HashMap<Tex, Texture2D>,
    pub mat: Option<Material>,
    batches: Vec<(Tex, Vec<Vertex>, Vec<u16>)>,
    /// Texturas extras (ex.: captura da tela da estação) desenhadas como emissivas.
    pub screen_tex: Option<Texture2D>,
    pub quality_high: bool,
    pub depth_mat: Option<Material>,
    /// Registro do carregamento de texturas externas (substituições aplicadas ou recusadas).
    pub asset_log: Vec<String>,
    /// Modelos externos (assets/models), por nome de arquivo sem extensão.
    pub models: std::collections::HashMap<String, Model>,
    pub shadow_rt: Option<RenderTarget>,
    pub shadows: bool,
    /// Assinatura da geometria do último mapa de sombras: se a cena não mudou, o mapa é reaproveitado.
    shadow_sig: u64,
    /// Objetos "em mãos" (presos à câmera): passe principal apenas — não projetam sombra
    /// nem entram na assinatura, para não refazer o mapa de sombras a cada movimento.
    vm_batches: Vec<(Tex, Vec<Vertex>, Vec<u16>)>,
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut tex = std::collections::HashMap::new();
        let mut asset_log = vec![];
        for t in ALL_TEX {
            // textura fornecida (opcional) tem prioridade; a procedural é o padrão
            if t != Tex::Desktop {
                match crate::assets::load_texture_override(t.file_name(), t != Tex::Blob && !t.is_image()) {
                    Ok(Some((tx, msg))) => {
                        asset_log.push(msg);
                        tex.insert(t, tx);
                        continue;
                    }
                    Ok(None) => {}
                    Err(e) => asset_log.push(e),
                }
            }
            let px = gen(t);
            tex.insert(t, make_texture(&px, 256, 256, t != Tex::Blob && !t.is_image()));
        }
        asset_log.extend(crate::assets::validate_models());
        for l in &asset_log {
            eprintln!("[assets] {}", l);
        }
        if !asset_log.is_empty() {
            // relatório fica na pasta de dados do usuário — nunca na pasta dos originais
            let dir = crate::save::data_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("assets_relatorio.txt"), asset_log.join("\n") + "\n");
        }
        let mat = load_material(
            ShaderSource::Glsl { vertex: VS, fragment: FS },
            MaterialParams {
                pipeline_params: PipelineParams {
                    depth_write: true,
                    depth_test: Comparison::LessOrEqual,
                    color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
                    ..Default::default()
                },
                uniforms: ["CamPos", "L0Pos", "L0Col", "L1Pos", "L1Col", "L2Pos", "L2Col", "SunDir", "SunCol", "SkyCol", "GroundCol", "FogCol"]
                    .iter()
                    .map(|n| UniformDesc::new(n, UniformType::Float3))
                    .chain(std::iter::once(UniformDesc::new("FogDen", UniformType::Float1)))
                    .chain(std::iter::once(UniformDesc::new("LightVP", UniformType::Mat4)))
                    .collect(),
                textures: vec!["ShadowMap".to_string()],
            },
        )
        .map_err(|e| eprintln!("shader principal: {:?}", e))
        .ok();
        let depth_mat = load_material(
            ShaderSource::Glsl { vertex: VS, fragment: DEPTH_FS },
            MaterialParams { pipeline_params: PipelineParams { depth_write: true, depth_test: Comparison::LessOrEqual, color_blend: None, ..Default::default() }, uniforms: vec![], textures: vec![] },
        )
        .map_err(|e| eprintln!("shader de profundidade: {:?}", e))
        .ok();
        let shadow_rt = render_target_ex(SHADOW_RES, SHADOW_RES, RenderTargetParams { sample_count: 1, depth: true });
        shadow_rt.texture.set_filter(FilterMode::Nearest);
        let (models, mlog) = crate::assets::load_models();
        for l in &mlog {
            eprintln!("[assets] {}", l);
        }
        Renderer { tex, mat, batches: vec![], screen_tex: None, quality_high: true, depth_mat, asset_log, models, shadow_rt: Some(shadow_rt), shadows: true, shadow_sig: 0, vm_batches: vec![] }
    }

    fn batch(&mut self, t: Tex) -> usize {
        if let Some(i) = self.batches.iter().position(|b| b.0 == t && b.1.len() + 4 <= MAX_BATCH_VERTS && b.2.len() + 6 <= MAX_BATCH_INDICES) {
            return i;
        }
        self.batches.push((t, Vec::with_capacity(4096), Vec::with_capacity(8192)));
        self.batches.len() - 1
    }

    /// Quadrilátero com UVs explícitas; `spec` = brilho especular; `emis` = sem iluminação.
    pub fn quad(&mut self, t: Tex, p: [Vec3; 4], uv: [Vec2; 4], n: Vec3, c: Color, spec: f32, emis: bool) {
        let i = self.batch(t);
        let b = &mut self.batches[i];
        let base = b.1.len() as u16;
        let w = if emis { 2.0 } else { spec };
        for k in 0..4 {
            // oclusão ambiente barata: escurece perto do piso
            let ao = if emis { 1.0 } else { (0.62 + p[k].y * 0.9).min(1.0) };
            let col = [(c.r * ao * 255.0) as u8, (c.g * ao * 255.0) as u8, (c.b * ao * 255.0) as u8, (c.a * 255.0) as u8];
            b.1.push(Vertex { position: p[k], uv: uv[k], color: col, normal: n.extend(w) });
        }
        b.2.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Câmera ortográfica da luz principal (luminárias do teto), cobrindo a sala.
    pub fn light_camera(&self) -> Camera3D {
        Camera3D {
            position: vec3(0.25, 2.75, 0.45),
            target: vec3(0.0, 0.0, -0.35),
            up: vec3(0.0, 0.0, -1.0),
            projection: Projection::Orthographics,
            fovy: 7.6,
            aspect: Some(1.0),
            z_near: 0.05,
            z_far: 6.0,
            render_target: self.shadow_rt.clone(),
            ..Default::default()
        }
    }

    /// Desenha tudo: passe de profundidade (sombra) e passe principal com a câmera `main`.
    pub fn flush(&mut self, lights: &Lights, main: &Camera3D) {
        let batches = std::mem::take(&mut self.batches);
        let vm = std::mem::take(&mut self.vm_batches);
        let mut order: Vec<usize> = (0..batches.len()).collect();
        order.sort_by_key(|&i| batches[i].1.iter().any(|v| v.color[3] < 255) as u8);
        let texture_of = |s: &Renderer, t: Tex| -> Option<Texture2D> { if t == Tex::Desktop { s.screen_tex.clone().or_else(|| s.tex.get(&Tex::Screen).cloned()) } else { s.tex.get(&t).cloned() } };
        // 1) mapa de sombras
        let use_shadows = self.shadows && self.depth_mat.is_some() && self.shadow_rt.is_some();
        let light_cam = self.light_camera();
        let light_vp = light_cam.matrix();
        // a luz é fixa: o mapa só precisa ser refeito quando a geometria muda (item aberto,
        // lacre, objeto movido...). Assinatura barata: amostra de vértices de cada lote.
        let mut sig: u64 = 0xcbf2_9ce4_8422_2325;
        if use_shadows {
            for (t, v, idx) in &batches {
                let mut mix = |x: u64| sig = (sig ^ x).wrapping_mul(0x100_0000_01b3);
                mix(*t as u64);
                mix(v.len() as u64);
                mix(idx.len() as u64);
                for vx in v.iter().step_by(37) {
                    mix(vx.position.x.to_bits() as u64);
                    mix(vx.position.y.to_bits() as u64);
                    mix(vx.position.z.to_bits() as u64);
                }
            }
        }
        if use_shadows && sig != self.shadow_sig {
            self.shadow_sig = sig;
            set_camera(&light_cam);
            clear_background(WHITE);
            gl_use_material(self.depth_mat.as_ref().unwrap());
            for &i in &order {
                let (t, v, idx) = &batches[i];
                if idx.is_empty() || matches!(t, Tex::Blob | Tex::Glass | Tex::Outside | Tex::Wall | Tex::Ceiling) || v.iter().any(|x| x.color[3] < 255) {
                    continue;
                }
                draw_mesh(&Mesh { vertices: v.clone(), indices: idx.clone(), texture: None });
            }
            gl_use_default_material();
        }
        // 2) passe principal
        set_camera(main);
        if let Some(m) = &self.mat {
            m.set_uniform("CamPos", lights.cam);
            for (k, (p, c)) in lights.l.iter().enumerate() {
                m.set_uniform(&format!("L{}Pos", k), *p);
                m.set_uniform(&format!("L{}Col", k), *c);
            }
            m.set_uniform("SunDir", lights.sun_dir.normalize());
            m.set_uniform("SunCol", lights.sun);
            m.set_uniform("SkyCol", lights.sky);
            m.set_uniform("GroundCol", lights.ground);
            m.set_uniform("FogCol", lights.fog);
            m.set_uniform("FogDen", lights.fog_den);
            m.set_uniform("LightVP", if use_shadows { light_vp } else { Mat4::from_translation(vec3(10.0, 10.0, 10.0)) });
            if let Some(rt) = &self.shadow_rt {
                m.set_texture("ShadowMap", rt.texture.clone());
            }
            gl_use_material(m);
        }
        for i in order {
            let (t, v, idx) = &batches[i];
            if idx.is_empty() {
                continue;
            }
            let mesh = Mesh { vertices: v.clone(), indices: idx.clone(), texture: texture_of(self, *t) };
            draw_mesh(&mesh);
        }
        for (t, v, idx) in vm {
            if !idx.is_empty() {
                let tx = texture_of(self, t);
                draw_mesh(&Mesh { vertices: v, indices: idx, texture: tx });
            }
        }
        gl_use_default_material();
    }
}

// ------------------------------------------------------------------ primitivas

#[derive(Clone, Copy)]
pub struct M {
    pub t: Tex,
    pub c: Color,
    /// metros por repetição da textura
    pub tile: f32,
    pub spec: f32,
    pub emis: bool,
}

pub fn m(t: Tex, c: Color, tile: f32, spec: f32) -> M {
    M { t, c, tile, spec, emis: false }
}

pub fn emis(t: Tex, c: Color) -> M {
    M { t, c, tile: 1.0, spec: 0.0, emis: true }
}

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

impl Renderer {
    /// Caixa (rotação em Y) com UV em escala de mundo — a textura não estica.
    pub fn boxr(&mut self, c: Vec3, size: Vec3, rot_y: f32, mt: M) {
        let h = size * 0.5;
        let (s, co) = rot_y.sin_cos();
        let tr = |x: f32, y: f32, z: f32| -> Vec3 { vec3(c.x + x * co + z * s, c.y + y, c.z - x * s + z * co) };
        let rn = |n: Vec3| -> Vec3 { vec3(n.x * co + n.z * s, n.y, -n.x * s + n.z * co) };
        let k = 1.0 / mt.tile.max(0.001);
        let (x, y, z) = (h.x, h.y, h.z);
        let uvr = |a: f32, b: f32| -> [Vec2; 4] { [vec2(0.0, 0.0), vec2(a * k, 0.0), vec2(a * k, b * k), vec2(0.0, b * k)] };
        // topo, base, frente(+z), trás(-z), direita(+x), esquerda(-x)
        self.quad(mt.t, [tr(-x, y, -z), tr(x, y, -z), tr(x, y, z), tr(-x, y, z)], uvr(size.x, size.z), vec3(0., 1., 0.), mt.c, mt.spec, mt.emis);
        self.quad(mt.t, [tr(-x, -y, z), tr(x, -y, z), tr(x, -y, -z), tr(-x, -y, -z)], uvr(size.x, size.z), vec3(0., -1., 0.), mt.c, mt.spec, mt.emis);
        self.quad(mt.t, [tr(-x, y, z), tr(x, y, z), tr(x, -y, z), tr(-x, -y, z)], uvr(size.x, size.y), rn(vec3(0., 0., 1.)), mt.c, mt.spec, mt.emis);
        self.quad(mt.t, [tr(x, y, -z), tr(-x, y, -z), tr(-x, -y, -z), tr(x, -y, -z)], uvr(size.x, size.y), rn(vec3(0., 0., -1.)), mt.c, mt.spec, mt.emis);
        self.quad(mt.t, [tr(x, y, z), tr(x, y, -z), tr(x, -y, -z), tr(x, -y, z)], uvr(size.z, size.y), rn(vec3(1., 0., 0.)), mt.c, mt.spec, mt.emis);
        self.quad(mt.t, [tr(-x, y, -z), tr(-x, y, z), tr(-x, -y, z), tr(-x, -y, -z)], uvr(size.z, size.y), rn(vec3(-1., 0., 0.)), mt.c, mt.spec, mt.emis);
    }

    pub fn bx(&mut self, c: Vec3, size: Vec3, mt: M) {
        self.boxr(c, size, 0.0, mt);
    }

    pub fn has_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    /// Desenha um modelo externo no piso em `at`, girado `rot_y` (mesma convenção de `boxr`).
    /// `over` substitui materiais pelo nome (ex.: ("tela", emissivo da área de trabalho)).
    /// Devolve false se o modelo não existe — quem chama desenha a versão procedural.
    pub fn model(&mut self, name: &str, at: Vec3, rot_y: f32, over: &[(&str, M)]) -> bool {
        self.model_s(name, at, rot_y, 1.0, over)
    }

    /// Como `model`, com escala uniforme.
    pub fn model_s(&mut self, name: &str, at: Vec3, rot_y: f32, scale: f32, over: &[(&str, M)]) -> bool {
        let Some(model) = self.models.remove(name) else { return false };
        let (s, co) = rot_y.sin_cos();
        let tr = |p: Vec3| -> Vec3 {
            let p = p * scale;
            vec3(at.x + p.x * co + p.z * s, at.y + p.y, at.z - p.x * s + p.z * co)
        };
        let rn = |n: Vec3| -> Vec3 { vec3(n.x * co + n.z * s, n.y, -n.x * s + n.z * co) };
        for part in &model.parts {
            let mt = over.iter().find(|(k, _)| *k == part.material).map(|(_, m)| *m).unwrap_or(part.mt);
            let k = 1.0 / mt.tile.max(0.001);
            let w = if mt.emis { 2.0 } else { mt.spec };
            for (verts, idx) in &part.chunks {
                let i = self.batch_room(mt.t, verts.len(), idx.len());
                let b = &mut self.batches[i];
                let base = b.1.len() as u16;
                let raw_uv = mt.tile >= 0.999 && (mt.emis || mt.t == Tex::Screen || mt.t == Tex::Desktop || mt.t.is_image());
                for &(p, n, uv) in verts {
                    let wp = tr(p);
                    let ao = if mt.emis { 1.0 } else { (0.62 + wp.y * 0.9).min(1.0) };
                    let col = [(mt.c.r * ao * 255.0) as u8, (mt.c.g * ao * 255.0) as u8, (mt.c.b * ao * 255.0) as u8, (mt.c.a * 255.0) as u8];
                    // imagens PNG (linha 0 no topo) e o "tela" do Blender (u invertido) usam convenções opostas
                    let uv = if mt.t.is_image() { vec2(uv.x, 1.0 - uv.y) } else if raw_uv { uv } else { uv * k };
                    b.1.push(Vertex { position: wp, uv, color: col, normal: rn(n).extend(w) });
                }
                b.2.extend(idx.iter().map(|&x| base + x));
            }
        }
        self.models.insert(name.to_string(), model);
        true
    }

    /// Desenha um modelo com transformação completa (`m`: local → mundo) como objeto em mãos:
    /// vai para os lotes à parte (sem sombra própria, sem invalidar o mapa de sombras).
    pub fn model_held(&mut self, name: &str, m: Mat4, over: &[(&str, M)]) -> bool {
        let Some(model) = self.models.get(name) else { return false };
        let nm = Mat3::from_mat4(m).inverse().transpose();
        for part in &model.parts {
            let mt = over.iter().find(|(k, _)| *k == part.material).map(|(_, x)| *x).unwrap_or(part.mt);
            let k = 1.0 / mt.tile.max(0.001);
            let w = if mt.emis { 2.0 } else { mt.spec };
            let raw_uv = mt.tile >= 0.999 && (mt.emis || mt.t == Tex::Screen || mt.t == Tex::Desktop || mt.t.is_image());
            let col = [(mt.c.r * 255.0) as u8, (mt.c.g * 255.0) as u8, (mt.c.b * 255.0) as u8, 255];
            for (verts, idx) in &part.chunks {
                let i = match self.vm_batches.iter().position(|b| b.0 == mt.t && b.1.len() + verts.len() <= MAX_BATCH_VERTS && b.2.len() + idx.len() <= MAX_BATCH_INDICES) {
                    Some(i) => i,
                    None => {
                        self.vm_batches.push((mt.t, Vec::with_capacity(verts.len()), Vec::with_capacity(idx.len())));
                        self.vm_batches.len() - 1
                    }
                };
                let b = &mut self.vm_batches[i];
                let base = b.1.len() as u16;
                for &(p, n, uv) in verts {
                    let uv = if mt.t.is_image() { vec2(uv.x, 1.0 - uv.y) } else if raw_uv { uv } else { uv * k };
                    b.1.push(Vertex { position: m.transform_point3(p), uv, color: col, normal: (nm * n).normalize_or_zero().extend(w) });
                }
                b.2.extend(idx.iter().map(|&x| base + x));
            }
        }
        true
    }

    /// Lote da textura com espaço para mais `n` vértices.
    fn batch_room(&mut self, t: Tex, nv: usize, ni: usize) -> usize {
        if let Some(i) = self.batches.iter().position(|b| b.0 == t && b.1.len() + nv <= MAX_BATCH_VERTS && b.2.len() + ni <= MAX_BATCH_INDICES) {
            return i;
        }
        self.batches.push((t, Vec::with_capacity(nv.max(4096)), Vec::with_capacity(ni.max(8192))));
        self.batches.len() - 1
    }

    /// Face única (ex.: tela) com UV 0..1 — usada para a captura da área de trabalho.
    pub fn panel(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, n: Vec3, mt: M, flip_v: bool) {
        let uv = if flip_v { [vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)] } else { [vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)] };
        self.quad(mt.t, [a, b, c, d], uv, n, mt.c, mt.spec, mt.emis);
    }

    /// Cilindro vertical.
    pub fn cyl(&mut self, c: Vec3, r: f32, h: f32, mt: M) {
        let n = 16;
        let top = vec3(0.0, h / 2.0, 0.0);
        for i in 0..n {
            let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
            let p0 = vec3(a0.cos() * r, 0.0, a0.sin() * r);
            let p1 = vec3(a1.cos() * r, 0.0, a1.sin() * r);
            let u0 = i as f32 / n as f32 * r * 6.28 / mt.tile;
            let u1 = (i + 1) as f32 / n as f32 * r * 6.28 / mt.tile;
            self.quad(mt.t, [c + p0 + top, c + p1 + top, c + p1 - top, c + p0 - top], [vec2(u0, 0.0), vec2(u1, 0.0), vec2(u1, h / mt.tile), vec2(u0, h / mt.tile)], (p0 + p1).normalize(), mt.c, mt.spec, mt.emis);
            self.quad(mt.t, [c + top, c + top, c + p1 + top, c + p0 + top], [vec2(0.5, 0.5), vec2(0.5, 0.5), vec2(0.5 + p1.x, 0.5 + p1.z), vec2(0.5 + p0.x, 0.5 + p0.z)], vec3(0., 1., 0.), mt.c, mt.spec, mt.emis);
        }
    }

    /// Placa inclinada em X (tela de notebook), com face frontal de material próprio.
    pub fn slab_tilt(&mut self, base: Vec3, w: f32, h: f32, d: f32, tilt: f32, body: M, face: M) {
        let (s, c) = tilt.sin_cos();
        let up = vec3(0.0, c, -s);
        let back = vec3(0.0, s, c) * -1.0;
        let x = vec3(w / 2.0, 0.0, 0.0);
        let top = up * h;
        let th = back * d;
        let fnorm = vec3(0.0, s, c);
        let p = |a: Vec3| base + a;
        self.panel(p(-x + top), p(x + top), p(x), p(-x), fnorm, face, true);
        let uv = [vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)];
        self.quad(body.t, [p(x + th), p(-x + th), p(-x + th + top), p(x + th + top)], uv, -fnorm, body.c, body.spec, false);
        self.quad(body.t, [p(-x + top), p(x + top), p(x + top + th), p(-x + top + th)], uv, up, body.c, body.spec, false);
        self.quad(body.t, [p(x), p(x + th), p(x + th + top), p(x + top)], uv, vec3(1., 0., 0.), body.c, body.spec, false);
        self.quad(body.t, [p(-x + th), p(-x), p(-x + top), p(-x + th + top)], uv, vec3(-1., 0., 0.), body.c, body.spec, false);
    }

    /// Sombra de contato suave: quadrilátero com degradê radial (textura Blob).
    pub fn shadow(&mut self, base: Vec3, half: Vec3) {
        let (hx, hz) = (half.x * 1.9 + 0.03, half.z * 1.9 + 0.03);
        let y = base.y + 0.0012;
        let uv = [vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)];
        self.quad(Tex::Blob, [vec3(base.x - hx, y, base.z - hz), vec3(base.x + hx, y, base.z - hz), vec3(base.x + hx, y, base.z + hz), vec3(base.x - hx, y, base.z + hz)], uv, vec3(0., 1., 0.), Color::new(0.0, 0.0, 0.0, 0.55), 0.0, true);
    }
}
