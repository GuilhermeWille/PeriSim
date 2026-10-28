//! "Perícia Linux": sistema operacional simulado da estação pericial.
//!
//! Não é uma VM: é um Linux recriado dentro do motor (determinístico e isolado). Tem
//! sistema de arquivos estilo Unix (/, /dev, /proc, /mnt, /home…), um shell compatível com
//! o básico do bash e programas que operam sobre as evidências do caso pelas regras do POP.
//! Nenhuma operação acessa o sistema real do usuário nem a internet.

pub mod cmds;
pub mod man;
pub mod shell;

use crate::engine::Sim;
use crate::model::*;
use crate::osapi::{OsEntry, OsPart};
use std::collections::BTreeMap;

pub const DISTRO: &str = "Perícia Linux";
pub const DISTRO_VERSION: &str = "12.4 \"Lacre\"";
pub const KERNEL: &str = "6.1.0-28-amd64";

#[derive(Clone, Debug)]
pub struct FileNode {
    pub data: String,
    pub mtime: i64,
    pub mode: u32,
    /// Origem do arquivo (produto derivado: de onde veio). Vazio = criado na estação.
    pub origin: String,
}

#[derive(Clone, Debug)]
pub enum Node {
    Dir(BTreeMap<String, Node>, i64),
    File(FileNode),
    /// Imagem forense (arquivo .dd/.E01) de uma mídia do caso.
    Image(String, i64),
}

#[derive(Clone, Debug)]
pub struct Mount {
    pub point: String,
    pub source: String,
    pub media: String,
    pub part: String,
    pub fstype: String,
    pub original: bool,
}

/// Sessão de shell (cada janela de terminal tem a sua).
#[derive(Clone, Debug)]
pub struct Shell {
    pub cwd: String,
    pub env: BTreeMap<String, String>,
    pub history: Vec<String>,
    pub last_code: i32,
    pub clear_requested: bool,
    pub exit_requested: bool,
}

impl Shell {
    pub fn new(home: &str) -> Shell {
        let mut env = BTreeMap::new();
        env.insert("HOME".into(), home.into());
        env.insert("USER".into(), "perito".into());
        env.insert("SHELL".into(), "/bin/bash".into());
        env.insert("PATH".into(), "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".into());
        env.insert("LANG".into(), "pt_BR.UTF-8".into());
        env.insert("TERM".into(), "xterm-256color".into());
        Shell { cwd: home.into(), env, history: vec![], last_code: 0, clear_requested: false, exit_requested: false }
    }
    pub fn prompt(&self, os: &Os) -> String {
        let home = &os.home;
        let cwd = if self.cwd == *home {
            "~".to_string()
        } else if let Some(r) = self.cwd.strip_prefix(&format!("{}/", home)) {
            format!("~/{}", r)
        } else {
            self.cwd.clone()
        };
        format!("perito@{}:{}$ ", os.hostname, cwd)
    }
}

/// O que existe num caminho.
#[derive(Clone, Debug)]
pub enum Kind {
    Dir,
    File { size: u64, mtime: i64, mode: u32 },
    Image { media: String, size: u64, mtime: i64 },
    Dev { media: Option<String>, part: Option<String>, size: u64 },
    Proc,
    MDir { mount: usize },
    MFile { mount: usize, entry: Option<OsEntry>, size: u64, mtime: i64 },
}

impl Kind {
    pub fn is_dir(&self) -> bool {
        matches!(self, Kind::Dir | Kind::MDir { .. })
    }
}

pub struct Os {
    pub root: Node,
    pub mounts: Vec<Mount>,
    pub hostname: String,
    pub home: String,
    pub case_dir: String,
    pub boot: i64,
    pub logged_in: bool,
}

pub fn norm(path: &str) -> String {
    let mut parts: Vec<&str> = vec![];
    for c in path.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            x => parts.push(x),
        }
    }
    format!("/{}", parts.join("/"))
}

pub fn basename(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

pub fn dirname(p: &str) -> String {
    match p.rfind('/') {
        Some(0) | None => "/".into(),
        Some(i) => p[..i].to_string(),
    }
}

impl Os {
    pub fn new(sim: &Sim) -> Os {
        let t = sim.case.start - 3 * 3600;
        let home = "/home/perito".to_string();
        let case_dir = format!("{}/casos/{}", home, sim.case.id);
        let mut os = Os { root: Node::Dir(BTreeMap::new(), t), mounts: vec![], hostname: "estacao-pericial".into(), home: home.clone(), case_dir: case_dir.clone(), boot: t, logged_in: false };
        for d in [
            "/bin", "/boot", "/etc", "/home", "/media", "/media/perito", "/mnt", "/opt", "/root", "/sbin", "/srv", "/tmp", "/usr", "/usr/bin", "/usr/share", "/usr/share/doc", "/var",
            "/var/log", "/home/perito", "/home/perito/Área de Trabalho", "/home/perito/Documentos", "/home/perito/Downloads", "/home/perito/Encaminhados", "/home/perito/casos",
        ] {
            os.mkdir_p(d, t);
        }
        os.mkdir_p(&case_dir, t);
        let f = |s: &str| s.to_string();
        os.put("/etc/hostname", f("estacao-pericial\n"), t, "");
        os.put(
            "/etc/os-release",
            format!("PRETTY_NAME=\"{d} {v}\"\nNAME=\"{d}\"\nVERSION_ID=\"12.4\"\nVERSION=\"{v}\"\nID=pericialinux\nID_LIKE=debian\nHOME_URL=\"(sem acesso à internet)\"\n", d = DISTRO, v = DISTRO_VERSION),
            t,
            "",
        );
        os.put("/etc/issue", format!("{} {} \\n \\l\n", DISTRO, DISTRO_VERSION), t, "");
        os.put("/etc/passwd", f("root:x:0:0:root:/root:/bin/bash\nperito:x:1000:1000:Perito,,,:/home/perito:/bin/bash\n"), t, "");
        os.put("/etc/group", f("root:x:0:\nperito:x:1000:\ndisk:x:6:perito\n"), t, "");
        os.put(
            "/etc/fstab",
            f("# /etc/fstab — estação pericial\n# Mídias de evidência NUNCA são montadas automaticamente.\nUUID=3f1c-estacao  /      ext4  errors=remount-ro  0 1\nUUID=8a2d-home     /home  ext4  defaults           0 2\n"),
            t,
            "",
        );
        os.put(
            "/etc/motd",
            format!(
                "\n{} {} — estação pericial isolada\n\n * Mídias de evidência não são montadas automaticamente.\n * Conecte-as somente via bloqueador de escrita.\n * Trabalhe sobre a imagem forense, nunca sobre o original.\n * Rede: apenas a rede emulada do caso (sem internet real).\n\nDigite 'man intro' para começar ou 'pericia ajuda' para os comandos do simulador.\n",
                DISTRO, DISTRO_VERSION
            ),
            t,
            "",
        );
        os.put("/var/log/syslog", f("Boot da estação pericial.\nautomount de mídias removíveis: DESATIVADO (política do laboratório)\n"), t, "");
        os.put(
            "/home/perito/Área de Trabalho/LEIA-ME.txt",
            format!(
                "Estação Pericial — {}\n\nCaso: {}\nPasta de trabalho: {}\n\nComo trabalhar nesta estação:\n  pericia caso                 # histórico, objetivo e quesitos\n  lsblk                        # mídias conectadas (via bloqueador)\n  pericia imagem m-01          # aquisição: gera ~/casos/<ID>/M-01.dd com hash\n  sudo mount -o ro,loop ~/casos/{}/M-01.dd /mnt/m01\n  ls -l /mnt/m01 · grep -ri termo /mnt/m01 · stat <arq> · sha256sum <arq>\n  pericia ajuda                # todos os comandos do simulador (recuperar, decifrar, rede...)\n\nDocumento: pericia achei <expressão> · pericia anotar <texto> · pericia laudo · pericia finalizar\nManual: man <comando>\n",
                DISTRO, sim.case.id, case_dir, sim.case.id
            ),
            t,
            "",
        );
        os.put(&format!("{}/.bash_history", home), String::new(), t, "");
        os.put(&format!("{}/.bashrc", home), f("# ~/.bashrc\nalias ll='ls -l'\nalias la='ls -la'\n"), t, "");
        os.put(
            &format!("{}/CASO.txt", case_dir),
            format!("{}\n", crate::tools::describe_case(sim)),
            t,
            "",
        );
        for p in &sim.case.provided {
            os.put(&format!("/home/perito/Encaminhados/{}", p.name()), p.content.clone(), p.mtime, &format!("provided:{}", p.name()));
        }
        os
    }

    // ------------------------------------------------------------------ árvore local

    fn node(&self, abs: &str) -> Option<&Node> {
        let mut cur = &self.root;
        for c in abs.split('/').filter(|c| !c.is_empty()) {
            match cur {
                Node::Dir(m, _) => cur = m.get(c)?,
                _ => return None,
            }
        }
        Some(cur)
    }

    fn node_mut(&mut self, abs: &str) -> Option<&mut Node> {
        let mut cur = &mut self.root;
        for c in abs.split('/').filter(|c| !c.is_empty()) {
            match cur {
                Node::Dir(m, _) => cur = m.get_mut(c)?,
                _ => return None,
            }
        }
        Some(cur)
    }

    pub fn mkdir_p(&mut self, abs: &str, t: i64) {
        let mut cur = &mut self.root;
        for c in abs.split('/').filter(|c| !c.is_empty()) {
            if let Node::Dir(m, _) = cur {
                cur = m.entry(c.to_string()).or_insert_with(|| Node::Dir(BTreeMap::new(), t));
            }
        }
    }

    pub fn put(&mut self, abs: &str, data: String, t: i64, origin: &str) {
        let parent = dirname(abs);
        self.mkdir_p(&parent, t);
        if let Some(Node::Dir(m, _)) = self.node_mut(&parent) {
            m.insert(basename(abs).to_string(), Node::File(FileNode { data, mtime: t, mode: 0o644, origin: origin.to_string() }));
        }
    }

    pub fn put_image(&mut self, abs: &str, media: &str, t: i64) {
        let parent = dirname(abs);
        if let Some(Node::Dir(m, _)) = self.node_mut(&parent) {
            m.insert(basename(abs).to_string(), Node::Image(media.to_string(), t));
        }
    }

    pub fn remove(&mut self, abs: &str) -> bool {
        let parent = dirname(abs);
        if let Some(Node::Dir(m, _)) = self.node_mut(&parent) {
            return m.remove(basename(abs)).is_some();
        }
        false
    }

    pub fn local_node(&self, abs: &str) -> Option<&Node> {
        self.node(abs)
    }

    /// Garante que toda imagem/extração já produzida pelo motor exista como arquivo.
    pub fn sync(&mut self, sim: &Sim) {
        for mid in sim.imaged.iter() {
            let exists = self.find_image(mid).is_some();
            if !exists {
                let m = sim.case.media_by_id(mid);
                let name = match m {
                    Some(m) if m.is_work_media => format!("{}_copia-volume.img", mid),
                    Some(m) if m.kind == MediaKind::FlashMovel => format!("{}_extracao.img", mid),
                    _ => format!("{}.dd", mid),
                };
                let path = format!("{}/{}", self.case_dir, name);
                self.put_image(&path, mid, sim.clock);
            }
        }
    }

    pub fn find_image(&self, media: &str) -> Option<String> {
        fn walk(n: &Node, path: String, media: &str) -> Option<String> {
            match n {
                Node::Image(m, _) if m == media => Some(path),
                Node::Dir(c, _) => c.iter().find_map(|(k, v)| walk(v, format!("{}/{}", if path == "/" { "" } else { &path }, k), media)),
                _ => None,
            }
        }
        walk(&self.root, "/".into(), media)
    }

    // ------------------------------------------------------------------ dispositivos

    /// Dispositivos de bloco: (nome, mídia, partição, tamanho, via bloqueador).
    pub fn block_devices(&self, sim: &Sim) -> Vec<(String, Option<String>, Option<String>, u64, bool)> {
        let mut v = vec![
            ("sda".to_string(), None, None, 512_000_000_000u64, false),
            ("sda1".to_string(), None, None, 100_000_000_000, false),
            ("sda2".to_string(), None, None, 412_000_000_000, false),
        ];
        let letters = "bcdefghij";
        for (k, (mid, wb)) in sim.attached_media().iter().enumerate() {
            let l = letters.chars().nth(k).unwrap_or('z');
            let m = sim.case.media_by_id(mid).unwrap();
            let dev = if m.kind == MediaKind::Nvme { format!("nvme{}n1", k) } else { format!("sd{}", l) };
            v.push((dev.clone(), Some(mid.clone()), None, m.capacity, *wb));
            for p in &m.partitions {
                let pn = if m.kind == MediaKind::Nvme { format!("{}p{}", dev, p.index) } else { format!("{}{}", dev, p.index) };
                v.push((pn, Some(mid.clone()), Some(p.id.clone()), p.size, *wb));
            }
        }
        v
    }

    pub fn dev_lookup(&self, sim: &Sim, path: &str) -> Option<(String, Option<String>, Option<String>, u64, bool)> {
        let name = path.strip_prefix("/dev/")?;
        self.block_devices(sim).into_iter().find(|d| d.0 == name)
    }

    // ------------------------------------------------------------------ montagem

    pub fn mount_for(&self, abs: &str) -> Option<(usize, String)> {
        let mut best: Option<(usize, usize)> = None;
        for (i, m) in self.mounts.iter().enumerate() {
            if abs == m.point || abs.starts_with(&format!("{}/", m.point)) {
                if best.map(|(_, l)| m.point.len() > l).unwrap_or(true) {
                    best = Some((i, m.point.len()));
                }
            }
        }
        best.map(|(i, l)| {
            let rel = &abs[l..];
            (i, if rel.is_empty() { "/".to_string() } else { rel.to_string() })
        })
    }

    pub fn mount_view(&self, sim: &Sim, idx: usize) -> Result<OsPart, String> {
        let m = &self.mounts[idx];
        sim.os_part(&m.media, &m.part)
    }

    // ------------------------------------------------------------------ consulta genérica

    pub fn kind(&self, sim: &Sim, abs: &str) -> Option<Kind> {
        let abs = norm(abs);
        if let Some((mi, rel)) = self.mount_for(&abs) {
            let view = self.mount_view(sim, mi).ok()?;
            if rel == "/" {
                return Some(Kind::MDir { mount: mi });
            }
            if let Some(e) = view.entries.iter().find(|e| !e.deleted && e.path == rel) {
                return Some(Kind::MFile { mount: mi, entry: Some(e.clone()), size: e.size, mtime: e.mtime });
            }
            if let Some((_, sz, mt)) = view.noise_paths.iter().find(|(p, _, _)| *p == rel) {
                return Some(Kind::MFile { mount: mi, entry: None, size: *sz, mtime: *mt });
            }
            let pre = format!("{}/", rel);
            if view.entries.iter().any(|e| !e.deleted && e.path.starts_with(&pre)) || view.noise_paths.iter().any(|(p, _, _)| p.starts_with(&pre)) {
                return Some(Kind::MDir { mount: mi });
            }
            return None;
        }
        if abs == "/dev" || abs == "/proc" {
            return Some(Kind::Dir);
        }
        if let Some(rest) = abs.strip_prefix("/dev/") {
            if ["null", "zero", "urandom", "random", "sr0", "tty"].contains(&rest) {
                return Some(Kind::Dev { media: None, part: None, size: 0 });
            }
            return self.dev_lookup(sim, &abs).map(|d| Kind::Dev { media: d.1, part: d.2, size: d.3 });
        }
        if let Some(rest) = abs.strip_prefix("/proc/") {
            return if ["version", "cpuinfo", "meminfo", "mounts", "uptime", "partitions"].contains(&rest) { Some(Kind::Proc) } else { None };
        }
        if abs == "/usr/bin" || abs == "/bin" {
            return Some(Kind::Dir);
        }
        if let Some(c) = abs.strip_prefix("/usr/bin/").or_else(|| abs.strip_prefix("/bin/")) {
            return if cmds::exists(c) { Some(Kind::File { size: 141_312, mtime: self.boot - 90 * 86400, mode: 0o755 }) } else { None };
        }
        match self.node(&abs)? {
            Node::Dir(_, _) => Some(Kind::Dir),
            Node::File(f) => Some(Kind::File { size: f.data.len() as u64, mtime: f.mtime, mode: f.mode }),
            Node::Image(m, t) => Some(Kind::Image { media: m.clone(), size: sim.case.media_by_id(m).map(|x| x.capacity).unwrap_or(0), mtime: *t }),
        }
    }

    pub fn list(&self, sim: &Sim, abs: &str) -> Result<Vec<(String, Kind)>, String> {
        let abs = norm(abs);
        if let Some((mi, rel)) = self.mount_for(&abs) {
            let view = self.mount_view(sim, mi)?;
            let pre = if rel == "/" { "/".to_string() } else { format!("{}/", rel) };
            let mut out: BTreeMap<String, Kind> = BTreeMap::new();
            let mut add = |path: &str, k: Kind| {
                if let Some(r) = path.strip_prefix(&pre) {
                    let (first, more) = match r.find('/') {
                        Some(i) => (&r[..i], true),
                        None => (r, false),
                    };
                    if first.is_empty() {
                        return;
                    }
                    if more {
                        out.entry(first.to_string()).or_insert(Kind::MDir { mount: mi });
                    } else {
                        out.insert(first.to_string(), k);
                    }
                }
            };
            for e in view.entries.iter().filter(|e| !e.deleted) {
                add(&e.path, Kind::MFile { mount: mi, entry: Some(e.clone()), size: e.size, mtime: e.mtime });
            }
            for (p, sz, mt) in &view.noise_paths {
                add(p, Kind::MFile { mount: mi, entry: None, size: *sz, mtime: *mt });
            }
            return Ok(out.into_iter().collect());
        }
        if abs == "/dev" {
            let mut v: Vec<(String, Kind)> = ["null", "zero", "urandom", "random", "sr0", "tty"].iter().map(|n| (n.to_string(), Kind::Dev { media: None, part: None, size: 0 })).collect();
            for d in self.block_devices(sim) {
                v.push((d.0, Kind::Dev { media: d.1, part: d.2, size: d.3 }));
            }
            v.sort_by(|a, b| a.0.cmp(&b.0));
            return Ok(v);
        }
        if abs == "/proc" {
            return Ok(["cpuinfo", "meminfo", "mounts", "partitions", "uptime", "version"].iter().map(|n| (n.to_string(), Kind::Proc)).collect());
        }
        if abs == "/usr/bin" || abs == "/bin" {
            return Ok(cmds::names().into_iter().map(|n| (n.to_string(), Kind::File { size: 141_312, mtime: self.boot - 90 * 86400, mode: 0o755 })).collect());
        }
        match self.node(&abs) {
            Some(Node::Dir(m, _)) => {
                let mut v = vec![];
                for (k, n) in m {
                    let kp = format!("{}/{}", if abs == "/" { "" } else { &abs }, k);
                    v.push((k.clone(), self.kind(sim, &kp).unwrap_or(match n {
                        Node::Dir(_, _) => Kind::Dir,
                        _ => Kind::Proc,
                    })));
                }
                Ok(v)
            }
            Some(_) => Err("Não é um diretório".into()),
            None => Err("Arquivo ou diretório inexistente".into()),
        }
    }

    /// Lê um arquivo (texto). Arquivos de evidência passam pelo motor (auditoria/regras).
    pub fn read(&mut self, sim: &mut Sim, abs: &str) -> Result<String, String> {
        let abs = norm(abs);
        if let Some((mi, rel)) = self.mount_for(&abs) {
            let (media, part) = (self.mounts[mi].media.clone(), self.mounts[mi].part.clone());
            let view = sim.os_part(&media, &part)?;
            if view.entries.iter().any(|e| !e.deleted && e.path == rel) {
                return sim.os_read(&media, &part, &rel);
            }
            if view.noise_paths.iter().any(|(p, _, _)| *p == rel) {
                return Ok(crate::osapi::binary_noise(&rel, 40));
            }
            return Err("Arquivo ou diretório inexistente".into());
        }
        if let Some(rest) = abs.strip_prefix("/proc/") {
            return Ok(self.proc(sim, rest));
        }
        if abs.starts_with("/dev/") {
            return match self.kind(sim, &abs) {
                Some(Kind::Dev { media: Some(m), .. }) => {
                    sim.os_mount_original(&m, sim.hashes.contains_key(&format!("wb:{}", m)));
                    Ok(crate::osapi::binary_noise(&m, 64))
                }
                Some(_) => Ok(String::new()),
                None => Err("Arquivo ou diretório inexistente".into()),
            };
        }
        match self.node(&abs) {
            Some(Node::File(f)) => {
                let data = f.data.clone();
                if let Some(name) = f.origin.strip_prefix("provided:") {
                    let name = name.to_string();
                    sim.os_read_provided(&name);
                }
                Ok(data)
            }
            Some(Node::Image(m, _)) => Ok(crate::osapi::binary_noise(m, 64)),
            Some(Node::Dir(_, _)) => Err("É um diretório".into()),
            None => {
                if abs.starts_with("/usr/bin/") || abs.starts_with("/bin/") {
                    Ok("\u{7f}ELF\u{2}\u{1}\u{1} (executável)".into())
                } else {
                    Err("Arquivo ou diretório inexistente".into())
                }
            }
        }
    }

    /// Escreve na árvore local. Mídias montadas são somente leitura; dispositivos de
    /// evidência passam pela checagem do bloqueador (pode contaminar se conectado direto).
    pub fn write(&mut self, sim: &mut Sim, abs: &str, data: &str, append: bool, origin: &str) -> Result<(), String> {
        let abs = norm(abs);
        if self.mount_for(&abs).is_some() {
            return Err("Sistema de arquivos somente para leitura".into());
        }
        if abs.starts_with("/proc") {
            return Err("Permissão negada".into());
        }
        if abs.starts_with("/dev/") {
            return match self.kind(sim, &abs) {
                Some(Kind::Dev { media: Some(m), .. }) => sim.os_write_attempt(&m, "dados gravados no dispositivo de evidência"),
                Some(_) => Ok(()),
                None => Err("Permissão negada".into()),
            };
        }
        if !abs.starts_with("/home/perito") && !abs.starts_with("/tmp") && !abs.starts_with("/mnt") && !abs.starts_with("/media") {
            return Err("Permissão negada".into());
        }
        let parent = dirname(&abs);
        match self.node(&parent) {
            Some(Node::Dir(_, _)) => {}
            _ => return Err("Arquivo ou diretório inexistente".into()),
        }
        let t = sim.clock;
        match self.node_mut(&abs) {
            Some(Node::File(f)) => {
                if append {
                    f.data.push_str(data);
                } else {
                    f.data = data.to_string();
                }
                f.mtime = t;
                if !origin.is_empty() {
                    f.origin = origin.to_string();
                }
                Ok(())
            }
            Some(Node::Dir(_, _)) => Err("É um diretório".into()),
            Some(Node::Image(_, _)) => Err("Permissão negada (imagem forense protegida: chattr +i)".into()),
            None => {
                self.put(&abs, data.to_string(), t, origin);
                Ok(())
            }
        }
    }

    pub fn proc(&self, sim: &Sim, name: &str) -> String {
        match name {
            "version" => format!("Linux version {} (debian-kernel@lists.debian.org) (gcc-12) #1 SMP PREEMPT_DYNAMIC Debian 6.1.119-1\n", KERNEL),
            "cpuinfo" => "processor\t: 0\nmodel name\t: Estação Pericial x86_64 @ 3.20GHz\ncpu cores\t: 8\nflags\t\t: fpu sse sse2 avx2 aes\n".into(),
            "meminfo" => "MemTotal:       32768000 kB\nMemFree:        29012000 kB\nMemAvailable:   30100000 kB\n".into(),
            "uptime" => format!("{} 0.00\n", sim.clock - self.boot),
            "partitions" => {
                let mut s = String::from("major minor  #blocks  name\n\n");
                for (k, d) in self.block_devices(sim).iter().enumerate() {
                    s.push_str(&format!("   8 {:>6} {:>12} {}\n", k * 16, d.3 / 1024, d.0));
                }
                s
            }
            "mounts" => {
                let mut s = String::from("/dev/sda1 / ext4 rw,relatime 0 0\n/dev/sda2 /home ext4 rw,relatime 0 0\nproc /proc proc rw 0 0\n");
                for m in &self.mounts {
                    s.push_str(&format!("{} {} {} ro,noexec,nodev 0 0\n", m.source, m.point, m.fstype));
                }
                s
            }
            _ => String::new(),
        }
    }
}

impl Sim {
    /// Leitura de material encaminhado (ex.: .eml) pela estação.
    pub fn os_read_provided(&mut self, name: &str) {
        self.rec_pub("fs-cat", &format!("Encaminhados/{}", name), "material encaminhado lido");
        let src = SourceRef::Provided { file: name.to_string() };
        if self.case.keyword_sources.contains(&src) {
            let d = format!("mensagem eletrônica encaminhada {}", name);
            self.mark_keyword_found(&d);
            self.notes.push(crate::engine::Note { at: self.clock, text: format!("Expressão de interesse localizada em: {}", d), source: Some(src) });
        }
    }
}
