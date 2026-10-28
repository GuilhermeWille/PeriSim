//! Visões do tablet do perito: só leitura, sem efeito no estado e sem revelar
//! o que ainda não foi descoberto (itens ocultos, slots sob componentes,
//! observações de inspeção não realizada, dados internos da geração).

use crate::assess;
use crate::engine::Sim;
use crate::model::{Item, SimWhere};
use crate::util::{fmt_dt, BRT};

/// Uma seção da ficha do objeto: título e linhas já formatadas.
#[derive(Clone, Debug, PartialEq)]
pub struct Secao {
    pub titulo: String,
    pub linhas: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Missao {
    pub titulo: String,
    pub objetivo: String,
    pub historico: String,
    pub quesitos: Vec<String>,
    pub briefing: Vec<String>,
    pub autorizacoes: Vec<String>,
    /// Tarefas pertinentes com o estado; vazio quando a dificuldade não as mostra.
    pub tarefas: Vec<(bool, String)>,
    /// (cumpridas, total) — sempre disponível, exceto no nível sem orientação.
    pub contagem: Option<(usize, usize)>,
    pub horario: String,
}

/// O item está visível para o perito (não oculto, ou já descoberto)?
pub fn visivel(it: &Item) -> bool {
    !it.hidden || it.flag("descoberto")
}

/// Itens disponíveis ao perito, na ordem do caso: (id, rótulo).
pub fn itens_visiveis(s: &Sim) -> Vec<(String, String)> {
    s.case.items.iter().filter(|i| visivel(i)).map(|i| (i.id.clone(), i.label.clone())).collect()
}

pub fn missao(s: &Sim) -> Missao {
    let c = &s.case;
    let cl = assess::checklist(s);
    let feitas = cl.iter().filter(|x| x.0).count();
    let (tarefas, contagem) = match c.difficulty {
        1 | 2 => (cl.clone(), Some((feitas, cl.len()))),
        3 => (vec![], Some((feitas, cl.len()))),
        _ => (vec![], None),
    };
    let mut autorizacoes = vec![];
    if c.legal.mandado {
        autorizacoes.push("Mandado judicial vigente".to_string());
    }
    autorizacoes.push(if c.legal.autorizacao_dados { "Acesso a dados: autorizado" } else { "Acesso a dados: sem autorização específica" }.to_string());
    if c.net.as_ref().map(|n| n.cloud.is_some()).unwrap_or(false) {
        autorizacoes.push(if c.legal.autorizacao_nuvem { "Dados em nuvem: autorizado" } else { "Dados em nuvem: NÃO autorizado" }.to_string());
    }
    Missao {
        titulo: format!("{} — {} (nível {})", c.id, c.title, c.difficulty),
        objetivo: c.objetivo.clone(),
        historico: c.historico.clone(),
        quesitos: c.quesitos.clone(),
        briefing: c.briefing.clone(),
        autorizacoes,
        tarefas,
        contagem,
        horario: fmt_dt(s.clock, BRT),
    }
}

fn feito(s: &Sim, acao: &str, id: &str) -> bool {
    s.audit.entries.iter().any(|e| e.action == acao && e.detail.to_uppercase().contains(&id.to_uppercase()))
}

/// Ficha do objeto de análise sem spoilers. `None` se o item não existir ou
/// ainda não tiver sido descoberto.
pub fn objeto(s: &Sim, id: &str) -> Option<Vec<Secao>> {
    let it = s.case.items.iter().find(|i| i.id.eq_ignore_ascii_case(id) && visivel(i))?;
    let mut out = vec![];

    let mut ident = vec![
        format!("{} — {}", it.id, it.label),
        format!("{}: {} {}", it.kind.label(), it.brand, it.model),
        format!("Nº de série: {}", it.serial),
    ];
    if let Some(o) = &it.owner {
        ident.push(format!("Atribuído a: {}", o));
    }
    out.push(Secao { titulo: "Identificação".into(), linhas: ident });

    // Preservação e cadeia de custódia
    let mut pres = vec![];
    match (&it.seal, &it.doc_seal) {
        (Some(sl), doc) => {
            let conf = if it.flag("divergencia_lacre") {
                " — DIVERGE da documentação".to_string()
            } else if feito(s, "lacre-conferir", &it.id) {
                " — conferido com a documentação".to_string()
            } else if doc.is_some() {
                " — ainda não conferido".to_string()
            } else {
                String::new()
            };
            pres.push(format!("Lacre {}: {}{}", sl.number, if sl.intact { "íntegro" } else { "rompido" }, conf));
        }
        (None, Some(d)) => pres.push(format!("Documentação cita o lacre {}; nenhum lacre físico presente", d)),
        _ => {}
    }
    for (flag, texto) in [
        ("fotografado", "Fotografado (fixação do estado)"),
        ("deslacrado", "Deslacrado pelo perito"),
        ("lacrado_perito", "Novo lacre aposto pelo perito"),
        ("embalado", "Embalado"),
        ("etiqueta", "Etiquetado"),
        ("etiqueta_camera", "Etiqueta fotografada"),
    ] {
        if it.flag(flag) {
            pres.push(texto.to_string());
        }
    }
    if pres.is_empty() {
        pres.push("Nenhum registro de preservação ainda.".into());
    }
    out.push(Secao { titulo: "Preservação".into(), linhas: pres });

    // Condição física (o que se vê ao olhar o objeto)
    let mut fis = vec![];
    if it.kind.is_computer() {
        fis.push(format!("Energia: {}", if it.power_on { "LIGADO" } else { "desligado" }));
    }
    if !it.physical.traces.is_empty() {
        fis.push(format!("Vestígios: {}", it.physical.traces.join("; ")));
    }
    if !it.physical.damage.is_empty() {
        fis.push(format!("Danos: {}", it.physical.damage.join("; ")));
    }
    if it.physical.wet {
        fis.push("Molhado".into());
    }
    if it.physical.battery_swollen {
        fis.push("Bateria estufada".into());
    }
    // a camuflagem só é conhecida depois da inspeção que a revelou
    if let Some(cf) = &it.physical.camouflaged {
        if (it.hidden && it.flag("descoberto")) || feito(s, "inspecionar", &it.id) {
            fis.push(format!("Observado na inspeção: {}", cf));
        }
    }
    if it.flag("aberto") {
        fis.push("Gabinete/tampa aberto pelo perito".into());
    }
    let removidos: Vec<&str> = it.components.iter().filter(|c| c.removed).map(|c| c.label.as_str()).collect();
    if !removidos.is_empty() {
        fis.push(format!("Componentes removidos: {}", removidos.join(", ")));
    }
    if !fis.is_empty() {
        out.push(Secao { titulo: "Condição física".into(), linhas: fis });
    }

    // Mídias acessíveis (slots ocultos não aparecem)
    let mut mid = vec![];
    for m in &it.media {
        let oculta = it.slots.iter().any(|sl| sl.media_id.as_deref() == Some(m.as_str()) && sl.hidden_by.is_some());
        if oculta {
            continue;
        }
        let Some(md) = s.case.media.iter().find(|x| &x.id == m) else { continue };
        let mut st = vec![];
        if s.hashes.contains_key(&format!("wb:{}", m)) {
            st.push("bloqueio de escrita");
        }
        if s.hashes.contains_key(&format!("fdeclone:{}", m)) {
            st.push("espelhada");
        } else if s.imaged.contains(m) {
            st.push("imagem adquirida");
        }
        if s.hashes.contains_key(&format!("hash:{}", m)) {
            st.push("hash verificado");
        }
        let st = if st.is_empty() { "sem aquisição".to_string() } else { st.join(", ") };
        mid.push(format!("{} — {} {} ({})", md.id, md.kind.label(), md.brand, st));
    }
    for sl in it.slots.iter().filter(|x| x.hidden_by.is_none() && x.media_id.is_none()) {
        mid.push(format!("{} ({}): vazio", sl.label, sl.iface));
    }
    if !mid.is_empty() {
        out.push(Secao { titulo: "Mídias".into(), linhas: mid });
    }

    if let Some(ph) = &it.phone {
        let mut t = vec![
            format!("Sistema: {} {}", ph.os, ph.os_version),
            format!("Estado: {}", ph.boot.label()),
            format!("Bateria: {:.0}%{}", ph.battery, if ph.charging { " (carregando)" } else { "" }),
            format!("Isolamento de RF: {}{}", ph.rf.label(), if it.flag("modo_aviao") { "; modo avião ativo" } else { "" }),
        ];
        if it.flag("imei_conferido") {
            t.push(format!("IMEI conferido: {}{}", ph.imeis.join(" / "), if it.flag("divergencia_imei") { " — DIVERGÊNCIA físico × lógico" } else { "" }));
        }
        for sim in &ph.sims {
            match sim.place {
                SimWhere::Removido => t.push(format!("SIM do slot {} removido: {} — ICCID {}", sim.slot, sim.operadora, sim.iccid)),
                SimWhere::Slot => t.push(format!("Slot {}: ocupado (ICCID legível após a remoção)", sim.slot)),
                SimWhere::AfixadoVerso => {}
            }
        }
        if ph.alarms_disabled {
            t.push("Alarmes desabilitados pelo perito".into());
        }
        if it.flag("tela_fotografada") {
            t.push("Tela fotografada".into());
        }
        out.push(Secao { titulo: "Telefonia".into(), linhas: t });
    }

    if let Some(lv) = &it.live {
        let mut v = vec![format!("Tela: {}", lv.screen), format!("Usuário na sessão: {}", lv.logged_user)];
        v.push(format!(
            "Rede: {}",
            if it.flag("rede_isolada") || !lv.network_connected { "isolada" } else { "conectada" }
        ));
        if lv.memory_dump.is_some() {
            v.push("Memória volátil coletada".into());
        }
        out.push(Secao { titulo: "Estado volátil (equipamento ligado)".into(), linhas: v });
    }

    // Produtos do exame derivados deste objeto
    let ids: Vec<String> = std::iter::once(it.id.clone()).chain(it.media.iter().cloned()).collect();
    let prods: Vec<String> = s
        .artifacts
        .iter()
        .filter(|a| ids.iter().any(|x| a.origin.to_uppercase().contains(&x.to_uppercase())))
        .map(|a| format!("{} {} — SHA-256 {}…", a.id, a.kind, &a.hash[..a.hash.len().min(12)]))
        .collect();
    if !prods.is_empty() {
        out.push(Secao { titulo: "Produtos do exame".into(), linhas: prods });
    }

    let vs: Vec<String> = s
        .case
        .vestigios
        .iter()
        .filter(|v| v.on_item.as_deref() == Some(it.id.as_str()))
        .map(|v| {
            let mut st = vec![];
            if let Some(n) = s.equip.markers.get(&v.id) {
                st.push(format!("placa nº {}", n));
            }
            if s.equip.photographed.contains(&v.id) {
                st.push("fotografado".to_string());
            }
            if s.equip.reported.contains(&v.id) {
                st.push("comunicado".to_string());
            }
            format!("{} — {}{}", v.id, v.desc, if st.is_empty() { String::new() } else { format!(" ({})", st.join(", ")) })
        })
        .collect();
    if !vs.is_empty() {
        out.push(Secao { titulo: "Vestígios não digitais".into(), linhas: vs });
    }
    let mut cuid = cuidados(it);
    if s.strict && it.physical.biohazard {
        cuid.insert(0, "Vestígio biológico: luvas obrigatórias; troque-as antes de tocar outro material. Não limpe nem colete.".into());
    }
    if !cuid.is_empty() {
        out.push(Secao { titulo: "Cuidados de manuseio".into(), linhas: cuid });
    }
    Some(out)
}

/// Orientações de manuseio que decorrem apenas do estado visível do objeto.
fn cuidados(it: &Item) -> Vec<String> {
    let mut v = vec![];
    if it.physical.wet {
        v.push("Risco elétrico: não energizar; secar e isolar, sinalizando a condição.".into());
    }
    if it.physical.battery_swollen {
        v.push("Bateria estufada: não pressionar nem carregar; manuseio mínimo.".into());
    }
    if it.seal.as_ref().map(|x| x.intact).unwrap_or(false) && !it.flag("fotografado") {
        v.push("Fotografe antes de deslacrar ou intervir.".into());
    }
    if it.kind.is_media() || !it.media.is_empty() {
        v.push("Mídia original: conecte só com bloqueador de escrita; examine a cópia.".into());
    }
    if let Some(ph) = &it.phone {
        if !matches!(ph.rf, crate::model::Rf::FaradayBag | crate::model::Rf::Aluminio3 | crate::model::Rf::SalaFaraday) && !it.flag("modo_aviao") {
            v.push("Celular sem isolamento de RF: risco de apagamento remoto.".into());
        }
        if ph.battery < 20.0 && !ph.charging {
            v.push("Bateria baixa: mantenha a alimentação para preservar o estado.".into());
        }
    }
    if let Some(lv) = &it.live {
        if lv.network_connected && !it.flag("rede_isolada") {
            v.push("Equipamento ligado e conectado: avalie o isolamento de rede antes de qualquer ação.".into());
        }
        v.push("Dados voláteis se perdem ao desligar: decida a coleta antes.".into());
    }
    v
}

#[cfg(test)]
mod t {
    use super::*;
    use crate::casegen;
    use crate::model::ModuleId;

    #[test]
    fn nao_revela_ocultos() {
        let (mut ocultos, mut camuf) = (0, 0);
        for m in ModuleId::ALL {
        for seed in 1..40 {
            for d in 1..=4 {
                let case = casegen::generate(m, d, seed);
                let s = Sim::new(case);
                for it in &s.case.items {
                    let v = objeto(&s, &it.id);
                    camuf += it.physical.camouflaged.is_some() as u32;
                    ocultos += it.slots.iter().filter(|x| x.hidden_by.is_some()).count() as u32;
                    if it.hidden {
                        assert!(v.is_none(), "item oculto exposto");
                        continue;
                    }
                    let txt = format!("{:?}", v.unwrap());
                    assert!(!txt.contains("OCULTO") && !txt.contains("oculto"), "{}", txt);
                    for sl in it.slots.iter().filter(|x| x.hidden_by.is_some()) {
                        if let Some(m) = &sl.media_id {
                            assert!(!txt.contains(m.as_str()), "mídia oculta {} exposta", m);
                        }
                    }
                    if let Some(cf) = &it.physical.camouflaged {
                        assert!(!txt.contains(cf.as_str()));
                    }
                    assert!(!txt.contains(&s.case.keyword) || s.case.objetivo.contains(&s.case.keyword));
                }
            }
        }
        }
        assert!(ocultos > 0 && camuf > 0, "amostra sem casos de ocultação");
    }

    #[test]
    fn inspecao_revela_so_depois() {
        let mut s = Sim::new(casegen::generate(ModuleId::M503, 4, 1));
        let it = s.case.items.iter().find(|i| i.slots.iter().any(|x| x.hidden_by.is_some())).unwrap().clone();
        let cf = it.physical.camouflaged.clone().unwrap();
        let antes = format!("{:?}", objeto(&s, &it.id).unwrap());
        assert!(!antes.contains(&cf));
        s.exec("pegar lanterna");
        s.exec(&format!("inspecionar {}", it.id.to_lowercase()));
        let depois = format!("{:?}", objeto(&s, &it.id).unwrap());
        assert!(depois.contains(&cf), "{}", depois);
    }

    #[test]
    fn missao_respeita_dificuldade() {
        let s1 = Sim::new(casegen::generate(ModuleId::M502, 1, 3));
        assert!(!missao(&s1).tarefas.is_empty());
        let s3 = Sim::new(casegen::generate(ModuleId::M502, 3, 3));
        let m3 = missao(&s3);
        assert!(m3.tarefas.is_empty() && m3.contagem.is_some());
        let s4 = Sim::new(casegen::generate(ModuleId::M502, 4, 3));
        let m4 = missao(&s4);
        assert!(m4.tarefas.is_empty() && m4.contagem.is_none());
    }

    #[test]
    fn leitura_nao_altera_estado() {
        let mut s = Sim::new(casegen::generate(ModuleId::M501, 2, 7));
        s.exec("fotografar M-01");
        let antes = (s.audit.entries.len(), s.clock, s.faults.len(), s.audit.entries.last().map(|e| e.hash.clone()));
        for it in s.case.items.clone() {
            let _ = objeto(&s, &it.id);
        }
        let _ = missao(&s);
        let depois = (s.audit.entries.len(), s.clock, s.faults.len(), s.audit.entries.last().map(|e| e.hash.clone()));
        assert_eq!(antes, depois);
    }
}
