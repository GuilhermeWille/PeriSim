//! Sistema de arquivos virtual das evidências. Os arquivos "de sistema" sem interesse
//! (ruído) NÃO são materializados: são sintetizados sob demanda a partir do índice da
//! partição, para controlar memória. Cálculo de hash, recuperação e decifragem vivem aqui.

use crate::model::*;
use crate::rng::Rng;
use crate::sha256::Sha256;
use crate::util::stable_media_repr;

/// Entrada de listagem apresentada ao usuário.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub mtime: i64,
    pub ftype: FileType,
    pub deleted: bool,
    pub real: bool,
}

/// Gera, de forma determinística, os caminhos de ruído de uma partição (sem guardá-los).
pub fn noise_paths(part: &Partition) -> Vec<(String, u64, i64)> {
    if part.noise == 0 || part.noise_roots.is_empty() {
        return vec![];
    }
    let seed = crate::rng::fnv1a(&part.id);
    let mut rng = Rng::new(seed);
    // limita a amostra visível a no máximo 400 nomes; o "total" é informado à parte
    let shown = part.noise.min(400);
    let names = [
        "config.dat", "cache.bin", "update.log", "driver.sys", "font.ttf", "icon.png", "locale.dat", "index.db",
        "thumbs.db", "temp.tmp", "manifest.xml", "readme.txt", "library.dll", "resource.pak", "settings.ini",
    ];
    let mut out = vec![];
    for _ in 0..shown {
        let root = rng.pick(&part.noise_roots).clone();
        let depth = rng.int(1, 3);
        let mut p = root;
        for _ in 0..depth {
            p.push('/');
            p.push_str(&format!("d{}", rng.int(0, 99)));
        }
        p.push('/');
        p.push_str(rng.pick(&names));
        out.push((p, rng.int(1024, 20_000_000) as u64, 1_700_000_000 + rng.int(0, 60_000_000)));
    }
    out
}

/// Número total de arquivos (reais + ruído) de uma partição.
pub fn total_files(part: &Partition) -> u64 {
    part.files.len() as u64 + part.noise as u64
}

/// Hash de uma mídia: combina serial, capacidade e o conteúdo de todas as partições
/// e arquivos (reais e ruído sintetizado), de forma estável e reprodutível.
/// Representa o hash da imagem forense da mídia.
pub fn media_hash(m: &Media) -> String {
    let mut h = Sha256::new();
    h.update(stable_media_repr(m).as_bytes());
    for p in &m.partitions {
        partition_into_hash(&mut h, p, true);
    }
    h.finish()
}

fn partition_into_hash(h: &mut Sha256, p: &Partition, include_bad: bool) {
    h.update(format!("P|{}|{}|{}|{}\n", p.id, p.fs, p.label, p.size).as_bytes());
    for f in &p.files {
        if f.bad_sector && !include_bad {
            continue;
        }
        file_into_hash(h, f);
    }
    for (path, size, mt) in noise_paths(p) {
        h.update(format!("N|{}|{}|{}\n", path, size, mt).as_bytes());
    }
    // nota: a parte "não amostrada" do ruído entra pelo total
    h.update(format!("NT|{}\n", p.noise).as_bytes());
    if let Some(e) = &p.encrypted {
        h.update(format!("E|{}\n", e.scheme).as_bytes());
    }
}

fn file_into_hash(h: &mut Sha256, f: &VFile) {
    h.update(format!("F|{}|{}|{}|{}|{}|{}\n", f.path, f.size, f.mtime, f.owner, f.deleted, f.ftype.label()).as_bytes());
    h.update(f.content.as_bytes());
    for c in &f.children {
        file_into_hash(h, c);
    }
    if let Some(c) = &f.container {
        partition_into_hash(h, c, true);
    }
}

/// Hash do conteúdo de um único arquivo (o que o comando `hash` do terminal calcula).
pub fn file_hash(f: &VFile) -> String {
    let mut h = Sha256::new();
    h.update(f.content.as_bytes());
    if let Some(ocr) = &f.ocr {
        let _ = ocr; // o hash é do dado bruto, não do OCR
    }
    h.finish()
}

/// Resultado de uma tentativa de recuperação de arquivo apagado.
pub enum RecoverResult {
    Ok(String),
    Partial(String),
    Failed(String),
}

pub fn recover(f: &VFile) -> RecoverResult {
    if !f.deleted {
        return RecoverResult::Ok(f.content.clone());
    }
    match f.recover {
        Some(Recover::Integral) => RecoverResult::Ok(f.content.clone()),
        Some(Recover::Parcial) => {
            let n = (f.content.len() as f64 * 0.55) as usize;
            let cut = f.content.char_indices().nth(n).map(|(i, _)| i).unwrap_or(f.content.len());
            RecoverResult::Partial(format!("{}\n[...restante sobrescrito; recuperação parcial...]", &f.content[..cut]))
        }
        _ => RecoverResult::Failed("Área do arquivo foi sobrescrita; recuperação inviável.".into()),
    }
}

/// Localiza um arquivo por caminho dentro de uma mídia (inclui filhos e contêineres já abertos).
pub fn find_file<'a>(m: &'a Media, path: &str) -> Option<(&'a Partition, &'a VFile)> {
    for p in &m.partitions {
        if let Some(f) = find_in_files(&p.files, path) {
            return Some((p, f));
        }
    }
    None
}

/// Busca respeitando acesso: só desce em contêineres abertos e compostos expandidos.
pub fn find_file_open<'a>(m: &'a Media, path: &str, open: &dyn Fn(&str) -> bool) -> Option<(&'a Partition, &'a VFile)> {
    for p in &m.partitions {
        if let Some(f) = find_open(&p.files, path, open) {
            return Some((p, f));
        }
    }
    None
}

fn find_open<'a>(files: &'a [VFile], path: &str, open: &dyn Fn(&str) -> bool) -> Option<&'a VFile> {
    for f in files {
        if f.path.eq_ignore_ascii_case(path) {
            return Some(f);
        }
        if !f.children.is_empty() && open(&f.id) {
            if let Some(x) = find_open(&f.children, path, open) {
                return Some(x);
            }
        }
        if let Some(c) = &f.container {
            if open(&c.id) {
                if let Some(x) = find_open(&c.files, path, open) {
                    return Some(x);
                }
            }
        }
    }
    None
}

fn find_in_files<'a>(files: &'a [VFile], path: &str) -> Option<&'a VFile> {
    for f in files {
        if f.path.eq_ignore_ascii_case(path) {
            return Some(f);
        }
        if let Some(found) = find_in_files(&f.children, path) {
            return Some(found);
        }
        if let Some(c) = &f.container {
            if let Some(found) = find_in_files(&c.files, path) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::casegen;
    use crate::model::ModuleId;

    #[test]
    fn hash_de_midia_e_estavel() {
        let case = casegen::generate(ModuleId::M501, 2, 123);
        let h1 = media_hash(&case.media[0]);
        let case2 = casegen::generate(ModuleId::M501, 2, 123);
        let h2 = media_hash(&case2.media[0]);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn hash_muda_com_conteudo() {
        let a = casegen::generate(ModuleId::M501, 2, 1);
        let b = casegen::generate(ModuleId::M501, 2, 2);
        assert_ne!(media_hash(&a.media[0]), media_hash(&b.media[0]));
    }
}
