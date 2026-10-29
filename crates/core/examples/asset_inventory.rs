//! Inventário de modelos 3D e texturas fornecidos (somente leitura).
//!
//!   cargo run --release -p pericia_core --example asset_inventory -- <pasta> [saida.md]
//!
//! Para cada arquivo: tipo, tamanho, SHA-256 (para provar depois que o original não foi
//! alterado), dimensões de imagens, contagens de OBJ/MTL, resumo de glTF/GLB, cabeçalhos
//! de FBX/.blend, referências a arquivos ausentes e possíveis licenças. Termina com a
//! compatibilidade com o pipeline atual (macroquad: PNG/TGA, OBJ/MTL) e o impacto estimado.
//! Nunca escreve na pasta examinada.

use pericia_core::sha256::sha256_bytes;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn png_dims(b: &[u8]) -> Option<(u32, u32, &'static str)> {
    if b.len() > 26 && &b[..8] == b"\x89PNG\r\n\x1a\n" {
        let ct = match b[25] { 0 => "cinza", 2 => "RGB", 3 => "paleta", 4 => "cinza+alfa", 6 => "RGBA", _ => "?" };
        return Some((be32(b, 16), be32(b, 20), ct));
    }
    None
}

fn jpeg_dims(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() < 4 || b[0] != 0xFF || b[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = b[i + 1];
        let len = ((b[i + 2] as usize) << 8) | b[i + 3] as usize;
        if (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC {
            let h = ((b[i + 5] as u32) << 8) | b[i + 6] as u32;
            let w = ((b[i + 7] as u32) << 8) | b[i + 8] as u32;
            return Some((w, h));
        }
        i += 2 + len;
    }
    None
}

fn tga_dims(b: &[u8]) -> Option<(u32, u32)> {
    (b.len() > 18).then(|| (u16::from_le_bytes([b[12], b[13]]) as u32, u16::from_le_bytes([b[14], b[15]]) as u32))
}

struct Row {
    rel: String,
    kind: String,
    size: u64,
    hash: String,
    details: String,
    compat: String,
    refs: Vec<String>,
}

fn gltf_summary(v: &serde_json::Value) -> (String, Vec<String>) {
    let n = |k: &str| v[k].as_array().map(|a| a.len()).unwrap_or(0);
    let mut refs = vec![];
    for k in ["images", "buffers"] {
        if let Some(a) = v[k].as_array() {
            for e in a {
                if let Some(u) = e["uri"].as_str() {
                    if !u.starts_with("data:") {
                        refs.push(u.to_string());
                    }
                }
            }
        }
    }
    let mut prims = 0;
    let mut tris_hint = 0u64;
    if let Some(ms) = v["meshes"].as_array() {
        for m in ms {
            if let Some(ps) = m["primitives"].as_array() {
                prims += ps.len();
                for p in ps {
                    if let Some(ix) = p["indices"].as_u64() {
                        tris_hint += v["accessors"][ix as usize]["count"].as_u64().unwrap_or(0) / 3;
                    }
                }
            }
        }
    }
    let gen = v["asset"]["generator"].as_str().unwrap_or("?");
    let copy = v["asset"]["copyright"].as_str().unwrap_or("(não informado)");
    (
        format!(
            "glTF {} · gerador: {} · cenas {} · nós {} · malhas {} ({} primitivas, ~{} triângulos) · materiais {} · texturas {} · imagens {} · animações {} · skins {} · copyright: {}",
            v["asset"]["version"].as_str().unwrap_or("?"),
            gen,
            n("scenes"),
            n("nodes"),
            n("meshes"),
            prims,
            tris_hint,
            n("materials"),
            n("textures"),
            n("images"),
            n("animations"),
            n("skins"),
            copy
        ),
        refs,
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(dir) = args.get(1).map(PathBuf::from) else {
        eprintln!("uso: asset_inventory <pasta> [saida.md]");
        std::process::exit(2);
    };
    let mut files = vec![];
    walk(&dir, &mut files);
    let mut rows = vec![];
    let mut total = 0u64;
    let mut licenses = vec![];
    for p in &files {
        let rel = p.strip_prefix(&dir).unwrap_or(p).display().to_string();
        let b = match std::fs::read(p) {
            Ok(b) => b,
            Err(e) => {
                rows.push(Row { rel, kind: "ilegível".into(), size: 0, hash: String::new(), details: e.to_string(), compat: "—".into(), refs: vec![] });
                continue;
            }
        };
        total += b.len() as u64;
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        let (mut kind, mut details, mut refs) = (ext.clone(), String::new(), vec![]);
        let mut compat: String;
        if name.contains("licen") || name.contains("copying") || name.contains("readme") || name.contains("credits") || name.contains("atribui") {
            licenses.push(rel.clone());
        }
        match ext.as_str() {
            "png" => {
                kind = "textura PNG".into();
                if let Some((w, h, ct)) = png_dims(&b) {
                    details = format!("{}×{} {}", w, h, ct);
                    let pot = w.is_power_of_two() && h.is_power_of_two();
                    compat = if w > 2048 || h > 2048 {
                        "carregável; reduzir para ≤ 2048 px".into()
                    } else if pot {
                        "compatível (PNG, potência de 2)".into()
                    } else {
                        "compatível só sem repetição; redimensionar para potência de 2 se for repetida".into()
                    };
                } else {
                    compat = "cabeçalho PNG inválido".into();
                }
            }
            "tga" => {
                kind = "textura TGA".into();
                if let Some((w, h)) = tga_dims(&b) {
                    details = format!("{}×{}", w, h);
                }
                compat = "compatível (TGA)".into();
            }
            "jpg" | "jpeg" => {
                kind = "textura JPEG".into();
                if let Some((w, h)) = jpeg_dims(&b) {
                    details = format!("{}×{}", w, h);
                }
                compat = "requer conversão para PNG (ou habilitar o decodificador JPEG)".into();
            }
            "obj" => {
                kind = "modelo OBJ".into();
                let t = String::from_utf8_lossy(&b);
                let c = |tag: &str| t.lines().filter(|l| l.trim_start().starts_with(tag)).count();
                let (v, vt, vn, f) = (c("v "), c("vt "), c("vn "), c("f "));
                for l in t.lines() {
                    if let Some(m) = l.trim().strip_prefix("mtllib ") {
                        refs.push(m.trim().to_string());
                    }
                }
                details = format!("vértices {} · UV {} · normais {} · faces {} · materiais usados {}", v, vt, vn, f, c("usemtl "));
                compat = if vt == 0 { "compatível; sem UV (só cor sólida)".into() } else { "compatível (leitor OBJ próprio)".into() };
                if v > 65_535 {
                    compat.push_str("; > 65 535 vértices: dividir a malha");
                }
            }
            "mtl" => {
                kind = "materiais MTL".into();
                let t = String::from_utf8_lossy(&b);
                let mats = t.lines().filter(|l| l.trim_start().starts_with("newmtl")).count();
                for l in t.lines() {
                    let l = l.trim();
                    if l.starts_with("map_") {
                        if let Some(f) = l.split_whitespace().last() {
                            refs.push(f.to_string());
                        }
                    }
                }
                details = format!("{} material(is)", mats);
                compat = "compatível (cor difusa e map_Kd)".into();
            }
            "gltf" => {
                kind = "modelo glTF (JSON)".into();
                match serde_json::from_slice::<serde_json::Value>(&b) {
                    Ok(v) => {
                        let (d, r) = gltf_summary(&v);
                        details = d;
                        refs = r;
                    }
                    Err(e) => details = format!("JSON inválido: {}", e),
                }
                compat = "requer leitor glTF (dependência nova) ou conversão para OBJ".into();
            }
            "glb" => {
                kind = "modelo glTF binário (GLB)".into();
                if b.len() > 20 && &b[..4] == b"glTF" {
                    let jlen = le32(&b, 12) as usize;
                    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b[20..(20 + jlen).min(b.len())]) {
                        let (d, r) = gltf_summary(&v);
                        details = d;
                        refs = r;
                    }
                } else {
                    details = "cabeçalho GLB inválido".into();
                }
                compat = "requer leitor glTF (dependência nova) ou conversão para OBJ".into();
            }
            "fbx" => {
                kind = "modelo FBX".into();
                details = if b.starts_with(b"Kaydara FBX Binary") { format!("binário, versão {}", le32(&b, 23)) } else { "ASCII".into() };
                compat = "incompatível sem conversão (exportar para OBJ ou glTF)".into();
            }
            "blend" => {
                kind = "arquivo do Blender".into();
                details = String::from_utf8_lossy(&b[..12.min(b.len())]).to_string();
                compat = "incompatível sem conversão (exportar no Blender para OBJ/glTF)".into();
            }
            "dae" => {
                kind = "modelo COLLADA".into();
                compat = "requer conversão para OBJ/glTF".into();
            }
            "stl" | "ply" => {
                kind = format!("modelo {}", ext.to_uppercase());
                compat = "geometria sem UV/material; conversão para OBJ recomendada".into();
            }
            "hdr" | "exr" | "dds" | "ktx" | "ktx2" | "tif" | "tiff" | "psd" | "webp" => {
                kind = format!("imagem {}", ext.to_uppercase());
                compat = "requer conversão para PNG".into();
            }
            _ => {
                compat = "não é recurso visual (ou formato não reconhecido)".into();
            }
        }
        // referências ausentes
        let base = p.parent().unwrap_or(&dir);
        let missing: Vec<String> = refs.iter().filter(|r| !base.join(r).exists()).cloned().collect();
        if !missing.is_empty() {
            details.push_str(&format!(" · ARQUIVOS REFERENCIADOS AUSENTES: {}", missing.join(", ")));
        }
        rows.push(Row { rel, kind, size: b.len() as u64, hash: sha256_bytes(&b), details, compat, refs });
    }
    let mut md = String::new();
    md.push_str(&format!("# Inventário de recursos visuais\n\nPasta: `{}` · {} arquivo(s) · {:.1} MB no total\n\n", dir.display(), rows.len(), total as f64 / 1048576.0));
    md.push_str("Os arquivos foram apenas lidos. Os hashes SHA-256 abaixo permitem conferir, a qualquer momento, que os originais não foram alterados.\n\n");
    md.push_str("| Arquivo | Tipo | Tamanho | Detalhes | Compatibilidade | SHA-256 |\n|---|---|---|---|---|---|\n");
    for r in &rows {
        md.push_str(&format!("| {} | {} | {:.1} KB | {} | {} | `{}` |\n", r.rel, r.kind, r.size as f64 / 1024.0, r.details.replace('|', "/"), r.compat, &r.hash.get(..16).unwrap_or("")));
    }
    md.push_str("\n## Licenças e restrições de uso\n\n");
    if licenses.is_empty() {
        md.push_str("Nenhum arquivo de licença/créditos encontrado. **Não presumir licença livre**: confirmar com quem forneceu os arquivos antes de distribuir.\n");
    } else {
        for l in &licenses {
            md.push_str(&format!("- `{}` (ler e registrar as condições)\n", l));
        }
    }
    let tex_bytes: u64 = rows.iter().filter(|r| r.kind.starts_with("textura")).map(|r| r.size).sum();
    md.push_str(&format!(
        "\n## Impacto estimado\n\n- Texturas: {:.1} MB em disco; em memória de vídeo, ~4 bytes/pixel + 33% de mipmaps por textura.\n- Referência atual: 21 texturas procedurais de 256×256 (~7 MB de VRAM no total).\n- Integração gradual: um elemento representativo primeiro, medindo quadro e memória antes de ampliar.\n",
        tex_bytes as f64 / 1048576.0
    ));
    let _ = rows.iter().map(|r| r.refs.len()).sum::<usize>();
    match args.get(2) {
        Some(out) => {
            let outp = PathBuf::from(out);
            if outp.starts_with(&dir) {
                eprintln!("recusado: o inventário não pode ser gravado dentro da pasta dos originais");
                std::process::exit(1);
            }
            std::fs::write(&outp, md).expect("gravar inventário");
            println!("inventário gravado em {}", outp.display());
        }
        None => print!("{}", md),
    }
}
