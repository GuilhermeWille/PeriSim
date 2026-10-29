//! Sistema operacional do aparelho celular (representação para treinamento).
//!
//! `view` monta, a partir do estado do caso, o que a tela do aparelho mostra: tela
//! apagada, tela bloqueada (BFU/AFU, sem conteúdo), assistente de configuração (após
//! restauração) ou a tela inicial com os aplicativos e seus dados (mensagens, chamadas,
//! contatos, galeria, arquivos, configurações, relógio).
//!
//! A tela NÃO oferece meio de desbloqueio: o acesso ao conteúdo depende do estado do
//! aparelho e das regras do motor. Cada aplicativo aberto passa pelo comando
//! `tela <item> <app>`, que registra a navegação manual na auditoria e aplica as regras
//! dos POPs (apreensão não é exame; navegar antes da extração altera o estado).

use crate::engine::{Output, Sim};
use crate::model::*;
use crate::util::{fmt_date, fmt_hm, BRT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// Desligado ou sem bateria.
    Off,
    /// Restaurado de fábrica: assistente de configuração.
    Setup,
    /// Ligado e bloqueado. `afu`: após o primeiro desbloqueio (notificações contadas, conteúdo oculto).
    Locked { afu: bool },
    /// Desbloqueado (ou sem bloqueio): tela inicial.
    Home,
}

#[derive(Clone, Debug)]
pub struct Msg {
    pub time: String,
    pub from: String,
    pub text: String,
    pub mine: bool,
}

#[derive(Clone, Debug)]
pub struct Thread {
    pub name: String,
    pub app: String,
    pub msgs: Vec<Msg>,
    pub unread: usize,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct Call {
    pub incoming: bool,
    pub number: String,
    pub name: Option<String>,
    pub when: String,
    pub duration_s: u32,
}

#[derive(Clone, Debug)]
pub struct Photo {
    pub name: String,
    pub path: String,
    pub when: String,
    pub meta: Vec<(String, String)>,
    pub seed: u64,
}

#[derive(Clone, Debug)]
pub struct FileRow {
    pub path: String,
    pub size: u64,
    pub when: String,
}

#[derive(Clone, Debug)]
pub struct PhoneView {
    pub item: String,
    pub screen: Screen,
    pub model: String,
    pub os: String,
    pub os_version: String,
    pub time: String,
    pub date: String,
    pub battery: u32,
    pub charging: bool,
    pub airplane: bool,
    pub wifi: bool,
    pub bt: bool,
    pub cellular: bool,
    pub operator: String,
    pub screen_broken: bool,
    pub camera_covered: bool,
    pub notifications: usize,
    pub lock: String,
    pub smartlock: Option<String>,
    pub encryption: String,
    pub threads: Vec<Thread>,
    pub calls: Vec<Call>,
    pub contacts: Vec<(String, String)>,
    pub photos: Vec<Photo>,
    pub files: Vec<FileRow>,
    pub alarms: Vec<String>,
    pub alarms_disabled: bool,
    pub second_space: Option<String>,
    pub sims: Vec<(u32, String, bool)>,
    pub esim: Option<String>,
}

/// Aplicativos da tela inicial (id, rótulo).
pub const APPS: &[(&str, &str)] = &[
    ("telefone", "Telefone"),
    ("mensagens", "Mensagens"),
    ("contatos", "Contatos"),
    ("galeria", "Galeria"),
    ("arquivos", "Arquivos"),
    ("relogio", "Relógio"),
    ("config", "Configurações"),
    ("cofre", "Espaço seguro"),
];

/// Aplicativos que expõem conteúdo do usuário (navegar neles é exame, e altera estado).
pub fn is_content_app(app: &str) -> bool {
    matches!(app, "mensagens" | "telefone" | "contatos" | "galeria" | "arquivos" | "cofre")
}

fn parse_chat(content: &str, owner_first: &str) -> Vec<Msg> {
    content
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix('[')?;
            let (ts, rest) = rest.split_once("] ")?;
            let (who, text) = rest.split_once(": ")?;
            let time = ts.get(0..16).unwrap_or(ts).to_string();
            Some(Msg { time, from: who.to_string(), text: text.to_string(), mine: who.eq_ignore_ascii_case(owner_first) })
        })
        .collect()
}

fn hash64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Monta a visão da tela do aparelho `id`. `None` se o item não for um celular.
pub fn view(sim: &Sim, id: &str) -> Option<PhoneView> {
    let it = sim.case.items.iter().find(|i| i.id.eq_ignore_ascii_case(id))?;
    let ph = it.phone.as_ref()?;
    let screen = if ph.boot == Boot::Desligado || ph.battery <= 0.0 && !ph.charging && !ph.external_battery || ph.battery_removed {
        Screen::Off
    } else if ph.wiped {
        Screen::Setup
    } else if ph.boot == Boot::Desbloqueado || ph.lock == "nenhum" && ph.boot != Boot::Bfu {
        Screen::Home
    } else {
        Screen::Locked { afu: ph.boot == Boot::Afu }
    };
    let owner_first = it.owner.as_deref().unwrap_or("").split_whitespace().next().unwrap_or("").to_string();
    let read_all = it.flag("mensagens_lidas");
    let mut v = PhoneView {
        item: it.id.clone(),
        screen,
        model: ph.model.clone(),
        os: ph.os.clone(),
        os_version: ph.os_version.clone(),
        time: fmt_hm(sim.clock, BRT),
        date: fmt_date(sim.clock, BRT),
        battery: ph.battery.round().clamp(0.0, 100.0) as u32,
        charging: ph.charging || ph.external_battery,
        airplane: ph.airplane,
        wifi: ph.wifi && !ph.airplane,
        bt: ph.bt && !ph.airplane,
        cellular: ph.cellular && !ph.airplane && !ph.rf.isolates(),
        operator: String::new(),
        screen_broken: ph.screen_broken,
        camera_covered: ph.front_camera_covered,
        notifications: 0,
        lock: ph.lock.clone(),
        smartlock: ph.smartlock.clone(),
        encryption: ph.encryption.clone(),
        threads: vec![],
        calls: vec![],
        contacts: vec![],
        photos: vec![],
        files: vec![],
        alarms: ph.alarms.clone(),
        alarms_disabled: ph.alarms_disabled,
        second_space: ph.second_space.as_ref().map(|s| s.name.clone()),
        sims: ph.sims.iter().map(|s| (s.slot, s.operadora.clone(), s.place == SimWhere::Slot)).collect(),
        esim: ph.esim.as_ref().map(|e| e.1.clone()),
    };
    let in_slot: Vec<&SimCard> = ph.sims.iter().filter(|s| s.place == SimWhere::Slot).collect();
    v.operator = if ph.airplane {
        "Modo avião".into()
    } else if ph.rf.isolates() {
        "Sem serviço".into()
    } else if let Some(s) = in_slot.first() {
        s.operadora.clone()
    } else if let Some(e) = &ph.esim {
        e.1.clone()
    } else {
        "Sem SIM".into()
    };
    for s in &ph.sims {
        for c in &s.contacts {
            if !v.contacts.contains(c) {
                v.contacts.push(c.clone());
            }
        }
    }
    v.contacts.sort();
    // SMS do(s) SIM(s) em uso
    for s in &in_slot {
        if !s.sms.is_empty() {
            let msgs: Vec<Msg> = s
                .sms
                .iter()
                .map(|t| {
                    let (from, text) = t.split_once(": ").unwrap_or(("SMS", t));
                    Msg { time: String::new(), from: from.to_string(), text: text.to_string(), mine: false }
                })
                .collect();
            let unread = if read_all { 0 } else { 1 };
            v.threads.push(Thread { name: format!("SMS · {}", s.operadora), app: "SMS".into(), msgs, unread, path: format!("SIM slot {} (ICCID {})", s.slot, s.iccid) });
        }
    }
    // conteúdo da partição de dados do aparelho
    if let Some(m) = sim.case.media_by_id(&ph.data_media) {
        for p in &m.partitions {
            for f in &p.files {
                if f.deleted {
                    continue;
                }
                let when = format!("{} {}", fmt_date(f.mtime, BRT), fmt_hm(f.mtime, BRT));
                if f.ftype == FileType::Chat {
                    let msgs = parse_chat(&f.content, &owner_first);
                    let other = msgs.iter().find(|m| !m.mine).map(|m| m.from.clone()).unwrap_or_else(|| "Conversa".into());
                    let unread = if read_all { 0 } else { msgs.iter().rev().take_while(|m| !m.mine).count() };
                    v.threads.insert(0, Thread { name: other, app: "Mensageiro".into(), msgs, unread, path: f.path.clone() });
                } else if f.path.ends_with("call_log.csv") {
                    for l in f.content.lines().skip(1) {
                        let c: Vec<&str> = l.split(';').collect();
                        if c.len() < 4 {
                            continue;
                        }
                        let name = v.contacts.iter().find(|(_, n)| n == c[1]).map(|(nm, _)| nm.clone());
                        let when = c[2].replace('T', " ").trim_end_matches('Z').to_string() + " UTC";
                        v.calls.push(Call { incoming: c[0] == "entrada", number: c[1].to_string(), name, when, duration_s: c[3].parse().unwrap_or(0) });
                    }
                } else if f.ftype == FileType::Imagem || f.path.contains("/DCIM/") {
                    let name = f.path.rsplit('/').next().unwrap_or(&f.path).to_string();
                    let meta = f.meta.iter().map(|(k, x)| (k.clone(), x.clone())).collect();
                    v.photos.push(Photo { name, path: f.path.clone(), when: when.clone(), meta, seed: hash64(&f.path) });
                }
                if f.path.starts_with("/data/media/0/") {
                    v.files.push(FileRow { path: f.path.trim_start_matches("/data/media/0").to_string(), size: f.size, when });
                }
            }
        }
    }
    v.notifications = v.threads.iter().map(|t| t.unread).sum();
    Some(v)
}

impl Sim {
    /// `tela <item> <app>` — navegação manual na tela do aparelho (registrada).
    /// `tela <item> foto` — fotografia da tela atual (documentação da navegação).
    pub(crate) fn cmd_tela(&mut self, id: &str, app: &str) -> Output {
        let Some(i) = self.item_mut_ro(id) else { return Output::err("Item não encontrado.") };
        let Some(v) = view(self, id) else { return Output::err("Não é um dispositivo móvel.") };
        let iid = self.case.items[i].id.clone();
        let app = if app.is_empty() { "inicio" } else { app };
        if app == "foto" {
            self.tick(1);
            self.case.items[i].set("tela_fotografada", "sim");
            self.rec("tela-foto", &iid, "fotografia da tela do aparelho (documentação)");
            return Output::ok(format!("Tela do aparelho {} fotografada (registro da navegação).", iid));
        }
        if app == "imei" {
            return self.exec(&format!("imei verificar {}", iid.to_lowercase()));
        }
        if !APPS.iter().any(|(a, _)| *a == app) && app != "inicio" {
            return Output::err("Uso: tela <item> inicio|telefone|mensagens|contatos|galeria|arquivos|relogio|config|cofre|imei|foto");
        }
        match v.screen {
            Screen::Off => return Output::err("A tela está apagada: o aparelho está desligado ou sem carga."),
            Screen::Setup => return Output::err("O aparelho foi restaurado de fábrica: só exibe o assistente de configuração. Não há dados de usuário."),
            Screen::Locked { .. } => return Output::err("A tela está bloqueada: o conteúdo não é acessível pela tela."),
            Screen::Home => {}
        }
        self.tick(1);
        self.case.items[i].set("navegacao_manual", "sim");
        self.rec("tela", &format!("{} {}", iid, app), "navegação manual na tela do aparelho");
        let mut out = format!("Tela do aparelho {}: {}.", iid, APPS.iter().find(|(a, _)| *a == app).map(|x| x.1).unwrap_or("tela inicial"));
        if is_content_app(app) {
            if self.case.module == ModuleId::M505 {
                if !self.case.items[i].flag("falta_exame_na_apreensao") {
                    self.case.items[i].set("falta_exame_na_apreensao", "sim");
                    self.fault("Na apreensão, o perito navegou no conteúdo do aparelho (POP 5.05: apreender e documentar, não examinar). Limite-se a configurações de isolamento, alarmes e IMEI.");
                }
                out.push_str("\nATENÇÃO: isto é exame de conteúdo, não apreensão. Registrado.");
            } else {
                let extracted = self.audit.entries.iter().any(|e| e.action == "extracao" && e.detail.starts_with(&iid) && !e.outcome.contains("não realizável"));
                if !extracted && !self.case.items[i].flag("falta_navegacao_previa") {
                    self.case.items[i].set("falta_navegacao_previa", "sim");
                    self.fault("Navegação manual no conteúdo antes da extração: altera o estado do aparelho (mensagens marcadas como lidas, registros de acesso). A navegação manual é complemento documentado, depois da extração.");
                    out.push_str("\nATENÇÃO: navegação antes da extração — o estado do aparelho foi alterado.");
                }
            }
        }
        if app == "mensagens" {
            if v.notifications > 0 {
                self.case.items[i].set("mensagens_lidas", "sim");
                out.push_str("\nAs mensagens não lidas foram marcadas como lidas pelo aplicativo.");
            }
            // conteúdo visto na tela, com a origem
            let kw = self.case.keyword.clone();
            if !kw.is_empty() && self.case.module != ModuleId::M505 {
                let fold = |s: &str| s.to_lowercase();
                if let Some(t) = v.threads.iter().find(|t| t.msgs.iter().any(|m| fold(&m.text).contains(&fold(&kw)))) {
                    let w = format!("tela do aparelho {} — aplicativo de mensagens, conversa com {} ({})", iid, t.name, t.path);
                    let src = self.case.keyword_sources.iter().find(|s| matches!(s, SourceRef::File { path, .. } if *path == t.path)).cloned();
                    if src.is_some() {
                        self.mark_keyword_found(&w);
                    }
                    self.notes.push(crate::engine::Note { at: self.clock, text: format!("Observado na tela (navegação manual): {}", w), source: src });
                }
            }
        }
        if app == "cofre" && v.second_space.is_some() {
            out.push_str("\nO espaço seguro pede a senha própria: o conteúdo permanece protegido (examinar a partir da extração).");
        }
        if !self.case.items[i].flag("tela_fotografada") {
            out.push_str("\nDocumente a navegação: 'tela <item> foto'.");
        }
        Output::ok(out)
    }
}
