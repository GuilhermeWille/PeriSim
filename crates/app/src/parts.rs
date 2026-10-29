//! Partes físicas dos equipamentos: onde fica cada parafuso, tampa, conector, porta, botão e
//! lacre. É a fonte única da geometria de encaixe: o mundo 3D desenha e cria os alvos de mira
//! ("P:<dono>:<parte>") a partir daqui, e as etapas do procedimento apontam para estas partes
//! — para soltar um parafuso é preciso mirar aquele parafuso; para ligar o cabo SATA, o
//! conector do disco e depois a porta SATA do bloqueador.
//!
//! Posições relativas à base do objeto (ponto no tampo onde ele apoia), frente em +z.

use macroquad::prelude::*;
use pericia_core::engine::Sim;
use pericia_core::equip::{cover_screws, slot_has_cables, slot_screws};
use pericia_core::model::*;

/// Dono das partes do bloqueador de escrita e da estação.
pub const BLOQ: &str = "BLOQ";
pub const ESTACAO: &str = "ESTACAO";

/// Que tipo de peça é (para o desenho).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tipo {
    /// Parafuso visto de frente (cabeça no plano XY, eixo em z) ou de lado (eixo em x) ou de cima (y).
    Parafuso(char),
    Tampa,
    Lacre,
    Porta,
    Botao,
    Conector,
    Midia,
    Componente,
    Gaveta,
    Camera,
    Chave,
}

#[derive(Clone, Debug)]
pub struct Parte {
    /// Nome da parte (o mesmo das etapas); várias peças iguais levam sufixo ".n".
    pub nome: String,
    pub c: Vec3,
    /// Meia-caixa de mira (um pouco maior que a peça, para dar para mirar).
    pub half: Vec3,
    pub tipo: Tipo,
}

fn p(nome: impl Into<String>, c: Vec3, half: Vec3, tipo: Tipo) -> Parte {
    Parte { nome: nome.into(), c, half, tipo }
}

/// Id do alvo de mira de uma parte.
pub fn pick_id(dono: &str, nome: &str) -> String {
    format!("P:{}:{}", dono, nome)
}

/// (dono, nome) de um alvo de parte.
pub fn split(pick: &str) -> Option<(&str, &str)> {
    let rest = pick.strip_prefix("P:")?;
    let i = rest.find(':')?;
    Some((&rest[..i], &rest[i + 1..]))
}

/// O alvo mirado é a parte `nome` do `dono` (ou uma das peças iguais "nome.n")?
pub fn hit(pick: &str, dono: &str, nome: &str) -> bool {
    match split(pick) {
        Some((d, n)) => d == dono && mesmo_nome(n, nome),
        None => false,
    }
}

/// `n` é a parte `nome` ou uma das peças iguais numeradas ("nome.0", "nome.1"…).
fn mesmo_nome(n: &str, nome: &str) -> bool {
    n == nome || n.strip_prefix(nome).and_then(|r| r.strip_prefix('.')).map(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit())).unwrap_or(false)
}

/// Alvo "pai" de uma parte (o objeto inteiro), para examinar e para a prioridade de mira.
pub fn parent(pick: &str) -> Option<String> {
    let (d, _) = split(pick)?;
    Some(match d {
        BLOQ => "TOOL-WB".into(),
        ESTACAO => return None,
        _ => match d.strip_prefix("M.") {
            Some(mid) => format!("M:{}", mid),
            None => d.to_string(),
        },
    })
}

/// Nome legível de uma parte.
pub fn nome_legivel(dono: &str, nome: &str) -> String {
    let base = nome.split('.').collect::<Vec<_>>();
    let s = match (dono, base.as_slice()) {
        (BLOQ, ["sata"]) => "porta SATA do bloqueador".to_string(),
        (BLOQ, ["m2"]) => "porta do adaptador M.2 no bloqueador".to_string(),
        (BLOQ, ["sd"]) => "entrada do leitor SD no bloqueador".to_string(),
        (BLOQ, ["usb_in"]) => "entrada USB (origem) do bloqueador".to_string(),
        (BLOQ, ["usb_host"]) => "saída USB do bloqueador (para a estação)".to_string(),
        (BLOQ, ["chave"]) => "chave de proteção contra escrita".to_string(),
        (ESTACAO, _) => "porta USB frontal da estação".to_string(),
        (_, ["porta"]) if dono.starts_with("M.") => format!("conector da mídia {}", &dono[2..]),
        (_, ["parafuso", ..]) => "parafusos da tampa".to_string(),
        (_, ["tampa"]) => "tampa".to_string(),
        (_, ["lacre"]) => "lacre".to_string(),
        (_, ["rede"]) => "porta de rede (cabo)".to_string(),
        (_, ["usb"]) => "porta USB".to_string(),
        (_, ["energia"]) => "botão de energia".to_string(),
        (_, ["porta"]) => "conector de dados/carga".to_string(),
        (_, ["gaveta"]) => "gaveta do chip SIM".to_string(),
        (_, ["camera"]) => "câmera frontal".to_string(),
        (_, ["comp", id]) => format!("componente {}", id),
        (_, [sl, "dados"]) => format!("conector de dados SATA do disco ({})", sl.to_uppercase()),
        (_, [sl, "energia"]) => format!("conector de energia do disco ({})", sl.to_uppercase()),
        (_, [sl, "parafuso", ..]) => format!("parafusos do suporte ({})", sl.to_uppercase()),
        (_, [sl, "midia"]) => format!("mídia no suporte ({})", sl.to_uppercase()),
        _ => nome.to_string(),
    };
    if dono == BLOQ || dono == ESTACAO || dono.starts_with("M.") {
        s
    } else {
        format!("{} de {}", s, dono)
    }
}

fn audited(sim: &Sim, action: &str, prefix: &str) -> bool {
    let p = prefix.to_lowercase();
    sim.audit.entries.iter().any(|e| e.action == action && e.detail.to_lowercase().starts_with(&p))
}

/// Notebook desligado e já inspecionado fica fechado e de cabeça para baixo (é por baixo que
/// ficam os parafusos e a tampa de acesso às mídias).
pub fn notebook_virado(sim: &Sim, it: &Item) -> bool {
    if it.kind != ItemKind::Notebook {
        return false;
    }
    if it.flag("aberto") || sim.equip.screws.get(&it.id).copied().unwrap_or(0) > 0 {
        return true;
    }
    let lacre_ok = it.seal.as_ref().map(|s| !s.intact || it.flag("deslacrado")).unwrap_or(true);
    !it.power_on && lacre_ok && audited(sim, "inspecionar", &it.id)
}

/// Tamanho do gabinete torre (largura, altura, profundidade).
pub const TORRE: (f32, f32, f32) = (0.21, 0.45, 0.46);

/// Mídia visível num slot do equipamento aberto: (slot, centro relativo à base, tipo).
pub fn slots_visiveis<'a>(sim: &Sim, it: &'a Item) -> Vec<(&'a Slot, Vec3, MediaKind)> {
    let mut v = vec![];
    if !it.flag("aberto") {
        return v;
    }
    let (_, h, d) = TORRE;
    let mut k_disco = 0.0;
    let mut k_nb = 0.0;
    for sl in &it.slots {
        let Some(mid) = &sl.media_id else { continue };
        if sl.hidden_by.is_some() || sim.equip.removed.contains(mid) {
            continue;
        }
        let kind = sim.case.media_by_id(mid).map(|m| m.kind).unwrap_or(MediaKind::Ssd);
        let pos = if it.kind == ItemKind::Notebook {
            let q = vec3(-0.08 + k_nb, 0.0245, 0.02);
            k_nb += 0.15;
            q
        } else if kind == MediaKind::Nvme || sl.iface.eq_ignore_ascii_case("NVMe") {
            // placa M.2 de pé, junto à placa-mãe (lado direito)
            if sl.label.contains("oculto") { vec3(0.075, h / 2.0 + 0.04, -0.08) } else { vec3(0.075, h / 2.0 - 0.01, 0.02) }
        } else {
            let q = vec3(0.0, 0.05 + k_disco, d / 2.0 - 0.1);
            k_disco += 0.04;
            q
        };
        v.push((sl, pos, kind));
    }
    v
}

/// Partes de um item da cena, no estado atual.
pub fn item_parts(sim: &Sim, it: &Item) -> Vec<Parte> {
    let mut v = vec![];
    let eq = &sim.equip;
    match it.kind {
        ItemKind::Desktop | ItemKind::Servidor => {
            let (w, h, d) = TORRE;
            let aberto = it.flag("aberto");
            v.push(p("energia", vec3(-0.05, h - 0.035, d / 2.0 + 0.006), vec3(0.014, 0.014, 0.008), Tipo::Botao));
            v.push(p("usb", vec3(0.01, h - 0.035, d / 2.0 + 0.005), vec3(0.02, 0.011, 0.008), Tipo::Porta));
            v.push(p("rede", vec3(0.06, 0.30, -d / 2.0 - 0.004), vec3(0.013, 0.012, 0.008), Tipo::Porta));
            if it.seal.is_some() {
                v.push(p("lacre", vec3(-w / 2.0 - 0.003, h / 2.0, 0.0), vec3(0.006, h * 0.45, 0.018), Tipo::Lacre));
            }
            if !aberto {
                // parafusos de dedo na traseira, prendendo a tampa lateral (lado -x)
                let total = cover_screws(it);
                let fora = eq.screws.get(&it.id).copied().unwrap_or(0).min(total);
                for k in fora..total {
                    let y = 0.07 + (h - 0.14) * (k as f32 / (total.max(2) - 1) as f32);
                    v.push(p(format!("parafuso.{}", k), vec3(-w / 2.0 + 0.012, y, -d / 2.0 - 0.006), vec3(0.012, 0.012, 0.01), Tipo::Parafuso('z')));
                }
                v.push(p("tampa", vec3(-w / 2.0 - 0.002, h / 2.0 + 0.1, -0.1), vec3(0.008, 0.08, 0.08), Tipo::Tampa));
            } else {
                for c in &it.components {
                    if c.removed || c.kind == "tampa" {
                        continue;
                    }
                    let (c0, hf) = match c.kind.as_str() {
                        "gpu" => (vec3(w / 2.0 - 0.06, h / 2.0 - 0.06, -0.03), vec3(0.042, 0.016, 0.13)),
                        "dissipador" => (vec3(w / 2.0 - 0.06, h / 2.0 + 0.10, -0.08), vec3(0.037, 0.047, 0.047)),
                        _ => (vec3(w / 2.0 - 0.05, h / 2.0, 0.08), vec3(0.03, 0.03, 0.03)),
                    };
                    v.push(p(format!("comp.{}", c.id.to_lowercase()), c0, hf, Tipo::Componente));
                }
                for (sl, pos, kind) in slots_visiveis(sim, it) {
                    let s = sl.id.to_lowercase();
                    let key = format!("{}:{}", it.id, sl.id.to_uppercase());
                    if kind == MediaKind::Nvme || sl.iface.eq_ignore_ascii_case("NVMe") {
                        v.push(p(format!("{}.midia", s), pos, vec3(0.008, 0.016, 0.04), Tipo::Midia));
                        let st = slot_screws(&sim.case, it, sl);
                        let sd = eq.screws.get(&key).copied().unwrap_or(0).min(st);
                        for k in sd..st {
                            v.push(p(format!("{}.parafuso.{}", s, k), pos + vec3(-0.004, 0.0, 0.046), vec3(0.008, 0.009, 0.009), Tipo::Parafuso('x')));
                        }
                        continue;
                    }
                    v.push(p(format!("{}.midia", s), pos + vec3(0.0, 0.0, 0.01), vec3(0.04, 0.012, 0.05), Tipo::Midia));
                    if slot_has_cables(it, sl) {
                        for (c, x) in [("dados", -0.022f32), ("energia", 0.02)] {
                            if !eq.unplugged.contains(&format!("{}:{}", key, c)) {
                                v.push(p(format!("{}.{}", s, c), pos + vec3(x, 0.0, -0.08), vec3(0.014, 0.01, 0.01), Tipo::Conector));
                            }
                        }
                    }
                    let st = slot_screws(&sim.case, it, sl);
                    let sd = eq.screws.get(&key).copied().unwrap_or(0).min(st);
                    for k in sd..st {
                        // na lateral do disco virada para a tampa aberta (-x), ao longo do comprimento
                        let z = -0.055 + 0.04 * (k as f32 % 4.0);
                        v.push(p(format!("{}.parafuso.{}", s, k), pos + vec3(-0.054, 0.0, z), vec3(0.008, 0.009, 0.009), Tipo::Parafuso('x')));
                    }
                }
            }
        }
        ItemKind::Notebook => {
            let virado = notebook_virado(sim, it);
            v.push(p("usb", vec3(-0.172, 0.009, 0.03), vec3(0.008, 0.009, 0.014), Tipo::Porta));
            v.push(p("rede", vec3(-0.172, 0.009, -0.04), vec3(0.008, 0.011, 0.014), Tipo::Porta));
            if !virado {
                v.push(p("energia", vec3(0.14, 0.0225, -0.095), vec3(0.012, 0.006, 0.01), Tipo::Botao));
            }
            if it.seal.is_some() {
                v.push(p("lacre", vec3(0.172, 0.011, 0.0), vec3(0.006, 0.013, 0.1), Tipo::Lacre));
            }
            if virado && !it.flag("aberto") {
                let total = cover_screws(it);
                let fora = eq.screws.get(&it.id).copied().unwrap_or(0).min(total);
                let cantos = [vec3(-0.15, 0.0, -0.1), vec3(0.15, 0.0, -0.1), vec3(-0.15, 0.0, 0.1), vec3(0.15, 0.0, 0.1)];
                for k in fora..total {
                    let c = cantos[k as usize % 4] + vec3(0.0, 0.0255, 0.0);
                    v.push(p(format!("parafuso.{}", k), c, vec3(0.01, 0.006, 0.01), Tipo::Parafuso('y')));
                }
                v.push(p("tampa", vec3(0.0, 0.026, 0.092), vec3(0.05, 0.005, 0.012), Tipo::Tampa));
            }
            for (sl, pos, kind) in slots_visiveis(sim, it) {
                let s = sl.id.to_lowercase();
                let key = format!("{}:{}", it.id, sl.id.to_uppercase());
                let (half, parafusos): (Vec3, Vec<Vec3>) = if kind == MediaKind::Nvme {
                    (vec3(0.042, 0.005, 0.013), vec![vec3(0.043, 0.0, 0.0)])
                } else {
                    (vec3(0.037, 0.006, 0.052), vec![vec3(-0.03, 0.0, -0.058), vec3(0.03, 0.0, -0.058)])
                };
                v.push(p(format!("{}.midia", s), pos + vec3(0.0, half.y, 0.0), half, Tipo::Midia));
                let st = slot_screws(&sim.case, it, sl);
                let sd = eq.screws.get(&key).copied().unwrap_or(0).min(st);
                for k in sd..st {
                    let o = parafusos[k as usize % parafusos.len()];
                    v.push(p(format!("{}.parafuso.{}", s, k), pos + o + vec3(0.0, 0.004, 0.0), vec3(0.008, 0.006, 0.008), Tipo::Parafuso('y')));
                }
            }
        }
        ItemKind::Celular | ItemKind::Tablet => {
            let tablet = it.kind == ItemKind::Tablet;
            let (w, d) = if tablet { (0.17, 0.25) } else { (0.072, 0.152) };
            v.push(p("porta", vec3(0.0, 0.0045, d / 2.0 + 0.002), vec3(0.012, 0.007, 0.01), Tipo::Porta));
            if it.phone.as_ref().map(|ph| !ph.sims.is_empty()).unwrap_or(false) {
                v.push(p("gaveta", vec3(-w / 2.0 - 0.001, 0.0045, d * 0.18), vec3(0.008, 0.007, 0.014), Tipo::Gaveta));
            }
            if sim.case.module == ModuleId::M505 {
                v.push(p("camera", vec3(0.0, 0.0089, -d * 0.44), vec3(0.011, 0.005, 0.011), Tipo::Camera));
            }
            if it.seal.is_some() {
                v.push(p("lacre", vec3(0.0, 0.0345, 0.0), vec3(w * 0.85, 0.005, 0.01), Tipo::Lacre));
            }
        }
        // discos avulsos ficam com o conector virado para o perito (+z)
        ItemKind::Hdd => {
            v.push(p("porta", vec3(0.0, 0.011, 0.078), vec3(0.04, 0.011, 0.01), Tipo::Porta));
            if it.seal.is_some() {
                v.push(p("lacre", vec3(0.0, 0.0272, 0.02), vec3(0.052, 0.004, 0.012), Tipo::Lacre));
            }
        }
        ItemKind::Ssd => v.push(p("porta", vec3(0.0, 0.0035, 0.055), vec3(0.03, 0.007, 0.008), Tipo::Porta)),
        ItemKind::Nvme => v.push(p("porta", vec3(-0.04, 0.002, 0.0), vec3(0.007, 0.006, 0.012), Tipo::Porta)),
        ItemKind::Pendrive => v.push(p("porta", vec3(0.037, 0.006, 0.0), vec3(0.012, 0.008, 0.01), Tipo::Porta)),
        ItemKind::CartaoSd => v.push(p("porta", vec3(0.0, 0.0012, -0.006), vec3(0.009, 0.005, 0.006), Tipo::Porta)),
    }
    v
}

/// Partes de uma mídia retirada (desenhada em `at` na mesa, conector virado para o perito, +z).
pub fn media_parts(kind: Option<MediaKind>) -> Vec<Parte> {
    vec![match kind {
        Some(MediaKind::Hdd) => p("porta", vec3(0.0, 0.011, 0.078), vec3(0.04, 0.011, 0.01), Tipo::Porta),
        Some(MediaKind::Nvme) => p("porta", vec3(-0.04, 0.002, 0.0), vec3(0.008, 0.007, 0.013), Tipo::Porta),
        _ => p("porta", vec3(0.0, 0.0035, 0.055), vec3(0.03, 0.007, 0.008), Tipo::Porta),
    }]
}

/// Porta do bloqueador onde cada tipo de mídia entra.
pub fn porta_bloqueador(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Hdd | MediaKind::Ssd => "sata",
        MediaKind::Nvme => "m2",
        MediaKind::Sd => "sd",
        MediaKind::Usb | MediaKind::FlashMovel => "usb_in",
    }
}

/// Partes do bloqueador (desenhado girado: portas de origem e destino viradas para o perito, +z).
pub fn blocker_parts(on: bool) -> Vec<Parte> {
    let _ = on;
    vec![
        p("sata", vec3(-0.07, 0.032, 0.076), vec3(0.02, 0.012, 0.01), Tipo::Porta),
        p("m2", vec3(-0.02, 0.032, 0.076), vec3(0.02, 0.012, 0.01), Tipo::Porta),
        p("usb_host", vec3(0.04, 0.032, 0.076), vec3(0.014, 0.012, 0.01), Tipo::Porta),
        p("sd", vec3(0.113, 0.03, 0.02), vec3(0.008, 0.01, 0.018), Tipo::Porta),
        p("usb_in", vec3(0.113, 0.03, -0.03), vec3(0.008, 0.011, 0.014), Tipo::Porta),
        p("chave", vec3(0.06, 0.075, 0.03), vec3(0.014, 0.008, 0.012), Tipo::Chave),
    ]
}

/// Estação pericial (um notebook de perícia): (base do notebook, posição do bloqueador).
/// No laboratório fica numa mesa encostada na ponta da bancada de exame; no local de busca,
/// na mesa de apoio do perito — em ambos, a um palmo do bloqueador.
pub fn station_layout(local: bool) -> (Vec3, Vec3) {
    if local {
        (vec3(2.98, 0.75, -2.68), vec3(2.32, 0.75, -2.55))
    } else {
        (vec3(-1.98, 0.905, -0.58), vec3(-1.52, 0.905, -0.32))
    }
}

/// Porta USB lateral do notebook da estação, do lado virado para o bloqueador.
pub fn station_usb(local: bool) -> Vec3 {
    let (nb, wb) = station_layout(local);
    let lado = if wb.x >= nb.x { 1.0 } else { -1.0 };
    nb + vec3(lado * 0.172, 0.009, 0.03)
}

/// O item tem essa parte agora? (Senão a etapa fica no objeto inteiro.)
pub fn tem(sim: &Sim, it: &Item, nome: &str) -> bool {
    item_parts(sim, it).iter().any(|x| mesmo_nome(&x.nome, nome))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_de_parte() {
        let id = pick_id("ITEM-01", "s1.parafuso.2");
        assert_eq!(split(&id), Some(("ITEM-01", "s1.parafuso.2")));
        assert!(hit(&id, "ITEM-01", "s1.parafuso"));
        assert!(!hit(&id, "ITEM-01", "s1"));
        assert!(!hit(&id, "ITEM-02", "s1.parafuso"));
        assert!(hit("P:BLOQ:sata", BLOQ, "sata"));
        assert_eq!(parent("P:M.HD-01:porta").as_deref(), Some("M:HD-01"));
        assert_eq!(parent("P:BLOQ:sata").as_deref(), Some("TOOL-WB"));
        assert_eq!(parent("P:ESTACAO:usb"), None);
    }
}

#[cfg(test)]
mod dump {
    #[test]
    #[ignore]
    fn listar() {
        use pericia_core::casegen::generate;
        use pericia_core::model::ModuleId::*;
        for m in [M501, M502, M503, M504, M505] {
            for d in 1..=3u8 {
                let s = pericia_core::engine::Sim::new(generate(m, d, 7));
                for it in &s.case.items {
                    println!("{:?} D{} {:?} {} {:?} slots={:?} live={} on={} seal={}", m, d, s.case.setting, it.id, it.kind, it.slots.iter().map(|x| (&x.id, &x.iface, &x.media_id)).collect::<Vec<_>>(), it.live.is_some(), it.power_on, it.seal.is_some());
                }
            }
        }
    }
}
