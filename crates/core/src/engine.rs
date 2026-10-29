//! Estado da simulação e despacho de ações. TODA ação do usuário (venha do terminal
//! ou de um botão da interface) passa por `exec`, que aplica as regras do POP, registra
//! a consequência na auditoria encadeada e avança o relógio simulado.

use crate::audit::Audit;
use crate::model::*;
use crate::util::*;
use crate::{tools, vfs};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Output {
    pub ok: bool,
    pub text: String,
}
impl Output {
    pub fn ok(t: impl Into<String>) -> Output {
        Output { ok: true, text: t.into() }
    }
    pub fn err(t: impl Into<String>) -> Output {
        Output { ok: false, text: t.into() }
    }
}

/// Produto derivado da análise (imagem forense, extração, página preservada, etc.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub kind: String,
    pub origin: String,
    pub hash: String,
    pub created: i64,
    pub note: String,
}

/// Uma anotação do perito (vira matéria-prima do laudo).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub at: i64,
    pub text: String,
    pub source: Option<SourceRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub preambulo: String,
    pub historico: String,
    pub material: String,
    pub objetivo: String,
    pub exame: String,
    pub consideracoes: String,
    pub conclusao: String,
    pub keyword_claim: String,
    pub keyword_source: String,
    pub submitted: bool,
}
impl Default for Report {
    fn default() -> Self {
        Report {
            preambulo: String::new(),
            historico: String::new(),
            material: String::new(),
            objetivo: String::new(),
            exame: String::new(),
            consideracoes: String::new(),
            conclusao: String::new(),
            keyword_claim: String::new(),
            keyword_source: String::new(),
            submitted: false,
        }
    }
}

pub struct Sim {
    pub case: CaseFile,
    pub audit: Audit,
    pub clock: i64,
    pub actor: String,
    pub artifacts: Vec<Artifact>,
    pub notes: Vec<Note>,
    pub report: Report,
    /// Mídias de trabalho (imagens) que podem ser navegadas, por id de origem.
    pub imaged: BTreeSet<String>,
    /// Volumes/arquivos cifrados abertos (caminho ou id de mídia).
    pub decrypted: BTreeSet<String>,
    /// Arquivos compostos já expandidos.
    pub expanded: BTreeSet<String>,
    /// Hashes calculados (alvo -> hash) para conferência.
    pub hashes: BTreeMap<String, String>,
    pub keyword_found: bool,
    pub keyword_where: String,
    /// Conexão escolhida para diligências de rede.
    pub net_conn: String,
    /// O investigado foi alertado (visita a host monitorado sem anonimato)?
    pub suspect_alerted: bool,
    /// Faltas registradas (contaminação, conclusões sem base, etc.).
    pub faults: Vec<String>,
    pub artifact_seq: u32,
}

impl Sim {
    pub fn new(case: CaseFile) -> Self {
        let start = case.start;
        let mut s = Sim {
            case,
            audit: Audit::new(),
            clock: start,
            actor: "Perito".to_string(),
            artifacts: vec![],
            notes: vec![],
            report: Report::default(),
            imaged: BTreeSet::new(),
            decrypted: BTreeSet::new(),
            expanded: BTreeSet::new(),
            hashes: BTreeMap::new(),
            keyword_found: false,
            keyword_where: String::new(),
            net_conn: "institucional".into(),
            suspect_alerted: false,
            faults: vec![],
            artifact_seq: 0,
        };
        s.audit.record(start, &s.actor, "abertura", &format!("Caso {} — {}", s.case.id, s.case.title), "sessão iniciada");
        s
    }

    fn tick(&mut self, minutes: i64) {
        self.clock += minutes * 60;
        self.run_events();
    }

    fn rec(&mut self, action: &str, detail: &str, outcome: &str) {
        let (a, d, o) = (action.to_string(), detail.to_string(), outcome.to_string());
        self.audit.record(self.clock, &self.actor.clone(), &a, &d, &o);
    }

    fn fault(&mut self, f: &str) {
        self.faults.push(format!("[{}] {}", fmt_dt(self.clock, BRT), f));
    }

    fn new_artifact(&mut self, kind: &str, origin: &str, hash: &str, note: &str) -> String {
        self.artifact_seq += 1;
        let id = format!("A-{:03}", self.artifact_seq);
        self.artifacts.push(Artifact { id: id.clone(), kind: kind.into(), origin: origin.into(), hash: hash.into(), created: self.clock, note: note.into() });
        id
    }

    /// Dispara eventos agendados cujo tempo já passou (apagamento remoto, etc.).
    fn run_events(&mut self) {
        let now = self.clock;
        // apagamento remoto de dispositivos móveis ainda conectados
        let mut wiped = vec![];
        for it in &mut self.case.items {
            if let Some(ph) = &mut it.phone {
                if let Some(t) = ph.remote_wipe_at {
                    let connected = !ph.rf.isolates() && !ph.airplane && ph.boot != Boot::Desligado && !ph.battery_removed;
                    if t <= now && !ph.wiped {
                        if connected {
                            ph.wiped = true;
                            wiped.push(it.id.clone());
                        }
                        ph.remote_wipe_at = None;
                    }
                }
            }
        }
        for id in wiped {
            self.rec("evento", &format!("{}: comando remoto de restauração recebido", id), "dados do aparelho APAGADOS");
            self.fault(&format!("O aparelho {} recebeu comando remoto de apagamento por não estar isolado.", id));
        }
        let mut fired = vec![];
        for (i, ev) in self.case.events.iter().enumerate() {
            if !ev.fired && ev.at <= now {
                fired.push(i);
            }
        }
        for i in fired {
            let (kind, target, note) = {
                let ev = &self.case.events[i];
                (ev.kind.clone(), ev.target.clone(), ev.note.clone())
            };
            match kind.as_str() {
                "remote-access-wipe" => {
                    if let Some(it) = self.case.items.iter_mut().find(|x| x.id == target) {
                        if let Some(live) = &it.live {
                            if live.network_connected && it.power_on {
                                // apaga a RAM e marca perda de dados voláteis
                                if let Some(live) = &mut it.live {
                                    live.ram_artifacts.clear();
                                    live.mounted_encrypted = None;
                                    live.screen = "Tela bloqueada; volume cifrado desmontado.".into();
                                }
                                self.case.events[i].fired = true;
                                self.rec("evento", &note, "dados voláteis perdidos: acesso remoto apagou a memória");
                                self.fault("Dados voláteis foram apagados por acesso remoto: o equipamento permaneceu conectado à rede.");
                                continue;
                            }
                        }
                    }
                }
                _ => {}
            }
            self.case.events[i].fired = true;
        }
    }

    pub fn item_mut(&mut self, id: &str) -> Option<usize> {
        self.case.items.iter().position(|i| i.id.eq_ignore_ascii_case(id) && (!i.hidden || i.flag("descoberto")))
    }

    // ------------------------------------------------------------------ exec

    pub fn exec(&mut self, line: &str) -> Output {
        let line = line.trim();
        if line.is_empty() {
            return Output::ok("");
        }
        let toks: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
        let cmd = toks[0].to_lowercase();
        let a = |i: usize| toks.get(i).cloned().unwrap_or_default();
        match cmd.as_str() {
            "ajuda" | "help" | "?" => Output::ok(tools::help(&a(1))),
            "caso" => Output::ok(tools::describe_case(self)),
            "itens" | "inventario" => Output::ok(tools::inventory(self)),
            "item" => tools::item_detail(self, &a(1)),
            "tempo" => Output::ok(format!("Hora simulada: {}", fmt_dtz(self.clock, BRT))),
            "notas" => Output::ok(tools::notes(self)),
            "custodia" => Output::ok(tools::custody(self)),
            "auditoria" => Output::ok(self.audit.render(BRT)),
            "progresso" => Output::ok(crate::assess::progress(self)),
            "linha-do-tempo" | "timeline" => Output::ok(tools::timeline_text(self)),
            "mentor" | "dica" => Output::ok(crate::assess::hint(self)),
            // --- físico / procedimental
            "fotografar" => self.cmd_fotografar(&a(1)),
            "lacre" => self.cmd_lacre(&a(1), &a(2)),
            "deslacrar" => self.cmd_deslacrar(&a(1)),
            "abrir" => self.cmd_abrir(&a(1)),
            "componente" => self.cmd_componente(&a(1), &a(2), &a(3)),
            "midia" => self.cmd_midia(&toks),
            "bloqueio" | "bloqueador" => self.cmd_bloqueio(&a(1)),
            "imagem" | "adquirir" => self.cmd_imagem(&a(1)),
            "sanitizar" => self.cmd_sanitizar(&a(1)),
            "clone" | "espelhar" => self.cmd_clone(&a(1), &a(2)),
            "hash" => self.cmd_hash(&a(1)),
            "verificar-hash" | "conferir-hash" => self.cmd_verificar_hash(&a(1), &a(2)),
            "energia" => self.cmd_energia(&a(1), &a(2)),
            "memoria" | "dump" => self.cmd_memoria(&a(1)),
            "volume" => self.cmd_volume(&a(1), &a(2)),
            "inspecionar" | "vasculhar" => self.cmd_inspecionar(&a(1)),
            "rede" => self.cmd_rede(&toks),
            // --- móveis
            "isolar" => self.cmd_isolar(&a(1), &a(2)),
            "airplane" | "aviao" => self.cmd_airplane(&a(1)),
            "sim" => self.cmd_sim(&a(1), &a(2), &a(3)),
            "alarmes" => self.cmd_alarmes(&a(1)),
            "carregar" => self.cmd_carregar(&a(1)),
            "desbloquear" => self.cmd_desbloquear(&a(1), &a(2)),
            "extrair" => self.cmd_extrair(&a(1), &a(2)),
            "senha" => self.cmd_senha_from_doc(&a(1)),
            "embalar" => self.cmd_embalar(&a(1)),
            "camera" => self.cmd_camera(&a(1), &a(2)),
            "solicitar-senhas" => self.cmd_solicitar_senhas(&a(1)),
            "etiquetar" => self.cmd_etiquetar(&a(1), &toks),
            // --- investigação (sistema de arquivos / rede) — delegado a tools
            "fs" => tools::fs(self, &toks),
            "recuperar" => tools::recuperar(self, &a(1), &a(2)),
            "expandir" => tools::expandir(self, &a(1), &a(2)),
            "decifrar" => tools::decifrar(self, &a(1), &a(2)),
            "ocr" => tools::ocr(self, &a(1), &a(2)),
            "dicionario" => tools::dicionario(self, &a(1)),
            "dns" => tools::dns(self, &a(1)),
            "whois" => tools::whois(self, &a(1)),
            "traceroute" | "rota" => tools::traceroute(self, &a(1)),
            "navegar" => tools::navegar(self, &a(1)),
            "preservar" => tools::preservar(self, &a(1)),
            "arquivo-web" | "historico-web" => tools::archive(self, &a(1)),
            "email" => tools::email(self, &toks),
            "conexao" => self.cmd_conexao(&a(1)),
            // --- relatório / avaliação
            "anotar" => self.cmd_anotar(&toks),
            "achei" | "palavra-chave" => self.cmd_achei(&toks),
            "laudo" => tools::laudo(self, &toks),
            "finalizar" | "avaliar" => crate::assess::finalize(self),
            _ => Output::err(format!("Comando desconhecido: '{}'. Digite 'ajuda' para a lista de comandos.", cmd)),
        }
    }

    // ---- handlers físicos

    fn cmd_fotografar(&mut self, id: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        self.tick(3);
        let label = self.case.items[i].label.clone();
        self.case.items[i].set("fotografado", "sim");
        self.rec("fotografar", &format!("{} ({})", id.to_uppercase(), label), "documentação fotográfica realizada");
        Output::ok(format!("Item {} fotografado e documentado.", id.to_uppercase()))
    }

    fn cmd_lacre(&mut self, sub: &str, id: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Uso: lacre conferir|aplicar <item>") };
        match sub {
            "conferir" => {
                self.tick(4);
                let it = &self.case.items[i];
                match (&it.seal, &it.doc_seal) {
                    (Some(seal), Some(doc)) => {
                        let s = seal.number.clone();
                        let d = doc.clone();
                        let intact = seal.intact;
                        if !intact {
                            self.fault("Lacre violado não foi registrado como tal antes do deslacre.");
                        }
                        if s == d {
                            self.rec("lacre-conferir", &format!("{} lacre {}", id.to_uppercase(), s), "número confere com a documentação; lacre íntegro");
                            Output::ok(format!("Lacre nº {} — confere com a documentação. Íntegro: {}.", s, if intact { "sim" } else { "NÃO" }))
                        } else {
                            self.rec("lacre-conferir", &format!("{} lacre físico {} x doc {}", id.to_uppercase(), s, d), "DIVERGÊNCIA registrada");
                            self.case.items[i].set("divergencia_lacre", &format!("fisico {} / doc {}", s, d));
                            Output::ok(format!(
                                "ATENÇÃO: divergência de lacre.\n  Lacre físico: {}\n  Documentação: {}\nDivergência registrada na auditoria — deve constar no laudo.",
                                s, d
                            ))
                        }
                    }
                    _ => {
                        self.rec("lacre-conferir", &id.to_uppercase(), "item sem lacre (coleta em campo)");
                        Output::ok("Este item não possui lacre pré-existente (coletado/lacrado pelo próprio perito).")
                    }
                }
            }
            "aplicar" => {
                self.tick(3);
                let n = format!("{}{}", self.case.seed % 9 + 1, (self.clock % 1_000_000).abs());
                self.case.items[i].seal = Some(Seal { number: n.clone(), intact: true });
                self.case.items[i].set("lacrado_perito", &n);
                self.rec("lacre-aplicar", &format!("{} lacre {}", id.to_uppercase(), n), "lacre aposto pelo perito");
                Output::ok(format!("Lacre nº {} aposto ao item {}.", n, id.to_uppercase()))
            }
            _ => Output::err("Uso: lacre conferir|aplicar <item>"),
        }
    }

    fn cmd_deslacrar(&mut self, id: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        if !self.case.items[i].flag("fotografado") {
            self.fault("Deslacre antes de fotografar o item lacrado.");
        }
        if self.case.items[i].flag("divergencia_lacre") == false && self.case.items[i].seal.is_some() && !self.case.items[i].flag("lacre_conferido_marker") {
            // conferência é esperada; registramos recomendação, sem bloquear
        }
        if self.case.items[i].seal.is_none() {
            return Output::err("Item não está lacrado.");
        }
        self.tick(3);
        self.case.items[i].set("deslacrado", "sim");
        if let Some(s) = &mut self.case.items[i].seal {
            s.intact = false;
        }
        self.rec("deslacrar", &id.to_uppercase(), "item aberto para exame");
        Output::ok(format!("Item {} deslacrado. Lembre-se de apor novo lacre ao final.", id.to_uppercase()))
    }

    fn cmd_abrir(&mut self, id: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        if self.case.items[i].kind.is_computer() && self.case.items[i].power_on {
            return Output::err("O equipamento está LIGADO. Avalie os dados voláteis antes de abri-lo/desligá-lo (comandos 'memoria', 'energia').");
        }
        self.tick(5);
        if let Some(c) = self.case.items[i].components.iter_mut().find(|c| c.kind == "tampa") {
            c.removed = true;
        }
        self.case.items[i].set("aberto", "sim");
        self.rec("abrir", &id.to_uppercase(), "gabinete aberto");
        Output::ok(format!("Gabinete do item {} aberto. Use 'item {}' para ver componentes e slots.", id.to_uppercase(), id.to_uppercase()))
    }

    fn cmd_componente(&mut self, sub: &str, id: &str, comp: &str) -> Output {
        if sub != "remover" {
            return Output::err("Uso: componente remover <item> <componente>");
        }
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        let Some(ci) = self.case.items[i].components.iter().position(|c| c.id.eq_ignore_ascii_case(comp) || c.kind.eq_ignore_ascii_case(comp)) else {
            return Output::err("Componente não encontrado. Veja 'item <id>'.");
        };
        self.tick(6);
        self.case.items[i].components[ci].removed = true;
        let label = self.case.items[i].components[ci].label.clone();
        // revela slots ocultos por este componente
        let kind = self.case.items[i].components[ci].kind.clone();
        let mut revealed = 0;
        for s in &mut self.case.items[i].slots {
            if s.hidden_by.as_deref() == Some(&kind) {
                s.hidden_by = None;
                revealed += 1;
            }
        }
        self.rec("componente-remover", &format!("{} / {}", id.to_uppercase(), label), &format!("removido; {} slot(s) revelado(s)", revealed));
        let extra = if revealed > 0 { format!("\nATENÇÃO: {} slot(s) de mídia estavam ocultos sob este componente.", revealed) } else { String::new() };
        Output::ok(format!("Componente '{}' removido do item {}.{}", label, id.to_uppercase(), extra))
    }

    fn cmd_midia(&mut self, toks: &[String]) -> Output {
        let sub = toks.get(1).map(|s| s.as_str()).unwrap_or("");
        match sub {
            "listar" => {
                let id = toks.get(2).cloned().unwrap_or_default();
                let Some(i) = self.item_mut(&id) else { return Output::err("Uso: midia listar <item>") };
                let it = &self.case.items[i];
                let mut out = format!("Mídias e slots do item {}:\n", id.to_uppercase());
                for s in &it.slots {
                    let occupied = s.media_id.as_deref().unwrap_or("vazio");
                    let hidden = if s.hidden_by.is_some() { "  [OCULTO — remova o componente]" } else { "" };
                    out.push_str(&format!("  {} ({}) — {}{}\n", s.label, s.iface, occupied, hidden));
                }
                if it.slots.is_empty() {
                    out.push_str("  (mídia integrada / sem slots acessíveis sem abertura)\n");
                }
                Output::ok(out)
            }
            "remover" => {
                let id = toks.get(2).cloned().unwrap_or_default();
                let slot = toks.get(3).cloned().unwrap_or_default();
                let Some(i) = self.item_mut(&id) else { return Output::err("Uso: midia remover <item> <slot>") };
                let Some(si) = self.case.items[i].slots.iter().position(|s| s.id.eq_ignore_ascii_case(&slot)) else {
                    return Output::err("Slot não encontrado. Veja 'midia listar <item>'.");
                };
                if self.case.items[i].slots[si].hidden_by.is_some() {
                    return Output::err("Esse slot está oculto sob um componente. Remova o componente antes.");
                }
                let Some(mid) = self.case.items[i].slots[si].media_id.clone() else {
                    return Output::err("Slot vazio.");
                };
                self.tick(5);
                let iface = self.case.items[i].slots[si].iface.clone();
                self.rec("midia-remover", &format!("{} do slot {} (item {})", mid, slot.to_uppercase(), id.to_uppercase()), &format!("interface registrada: {}", iface));
                Output::ok(format!("Mídia {} removida do slot {} (interface {}). Registre a interface no laudo.", mid, slot.to_uppercase(), iface))
            }
            _ => Output::err("Uso: midia listar|remover ..."),
        }
    }

    fn cmd_bloqueio(&mut self, mid: &str) -> Output {
        let Some(mi) = self.case.media.iter().position(|m| m.id.eq_ignore_ascii_case(mid)) else {
            return Output::err("Mídia não encontrada.");
        };
        self.tick(4);
        self.case.media[mi].writes.clear();
        let id = self.case.media[mi].id.clone();
        self.hashes.insert(format!("wb:{}", id), "on".into());
        self.rec("bloqueio-escrita", &id, "bloqueador de escrita conectado (hardware)");
        Output::ok(format!("Bloqueador de escrita conectado à mídia {}. Leituras subsequentes não alteram o original.", id))
    }

    fn write_blocked(&self, mid: &str) -> bool {
        self.hashes.contains_key(&format!("wb:{}", mid.to_uppercase()))
            || self.hashes.contains_key(&format!("wb:{}", mid))
    }

    fn cmd_imagem(&mut self, mid: &str) -> Output {
        let Some(mi) = self.case.media.iter().position(|m| m.id.eq_ignore_ascii_case(mid)) else {
            return Output::err("Mídia não encontrada.");
        };
        let id = self.case.media[mi].id.clone();
        if self.case.media[mi].fde.is_some() {
            return Output::err(format!(
                "A mídia {} está cifrada integralmente por hardware (dependente do equipamento). A imagem seria ilegível; use 'clone' (espelhamento mídia-para-mídia) ou decifre no equipamento de origem.",
                id
            ));
        }
        if let Some(host) = self.case.items.iter().find(|it| it.kind.is_computer() && it.power_on && it.media.contains(&id)) {
            return Output::err(format!(
                "A mídia {} está em uso no equipamento {} LIGADO. Avalie os dados voláteis ('memoria', 'volume copiar'), desligue ('energia desligar') e então adquira com bloqueador.",
                id, host.id
            ));
        }
        let blocked = self.write_blocked(&id);
        if !blocked {
            self.fault(&format!("Aquisição da mídia {} sem bloqueador de escrita.", id));
        }
        self.tick(25);
        let h = vfs::media_hash(&self.case.media[mi]);
        let aid = self.new_artifact("imagem-forense", &id, &h, "cópia bit a bit (imagem forense)");
        self.imaged.insert(id.clone());
        self.hashes.insert(format!("img:{}", id), h.clone());
        let warn = if blocked { "" } else { "\nAVISO: feita SEM bloqueador de escrita — risco de contaminação do original (registrado)." };
        self.rec("aquisicao", &format!("imagem de {} -> {}", id, aid), &format!("hash {}", &h[..16]));
        Output::ok(format!(
            "Imagem forense de {} concluída ({}).\n  SHA-256: {}\nUse 'verificar-hash' para conferir e 'fs montar {}' para examinar a cópia.{}",
            id, aid, h, id, warn
        ))
    }

    fn cmd_sanitizar(&mut self, mid: &str) -> Output {
        let Some(mi) = self.case.media.iter().position(|m| m.id.eq_ignore_ascii_case(mid)) else {
            // cria uma mídia de destino lógica se não existir
            self.tick(10);
            self.hashes.insert(format!("san:{}", mid.to_uppercase()), "ok".into());
            self.rec("sanitizar", &mid.to_uppercase(), "mídia de destino sanitizada (zerada)");
            return Output::ok(format!("Mídia de destino '{}' sanitizada e pronta para receber espelhamento.", mid.to_uppercase()));
        };
        self.tick(10);
        self.case.media[mi].sanitized = true;
        let id = self.case.media[mi].id.clone();
        self.hashes.insert(format!("san:{}", id), "ok".into());
        self.rec("sanitizar", &id, "mídia sanitizada");
        Output::ok(format!("Mídia {} sanitizada.", id))
    }

    fn cmd_clone(&mut self, src: &str, dst: &str) -> Output {
        let Some(mi) = self.case.media.iter().position(|m| m.id.eq_ignore_ascii_case(src)) else {
            return Output::err("Uso: clone <mídia-origem> <destino>");
        };
        let id = self.case.media[mi].id.clone();
        let dest = if dst.is_empty() { "DEST-01".to_string() } else { dst.to_uppercase() };
        let sanitized = self.hashes.contains_key(&format!("san:{}", dest)) || self.case.media.iter().any(|m| m.id == dest && m.sanitized);
        if !sanitized {
            self.fault(&format!("Espelhamento para destino '{}' não sanitizado previamente.", dest));
        }
        self.tick(30);
        let h = vfs::media_hash(&self.case.media[mi]);
        let aid = self.new_artifact("clone", &id, &h, &format!("espelhamento mídia-para-mídia para {}", dest));
        self.imaged.insert(id.clone());
        self.hashes.insert(format!("img:{}", id), h.clone());
        let warn = if sanitized { "" } else { "\nAVISO: destino não sanitizado — risco de mistura de dados (registrado)." };
        self.rec("espelhamento", &format!("{} -> {} ({})", id, dest, aid), &format!("hash {}", &h[..16]));
        Output::ok(format!(
            "Espelhamento de {} para {} concluído ({}).\n  SHA-256: {}\nObservação: um clone de mídia cifrada por hardware só é legível no equipamento de origem.{}",
            id, dest, aid, h, warn
        ))
    }

    fn cmd_hash(&mut self, target: &str) -> Output {
        if let Some(mi) = self.case.media.iter().position(|m| m.id.eq_ignore_ascii_case(target)) {
            self.tick(5);
            let h = vfs::media_hash(&self.case.media[mi]);
            let id = self.case.media[mi].id.clone();
            self.hashes.insert(format!("hash:{}", id), h.clone());
            self.rec("hash", &format!("mídia {}", id), &h[..16]);
            return Output::ok(format!("SHA-256 da mídia {}:\n  {}", id, h));
        }
        // arquivo dentro de uma imagem adquirida (nunca no original)
        let open = self.open_set();
        for m in self.case.media.iter().filter(|m| self.imaged.contains(&m.id)) {
            if let Some((_, f)) = vfs::find_file_open(m, target, &|id| open.contains(id)) {
                let h = vfs::file_hash(f);
                self.rec("hash", &format!("arquivo {}", target), &h[..16]);
                return Output::ok(format!("SHA-256 do arquivo {}:\n  {}", target, h));
            }
        }
        Output::err("Alvo não encontrado (informe uma mídia ou o caminho de um arquivo de uma imagem adquirida).")
    }

    fn cmd_verificar_hash(&mut self, a: &str, b: &str) -> Output {
        // compara: hash de aquisição x hash documentado / recalculado
        let get = |s: &Sim, key: &str| -> Option<String> {
            s.hashes.get(&format!("img:{}", key.to_uppercase())).cloned().or_else(|| s.hashes.get(&format!("hash:{}", key.to_uppercase())).cloned())
        };
        if b.is_empty() {
            // confere imagem x original x hash documentado
            let Some(mi) = self.case.media.iter().position(|m| m.id.eq_ignore_ascii_case(a)) else {
                return Output::err("Uso: verificar-hash <mídia> [mídia2]");
            };
            let id = self.case.media[mi].id.clone();
            let cur = vfs::media_hash(&self.case.media[mi]);
            let img = get(self, &id);
            let doc = self.case.documented_hashes.get(&id).cloned().filter(|s| !s.is_empty());
            self.tick(3);
            let mut out = format!("Conferência de hash — mídia {}:\n  Recalculado agora: {}\n", id, cur);
            if let Some(h) = &img {
                out.push_str(&format!("  Imagem adquirida:  {}  [{}]\n", h, if *h == cur { "CONFERE" } else { "DIVERGE" }));
            }
            if let Some(h) = &doc {
                let ok = *h == cur;
                out.push_str(&format!("  Documentado antes: {}  [{}]\n", h, if ok { "CONFERE" } else { "DIVERGE" }));
                if !ok {
                    self.fault("Hash documentado não confere — possível alteração/contaminação do original.");
                }
            }
            self.rec("verificar-hash", &id, "conferência realizada");
            Output::ok(out)
        } else {
            let ha = get(self, a).or_else(|| self.case.media.iter().find(|m| m.id.eq_ignore_ascii_case(a)).map(|m| vfs::media_hash(m)));
            let hb = get(self, b).or_else(|| self.case.media.iter().find(|m| m.id.eq_ignore_ascii_case(b)).map(|m| vfs::media_hash(m)));
            match (ha, hb) {
                (Some(x), Some(y)) => {
                    self.rec("verificar-hash", &format!("{} x {}", a, b), if x == y { "iguais" } else { "diferentes" });
                    Output::ok(format!("{}: {}\n{}: {}\n→ {}", a.to_uppercase(), x, b.to_uppercase(), y, if x == y { "HASHES IGUAIS" } else { "HASHES DIFERENTES" }))
                }
                _ => Output::err("Não há hash calculado para um dos alvos. Use 'hash' ou 'imagem' antes."),
            }
        }
    }

    fn cmd_energia(&mut self, sub: &str, id: &str) -> Output {
        if sub != "desligar" {
            return Output::err("Uso: energia desligar <item>");
        }
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        if !self.case.items[i].power_on {
            return Output::err("O item já está desligado.");
        }
        // se houver volume cifrado montado e memória não coletada, alertar perda
        let live_loss = self.case.items[i].live.as_ref().map(|l| l.mounted_encrypted.is_some() && l.memory_dump.is_none()).unwrap_or(false);
        self.tick(2);
        self.case.items[i].power_on = false;
        if let Some(live) = &mut self.case.items[i].live {
            live.mounted_encrypted = None;
            live.ram_artifacts.clear();
        }
        if live_loss {
            self.fault("Desligou com volume cifrado montado sem coletar a memória/conteúdo: a chave na RAM e o acesso ao volume foram perdidos.");
            self.rec("energia-desligar", &id.to_uppercase(), "desligado; PERDA de dados voláteis (volume cifrado estava montado)");
            Output::ok(format!("Item {} desligado.\nATENÇÃO: havia um volume cifrado montado. Com o desligamento, a chave presente na memória foi perdida.", id.to_uppercase()))
        } else {
            self.rec("energia-desligar", &id.to_uppercase(), "desligado (corte de energia)");
            Output::ok(format!("Item {} desligado. Prossiga com bloqueador/boot forense para o exame do disco.", id.to_uppercase()))
        }
    }

    fn cmd_memoria(&mut self, id: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        if !self.case.items[i].power_on {
            return Output::err("O equipamento está desligado; não há memória volátil a coletar.");
        }
        self.tick(12);
        let found_kw;
        let text;
        {
            let live = self.case.items[i].live.clone();
            if let Some(mut live) = live {
                found_kw = live.ram_artifacts.iter().any(|(_, _, k)| *k);
                let dump_note: Vec<String> = live.ram_artifacts.iter().map(|(k, t, _)| format!("  - {}: {}", k, t)).collect();
                live.memory_dump = Some("coletado".into());
                self.case.items[i].live = Some(live);
                text = if dump_note.is_empty() {
                    "Dump de memória coletado. Nenhum artefato de interesse evidente nesta amostra.".to_string()
                } else {
                    format!("Dump de memória coletado. Artefatos observados:\n{}", dump_note.join("\n"))
                };
            } else {
                found_kw = false;
                text = "Dump de memória coletado.".to_string();
            }
        }
        let h = crate::sha256::sha256(&format!("ramdump:{}:{}", id, self.clock));
        let aid = self.new_artifact("dump-memoria", &id.to_uppercase(), &h, "captura de memória volátil");
        self.rec("memoria", &format!("{} -> {}", id.to_uppercase(), aid), "memória volátil preservada");
        if found_kw && !self.case.keyword.is_empty() {
            let src = SourceRef::Ram { item: self.case.items[i].id.clone() };
            if self.case.keyword_sources.contains(&src) {
                let w = format!("memória volátil do item {} (dump {})", self.case.items[i].id, aid);
                self.mark_keyword_found(&w);
                self.notes.push(Note { at: self.clock, text: format!("Expressão de interesse observada em: {}", w), source: Some(src) });
            }
        }
        Output::ok(format!("{}\nArtefato {} (SHA-256 {}…).", text, aid, &h[..12]))
    }

    fn item_mut_ro(&self, id: &str) -> Option<usize> {
        self.case.items.iter().position(|i| i.id.eq_ignore_ascii_case(id) && (!i.hidden || i.flag("descoberto")))
    }

    /// Copia o conteúdo de um volume cifrado MONTADO no equipamento ligado (POP 5.03 §4.3).
    fn cmd_volume(&mut self, sub: &str, id: &str) -> Output {
        if sub != "copiar" {
            return Output::err("Uso: volume copiar <item>");
        }
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        let Some(live) = self.case.items[i].live.clone() else { return Output::err("O item não está ligado ou não possui estado volátil.") };
        if !self.case.items[i].power_on {
            return Output::err("O equipamento está desligado.");
        }
        let Some((pid, letra)) = live.mounted_encrypted.clone() else { return Output::err("Não há volume cifrado montado neste equipamento.") };
        self.tick(18);
        // localiza o contêiner e registra o produto; o conteúdo passa a ser examinável
        let mut found = None;
        for m in &self.case.media {
            for p in &m.partitions {
                for f in &p.files {
                    if let Some(c) = &f.container {
                        if c.id == pid {
                            found = Some((m.id.clone(), f.id.clone(), c.files.clone(), f.path.clone()));
                        }
                    }
                }
            }
        }
        let Some((mid, fid, files, cpath)) = found else { return Output::err("Volume não localizado.") };
        let mut hsh = crate::sha256::Sha256::new();
        for f in &files {
            hsh.update(f.content.as_bytes());
        }
        let h = hsh.finish();
        let aid = self.new_artifact("copia-volume-montado", &format!("{} {}", self.case.items[i].id, letra), &h, "cópia lógica do volume cifrado montado");
        // o produto é uma MÍDIA DE TRABALHO separada: o disco original continua exigindo aquisição
        let work = Media {
            id: aid.clone(),
            kind: MediaKind::Usb,
            iface: "cópia lógica".into(),
            brand: "Mídia de trabalho".into(),
            model: "pericial".into(),
            serial: aid.clone(),
            capacity: files.iter().map(|f| f.size).sum::<u64>().max(1),
            health: Health::Ok,
            bad_sectors: 0,
            fde: None,
            partitions: vec![Partition {
                id: format!("{}:p1", aid),
                index: 1,
                fs: "exFAT".into(),
                label: format!("COPIA-{}", letra.trim_end_matches(':')),
                size: files.iter().map(|f| f.size).sum(),
                files: files.clone(),
                noise: 0,
                noise_roots: vec![],
                encrypted: None,
                tz_offset: BRT,
                dst_note: None,
            }],
            raid: None,
            writes: vec![],
            clone_of: Some(mid.clone()),
            sanitized: true,
            is_work_media: true,
        };
        let _ = fid;
        self.case.media.push(work);
        self.imaged.insert(aid.clone());
        let mut list = vec![];
        for f in &files {
            list.push(format!("  {}", f.path));
            if f.keyword {
                let src = SourceRef::File { media: mid.clone(), file: f.id.clone(), path: format!("{}#{}", cpath, f.path) };
                if self.case.keyword_sources.contains(&src) {
                    let w = format!("arquivo {} no volume cifrado {} copiado com o equipamento ligado ({})", f.path, letra, aid);
                    self.mark_keyword_found(&w);
                    self.notes.push(Note { at: self.clock, text: format!("Expressão de interesse localizada em: {}", w), source: Some(src) });
                }
            }
        }
        self.rec("volume-copiar", &format!("{} {} -> {}", id.to_uppercase(), letra, aid), &format!("hash {}", &h[..16]));
        Output::ok(format!("Conteúdo do volume {} copiado ({}), SHA-256 {}.\nArquivos:\n{}\nLeia com 'fs cat <caminho>'.", letra, aid, h, list.join("\n")))
    }

    /// Busca minuciosa no local/equipamento: revela mídias camufladas ou ocultas.
    fn cmd_inspecionar(&mut self, target: &str) -> Output {
        self.tick(10);
        let mut found = vec![];
        for it in &mut self.case.items {
            if it.hidden && !it.flag("descoberto") && (target.is_empty() || target == "local") {
                it.set("descoberto", "sim");
                found.push(format!("{} — {} ({})", it.id, it.label, it.physical.camouflaged.clone().unwrap_or_default()));
            }
        }
        let mut extra = vec![];
        if !target.is_empty() && target != "local" {
            if let Some(it) = self.case.items.iter().find(|x| x.id.eq_ignore_ascii_case(target)) {
                if let Some(c) = &it.physical.camouflaged {
                    extra.push(format!("Observação em {}: {}", it.id, c));
                }
                if it.slots.iter().any(|s| s.hidden_by.is_some()) {
                    extra.push("Há indício de componente instalado fora do padrão; considere abrir e remover componentes.".to_string());
                }
            }
        }
        self.rec("inspecionar", if target.is_empty() { "local" } else { target }, &format!("{} item(ns) revelado(s)", found.len()));
        let mut out = String::from("Inspeção minuciosa realizada.\n");
        if found.is_empty() && extra.is_empty() {
            out.push_str("Nada adicional encontrado.");
        }
        for f in found {
            out.push_str(&format!("ENCONTRADO: {}\n", f));
        }
        for x in extra {
            out.push_str(&format!("{}\n", x));
        }
        Output::ok(out)
    }

    fn cmd_rede(&mut self, toks: &[String]) -> Output {
        // 'rede isolar <item>' (isolamento digital no local) — distinto de 'rede logs/dns' da internet emulada
        match toks.get(1).map(|s| s.as_str()).unwrap_or("") {
            "isolar" => {
                let id = toks.get(2).cloned().unwrap_or_default();
                let Some(i) = self.item_mut(&id) else { return Output::err("Uso: rede isolar <item>") };
                self.tick(2);
                if let Some(live) = &mut self.case.items[i].live {
                    live.network_connected = false;
                }
                self.case.items[i].set("rede_isolada", "sim");
                self.rec("rede-isolar", &id.to_uppercase(), "isolamento digital (desconexão de rede)");
                Output::ok(format!("Item {} isolado da rede (desconectado). Isso impede comandos remotos de apagamento.", id.to_uppercase()))
            }
            "logs" => tools::net_logs(self, toks.get(2).cloned().unwrap_or_default().as_str()),
            _ => Output::err("Uso: rede isolar <item> | rede logs <ip>"),
        }
    }

    // ---- handlers móveis

    fn cmd_isolar(&mut self, id: &str, method: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        if self.case.items[i].phone.is_none() {
            return Output::err("Isolamento RF aplica-se a dispositivos móveis. Para computadores use 'rede isolar'.");
        }
        let rf = match method {
            "faraday" | "bag" => Rf::FaradayBag,
            "aluminio" | "aluminio3" => Rf::Aluminio3,
            "aluminio1" => Rf::AluminioInsuficiente,
            "sala" => Rf::SalaFaraday,
            "jammer" | "bloqueador" => Rf::Jammer,
            _ => return Output::err("Métodos: faraday | aluminio | sala | jammer"),
        };
        self.tick(4);
        let ph = self.case.items[i].phone.as_mut().unwrap();
        ph.rf = rf;
        ph.wifi = false;
        ph.bt = false;
        ph.cellular = false;
        let isolates = rf.isolates();
        self.case.items[i].set("isolado_rf", rf.label());
        self.rec("isolar-rf", &format!("{} ({})", id.to_uppercase(), rf.label()), if isolates { "comunicações isoladas" } else { "isolamento INSUFICIENTE" });
        if isolates {
            Output::ok(format!("Dispositivo {} isolado por {}. Comunicações bloqueadas; planeje a alimentação (bateria externa) se em AFU.", id.to_uppercase(), rf.label()))
        } else {
            self.fault("Isolamento RF insuficiente: o dispositivo ainda pode receber comandos remotos.");
            Output::ok(format!("Cobertura de alumínio insuficiente ({}). O dispositivo PODE ainda receber sinal — reforce o isolamento.", rf.label()))
        }
    }

    fn phone_idx(&self, id: &str) -> Result<usize, Output> {
        let Some(i) = self.item_mut_ro(id) else { return Err(Output::err("Item não encontrado.")) };
        if self.case.items[i].phone.is_none() {
            return Err(Output::err("Não é um dispositivo móvel."));
        }
        Ok(i)
    }

    fn ph(&mut self, i: usize) -> &mut PhoneState {
        self.case.items[i].phone.as_mut().unwrap()
    }

    fn cmd_airplane(&mut self, id: &str) -> Output {
        let i = match self.phone_idx(id) { Ok(i) => i, Err(e) => return e };
        let snap = self.case.items[i].phone.clone().unwrap();
        if snap.boot == Boot::Bfu || (snap.lock != "nenhum" && snap.boot != Boot::Desbloqueado) {
            return Output::err("Não é possível ativar o modo avião: a tela está bloqueada. Considere o isolamento RF (Faraday).");
        }
        self.tick(2);
        {
            let ph = self.ph(i);
            ph.airplane = true;
            ph.wifi = false;
            ph.bt = false;
            ph.cellular = false;
        }
        self.case.items[i].set("modo_aviao", "sim");
        self.rec("modo-aviao", &id.to_uppercase(), "modo avião ativado e conexões verificadas");
        Output::ok(format!("Modo avião ativado no item {} e Wi-Fi/Bluetooth desligados. Registre que foi possível ativá-lo.", id.to_uppercase()))
    }

    fn cmd_sim(&mut self, sub: &str, id: &str, slot: &str) -> Output {
        if sub != "remover" {
            return Output::err("Uso: sim remover <item> <slot>");
        }
        let i = match self.phone_idx(id) { Ok(i) => i, Err(e) => return e };
        let sn: u32 = slot.parse().unwrap_or(1);
        let Some(si) = self.case.items[i].phone.as_ref().unwrap().sims.iter().position(|s| s.slot == sn && s.place == SimWhere::Slot) else {
            return Output::err("SIM não encontrado nesse slot (ou já removido).");
        };
        self.tick(3);
        let (icc, op) = {
            let ph = self.ph(i);
            ph.sims[si].place = SimWhere::Removido;
            (ph.sims[si].iccid.clone(), ph.sims[si].operadora.clone())
        };
        self.case.items[i].set(&format!("sim{}_removido", sn), &icc);
        self.rec("sim-remover", &format!("{} slot {} ICCID {}", id.to_uppercase(), sn, icc), &format!("operadora {}", op));
        Output::ok(format!("SIM do slot {} removido. ICCID {} / operadora {}. Registre o slot e afixe o chip ao material.", sn, icc, op))
    }

    fn cmd_alarmes(&mut self, id: &str) -> Output {
        let i = match self.phone_idx(id) { Ok(i) => i, Err(e) => return e };
        let snap = self.case.items[i].phone.clone().unwrap();
        if snap.boot != Boot::Desbloqueado && snap.lock != "nenhum" {
            self.tick(1);
            self.rec("alarmes", &id.to_uppercase(), "não foi possível: tela bloqueada (registrado)");
            return Output::ok("Não foi possível desabilitar os alarmes: a tela está bloqueada. A impossibilidade foi registrada (deve constar no termo/laudo).");
        }
        self.tick(2);
        self.ph(i).alarms_disabled = true;
        self.rec("alarmes", &id.to_uppercase(), "alarmes desabilitados");
        Output::ok("Alarmes desabilitados para evitar toques/alertas durante a custódia.")
    }

    fn cmd_carregar(&mut self, id: &str) -> Output {
        let i = match self.phone_idx(id) { Ok(i) => i, Err(e) => return e };
        self.tick(1);
        {
            let ph = self.ph(i);
            ph.charging = true;
            ph.external_battery = true;
        }
        self.case.items[i].set("alimentacao", "bateria externa");
        self.rec("carregar", &id.to_uppercase(), "alimentação garantida (bateria externa)");
        Output::ok("Dispositivo conectado a bateria externa carregada. Em AFU, manter energizado preserva o estado.")
    }

    fn cmd_desbloquear(&mut self, id: &str, pass: &str) -> Output {
        let i = match self.phone_idx(id) { Ok(i) => i, Err(e) => return e };
        if pass.is_empty() {
            return Output::err("Informe a senha/PIN: desbloquear <item> <senha>");
        }
        let snap = self.case.items[i].phone.clone().unwrap();
        if snap.wiped {
            return Output::err("O dispositivo foi restaurado de fábrica.");
        }
        if snap.boot == Boot::Desbloqueado {
            return Output::ok("Dispositivo já está desbloqueado.");
        }
        self.tick(2);
        if pass == snap.passcode {
            self.ph(i).boot = Boot::Desbloqueado;
            self.rec("desbloquear", &id.to_uppercase(), "desbloqueado com a senha informada");
            return Output::ok("Dispositivo desbloqueado com a senha informada.");
        }
        let (fails, wiped) = {
            let ph = self.ph(i);
            ph.failed_unlocks += 1;
            let w = ph.wipe_after_fails.map(|w| ph.failed_unlocks >= w).unwrap_or(false);
            if w {
                ph.wiped = true;
            }
            (ph.failed_unlocks, w)
        };
        let mut extra = String::new();
        if wiped {
            extra = "\nATENÇÃO: o dispositivo atingiu o limite de tentativas e executou restauração de fábrica. Dados perdidos.".into();
            self.fault("Tentativas de senha no aparelho levaram ao apagamento (limite de tentativas atingido).");
        }
        self.rec("desbloquear", &format!("{} tentativa {}", id.to_uppercase(), fails), "senha incorreta");
        Output::ok(format!("Senha incorreta ({}ª tentativa).{}", fails, extra))
    }

    fn cmd_senha_from_doc(&mut self, _sub: &str) -> Output {
        if self.case.provided_passwords.is_empty() {
            return Output::ok("Nenhuma senha consta na documentação que acompanha o material.");
        }
        let mut out = String::from("Senhas constantes na documentação (termo/ofício):\n");
        for (desc, pw) in &self.case.provided_passwords {
            out.push_str(&format!("  • {}: {}\n", desc, pw));
        }
        Output::ok(out)
    }

    fn cmd_extrair(&mut self, id: &str, kind: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        let Some(ph) = self.case.items[i].phone.clone() else { return Output::err("Extração aplica-se a dispositivos móveis.") };
        if ph.wiped {
            return Output::err("O dispositivo foi restaurado de fábrica; não há dados de usuário a extrair.");
        }
        // o isolamento é pré-requisito quando o aparelho está ligado e conectável
        if self.case.items[i].phone.as_ref().unwrap().boot != Boot::Desligado && !self.case.items[i].flag("isolado_rf") && !self.case.items[i].flag("modo_aviao") {
            self.fault("Extração iniciada sem isolar as comunicações do aparelho ligado.");
        }
        let kind = if kind.is_empty() { "auto" } else { kind };
        let can = match kind {
            "fisica" => (ph.support_fisica, "física"),
            "fs" | "sistema" => (matches!((ph.support_fs, ph.boot),
                (FsSupport::Sempre, _) | (FsSupport::Afu, Boot::Afu) | (FsSupport::Afu, Boot::Desbloqueado) | (FsSupport::Desbloqueado, Boot::Desbloqueado)),
                "sistema de arquivos"),
            "logica" => (ph.support_logica && ph.boot == Boot::Desbloqueado, "lógica"),
            _ => {
                // auto: escolhe a mais completa possível
                if ph.support_fisica { (true, "física") }
                else if matches!((ph.support_fs, ph.boot), (FsSupport::Sempre,_)|(FsSupport::Afu, Boot::Afu)|(FsSupport::Afu, Boot::Desbloqueado)|(FsSupport::Desbloqueado, Boot::Desbloqueado)) { (true, "sistema de arquivos") }
                else if ph.boot == Boot::Desbloqueado { (true, "lógica") }
                else { (false, "indisponível") }
            }
        };
        self.tick(20);
        if !can.0 {
            self.rec("extracao", &format!("{} ({})", id.to_uppercase(), can.1), "não realizável no estado atual");
            let dica = if ph.boot == Boot::Bfu {
                "O aparelho está em BFU (bloqueado, criptografia baseada em arquivos): a extração completa depende de desbloqueio."
            } else if ph.boot == Boot::Desligado {
                "O aparelho está desligado: ligue dentro do isolamento RF e avalie o estado."
            } else {
                "O método solicitado não é suportado por este aparelho no estado atual."
            };
            return Output::err(format!("Extração {} não foi possível. {}", can.1, dica));
        }
        let mid = ph.data_media.clone();
        let h = vfs::media_hash(self.case.media.iter().find(|m| m.id == mid).unwrap());
        let aid = self.new_artifact("extracao-movel", &id.to_uppercase(), &h, &format!("extração {} do aparelho", can.1));
        self.imaged.insert(mid.clone());
        self.hashes.insert(format!("img:{}", mid), h.clone());
        self.rec("extracao", &format!("{} ({}) -> {}", id.to_uppercase(), can.1, aid), &format!("hash {}", &h[..16]));
        Output::ok(format!(
            "Extração {} concluída ({}).\n  SHA-256: {}\n  Examine com 'fs montar {}'. Limitação: uma extração {} pode não conter tudo que uma física conteria.",
            can.1, aid, h, mid, can.1
        ))
    }

    /// POP 5.05: etiqueta sobre a câmera frontal (e nada mais sobre a tela).
    fn cmd_camera(&mut self, sub: &str, id: &str) -> Output {
        if sub != "cobrir" {
            return Output::err("Uso: camera cobrir <item>");
        }
        let i = match self.phone_idx(id) { Ok(i) => i, Err(e) => return e };
        self.tick(1);
        self.ph(i).front_camera_covered = true;
        self.case.items[i].set("etiqueta_camera", "sim");
        self.rec("camera-cobrir", &id.to_uppercase(), "etiqueta aposta sobre a câmera frontal");
        Output::ok("Etiqueta aposta sobre a câmera frontal (a tela permanece livre).")
    }

    /// POP 5.05: solicitar ao proprietário as senhas, inclusive de segundo espaço/cofres, e verificar smartlock.
    fn cmd_solicitar_senhas(&mut self, id: &str) -> Output {
        let i = match self.phone_idx(id) { Ok(i) => i, Err(e) => return e };
        self.tick(5);
        let ph = self.case.items[i].phone.clone().unwrap();
        let mut out = String::from("O proprietário informou (registrar no termo e em etiqueta no verso do aparelho):\n");
        let mut got = vec![("PIN/senha de desbloqueio".to_string(), ph.passcode.clone())];
        if let Some(ss) = &ph.second_space {
            got.push((format!("senha do {}", ss.name), ss.password.clone()));
        }
        for (k, v) in &got {
            out.push_str(&format!("  • {}: {}\n", k, v));
            if !self.case.provided_passwords.iter().any(|(_, p)| p == v) {
                self.case.provided_passwords.push((format!("{} (informada na apreensão)", k), v.clone()));
            }
        }
        match &ph.smartlock {
            Some(s) => out.push_str(&format!("Smartlock ATIVO ({}): o aparelho pode desbloquear sozinho perto do dispositivo/local confiável — registre.\n", s)),
            None => out.push_str("Smartlock: não configurado.\n"),
        }
        self.case.items[i].set("senhas_solicitadas", "sim");
        self.rec("solicitar-senhas", &id.to_uppercase(), &format!("{} senha(s) informada(s)", got.len()));
        Output::ok(out)
    }

    fn cmd_embalar(&mut self, id: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        self.tick(3);
        let anti = self.case.items[i].kind.is_media() || self.case.items[i].kind.is_computer();
        self.case.items[i].set("embalado", if anti { "embalagem antiestática" } else { "embalado" });
        self.rec("embalar", &id.to_uppercase(), if anti { "embalado (antiestática)" } else { "embalado" });
        Output::ok(format!("Item {} embalado{}.", id.to_uppercase(), if anti { " em embalagem antiestática" } else { "" }))
    }

    fn cmd_etiquetar(&mut self, id: &str, toks: &[String]) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Uso: etiquetar <item> <texto>") };
        let txt = toks[2..].join(" ");
        self.tick(2);
        self.case.items[i].set("etiqueta", if txt.is_empty() { "identificado" } else { &txt });
        self.rec("etiquetar", &id.to_uppercase(), "material identificado/etiquetado");
        Output::ok(format!("Item {} etiquetado.", id.to_uppercase()))
    }

    fn cmd_conexao(&mut self, kind: &str) -> Output {
        let k = match kind {
            "institucional" | "dedicada" | "anonima" => kind,
            "" => return Output::ok(format!("Conexão atual: {}. Opções: institucional | dedicada | anonima.", self.net_conn)),
            _ => return Output::err("Opções: institucional | dedicada | anonima"),
        };
        self.net_conn = k.to_string();
        self.rec("conexao", k, "conexão de diligência selecionada");
        let nota = match k {
            "institucional" => "ATENÇÃO: a conexão institucional pode vincular a diligência ao órgão. Prefira uma conexão dedicada/desvinculada.",
            "dedicada" => "Conexão dedicada/desvinculada selecionada — adequada para diligências.",
            _ => "Conexão anônima selecionada — reduz o risco de alertar o investigado em hosts monitorados.",
        };
        Output::ok(format!("Conexão '{}' selecionada.\n{}", k, nota))
    }

    fn cmd_anotar(&mut self, toks: &[String]) -> Output {
        let txt = toks[1..].join(" ");
        if txt.is_empty() {
            return Output::err("Uso: anotar <texto da observação>");
        }
        self.notes.push(Note { at: self.clock, text: txt.clone(), source: None });
        self.rec("anotar", &txt.chars().take(60).collect::<String>(), "anotação registrada");
        Output::ok("Anotação registrada (disponível para o laudo).")
    }

    fn cmd_achei(&mut self, toks: &[String]) -> Output {
        let claim = toks[1..].join(" ");
        if claim.is_empty() {
            return Output::err("Uso: achei <expressão> (indique também a origem com 'anotar' ou pelo laudo).");
        }
        if self.case.keyword.is_empty() {
            return Output::err("Este caso (apreensão) não tem expressão-alvo a localizar.");
        }
        let ok = fold(&claim).contains(&fold(&self.case.keyword)) || fold(&self.case.keyword).contains(&fold(&claim));
        self.tick(1);
        if ok {
            self.report.keyword_claim = self.case.keyword.clone();
            self.rec("palavra-chave", &self.case.keyword.clone(), "expressão de interesse apontada pelo perito");
            Output::ok(format!(
                "Expressão registrada: \"{}\".\nIsso NÃO conclui o caso: você ainda precisa indicar a ORIGEM (arquivo/página/fonte) e fundamentar no laudo. Use 'laudo origem <...>' e 'laudo conclusao <...>'.",
                self.case.keyword
            ))
        } else {
            self.fault(&format!("Expressão apontada ('{}') não corresponde à de interesse do caso.", claim));
            self.rec("palavra-chave", &claim, "não corresponde");
            Output::ok("A expressão informada não corresponde à de interesse do caso. Continue a investigação.")
        }
    }

    /// Marca a palavra-chave como efetivamente localizada numa fonte alcançada.
    pub fn mark_keyword_found(&mut self, where_desc: &str) {
        if !self.keyword_found {
            self.keyword_found = true;
            self.keyword_where = where_desc.to_string();
        }
    }
}
