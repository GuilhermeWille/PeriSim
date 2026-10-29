//! Interface entre o Linux simulado da estação (módulo `os`) e o motor pericial.
//! Toda operação sobre evidência continua passando por aqui: regras do POP, auditoria
//! encadeada e registro da expressão de interesse.

use crate::engine::{Note, Sim};
use crate::model::*;
use crate::util::*;

/// Visão somente-leitura de um arquivo dentro de uma partição montada.
#[derive(Clone, Debug)]
pub struct OsEntry {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub mtime: i64,
    pub ctime: i64,
    pub atime: i64,
    pub owner: String,
    pub ftype: FileType,
    pub deleted: bool,
    pub container: bool,
    pub composite: bool,
    pub bad_sector: bool,
}

impl OsEntry {
    fn from(f: &VFile) -> OsEntry {
        OsEntry {
            id: f.id.clone(),
            path: f.path.clone(),
            size: f.size,
            mtime: f.mtime,
            ctime: f.ctime,
            atime: f.atime,
            owner: f.owner.clone(),
            ftype: f.ftype,
            deleted: f.deleted,
            container: f.container.is_some(),
            composite: !f.children.is_empty(),
            bad_sector: f.bad_sector,
        }
    }
}

/// Partição (ou volume interno de contêiner) vista pelo SO.
#[derive(Clone, Debug)]
pub struct OsPart {
    pub id: String,
    pub index: u32,
    pub fs: String,
    pub label: String,
    pub size: u64,
    pub encrypted: Option<String>,
    pub tz_offset: i32,
    pub noise: u32,
    pub entries: Vec<OsEntry>,
    pub noise_paths: Vec<(String, u64, i64)>,
}

fn find_part<'a>(m: &'a Media, part_id: &str) -> Option<(&'a Partition, bool)> {
    for p in &m.partitions {
        if p.id == part_id {
            return Some((p, false));
        }
        for f in &p.files {
            if let Some(c) = &f.container {
                if c.id == part_id {
                    return Some((c, true));
                }
            }
        }
    }
    None
}

impl Sim {
    /// Mídias conectadas fisicamente à estação: (id, via bloqueador de escrita).
    pub fn attached_media(&self) -> Vec<(String, bool)> {
        let mut v = vec![];
        for m in &self.case.media {
            if m.is_work_media || m.kind == MediaKind::FlashMovel {
                continue;
            }
            if self.hashes.contains_key(&format!("wb:{}", m.id)) {
                v.push((m.id.clone(), true));
            } else if self.hashes.contains_key(&format!("att:{}", m.id)) {
                v.push((m.id.clone(), false));
            }
        }
        v
    }

    pub fn is_examinable(&self, media: &str) -> bool {
        self.imaged.contains(media) || self.hashes.contains_key(&format!("origmnt:{}", media))
    }

    /// Descrição das partições de uma mídia (para mmls, fdisk, lsblk, montagem).
    pub fn os_partitions(&self, media: &str) -> Vec<OsPart> {
        let Some(m) = self.case.media_by_id(media) else { return vec![] };
        m.partitions.iter().map(|p| self.os_part_view(p)).collect()
    }

    fn os_part_view(&self, p: &Partition) -> OsPart {
        OsPart {
            id: p.id.clone(),
            index: p.index,
            fs: p.fs.clone(),
            label: p.label.clone(),
            size: p.size,
            encrypted: p.encrypted.as_ref().map(|e| e.scheme.clone()),
            tz_offset: p.tz_offset,
            noise: p.noise,
            entries: p.files.iter().map(OsEntry::from).collect(),
            noise_paths: crate::vfs::noise_paths(p),
        }
    }

    /// Visão de uma partição ou volume interno, respeitando cifragem.
    pub fn os_part(&self, media: &str, part_id: &str) -> Result<OsPart, String> {
        let m = self.case.media_by_id(media).ok_or("mídia inexistente")?;
        let (p, inner) = find_part(m, part_id).ok_or("partição inexistente")?;
        if (p.encrypted.is_some() || inner) && !self.decrypted.contains(&p.id) {
            return Err(format!("volume cifrado ({})", p.encrypted.as_ref().map(|e| e.scheme.as_str()).unwrap_or("contêiner")));
        }
        Ok(self.os_part_view(p))
    }

    /// Lê o conteúdo de um arquivo de uma partição montada (somente leitura).
    pub fn os_read(&mut self, media: &str, part_id: &str, path: &str) -> Result<String, String> {
        if !self.is_examinable(media) {
            return Err("mídia não adquirida nem montada".into());
        }
        let m = self.case.media_by_id(media).ok_or("mídia inexistente")?.clone();
        let (p, inner) = find_part(&m, part_id).ok_or("partição inexistente")?;
        if (p.encrypted.is_some() || inner) && !self.decrypted.contains(&p.id) {
            return Err("volume cifrado".into());
        }
        let f = p.files.iter().find(|f| f.path == path && !f.deleted).ok_or("Arquivo ou diretório inexistente")?;
        if f.bad_sector {
            self.rec_pub("fs-cat", path, "erro de E/S: setores ilegíveis");
            return Err("Erro de entrada/saída (setores ilegíveis)".into());
        }
        let kw = f.keyword;
        let fid = f.id.clone();
        let content = if f.container.is_some() {
            binary_noise(&f.id, 24)
        } else if !f.children.is_empty() {
            format!("PK\u{3}\u{4}{}", binary_noise(&f.id, 6))
        } else {
            f.content.clone()
        };
        self.rec_pub("fs-cat", &format!("{}:{}", media, path), "conteúdo lido");
        if kw && f.container.is_none() && f.children.is_empty() {
            self.register_keyword_file(media, &fid, &format!("arquivo {} (mídia {})", path, media));
        }
        Ok(content)
    }

    /// Arquivos apagados (listagem do Sleuth Kit — fls -d).
    pub fn os_deleted(&self, media: &str, part_id: &str) -> Vec<(u32, OsEntry)> {
        let Some(m) = self.case.media_by_id(media) else { return vec![] };
        let Some((p, _)) = find_part(m, part_id) else { return vec![] };
        p.files
            .iter()
            .enumerate()
            .map(|(i, f)| (inode(i), OsEntry::from(f)))
            .filter(|(_, e)| e.deleted)
            .collect()
    }

    /// Recupera um arquivo pelo inode (icat). Retorna (conteúdo, parcial).
    pub fn os_recover(&mut self, media: &str, part_id: &str, ino: u32) -> Result<(String, bool), String> {
        if !self.imaged.contains(media) {
            return Err("recupere a partir da imagem adquirida, não do original".into());
        }
        let m = self.case.media_by_id(media).ok_or("mídia inexistente")?.clone();
        let (p, _) = find_part(&m, part_id).ok_or("partição inexistente")?;
        if p.encrypted.is_some() && !self.decrypted.contains(&p.id) {
            return Err("volume cifrado".into());
        }
        let idx = p.files.iter().enumerate().position(|(i, _)| inode(i) == ino).ok_or("inode inválido")?;
        let f = &p.files[idx];
        if f.bad_sector {
            self.rec_pub("recuperar", &f.path, "inviável (setores ilegíveis)");
            return Err("erro de leitura: setores ilegíveis".into());
        }
        let r = crate::vfs::recover(f);
        let (path, fid, kw) = (f.path.clone(), f.id.clone(), f.keyword);
        match r {
            crate::vfs::RecoverResult::Ok(c) => {
                self.rec_pub("recuperar", &format!("{}:{} (inode {})", media, path, ino), "recuperação integral");
                if kw {
                    self.register_keyword_file(media, &fid, &format!("arquivo apagado recuperado {} (mídia {}, inode {})", path, media, ino));
                }
                Ok((c, false))
            }
            crate::vfs::RecoverResult::Partial(c) => {
                self.rec_pub("recuperar", &format!("{}:{} (inode {})", media, path, ino), "recuperação parcial");
                if kw {
                    self.register_keyword_file(media, &fid, &format!("arquivo apagado recuperado parcialmente {} (mídia {})", path, media));
                }
                Ok((c, true))
            }
            crate::vfs::RecoverResult::Failed(e) => {
                self.rec_pub("recuperar", &format!("{}:{}", media, path), "inviável");
                Err(e)
            }
        }
    }

    /// Exame montando diretamente a mídia original (não a cópia): permitido, mas registrado.
    pub fn os_mount_original(&mut self, media: &str, via_blocker: bool) {
        let key = format!("origmnt:{}", media);
        if !self.hashes.contains_key(&key) {
            self.hashes.insert(key, "1".into());
            self.add_fault_pub(&format!("Exame realizado diretamente sobre a mídia original {} (o POP 5.01 manda examinar a cópia).", media));
            self.rec_pub("montar-original", media, if via_blocker { "montada via bloqueador (somente leitura)" } else { "montada SEM bloqueador" });
            if !via_blocker {
                self.os_contaminate(media, "montagem direta do sistema de arquivos (atualização de metadados/journal)");
            }
        }
    }

    /// Tentativa de escrita na mídia original. Com bloqueador: negada. Sem: contamina.
    pub fn os_write_attempt(&mut self, media: &str, what: &str) -> Result<(), String> {
        if self.hashes.contains_key(&format!("wb:{}", media)) {
            self.rec_pub("escrita-bloqueada", media, &format!("bloqueador impediu: {}", what));
            return Err("Operação não permitida (dispositivo protegido contra escrita pelo bloqueador)".into());
        }
        self.os_contaminate(media, what);
        Ok(())
    }

    fn os_contaminate(&mut self, media: &str, what: &str) {
        if let Some(m) = self.case.media.iter_mut().find(|m| m.id == media) {
            m.writes.push(what.to_string());
        }
        self.add_fault_pub(&format!("Contaminação da mídia original {}: {}.", media, what));
        self.rec_pub("contaminacao", media, what);
    }

    /// Hash SHA-256 do conteúdo de um arquivo montado (não conta como leitura do conteúdo).
    pub fn os_file_hash(&mut self, media: &str, part_id: &str, path: &str) -> Result<String, String> {
        let m = self.case.media_by_id(media).ok_or("mídia inexistente")?;
        let (p, inner) = find_part(m, part_id).ok_or("partição inexistente")?;
        if (p.encrypted.is_some() || inner) && !self.decrypted.contains(&p.id) {
            return Err("volume cifrado".into());
        }
        let f = p.files.iter().find(|f| f.path == path && !f.deleted).ok_or("Arquivo ou diretório inexistente")?;
        if f.bad_sector {
            return Err("Erro de entrada/saída (setores ilegíveis)".into());
        }
        let h = crate::vfs::file_hash(f);
        self.rec_pub("hash", &format!("arquivo {}:{}", media, path), &h[..16]);
        Ok(h)
    }

    /// Identificador do volume interno de um contêiner cifrado (arquivo dentro de partição).
    pub fn os_container_part(&self, media: &str, part_id: &str, path: &str) -> Option<String> {
        let m = self.case.media_by_id(media)?;
        let (p, _) = find_part(m, part_id)?;
        p.files.iter().find(|f| f.path == path).and_then(|f| f.container.as_ref().map(|c| c.id.clone()))
    }

    /// Conteúdo dos itens de um arquivo composto já expandido pelo motor.
    pub fn os_children(&self, media: &str, part_id: &str, path: &str) -> Vec<(String, String, i64)> {
        let Some(m) = self.case.media_by_id(media) else { return vec![] };
        let Some((p, _)) = find_part(m, part_id) else { return vec![] };
        match p.files.iter().find(|f| f.path == path) {
            Some(f) if self.expanded.contains(&f.id) => f.children.iter().map(|c| (c.path.clone(), c.content.clone(), c.mtime)).collect(),
            _ => vec![],
        }
    }

    /// Registro literal do comando de shell (rastreabilidade das ações na estação).
    pub fn os_shell_audit(&mut self, line: &str, outcome: &str) {
        let l: String = line.chars().take(160).collect();
        self.rec_pub("shell", &l, outcome);
    }

    pub fn os_tick(&mut self, minutes: i64) {
        self.clock += minutes * 60;
    }

    pub(crate) fn register_keyword_file(&mut self, media: &str, file_id: &str, desc: &str) {
        let hit = self.case.keyword_sources.iter().find(|k| matches!(k, SourceRef::File { media: m, file, .. } if m == media && file == file_id)).cloned();
        if let Some(src) = hit {
            self.mark_keyword_found(desc);
            self.notes.push(Note { at: self.clock, text: format!("Expressão de interesse localizada em: {}", desc), source: Some(src) });
        }
    }
}

pub fn inode(index: usize) -> u32 {
    (index as u32) * 7 + 64
}

/// Bytes “binários” determinísticos (exibição de dados cifrados/compactados).
pub fn binary_noise(seed: &str, n: usize) -> String {
    let mut r = crate::rng::Rng::new(crate::rng::fnv1a(seed));
    (0..n).map(|_| char::from_u32(r.int(0xA1, 0x17E) as u32).unwrap_or('?')).collect()
}

pub fn fmt_ls_time(t: i64, off: i32) -> String {
    const M: [&str; 12] = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
    let p = parts(t, off);
    format!("{} {:>2} {:02}:{:02}", M[(p.mo - 1) as usize], p.d, p.h, p.mi)
}
