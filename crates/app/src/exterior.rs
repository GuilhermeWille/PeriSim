//! Vida lá fora, vista pela janela: árvores das calçadas balançando ao vento e carros
//! passando (e estacionados) na rua. Poucos polígonos, formas arredondadas (copas em
//! icosfera, carroceria chanfrada, rodas de 8 lados). Tudo emissivo, com o sol e a névoa
//! "assados" por face — o mesmo visual da paisagem pré-renderizada atrás.

use crate::render::{Renderer, Tex};
use macroquad::prelude::*;
use std::sync::OnceLock;

/// Piso externo (5º andar: a rua fica 14 m abaixo da sala).
const GY: f32 = -14.0;
const JANELA: Vec3 = Vec3::new(-3.5, 1.5, 0.75);

fn sombra(n: Vec3) -> f32 {
    let sun = vec3(0.45, 0.75, 0.48).normalize();
    0.62 + 0.38 * n.dot(sun).max(0.0) + if n.y > 0.5 { 0.05 } else { 0.0 }
}

fn cor(base: [f32; 3], n: Vec3, p: Vec3) -> Color {
    let light = sombra(n);
    let hz = [0.78, 0.84, 0.90];
    let k = 1.0 - (-(p - JANELA).length() / 220.0).exp();
    let f = |a: f32, b: f32| (a * light) * (1.0 - k) + b * k;
    Color::new(f(base[0], hz[0]), f(base[1], hz[1]), f(base[2], hz[2]), 1.0)
}

/// `dentro`: um ponto no interior do sólido — a normal sempre aponta para fora.
fn tri(r: &mut Renderer, a: Vec3, b: Vec3, c: Vec3, base: [f32; 3], dentro: Vec3) {
    let mut n = (b - a).cross(c - a).normalize_or_zero();
    if n.dot((a + b + c) / 3.0 - dentro) < 0.0 {
        n = -n;
    }
    let col = cor(base, n, a);
    r.quad(Tex::White, [a, b, c, c], [vec2(0.0, 0.0); 4], n, col, 0.0, true);
}

fn quad(r: &mut Renderer, p: [Vec3; 4], base: [f32; 3], dentro: Vec3) {
    let mut n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
    if n.dot((p[0] + p[2]) * 0.5 - dentro) < 0.0 {
        n = -n;
    }
    let col = cor(base, n, p[0]);
    r.quad(Tex::White, p, [vec2(0.0, 0.0); 4], n, col, 0.0, true);
}

/// Icosaedro subdividido uma vez (80 triângulos), vértices na esfera unitária.
fn icosfera() -> &'static (Vec<Vec3>, Vec<[usize; 3]>) {
    static ICO: OnceLock<(Vec<Vec3>, Vec<[usize; 3]>)> = OnceLock::new();
    ICO.get_or_init(|| {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let mut v: Vec<Vec3> = [
            (-1.0, t, 0.0), (1.0, t, 0.0), (-1.0, -t, 0.0), (1.0, -t, 0.0),
            (0.0, -1.0, t), (0.0, 1.0, t), (0.0, -1.0, -t), (0.0, 1.0, -t),
            (t, 0.0, -1.0), (t, 0.0, 1.0), (-t, 0.0, -1.0), (-t, 0.0, 1.0),
        ]
        .iter()
        .map(|&(x, y, z)| vec3(x, y, z).normalize())
        .collect();
        let f0: [[usize; 3]; 20] = [
            [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11], [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
            [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9], [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
        ];
        let mut cache = std::collections::HashMap::new();
        let mut meio = |a: usize, b: usize, v: &mut Vec<Vec3>| -> usize {
            let k = (a.min(b), a.max(b));
            *cache.entry(k).or_insert_with(|| {
                v.push(((v[a] + v[b]) * 0.5).normalize());
                v.len() - 1
            })
        };
        let mut f = vec![];
        for [a, b, c] in f0 {
            let ab = meio(a, b, &mut v);
            let bc = meio(b, c, &mut v);
            let ca = meio(c, a, &mut v);
            f.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        (v, f)
    })
}

/// Volume arredondado (elipsoide de 80 faces), com leve irregularidade por semente.
fn bolota(r: &mut Renderer, c: Vec3, raio: Vec3, base: [f32; 3], semente: f32) {
    let (v, f) = icosfera();
    let p: Vec<Vec3> = v.iter().enumerate().map(|(i, q)| {
        let ruido = 1.0 + 0.08 * ((i as f32 * 12.9898 + semente * 78.233).sin() * 43758.545).fract();
        c + *q * raio * ruido
    }).collect();
    for [a, b, d] in f {
        tri(r, p[*a], p[*b], p[*d], base, c);
    }
}

/// Prisma de `lados` faces em pé (tronco), afinando para cima.
fn tronco(r: &mut Renderer, base: Vec3, raio: f32, alt: f32, lados: usize, cor_: [f32; 3]) {
    for k in 0..lados {
        let a0 = k as f32 / lados as f32 * std::f32::consts::TAU;
        let a1 = (k + 1) as f32 / lados as f32 * std::f32::consts::TAU;
        let (d0, d1) = (vec3(a0.cos(), 0.0, a0.sin()), vec3(a1.cos(), 0.0, a1.sin()));
        quad(r, [base + d0 * raio * 0.7 + vec3(0.0, alt, 0.0), base + d1 * raio * 0.7 + vec3(0.0, alt, 0.0), base + d1 * raio, base + d0 * raio], cor_, base + vec3(0.0, alt * 0.5, 0.0));
    }
}

/// Árvore: tronco e copa em três volumes que balançam ao vento (fase própria).
fn arvore(r: &mut Renderer, pos: Vec3, alt: f32, larg: f32, semente: f32, t: f32) {
    tronco(r, pos, 0.2, alt * 0.75, 6, [0.38, 0.28, 0.2]);
    let vento = vec3((t * 0.9 + semente).sin(), 0.0, (t * 0.7 + semente * 1.3).cos() * 0.6) * (0.10 + 0.05 * (t * 2.3 + semente).sin());
    let topo = pos + vec3(0.0, alt, 0.0);
    let folha = |j: f32| [0.22 + 0.06 * ((semente + j) * 3.1).sin().abs(), 0.42 + 0.1 * ((semente + j) * 1.7).cos().abs(), 0.2];
    let lobos = [(vec3(0.0, 0.0, 0.0), 1.0), (vec3(larg * 0.32, -larg * 0.12, larg * 0.18), 0.72), (vec3(-larg * 0.28, larg * 0.08, -larg * 0.22), 0.78)];
    for (k, (off, s)) in lobos.iter().enumerate() {
        // o alto da copa mexe mais que a base
        let c = topo + *off + vento * (1.0 + k as f32 * 0.4);
        bolota(r, c, vec3(larg * 0.55, larg * 0.42, larg * 0.55) * *s, folha(k as f32), semente + k as f32);
    }
}

/// Seção da carroceria (meia-largura, altura) com cantos chanfrados, extrudada ao longo de z.
fn extrusao(r: &mut Renderer, c: Vec3, secao: &[(f32, f32)], comp: f32, base: [f32; 3]) {
    let n = secao.len();
    let (z0, z1) = (c.z - comp / 2.0, c.z + comp / 2.0);
    let p = |(x, y): (f32, f32), z: f32| vec3(c.x + x, c.y + y, z);
    let cy = secao.iter().map(|q| q.1).sum::<f32>() / n as f32;
    let dentro = vec3(c.x, c.y + cy, c.z);
    for k in 0..n {
        let (a, b) = (secao[k], secao[(k + 1) % n]);
        quad(r, [p(a, z1), p(b, z1), p(b, z0), p(a, z0)], base, dentro);
    }
    // tampas (leque)
    for k in 1..n - 1 {
        tri(r, p(secao[0], z1), p(secao[k + 1], z1), p(secao[k], z1), base, dentro);
        tri(r, p(secao[0], z0), p(secao[k], z0), p(secao[k + 1], z0), base, dentro);
    }
}

fn roda(r: &mut Renderer, c: Vec3, lado: f32) {
    let (rr, w) = (0.33, 0.22);
    let mut ring = vec![];
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU;
        ring.push(vec3(0.0, a.sin() * rr, a.cos() * rr));
    }
    let x0 = c.x - lado * w / 2.0;
    let x1 = c.x + lado * w / 2.0;
    for k in 0..8 {
        let (a, b) = (ring[k], ring[(k + 1) % 8]);
        quad(r, [vec3(x1, c.y, c.z) + a, vec3(x1, c.y, c.z) + b, vec3(x0, c.y, c.z) + b, vec3(x0, c.y, c.z) + a], [0.08, 0.08, 0.09], c);
    }
    for k in 1..7 {
        tri(r, vec3(x1, c.y, c.z) + ring[0], vec3(x1, c.y, c.z) + ring[k], vec3(x1, c.y, c.z) + ring[k + 1], [0.55, 0.56, 0.58], c);
    }
}

/// Carro de passeio (≈ 4,2 × 1,8 m): carroceria e cabine chanfradas, rodas, vidros e faróis.
/// `sentido` = +1 indo para +z, −1 para −z.
fn carro(r: &mut Renderer, pos: Vec3, sentido: f32, tinta: [f32; 3]) {
    let corpo = [(-0.9, 0.25), (0.9, 0.25), (0.92, 0.55), (0.85, 0.82), (-0.85, 0.82), (-0.92, 0.55)];
    extrusao(r, pos, &corpo, 4.2, tinta);
    // cabine: mais estreita e afinando no teto, deslocada para trás
    let cab = [(-0.8, 0.82), (0.8, 0.82), (0.66, 1.3), (-0.66, 1.3)];
    let cc = pos + vec3(0.0, 0.0, -0.25 * sentido);
    extrusao(r, cc, &cab, 2.1, [tinta[0] * 0.85, tinta[1] * 0.85, tinta[2] * 0.85]);
    // vidros laterais e para-brisa (escuros)
    let vidro = [0.16, 0.2, 0.26];
    for sx in [-1.0f32, 1.0] {
        let x = cc.x + sx * 0.74;
        quad(r, [vec3(x, cc.y + 0.88, cc.z - 0.9), vec3(x, cc.y + 0.88, cc.z + 0.9), vec3(x - sx * 0.1, cc.y + 1.24, cc.z + 0.8), vec3(x - sx * 0.1, cc.y + 1.24, cc.z - 0.8)], vidro, cc + vec3(0.0, 1.0, 0.0));
    }
    let zf = cc.z + sentido * 1.06;
    quad(r, [vec3(cc.x - 0.75, cc.y + 0.86, zf), vec3(cc.x + 0.75, cc.y + 0.86, zf), vec3(cc.x + 0.6, cc.y + 1.26, zf - sentido * 0.25), vec3(cc.x - 0.6, cc.y + 1.26, zf - sentido * 0.25)], vidro, cc + vec3(0.0, 1.0, 0.0));
    // faróis (frente) e lanternas (trás)
    for sx in [-0.62f32, 0.62] {
        let fr = vec3(pos.x + sx, pos.y + 0.6, pos.z + sentido * 2.11);
        r.quad(Tex::White, [fr + vec3(-0.16, 0.06, 0.0), fr + vec3(0.16, 0.06, 0.0), fr + vec3(0.16, -0.06, 0.0), fr + vec3(-0.16, -0.06, 0.0)], [vec2(0.0, 0.0); 4], vec3(0.0, 0.0, sentido), Color::new(1.0, 0.97, 0.85, 1.0), 0.0, true);
        let tr = vec3(pos.x + sx, pos.y + 0.62, pos.z - sentido * 2.11);
        r.quad(Tex::White, [tr + vec3(-0.15, 0.05, 0.0), tr + vec3(0.15, 0.05, 0.0), tr + vec3(0.15, -0.05, 0.0), tr + vec3(-0.15, -0.05, 0.0)], [vec2(0.0, 0.0); 4], vec3(0.0, 0.0, -sentido), Color::new(0.85, 0.12, 0.1, 1.0), 0.0, true);
    }
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        roda(r, pos + vec3(sx * 0.82, 0.33, sz * 1.35), sx);
    }
}

/// Posições das árvores das duas calçadas (determinísticas).
fn arvores() -> &'static Vec<(Vec3, f32, f32, f32)> {
    static A: OnceLock<Vec<(Vec3, f32, f32, f32)>> = OnceLock::new();
    A.get_or_init(|| {
        let mut seed = 7u32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            ((seed >> 16) & 0x7fff) as f32 / 32767.0
        };
        let mut v = vec![];
        for x in [-12.0f32, -30.5] {
            let mut z = -48.0;
            while z < 50.0 {
                v.push((vec3(x, GY, z), 5.5 + rnd() * 3.0, 3.0 + rnd() * 1.5, rnd() * 10.0));
                z += 9.0 + rnd() * 6.0;
            }
        }
        v
    })
}

/// Desenha a vida da rua no instante `t` (segundos).
pub fn rua_viva(r: &mut Renderer, t: f32) {
    for (p, alt, larg, s) in arvores() {
        arvore(r, *p, *alt, *larg, *s, t);
    }
    // estacionados junto às calçadas
    let parados = [(-15.2f32, -22.0f32, 1.0f32, [0.7, 0.12, 0.1]), (-15.2, 4.0, 1.0, [0.2, 0.2, 0.22]), (-15.2, 31.0, -1.0, [0.75, 0.7, 0.55]), (-26.8, -9.0, -1.0, [0.85, 0.85, 0.88]), (-26.8, 15.0, 1.0, [0.15, 0.2, 0.35])];
    for (x, z, s, c) in parados {
        carro(r, vec3(x, GY, z), s, c);
    }
    // trânsito: duas mãos, velocidades diferentes, voltas de 180 m
    let fluxo = [(-18.6f32, 1.0f32, 9.0f32, 0.0f32, [0.9, 0.9, 0.92]), (-18.6, 1.0, 9.0, 70.0, [0.12, 0.25, 0.5]), (-18.6, 1.0, 11.0, 130.0, [0.55, 0.08, 0.1]), (-23.4, -1.0, 8.0, 20.0, [0.18, 0.18, 0.2]), (-23.4, -1.0, 10.0, 95.0, [0.8, 0.6, 0.15]), (-23.4, -1.0, 8.5, 150.0, [0.35, 0.45, 0.4])];
    for (x, s, v, fase, c) in fluxo {
        let z = ((t * v + fase) % 180.0) - 90.0;
        carro(r, vec3(x, GY, z * s), s, c);
    }
}
