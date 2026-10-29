//! Recursos visuais fornecidos externamente (modelos 3D e texturas).
//!
//! Princípios:
//! * os arquivos originais NUNCA são alterados: são apenas lidos;
//! * nada é obrigatório: sem a pasta de assets, o simulador usa a geometria e as texturas
//!   procedurais de sempre;
//! * cada substituição é validada (formato, dimensões) e registrada; o que não puder ser
//!   usado é recusado com o motivo, sem interromper o programa.
//!
//! Pasta procurada, nesta ordem: variável `PERICIA_ASSETS`; `assets/` ao lado do
//! executável; `assets/` no diretório atual.
//!
//! Formatos suportados hoje sem dependências novas: texturas PNG e TGA (decodificador do
//! macroquad) e modelos OBJ/MTL (leitor abaixo). glTF, FBX e .blend exigem avaliação
//! quando os arquivos chegarem (ver `assets/LEIA-ME.md` e o inventário).

use macroquad::prelude::*;
use std::path::{Path, PathBuf};

/// Lado máximo aceito para uma textura (memória de vídeo e desempenho em GPUs fracas).
pub const MAX_TEX_SIDE: u32 = 2048;

pub fn assets_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("PERICIA_ASSETS") {
        let p = PathBuf::from(p);
        return p.is_dir().then_some(p);
    }
    let exe_side = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("assets")));
    for c in exe_side.into_iter().chain(std::iter::once(PathBuf::from("assets"))) {
        if c.is_dir() {
            return Some(c);
        }
    }
    None
}

/// Verifica se a textura pode ser usada com repetição (GLES2/WebGL exigem potência de 2
/// para `Repeat` com mipmaps) e dentro do limite de tamanho.
pub fn validate_texture_dims(w: u32, h: u32, repeat: bool) -> Result<(), String> {
    if w == 0 || h == 0 {
        return Err("dimensões nulas".into());
    }
    if w > MAX_TEX_SIDE || h > MAX_TEX_SIDE {
        return Err(format!("{}×{} excede o limite de {} px por lado", w, h, MAX_TEX_SIDE));
    }
    if repeat && (!w.is_power_of_two() || !h.is_power_of_two()) {
        return Err(format!("{}×{} não é potência de 2 (necessário para textura repetida em GL 2.1/GLES 2)", w, h));
    }
    Ok(())
}

/// Procura `textures/<nome>.png` ou `.tga`. `Ok(None)` = não há substituição.
pub fn load_texture_override(name: &str, repeat: bool) -> Result<Option<(Texture2D, String)>, String> {
    let Some(dir) = assets_dir() else { return Ok(None) };
    let cands = [dir.join("textures").join(format!("{}.png", name)), dir.join("textures").join(format!("{}.tga", name))];
    let Some(path) = cands.iter().find(|p| p.is_file()) else { return Ok(None) };
    let bytes = std::fs::read(path).map_err(|e| format!("{}: não foi possível ler ({})", path.display(), e))?;
    let img = Image::from_file_with_format(&bytes, None).map_err(|e| format!("{}: formato não reconhecido ({:?}) — mantida a textura procedural", path.display(), e))?;
    let (w, h) = (img.width as u32, img.height as u32);
    validate_texture_dims(w, h, repeat).map_err(|e| format!("{}: {} — mantida a textura procedural", path.display(), e))?;
    let tx = crate::render::make_texture(&img.bytes, w, h, repeat);
    Ok(Some((tx, format!("textura '{}' substituída por {} ({}×{})", name, path.display(), w, h))))
}

// ------------------------------------------------------------------ OBJ / MTL

#[derive(Clone, Debug, Default)]
pub struct ObjMesh {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub normals: Vec<[f32; 3]>,
    /// Vértices já desindexados por combinação (v, vt, vn) e triangulados.
    pub vertices: Vec<ObjVertex>,
    pub indices: Vec<u32>,
    /// Faixas de índices por material: (nome do material, início, quantidade).
    pub groups: Vec<(String, usize, usize)>,
    pub mtllibs: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MtlMaterial {
    pub name: String,
    pub diffuse: [f32; 3],
    pub map_kd: Option<String>,
}

/// Leitor de OBJ (texto). Suporta v/vt/vn/f (polígonos triangulados em leque, índices
/// negativos), usemtl e mtllib. Calcula normais por face quando ausentes.
pub fn parse_obj(text: &str) -> Result<ObjMesh, String> {
    let mut m = ObjMesh::default();
    let mut map: std::collections::HashMap<(usize, Option<usize>, Option<usize>), u32> = Default::default();
    let mut cur_mat = String::from("(padrão)");
    let mut group_start = 0usize;
    for (ln, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let tag = it.next().unwrap_or("");
        let nums = |it: std::str::SplitWhitespace| -> Vec<f32> { it.filter_map(|x| x.parse::<f32>().ok()).collect() };
        match tag {
            "v" => {
                let n = nums(it);
                if n.len() < 3 {
                    return Err(format!("linha {}: vértice com menos de 3 coordenadas", ln + 1));
                }
                m.positions.push([n[0], n[1], n[2]]);
            }
            "vt" => {
                let n = nums(it);
                m.uvs.push([*n.first().unwrap_or(&0.0), *n.get(1).unwrap_or(&0.0)]);
            }
            "vn" => {
                let n = nums(it);
                if n.len() >= 3 {
                    m.normals.push([n[0], n[1], n[2]]);
                }
            }
            "usemtl" => {
                if m.indices.len() > group_start {
                    m.groups.push((cur_mat.clone(), group_start, m.indices.len() - group_start));
                }
                cur_mat = it.collect::<Vec<_>>().join(" ");
                group_start = m.indices.len();
            }
            "mtllib" => m.mtllibs.push(it.collect::<Vec<_>>().join(" ")),
            "f" => {
                let mut poly: Vec<u32> = vec![];
                for tok in it {
                    let mut p = tok.split('/');
                    let resolve = |s: Option<&str>, len: usize| -> Result<Option<usize>, String> {
                        match s.filter(|s| !s.is_empty()) {
                            None => Ok(None),
                            Some(s) => {
                                let i: i64 = s.parse().map_err(|_| format!("linha {}: índice inválido '{}'", ln + 1, s))?;
                                let idx = if i < 0 { len as i64 + i } else { i - 1 };
                                if idx < 0 || idx as usize >= len {
                                    return Err(format!("linha {}: índice {} fora do intervalo", ln + 1, i));
                                }
                                Ok(Some(idx as usize))
                            }
                        }
                    };
                    let vi = resolve(p.next(), m.positions.len())?.ok_or(format!("linha {}: face sem vértice", ln + 1))?;
                    let ti = resolve(p.next(), m.uvs.len())?;
                    let ni = resolve(p.next(), m.normals.len())?;
                    let key = (vi, ti, ni);
                    let id = if let Some(&id) = map.get(&key) {
                        id
                    } else {
                        let v = ObjVertex {
                            pos: m.positions[vi],
                            uv: ti.map(|t| m.uvs[t]).unwrap_or([0.0, 0.0]),
                            normal: ni.map(|n| m.normals[n]).unwrap_or([0.0, 0.0, 0.0]),
                        };
                        m.vertices.push(v);
                        let id = (m.vertices.len() - 1) as u32;
                        map.insert(key, id);
                        id
                    };
                    poly.push(id);
                }
                if poly.len() < 3 {
                    m.warnings.push(format!("linha {}: face com menos de 3 vértices ignorada", ln + 1));
                    continue;
                }
                for k in 1..poly.len() - 1 {
                    m.indices.extend_from_slice(&[poly[0], poly[k], poly[k + 1]]);
                }
            }
            "o" | "g" | "s" => {}
            other => {
                if !m.warnings.iter().any(|w| w.contains(&format!("'{}'", other))) {
                    m.warnings.push(format!("diretiva '{}' ignorada", other));
                }
            }
        }
    }
    if m.indices.len() > group_start {
        m.groups.push((cur_mat, group_start, m.indices.len() - group_start));
    }
    if m.vertices.len() > u16::MAX as usize {
        m.warnings.push(format!("{} vértices: acima de 65 535 — será preciso dividir a malha (índices de 16 bits no macroquad)", m.vertices.len()));
    }
    // normais ausentes: média das normais das faces
    if m.vertices.iter().any(|v| v.normal == [0.0, 0.0, 0.0]) {
        let mut acc = vec![Vec3::ZERO; m.vertices.len()];
        for t in m.indices.chunks(3) {
            let (a, b, c) = (Vec3::from(m.vertices[t[0] as usize].pos), Vec3::from(m.vertices[t[1] as usize].pos), Vec3::from(m.vertices[t[2] as usize].pos));
            let n = (b - a).cross(c - a);
            for &i in t {
                acc[i as usize] += n;
            }
        }
        for (v, n) in m.vertices.iter_mut().zip(acc) {
            if v.normal == [0.0, 0.0, 0.0] {
                v.normal = n.normalize_or_zero().into();
            }
        }
    }
    Ok(m)
}

pub fn parse_mtl(text: &str) -> Vec<MtlMaterial> {
    let mut out: Vec<MtlMaterial> = vec![];
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        let mut it = line.split_whitespace();
        match it.next() {
            Some("newmtl") => out.push(MtlMaterial { name: it.collect::<Vec<_>>().join(" "), diffuse: [1.0, 1.0, 1.0], map_kd: None }),
            Some("Kd") => {
                let n: Vec<f32> = it.filter_map(|x| x.parse().ok()).collect();
                if let (Some(m), true) = (out.last_mut(), n.len() >= 3) {
                    m.diffuse = [n[0], n[1], n[2]];
                }
            }
            Some("map_Kd") => {
                if let Some(m) = out.last_mut() {
                    // a última palavra é o arquivo (opções como -s/-o vêm antes)
                    m.map_kd = line.split_whitespace().last().map(|s| s.to_string());
                }
            }
            _ => {}
        }
    }
    out
}

/// Caixa envolvente (mín., máx.) — usada para checar a escala antes de integrar um modelo.
pub fn bounds(m: &ObjMesh) -> Option<([f32; 3], [f32; 3])> {
    let mut it = m.vertices.iter().map(|v| v.pos);
    let first = it.next()?;
    Some(it.fold((first, first), |(mn, mx), p| ([mn[0].min(p[0]), mn[1].min(p[1]), mn[2].min(p[2])], [mx[0].max(p[0]), mx[1].max(p[1]), mx[2].max(p[2])])))
}

/// Valida (sem integrar à cena) os modelos OBJ presentes em `assets/models`: lê, triangula,
/// mede a caixa envolvente e confere as texturas referenciadas. É o primeiro passo da
/// integração gradual: o relatório diz o que pode ser usado e em que escala.
pub fn validate_models() -> Vec<String> {
    let Some(dir) = assets_dir().map(|d| d.join("models")) else { return vec![] };
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    let mut out = vec![];
    let mut paths: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for p in paths {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "obj" => match read_model(&p) {
                Ok((m, mats)) => {
                    let (mn, mx) = bounds(&m).unwrap_or(([0.0; 3], [0.0; 3]));
                    let size = [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]];
                    let tex_missing: Vec<String> = mats.iter().filter_map(|mt| mt.map_kd.clone()).filter(|t| !p.parent().map(|d| d.join(t).exists()).unwrap_or(false)).collect();
                    out.push(format!(
                        "modelo {}: {} vértices, {} triângulos, {} material(is); dimensões {:.2} × {:.2} × {:.2} (unidades do arquivo; a cena usa metros){}{}",
                        p.display(),
                        m.vertices.len(),
                        m.indices.len() / 3,
                        mats.len(),
                        size[0],
                        size[1],
                        size[2],
                        if tex_missing.is_empty() { String::new() } else { format!("; texturas ausentes: {}", tex_missing.join(", ")) },
                        if m.warnings.is_empty() { String::new() } else { format!("; avisos: {}", m.warnings.join("; ")) }
                    ));
                }
                Err(e) => out.push(format!("modelo {}: recusado ({})", p.display(), e)),
            },
            "gltf" | "glb" | "fbx" | "blend" | "dae" => out.push(format!("modelo {}: formato .{} ainda não suportado — ver inventário (conversão para OBJ ou leitor glTF)", p.display(), ext)),
            _ => {}
        }
    }
    out
}

pub fn read_model(path: &Path) -> Result<(ObjMesh, Vec<MtlMaterial>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut mesh = parse_obj(&text)?;
    let mut mats = vec![];
    for lib in mesh.mtllibs.clone() {
        match path.parent().map(|d| std::fs::read_to_string(d.join(&lib))) {
            Some(Ok(t)) => mats.extend(parse_mtl(&t)),
            _ => mesh.warnings.push(format!("biblioteca de materiais '{}' ausente", lib)),
        }
    }
    Ok((mesh, mats))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBE_QUAD: &str = "# quadrado com UV e normal\nmtllib caixa.mtl\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvn 0 0 1\nusemtl Madeira\nf 1/1/1 2/2/1 3/3/1 4/4/1\n";

    #[test]
    fn obj_quadrilatero_e_triangulado_com_uv_e_material() {
        let m = parse_obj(CUBE_QUAD).unwrap();
        assert_eq!(m.vertices.len(), 4);
        assert_eq!(m.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(m.groups, vec![("Madeira".to_string(), 0, 6)]);
        assert_eq!(m.mtllibs, vec!["caixa.mtl".to_string()]);
        assert_eq!(m.vertices[2].uv, [1.0, 1.0]);
        assert_eq!(bounds(&m), Some(([0.0, 0.0, 0.0], [1.0, 1.0, 0.0])));
    }

    #[test]
    fn obj_indices_negativos_e_normais_calculadas() {
        let m = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
        assert_eq!(m.indices.len(), 3);
        let n = m.vertices[0].normal;
        assert!((n[2] - 1.0).abs() < 1e-5, "{:?}", n);
    }

    #[test]
    fn obj_invalido_e_recusado_com_linha() {
        let e = parse_obj("v 0 0 0\nf 1 2 3\n").unwrap_err();
        assert!(e.contains("linha 2"), "{}", e);
    }

    #[test]
    fn mtl_le_cor_e_textura() {
        let m = parse_mtl("newmtl Madeira\nKd 0.5 0.3 0.1\nmap_Kd -s 2 2 1 texturas/madeira.png\n");
        assert_eq!(m, vec![MtlMaterial { name: "Madeira".into(), diffuse: [0.5, 0.3, 0.1], map_kd: Some("texturas/madeira.png".into()) }]);
    }

    #[test]
    fn dimensoes_de_textura_sao_validadas() {
        assert!(validate_texture_dims(512, 512, true).is_ok());
        assert!(validate_texture_dims(500, 512, true).is_err());
        assert!(validate_texture_dims(500, 512, false).is_ok());
        assert!(validate_texture_dims(4096, 4096, true).is_err());
    }
}
