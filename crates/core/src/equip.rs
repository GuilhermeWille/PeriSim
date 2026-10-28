//! Equipamento do perito e regras "passo a passo" (gerador 1.2.0 em diante).
//!
//! Cada atividade física exige o objeto certo na mão (câmera para fotografar, chave para
//! parafusos, cabo SATA para ligar a mídia ao bloqueador…). Luvas e pulseira antiestática
//! são vestidas: sem elas a ação acontece, mas fica registrada como falta — como na vida
//! real, o erro é possível e tem consequência. Operações compostas viram etapas: a tampa
//! sai depois dos parafusos; a mídia, depois dos cabos e dos parafusos do suporte; a
//! aquisição exige a cadeia mídia → bloqueador → estação montada.
//!
//! Casos gerados em versões anteriores (1.0.0 / 1.1.0) seguem as regras clássicas, para
//! que salvamentos antigos continuem reproduzíveis.

use crate::engine::{Output, Sim};
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tool {
    Camera,
    Oficio,
    Tesoura,
    Chave,
    Lanterna,
    Placas,
    CaboSata,
    AdaptadorM2,
    LeitorSd,
    CaboUsb,
    CaboUsbC,
    CaboLightning,
    Bateria,
    Faraday,
    Aluminio,
    Ejetor,
    PendriveColeta,
    Saco,
    FitaLacre,
    Etiquetas,
}

/// (ferramenta, id do comando, nome, para que serve)
pub const TOOLS: &[(Tool, &str, &str, &str)] = &[
    (Tool::Camera, "camera", "Câmera fotográfica", "fotografar o material e os vestígios"),
    (Tool::Oficio, "oficio", "Ofício e documentação", "conferir lacre e descrição"),
    (Tool::Tesoura, "tesoura", "Tesoura", "romper o lacre da embalagem"),
    (Tool::Chave, "chave", "Chave Phillips", "parafusos da tampa, do suporte e do dissipador"),
    (Tool::Lanterna, "lanterna", "Lanterna", "inspeção minuciosa"),
    (Tool::Placas, "placas", "Placas numeradas e escala", "sinalizar vestígios"),
    (Tool::CaboSata, "cabo-sata", "Cabo SATA (dados + energia)", "ligar HD/SSD ao bloqueador"),
    (Tool::AdaptadorM2, "adaptador-m2", "Adaptador M.2 NVMe", "ligar SSD NVMe ao bloqueador"),
    (Tool::LeitorSd, "leitor-sd", "Leitor de cartão SD", "ligar cartão de memória ao bloqueador"),
    (Tool::CaboUsb, "cabo-usb", "Cabo USB do bloqueador", "ligar o bloqueador à estação"),
    (Tool::CaboUsbC, "cabo-usbc", "Cabo USB-C de dados", "extração de aparelho com USB-C"),
    (Tool::CaboLightning, "cabo-lightning", "Cabo Lightning de dados", "extração de aparelho com Lightning"),
    (Tool::Bateria, "bateria", "Bateria externa", "manter o celular energizado"),
    (Tool::Faraday, "faraday", "Faraday bag", "isolar o celular de redes"),
    (Tool::Aluminio, "aluminio", "Papel-alumínio", "isolamento improvisado (3 camadas)"),
    (Tool::Ejetor, "ejetor", "Ejetor de SIM", "abrir a bandeja do chip"),
    (Tool::PendriveColeta, "pendrive-coleta", "Pendrive de coleta", "memória e volume com o equipamento ligado"),
    (Tool::Saco, "saco", "Saco de evidência antiestático", "embalar"),
    (Tool::FitaLacre, "fita-lacre", "Lacre numerado", "apor novo lacre"),
    (Tool::Etiquetas, "etiquetas", "Etiquetas e caneta", "identificar o material e cobrir câmera"),
];

pub fn tool_by_id(id: &str) -> Option<Tool> {
    let id = id.to_lowercase();
    TOOLS.iter().find(|t| t.1 == id || t.1.replace('-', "") == id.replace('-', "")).map(|t| t.0)
}

pub fn tool_name(t: Tool) -> &'static str {
    TOOLS.iter().find(|x| x.0 == t).map(|x| x.2).unwrap_or("?")
}

pub fn tool_id(t: Tool) -> &'static str {
    TOOLS.iter().find(|x| x.0 == t).map(|x| x.1).unwrap_or("?")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Gloves {
    #[default]
    Nenhuma,
    Limpas,
    /// Usadas em material com vestígio biológico (id do item ou vestígio).
    Contaminadas(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Equip {
    pub hand: Option<Tool>,
    pub gloves: Gloves,
    pub gloves_pairs: u32,
    pub strap: bool,
    /// Parafusos já retirados: "ITEM" (tampa) ou "ITEM:SLOT" (suporte da mídia).
    pub screws: BTreeMap<String, u8>,
    /// Cabos desconectados: "ITEM:SLOT:dados" / "ITEM:SLOT:energia".
    pub unplugged: BTreeSet<String>,
    /// Mídias fora do equipamento.
    pub removed: BTreeSet<String>,
    /// Mídia ligada ao bloqueador ("M-01" → "bloqueador") ou direto à estação ("estacao").
    pub linked: BTreeMap<String, String>,
    pub blocker_host: bool,
    /// Vestígio → número da placa.
    pub markers: BTreeMap<String, u32>,
    pub photographed: BTreeSet<String>,
    pub reported: BTreeSet<String>,
}

pub fn is_strict(case: &CaseFile) -> bool {
    crate::casegen_ext::version_at_least(&case.gen_version, "1.2.0")
}

/// Parafusos da tampa do equipamento.
pub fn cover_screws(it: &Item) -> u8 {
    match it.kind {
        ItemKind::Desktop | ItemKind::Servidor => 2,
        ItemKind::Notebook => 4,
        _ => 0,
    }
}

/// Parafusos do suporte da mídia num slot.
pub fn slot_screws(case: &CaseFile, it: &Item, sl: &Slot) -> u8 {
    let Some(mid) = &sl.media_id else { return 0 };
    let kind = case.media.iter().find(|m| &m.id == mid).map(|m| m.kind);
    match (it.kind, kind) {
        (_, Some(MediaKind::Nvme)) => 1,
        (ItemKind::Notebook, _) => 2,
        (_, Some(MediaKind::Hdd)) => 4,
        (_, Some(MediaKind::Ssd)) => 2,
        _ => 0,
    }
}

/// Slot com cabos (dados e energia) — discos SATA em gabinete.
pub fn slot_has_cables(it: &Item, sl: &Slot) -> bool {
    matches!(it.kind, ItemKind::Desktop | ItemKind::Servidor) && sl.iface.eq_ignore_ascii_case("SATA") && sl.media_id.is_some()
}

/// Conector necessário para ligar uma mídia ao bloqueador (None = encaixe direto, ex.: pendrive).
pub fn media_connector(m: &Media) -> Option<Tool> {
    match m.kind {
        MediaKind::Hdd | MediaKind::Ssd => Some(Tool::CaboSata),
        MediaKind::Nvme => Some(Tool::AdaptadorM2),
        MediaKind::Sd => Some(Tool::LeitorSd),
        MediaKind::Usb => None,
        MediaKind::FlashMovel => None,
    }
}

/// Cabo de dados que serve no aparelho.
pub fn phone_cable(ph: &PhoneState) -> Tool {
    if ph.os.to_lowercase().contains("ios") { Tool::CaboLightning } else { Tool::CaboUsbC }
}

fn host_of<'a>(case: &'a CaseFile, mid: &str) -> Option<(&'a Item, &'a Slot)> {
    case.items.iter().find_map(|it| it.slots.iter().find(|s| s.media_id.as_deref() == Some(mid)).map(|s| (it, s)))
}

/// A mídia já pode ser manuseada fora do equipamento?
pub fn media_free(s: &Sim, mid: &str) -> bool {
    let up = mid.to_uppercase();
    if s.equip.removed.contains(&up) {
        return true;
    }
    // mídia avulsa (pendrive, cartão, HD entregue solto)
    !s.case.items.iter().any(|it| it.slots.iter().any(|sl| sl.media_id.as_deref() == Some(up.as_str()))) && s.case.media.iter().any(|m| m.id == up)
}

/// O que um comando exige, para avisos da interface: (ferramenta, ação física em item, motivo do bloqueio).
pub struct Need {
    pub tool: Option<Tool>,
    /// Item manuseado (luvas / contaminação).
    pub touches: Option<String>,
    /// Manipulação de componentes eletrônicos (pulseira antiestática).
    pub esd: bool,
    /// Impedimento físico (falta etapa anterior).
    pub blocked: Option<String>,
}

fn need(tool: Option<Tool>, touches: Option<String>, esd: bool) -> Need {
    Need { tool, touches, esd, blocked: None }
}

fn item_of_media(case: &CaseFile, mid: &str) -> Option<String> {
    host_of(case, &mid.to_uppercase()).map(|(it, _)| it.id.clone()).or_else(|| case.items.iter().find(|i| i.media.iter().any(|m| m.eq_ignore_ascii_case(mid))).map(|i| i.id.clone()))
}

/// Requisitos de um comando no modo passo a passo (None = sem requisito físico).
pub fn requirements(s: &Sim, toks: &[String]) -> Option<Need> {
    let a = |i: usize| toks.get(i).map(|x| x.to_lowercase()).unwrap_or_default();
    let up = |i: usize| a(i).to_uppercase();
    let case = &s.case;
    let item = |id: &str| case.items.iter().find(|i| i.id.eq_ignore_ascii_case(id));
    let vest = |id: &str| case.vestigios.iter().any(|v| v.id.eq_ignore_ascii_case(id));
    Some(match a(0).as_str() {
        "fotografar" => need(Some(Tool::Camera), None, false),
        "lacre" if a(1) == "conferir" => need(Some(Tool::Oficio), Some(up(2)), false),
        "lacre" if a(1) == "aplicar" => need(Some(Tool::FitaLacre), Some(up(2)), false),
        "deslacrar" => need(Some(Tool::Tesoura), Some(up(1)), false),
        "inspecionar" => need(Some(Tool::Lanterna), if a(1).is_empty() || a(1) == "local" { None } else { Some(up(1)) }, false),
        "marcar" => need(Some(Tool::Placas), None, false),
        "desparafusar" => need(Some(Tool::Chave), Some(up(1)), true),
        "abrir" => {
            let mut n = need(None, Some(up(1)), true);
            if let Some(it) = item(&a(1)) {
                let total = cover_screws(it);
                let done = *s.equip.screws.get(&it.id).unwrap_or(&0);
                if done < total {
                    n.blocked = Some(format!("A tampa ainda está presa: faltam {} parafuso(s). Segure a chave Phillips e use 'desparafusar {}'.", total - done, it.id.to_lowercase()));
                }
            }
            n
        }
        "componente" => need(Some(Tool::Chave), Some(up(2)), true),
        "cabo" => need(None, Some(up(2)), true),
        "midia" if a(1) == "remover" => {
            let mut n = need(None, Some(up(2)), true);
            if let Some(it) = item(&a(2)) {
                if it.kind.is_computer() && !it.flag("aberto") {
                    n.blocked = Some(format!("Abra o equipamento antes ('abrir {}').", it.id.to_lowercase()));
                } else if let Some(sl) = it.slots.iter().find(|x| x.id.eq_ignore_ascii_case(&a(3))) {
                    let key = format!("{}:{}", it.id, sl.id.to_uppercase());
                    let total = slot_screws(case, it, sl);
                    let done = *s.equip.screws.get(&key).unwrap_or(&0);
                    let cabos = slot_has_cables(it, sl) && ["dados", "energia"].iter().any(|c| !s.equip.unplugged.contains(&format!("{}:{}", key, c)));
                    if cabos {
                        n.blocked = Some(format!("Desconecte antes os cabos de dados e de energia do {} ('cabo desconectar {} {} dados|energia').", sl.label, it.id.to_lowercase(), sl.id.to_lowercase()));
                    } else if done < total {
                        n.blocked = Some(format!("O suporte da mídia ainda tem {} parafuso(s). Segure a chave e use 'desparafusar {} {}'.", total - done, it.id.to_lowercase(), sl.id.to_lowercase()));
                    }
                }
            }
            n
        }
        "conectar" => {
            if a(1) == "bloqueador" {
                need(Some(Tool::CaboUsb), None, false)
            } else {
                let mid = up(1);
                let tool = case.media.iter().find(|m| m.id == mid).and_then(media_connector);
                let mut n = need(tool, item_of_media(case, &mid), true);
                if case.media.iter().any(|m| m.id == mid) && !media_free(s, &mid) {
                    n.blocked = Some(format!("A mídia {} ainda está instalada no equipamento: remova-a antes de conectá-la.", mid));
                } else if let Some(h) = case.items.iter().find(|i| i.kind.is_computer() && i.media.contains(&mid)) {
                    if h.power_on {
                        n.blocked = Some(format!("A mídia {} está no equipamento {} LIGADO: trate os dados voláteis e desligue antes.", mid, h.id));
                    } else if h.slots.iter().all(|sl| sl.media_id.as_deref() != Some(mid.as_str())) && !h.flag("aberto") && cover_screws(h) > 0 {
                        n.blocked = Some(format!("Abra o equipamento {} para chegar à mídia {}.", h.id, mid));
                    }
                }
                n
            }
        }
        "bloqueio" | "bloqueador" => {
            let mid = up(1);
            let mut n = need(None, None, false);
            if s.equip.linked.get(&mid).map(|x| x.as_str()) != Some("bloqueador") {
                n.blocked = Some(format!("Conecte a mídia {} ao bloqueador ('conectar {} bloqueador', com o conector certo na mão).", mid, mid.to_lowercase()));
            } else if !s.equip.blocker_host {
                n.blocked = Some("Ligue o bloqueador à estação com o cabo USB ('conectar bloqueador estacao').".into());
            }
            n
        }
        "imagem" | "adquirir" => {
            let mid = up(1);
            let mut n = need(None, None, false);
            let cifrada = case.media.iter().any(|m| m.id == mid && m.fde.is_some());
            let ligado = case.items.iter().any(|it| it.kind.is_computer() && it.power_on && it.media.contains(&mid));
            let link = s.equip.linked.get(&mid).map(|x| x.as_str());
            if !cifrada && !ligado && case.media.iter().any(|m| m.id == mid && m.kind != MediaKind::FlashMovel) {
                match link {
                    Some("estacao") => {}
                    Some("bloqueador") if s.equip.blocker_host => {}
                    Some("bloqueador") => n.blocked = Some("O bloqueador não está ligado à estação ('conectar bloqueador estacao').".into()),
                    _ => n.blocked = Some(format!("A mídia {} não está conectada à estação. Monte a cadeia: mídia → bloqueador → estação.", mid)),
                }
            }
            n
        }
        "energia" | "rede" if a(1) != "logs" && a(1) != "dns" => need(None, Some(up(2)), false),
        "memoria" | "dump" => need(Some(Tool::PendriveColeta), Some(up(1)), false),
        "volume" => need(Some(Tool::PendriveColeta), Some(up(2)), false),
        "isolar" => {
            let t = if a(2).starts_with("alum") { Tool::Aluminio } else { Tool::Faraday };
            need(Some(t), Some(up(1)), false)
        }
        "airplane" | "aviao" | "alarmes" | "desbloquear" | "imei" | "tela" => need(None, Some(up(1)), false),
        "sim" => need(Some(Tool::Ejetor), Some(up(2)), false),
        "carregar" => need(Some(Tool::Bateria), Some(up(1)), false),
        "extrair" => {
            let t = item(&a(1)).and_then(|i| i.phone.as_ref()).map(phone_cable);
            need(t, Some(up(1)), false)
        }
        "embalar" => need(Some(Tool::Saco), Some(up(1)), false),
        "etiquetar" => need(Some(Tool::Etiquetas), Some(up(1)), false),
        "camera" => need(Some(Tool::Etiquetas), Some(up(2)), false),
        "comunicar" if vest(&a(1)) => need(None, None, false),
        _ => return None,
    })
}

impl Sim {
    /// Verifica os requisitos físicos; Err com a explicação quando a ação é impossível.
    pub(crate) fn check_equip(&mut self, toks: &[String]) -> Result<Option<Need>, Output> {
        if !self.strict {
            return Ok(None);
        }
        let Some(n) = requirements(self, toks) else { return Ok(None) };
        // "fotografar V-01" e "marcar" só valem para itens ou vestígios existentes
        if let Some(t) = n.tool {
            if self.equip.hand != Some(t) {
                let atual = self.equip.hand.map(|h| format!(" Na mão agora: {}.", tool_name(h))).unwrap_or_default();
                return Err(Output::err(format!("Para isso, segure: {} ('pegar {}').{}", tool_name(t), tool_id(t), atual)));
            }
        }
        if let Some(b) = &n.blocked {
            return Err(Output::err(b.clone()));
        }
        Ok(Some(n))
    }

    /// Efeitos de manuseio depois de uma ação bem-sucedida: luvas, contaminação, pulseira.
    pub(crate) fn after_equip(&mut self, n: &Need) {
        let Some(target) = n.touches.clone().filter(|t| !t.is_empty()) else { return };
        let is_item = self.case.items.iter().any(|i| i.id == target);
        if !is_item {
            return;
        }
        let bio = self.case.items.iter().any(|i| i.id == target && i.physical.biohazard);
        match self.equip.gloves.clone() {
            Gloves::Nenhuma => {
                if !self.faults.iter().any(|f| f.contains(&format!("{} sem luvas", target))) {
                    self.fault(&format!("Manuseio do item {} sem luvas (risco de contaminar impressões e vestígios).", target));
                }
            }
            Gloves::Contaminadas(src) if src != target => {
                if !self.faults.iter().any(|f| f.contains("contaminação cruzada") && f.contains(&target)) {
                    self.fault(&format!("Possível contaminação cruzada: luvas usadas em {} tocaram o item {} (troque as luvas).", src, target));
                }
            }
            _ => {}
        }
        if bio && self.equip.gloves != Gloves::Nenhuma {
            self.equip.gloves = Gloves::Contaminadas(target.clone());
        }
        if n.esd && !self.equip.strap && !self.faults.iter().any(|f| f.contains("pulseira antiestática")) {
            self.fault("Manipulação de componentes eletrônicos sem pulseira antiestática.");
        }
    }

    pub(crate) fn cmd_pegar(&mut self, what: &str) -> Output {
        let Some(t) = tool_by_id(what) else {
            let lista: Vec<&str> = TOOLS.iter().map(|t| t.1).collect();
            return Output::err(format!("Ferramenta desconhecida. Disponíveis: {}.", lista.join(", ")));
        };
        if self.equip.hand == Some(t) {
            return Output::ok(format!("{} já está na mão.", tool_name(t)));
        }
        let antes = self.equip.hand.replace(t);
        self.rec("pegar", tool_name(t), &antes.map(|a| format!("{} devolvido à maleta", tool_name(a))).unwrap_or_else(|| "mão livre".into()));
        Output::ok(format!("Na mão: {}.{}", tool_name(t), antes.map(|a| format!(" ({} voltou para a maleta.)", tool_name(a))).unwrap_or_default()))
    }

    pub(crate) fn cmd_guardar(&mut self) -> Output {
        match self.equip.hand.take() {
            Some(t) => {
                self.rec("guardar", tool_name(t), "devolvido à maleta");
                Output::ok(format!("{} guardado na maleta. Mãos livres.", tool_name(t)))
            }
            None => Output::ok("As mãos já estão livres."),
        }
    }

    pub(crate) fn cmd_luvas(&mut self, sub: &str) -> Output {
        match sub {
            "calcar" | "calçar" | "colocar" | "" => {
                if self.equip.gloves != Gloves::Nenhuma {
                    return Output::ok("Você já está de luvas. Para um par novo: 'luvas trocar'.");
                }
                self.equip.gloves = Gloves::Limpas;
                self.equip.gloves_pairs += 1;
                self.rec("luvas", "calçar", "luvas de nitrilo");
                Output::ok("Luvas de nitrilo calçadas.")
            }
            "trocar" => {
                self.equip.gloves = Gloves::Limpas;
                self.equip.gloves_pairs += 1;
                self.rec("luvas", "trocar", "par descartado em saco de resíduo; novo par calçado");
                Output::ok("Luvas descartadas no saco de resíduo e novo par calçado.")
            }
            "retirar" | "tirar" => {
                self.equip.gloves = Gloves::Nenhuma;
                self.rec("luvas", "retirar", "luvas descartadas");
                Output::ok("Luvas retiradas e descartadas.")
            }
            _ => Output::err("Uso: luvas calcar | trocar | retirar"),
        }
    }

    pub(crate) fn cmd_pulseira(&mut self, sub: &str) -> Output {
        let on = !matches!(sub, "retirar" | "tirar");
        self.equip.strap = on;
        self.rec("pulseira", if on { "colocar" } else { "retirar" }, if on { "pulseira antiestática aterrada" } else { "pulseira retirada" });
        Output::ok(if on { "Pulseira antiestática colocada e aterrada na manta da bancada." } else { "Pulseira antiestática retirada." })
    }

    pub(crate) fn cmd_desparafusar(&mut self, id: &str, slot: &str) -> Output {
        let Some(i) = self.item_mut(id) else { return Output::err("Uso: desparafusar <item> [slot]") };
        let it = self.case.items[i].clone();
        if it.kind.is_computer() && it.power_on {
            return Output::err("O equipamento está LIGADO. Não abra um equipamento energizado.");
        }
        if slot.is_empty() {
            let total = cover_screws(&it);
            if total == 0 {
                return Output::err("Este item não tem tampa parafusada.");
            }
            let done = self.equip.screws.entry(it.id.clone()).or_insert(0);
            if *done >= total {
                return Output::ok("Todos os parafusos da tampa já foram retirados. Use 'abrir'.");
            }
            *done += 1;
            let d = *done;
            self.tick(1);
            self.rec("desparafusar", &format!("{} tampa {}/{}", it.id, d, total), "parafuso retirado e guardado");
            return Output::ok(format!("Parafuso {}/{} da tampa retirado.{}", d, total, if d == total { " A tampa está solta." } else { "" }));
        }
        let Some(sl) = it.slots.iter().find(|x| x.id.eq_ignore_ascii_case(slot)) else { return Output::err("Slot não encontrado.") };
        if sl.hidden_by.is_some() {
            return Output::err("Esse slot está sob um componente: remova-o antes.");
        }
        if it.kind.is_computer() && !it.flag("aberto") {
            return Output::err("Abra o equipamento antes.");
        }
        let total = slot_screws(&self.case, &it, sl);
        if total == 0 {
            return Output::err("Esse suporte não tem parafusos.");
        }
        let key = format!("{}:{}", it.id, sl.id.to_uppercase());
        let done = self.equip.screws.entry(key.clone()).or_insert(0);
        if *done >= total {
            return Output::ok("O suporte já está solto.");
        }
        *done += 1;
        let d = *done;
        self.tick(1);
        self.rec("desparafusar", &format!("{} {}/{}", key, d, total), "parafuso do suporte retirado");
        Output::ok(format!("Parafuso {}/{} do suporte ({}) retirado.{}", d, total, sl.label, if d == total { " A mídia está solta no suporte." } else { "" }))
    }

    pub(crate) fn cmd_cabo(&mut self, sub: &str, id: &str, slot: &str, which: &str) -> Output {
        if sub != "desconectar" {
            return Output::err("Uso: cabo desconectar <item> <slot> dados|energia");
        }
        let Some(i) = self.item_mut(id) else { return Output::err("Item não encontrado.") };
        let it = self.case.items[i].clone();
        if it.power_on {
            return Output::err("O equipamento está LIGADO.");
        }
        if !it.flag("aberto") {
            return Output::err("Abra o equipamento antes.");
        }
        let Some(sl) = it.slots.iter().find(|x| x.id.eq_ignore_ascii_case(slot)) else { return Output::err("Slot não encontrado.") };
        if !slot_has_cables(&it, sl) {
            return Output::err("Esse slot não tem cabos (encaixe direto).");
        }
        let which = match which {
            "dados" | "energia" => which,
            _ => return Output::err("Indique o cabo: dados | energia"),
        };
        let key = format!("{}:{}:{}", it.id, sl.id.to_uppercase(), which);
        if !self.equip.unplugged.insert(key) {
            return Output::ok("Esse cabo já está desconectado.");
        }
        self.tick(1);
        self.rec("cabo-desconectar", &format!("{} {} {}", it.id, sl.id.to_uppercase(), which), "cabo desconectado da mídia");
        Output::ok(format!("Cabo de {} desconectado do {}.", which, sl.label))
    }

    pub(crate) fn cmd_conectar(&mut self, what: &str, to: &str) -> Output {
        if what.eq_ignore_ascii_case("bloqueador") {
            if !matches!(to, "estacao" | "estação" | "") {
                return Output::err("Uso: conectar bloqueador estacao");
            }
            self.equip.blocker_host = true;
            self.tick(1);
            self.rec("conectar", "bloqueador → estação", "cabo USB do bloqueador conectado à estação");
            return Output::ok("Bloqueador conectado à estação pelo cabo USB.");
        }
        let mid = what.to_uppercase();
        let Some(m) = self.case.media.iter().find(|m| m.id == mid).cloned() else { return Output::err("Mídia não encontrada.") };
        if m.kind == MediaKind::FlashMovel {
            return Output::err("A memória interna do aparelho é extraída pelo cabo de dados ('extrair').");
        }
        let dest = match to {
            "bloqueador" => "bloqueador",
            "estacao" | "estação" => "estacao",
            _ => return Output::err(format!("Uso: conectar {} bloqueador|estacao", mid.to_lowercase())),
        };
        self.equip.linked.insert(mid.clone(), dest.to_string());
        self.tick(1);
        let via = media_connector(&m).map(tool_name).unwrap_or("encaixe direto");
        if dest == "estacao" {
            self.rec("conectar", &format!("{} → estação (sem bloqueador)", mid), via);
            Output::ok(format!("Mídia {} ligada DIRETAMENTE à estação ({}). Atenção: sem bloqueador de escrita.", mid, via))
        } else {
            self.rec("conectar", &format!("{} → bloqueador", mid), via);
            Output::ok(format!("Mídia {} conectada à porta de origem do bloqueador ({}).", mid, via))
        }
    }

    pub(crate) fn cmd_marcar(&mut self, vid: &str) -> Output {
        let up = vid.to_uppercase();
        let Some(v) = self.case.vestigios.iter().find(|v| v.id == up).cloned() else { return Output::err("Vestígio não encontrado (ex.: marcar v-01).") };
        if let Some(n) = self.equip.markers.get(&up) {
            return Output::ok(format!("{} já está sinalizado com a placa nº {}.", up, n));
        }
        let n = self.equip.markers.len() as u32 + 1;
        self.equip.markers.insert(up.clone(), n);
        self.tick(1);
        self.rec("marcar", &format!("{} placa {}", up, n), &v.label);
        Output::ok(format!("Placa nº {} posicionada junto a {} ({}), com escala.", n, up, v.label))
    }

    pub(crate) fn cmd_comunicar(&mut self, vid: &str) -> Output {
        let up = vid.to_uppercase();
        let Some(v) = self.case.vestigios.iter().find(|v| v.id == up).cloned() else { return Output::err("Vestígio não encontrado.") };
        if !self.equip.reported.insert(up.clone()) {
            return Output::ok(format!("{} já foi comunicado.", up));
        }
        self.tick(2);
        let quem = if v.biologico { "equipe de perícia de local / biologia forense" } else { "equipe de perícia de local" };
        self.rec("comunicar-vestigio", &format!("{} — {}", up, v.label), &format!("comunicado à {}; preservado sem coleta pelo perito de informática", quem));
        Output::ok(format!("{} comunicado à {}. O vestígio fica preservado no lugar para a coleta especializada.", up, quem))
    }

    pub(crate) fn cmd_fotografar_vestigio(&mut self, vid: &str) -> Output {
        let up = vid.to_uppercase();
        let Some(v) = self.case.vestigios.iter().find(|v| v.id == up).cloned() else { return Output::err("Vestígio não encontrado.") };
        self.tick(2);
        self.equip.photographed.insert(up.clone());
        let placa = self.equip.markers.get(&up).copied();
        let det = match placa {
            Some(n) => format!("com placa nº {} e escala", n),
            None => "sem placa/escala (sinalize antes para documentar a posição)".to_string(),
        };
        self.rec("fotografar-vestigio", &format!("{} — {}", up, v.label), &det);
        Output::ok(format!("{} fotografado {}.", up, det))
    }

    /// Resumo do que o perito carrega e veste.
    pub fn equip_status(&self) -> String {
        let mut o = String::from("EQUIPAMENTO\n");
        o.push_str(&format!("  Na mão: {}\n", self.equip.hand.map(tool_name).unwrap_or("(livre)")));
        o.push_str(&format!(
            "  Luvas: {}\n",
            match &self.equip.gloves {
                Gloves::Nenhuma => "não".to_string(),
                Gloves::Limpas => "limpas".to_string(),
                Gloves::Contaminadas(x) => format!("usadas em {} (troque antes de tocar outro material)", x),
            }
        ));
        o.push_str(&format!("  Pulseira antiestática: {}\n", if self.equip.strap { "sim" } else { "não" }));
        o.push_str("Maleta: ");
        o.push_str(&TOOLS.iter().map(|t| t.1).collect::<Vec<_>>().join(", "));
        o
    }
}
