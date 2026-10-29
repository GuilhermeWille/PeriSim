//! Etapas físicas de cada objeto, na ordem do procedimento, para a interação na cena 3D.
//!
//! Em vez de botões, o perito mira o objeto com a ferramenta certa na mão e aperta E: o
//! jogo procura a próxima etapa que aquela ferramenta faz naquele alvo. Etapas obrigatórias
//! anteriores ainda pendentes impedem pular a ordem; as opcionais (marcadas) não travam.
//! Cada etapa vira um comando do motor — auditado e avaliado como antes.

use pericia_core::engine::Sim;
use pericia_core::equip::{cover_screws, media_free, requirements, slot_has_cables, slot_screws, Tool};
use pericia_core::model::*;

/// Onde a etapa acontece (o que o perito mira).
#[derive(Clone, Debug, PartialEq)]
pub enum Alvo {
    Objeto(String),
    Midia(String),
    Bloqueador,
    Estacao,
    Vestigio(String),
    /// Peça física (dono, parte): parafuso, tampa, conector, porta… — ver `parts`.
    Parte(String, String),
}

#[derive(Clone, Debug)]
pub struct Step {
    pub label: String,
    pub cmd: String,
    pub done: bool,
    pub optional: bool,
    /// Ferramenta exigida na mão (modo passo a passo); `None` = não exige.
    pub tool: Option<Tool>,
    /// Um alvo = ação direta; dois = cabo conectado ponta a ponta (em qualquer ordem).
    pub alvos: Vec<Alvo>,
}

fn audited(sim: &Sim, action: &str, prefix: &str) -> bool {
    let p = prefix.to_lowercase();
    sim.audit.entries.iter().any(|e| e.action == action && e.detail.to_lowercase().starts_with(&p))
}

fn tool_for(sim: &Sim, cmd: &str) -> Option<Tool> {
    if !sim.strict {
        return None;
    }
    let toks: Vec<String> = cmd.split_whitespace().map(|x| x.to_string()).collect();
    requirements(sim, &toks).and_then(|n| n.tool)
}

struct B<'a> {
    sim: &'a Sim,
    v: Vec<Step>,
}

impl B<'_> {
    fn add(&mut self, label: String, cmd: String, done: bool, optional: bool, alvos: Vec<Alvo>) {
        let tool = tool_for(self.sim, &cmd);
        self.v.push(Step { label, cmd, done, optional, tool, alvos });
    }
}

/// Lista completa (feitas e pendentes) das etapas de um objeto, na ordem do procedimento.
pub fn item_steps(sim: &Sim, it: &Item) -> Vec<Step> {
    let id = it.id.to_lowercase();
    let up = it.id.clone();
    let me = || vec![Alvo::Objeto(up.clone())];
    // a etapa acontece numa peça, quando ela existe no modelo; senão, no objeto inteiro
    let pt = |nome: &str| if crate::parts::tem(sim, it, nome) { vec![Alvo::Parte(up.clone(), nome.to_string())] } else { vec![Alvo::Objeto(up.clone())] };
    let eq = &sim.equip;
    let mut b = B { sim, v: vec![] };
    b.add("Fotografar".into(), format!("fotografar {}", id), audited(sim, "fotografar", &up), false, me());
    // celular: isolar das redes logo depois da fotografia (apagamento remoto não espera)
    if let Some(ph) = &it.phone {
        b.add("Isolar (Faraday bag ou alumínio)".into(), format!("isolar {} faraday", id), ph.rf.isolates(), false, me());
        // e manter energizado (bateria baixa pode desligar e levar o aparelho a BFU)
        b.add("Ligar à bateria externa".into(), format!("carregar {}", id), ph.charging, false, pt("porta"));
    }
    if it.seal.is_some() || it.doc_seal.is_some() {
        b.add("Conferir lacre × ofício".into(), format!("lacre conferir {}", id), audited(sim, "lacre-conferir", &up), false, pt("lacre"));
        if let Some(s) = &it.seal {
            b.add("Romper o lacre".into(), format!("deslacrar {}", id), !s.intact || it.flag("deslacrado"), false, pt("lacre"));
        }
    }
    b.add("Inspecionar (lanterna)".into(), format!("inspecionar {}", id), audited(sim, "inspecionar", &id), false, me());
    if it.kind.is_computer() {
        if let Some(live) = &it.live {
            // estado volátil: só enquanto ligado; desligado sem coletar fica como não realizado
            b.add("Isolar da rede (cabo)".into(), format!("rede isolar {}", id), !live.network_connected || !it.power_on, false, pt("rede"));
            b.add("Coletar memória (pendrive de coleta)".into(), format!("memoria {}", id), live.memory_dump.is_some() || !it.power_on, false, pt("usb"));
            if live.mounted_encrypted.is_some() {
                b.add("Copiar volume montado".into(), format!("volume copiar {}", id), !it.power_on, true, pt("usb"));
            }
        }
        if it.power_on || it.live.is_some() {
            b.add("Desligar (energia)".into(), format!("energia desligar {}", id), !it.power_on, false, pt("energia"));
        }
        let total = cover_screws(it);
        let aberto = it.flag("aberto");
        if total > 0 {
            let done = (*eq.screws.get(&it.id).unwrap_or(&0)).min(total);
            b.add(format!("Soltar os parafusos da tampa ({}/{})", done, total), format!("desparafusar {}", id), done >= total || aberto, false, pt("parafuso"));
            b.add("Retirar a tampa".into(), format!("abrir {}", id), aberto, false, pt("tampa"));
        }
        if aberto {
            for c in &it.components {
                if c.kind != "tampa" {
                    b.add(format!("Remover {}", c.label.to_lowercase()), format!("componente remover {} {}", id, c.id), c.removed, true, pt(&format!("comp.{}", c.id.to_lowercase())));
                }
            }
            for sl in &it.slots {
                let (Some(m), None) = (&sl.media_id, &sl.hidden_by) else { continue };
                let fora = eq.removed.contains(m);
                let key = format!("{}:{}", it.id, sl.id.to_uppercase());
                if slot_has_cables(it, sl) {
                    for c in ["dados", "energia"] {
                        let done = fora || eq.unplugged.contains(&format!("{}:{}", key, c));
                        b.add(format!("Desconectar o cabo de {} ({})", c, sl.id), format!("cabo desconectar {} {} {}", id, sl.id.to_lowercase(), c), done, false, pt(&format!("{}.{}", sl.id.to_lowercase(), c)));
                    }
                }
                let st = slot_screws(&sim.case, it, sl);
                if st > 0 {
                    let sd = (*eq.screws.get(&key).unwrap_or(&0)).min(st);
                    b.add(format!("Soltar os parafusos do suporte {} ({}/{})", sl.id, sd, st), format!("desparafusar {} {}", id, sl.id.to_lowercase()), fora || sd >= st, false, pt(&format!("{}.parafuso", sl.id.to_lowercase())));
                }
                b.add(format!("Retirar a mídia {} ({})", m, sl.id), format!("midia remover {} {}", id, sl.id.to_lowercase()), fora, false, pt(&format!("{}.midia", sl.id.to_lowercase())));
            }
        }
    }
    if let Some(ph) = &it.phone {
        b.add("Modo avião".into(), format!("airplane {}", id), ph.airplane, true, me());
        b.add("Desabilitar alarmes".into(), format!("alarmes {}", id), ph.alarms_disabled, true, me());
        for s in &ph.sims {
            b.add(format!("Remover o chip SIM (slot {})", s.slot), format!("sim remover {} {}", id, s.slot), s.place != SimWhere::Slot, false, pt("gaveta"));
        }
        if sim.case.module == ModuleId::M505 {
            b.add("Solicitar senhas".into(), format!("solicitar-senhas {}", id), it.flag("senhas_solicitadas"), false, me());
            b.add("Etiqueta na câmera frontal".into(), format!("camera cobrir {}", id), ph.front_camera_covered, false, pt("camera"));
        } else {
            // em BFU a extração completa não é possível no local: documenta-se e segue (o
            // checklist do motor avalia); por isso a etapa não trava a embalagem
            b.add("Extrair os dados (cabo de dados)".into(), format!("extrair {}", id), audited(sim, "extracao", &up) || ph.wiped, true, pt("porta"));
        }
    }
    // cadeia de aquisição das mídias acessíveis
    for mid in &it.media {
        let Some(m) = sim.case.media_by_id(mid) else { continue };
        if m.kind == MediaKind::FlashMovel || !media_free(sim, mid) {
            continue;
        }
        let ml = mid.to_lowercase();
        let ligado = eq.linked.contains_key(mid);
        // ponta na mídia (conector do disco, do pendrive, do cartão) e na porta certa do bloqueador
        let ponta_midia = midia_alvo(sim, mid);
        let porta = Alvo::Parte(crate::parts::BLOQ.into(), crate::parts::porta_bloqueador(m.kind).into());
        b.add(format!("Conectar {} ao bloqueador", mid), format!("conectar {} bloqueador", ml), ligado, false, vec![ponta_midia, porta]);
        let usb = || vec![Alvo::Parte(crate::parts::BLOQ.into(), "usb_host".into()), Alvo::Parte(crate::parts::ESTACAO.into(), "usb".into())];
        b.add("Ligar o bloqueador à estação (cabo USB)".into(), "conectar bloqueador estacao".into(), eq.blocker_host || eq.linked.get(mid).map(|x| x == "estacao").unwrap_or(false), false, usb());
        b.add(format!("Ativar o bloqueio de escrita ({})", mid), format!("bloqueio {}", ml), sim.hashes.contains_key(&format!("wb:{}", mid)), false, vec![Alvo::Parte(crate::parts::BLOQ.into(), "chave".into())]);
        // cifrada por hardware: a imagem seria ilegível fora do equipamento — o caminho é clonar
        // e iniciar o original pela BIOS, na estação (terminal); aqui as etapas não travam
        let fde = m.fde.is_some();
        let nota = if fde { " — cifrada por hardware: clone + BIOS na estação" } else { "" };
        b.add(format!("Adquirir a imagem de {}{}", mid, nota), format!("imagem {}", ml), sim.imaged.contains(mid), fde, vec![Alvo::Bloqueador]);
        b.add(format!("Verificar o hash de {}", mid), format!("verificar-hash {}", ml), audited(sim, "verificar-hash", mid), fde, vec![Alvo::Bloqueador]);
    }
    b.add("Embalar (saco de evidência)".into(), format!("embalar {}", id), it.flag("embalado"), false, me());
    b.add("Apor lacre numerado".into(), format!("lacre aplicar {}", id), audited(sim, "lacre-aplicar", &up), false, me());
    b.add("Etiquetar".into(), format!("etiquetar {} {}", id, it.label), it.flag("etiqueta"), false, me());
    b.v
}

/// Ponta de cabo do lado da mídia: o conector da mídia retirada, ou o do item avulso (HD
/// externo, pendrive, cartão).
fn midia_alvo(sim: &Sim, mid: &str) -> Alvo {
    if sim.equip.removed.contains(mid) {
        return Alvo::Parte(format!("M.{}", mid), "porta".into());
    }
    let avulso = sim.case.items.iter().find(|it| it.slots.is_empty() && it.media.len() == 1 && it.media[0] == mid && crate::parts::tem(sim, it, "porta"));
    match avulso {
        Some(it) => Alvo::Parte(it.id.clone(), "porta".into()),
        None => Alvo::Midia(mid.to_string()),
    }
}

/// Vestígio não digital: sinalizar, fotografar com escala, comunicar (sem coletar).
pub fn vestigio_steps(sim: &Sim, v: &Vestigio) -> Vec<Step> {
    let id = v.id.to_lowercase();
    let eq = &sim.equip;
    let me = || vec![Alvo::Vestigio(v.id.clone())];
    let mut b = B { sim, v: vec![] };
    b.add("Sinalizar com placa e escala".into(), format!("marcar {}", id), eq.markers.contains_key(&v.id), false, me());
    b.add("Fotografar com escala".into(), format!("fotografar {}", id), eq.photographed.contains(&v.id), false, me());
    b.add("Comunicar à equipe de local".into(), format!("comunicar {}", id), eq.reported.contains(&v.id), false, me());
    b.v
}

/// Etapas do objeto (ou vestígio) pelo id da seleção.
pub fn steps_of(sim: &Sim, id: &str) -> Vec<Step> {
    if let Some(v) = sim.case.vestigios.iter().find(|v| v.id == id) {
        return vestigio_steps(sim, v);
    }
    sim.case.item(id).map(|it| item_steps(sim, it)).unwrap_or_default()
}

/// Objetos com etapas (itens visíveis e vestígios), na ordem da cena.
pub fn owners(sim: &Sim) -> Vec<String> {
    let mut v: Vec<String> = sim.case.items.iter().filter(|i| !i.hidden || i.flag("descoberto")).map(|i| i.id.clone()).collect();
    v.extend(sim.case.vestigios.iter().map(|x| x.id.clone()));
    v
}

/// Primeira etapa pendente (a "próxima" do procedimento), contando as opcionais.
pub fn next(steps: &[Step]) -> Option<&Step> {
    steps.iter().find(|s| !s.done)
}

/// Primeira etapa obrigatória pendente.
pub fn next_required(steps: &[Step]) -> Option<&Step> {
    steps.iter().find(|s| !s.done && !s.optional)
}

/// O comando efetivo de uma etapa para a ferramenta na mão (ex.: isolar com alumínio).
pub fn cmd_for_tool(st: &Step, hand: Option<Tool>) -> String {
    if st.cmd.starts_with("isolar ") && hand == Some(Tool::Aluminio) {
        return st.cmd.replace(" faraday", " aluminio");
    }
    st.cmd.clone()
}

/// A ferramenta serve para a etapa? (Isolar aceita Faraday bag ou alumínio.)
pub fn tool_fits(st: &Step, hand: Option<Tool>) -> bool {
    if st.cmd.starts_with("isolar ") {
        return matches!(hand, Some(Tool::Faraday) | Some(Tool::Aluminio)) || st.tool.is_none();
    }
    match st.tool {
        None => true,
        Some(t) => hand == Some(t),
    }
}

/// Resultado de procurar o que fazer com a ferramenta na mão num alvo.
pub enum Uso {
    /// Etapa liberada.
    Fazer(Step),
    /// Há etapa para esta ferramenta aqui, mas falta uma obrigatória antes.
    Antes { quer: Step, falta: Step },
    /// Nada a fazer com esta ferramenta neste alvo; dica da próxima etapa (se houver).
    Nada(Option<Step>),
}

/// Procura, na lista do dono, a próxima etapa que a ferramenta `hand` faz no alvo `alvo`.
/// Primeiro as etapas que pedem exatamente essa ferramenta; depois as que não pedem nenhuma
/// (com as mãos vazias, só estas). Uma obrigatória pendente antes da escolhida trava.
/// Opcionais sem ferramenta (modo avião, alarmes) só acontecem se forem o objetivo marcado
/// no tablet (`marcado` = comando da etapa): sem isso, E segue para a próxima obrigatória.
pub fn uso(steps: &[Step], alvo: &dyn Fn(&Alvo) -> bool, hand: Option<Tool>, marcado: Option<&str>) -> Uso {
    let aqui = |st: &Step| !st.done && st.alvos.iter().any(|a| alvo(a));
    let escolhida = marcado.and_then(|mc| steps.iter().position(|st| aqui(st) && st.cmd == mc && (st.tool.is_none() || tool_fits(st, hand))));
    let com_ferramenta = || hand.and_then(|_| steps.iter().position(|st| aqui(st) && st.tool.is_some() && tool_fits(st, hand)));
    let sem_ferramenta = || steps.iter().position(|st| aqui(st) && st.tool.is_none() && !st.optional);
    let pela_ferramenta = escolhida.or_else(com_ferramenta);
    let Some(i) = pela_ferramenta.or_else(sem_ferramenta) else { return Uso::Nada(next_required(steps).or(next(steps)).cloned()) };
    match steps[..i].iter().find(|st| !st.done && !st.optional) {
        // com uma ferramenta na mão que não serve aqui, uma etapa "de mãos" travada não é o
        // assunto: a dica é a próxima etapa obrigatória
        Some(_) if pela_ferramenta.is_none() && hand.is_some() => Uso::Nada(next_required(steps).cloned()),
        Some(f) => Uso::Antes { quer: steps[i].clone(), falta: f.clone() },
        None => Uso::Fazer(steps[i].clone()),
    }
}

#[cfg(test)]
mod t {
    use super::*;
    use pericia_core::casegen::generate;

    fn run(s: &mut Sim, c: &str) {
        let o = s.exec(c);
        assert!(o.ok, "{} -> {}", c, o.text);
    }

    #[test]
    fn ordem_obrigatoria_e_ferramenta() {
        let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
        let st = steps_of(&s, "ITEM-01");
        assert_eq!(st[0].label, "Fotografar");
        assert_eq!(st[0].tool, Some(Tool::Camera));
        let no_item = |a: &Alvo| *a == Alvo::Objeto("ITEM-01".into());
        // romper e conferir o lacre acontecem mirando o próprio lacre (a peça), não o objeto
        let no_lacre = |a: &Alvo| *a == Alvo::Parte("ITEM-01".into(), "lacre".into());
        assert!(matches!(uso(&st, &no_item, Some(Tool::Tesoura), None), Uso::Nada(_)), "no corpo do objeto a tesoura não rompe o lacre");
        // com a tesoura no lacre, romper ainda não: falta fotografar
        match uso(&st, &no_lacre, Some(Tool::Tesoura), None) {
            Uso::Antes { quer, falta } => {
                assert!(quer.cmd.starts_with("deslacrar"));
                assert_eq!(falta.label, "Fotografar");
            }
            _ => panic!("deveria exigir a etapa anterior"),
        }
        run(&mut s, "pegar camera");
        run(&mut s, "fotografar item-01");
        let st = steps_of(&s, "ITEM-01");
        assert!(st[0].done, "fotografia registrada na auditoria conta como feita");
        match uso(&st, &no_lacre, Some(Tool::Oficio), None) {
            Uso::Fazer(x) => assert!(x.cmd.starts_with("lacre conferir")),
            _ => panic!("conferir o lacre está liberado"),
        }
    }

    #[test]
    fn fluxo_completo_por_etapas_termina_tudo() {
        // o perito automático só faz o que `uso` libera: pega a ferramenta da próxima etapa e usa
        let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
        run(&mut s, "luvas calcar");
        run(&mut s, "pulseira colocar");
        for _ in 0..80 {
            let st = steps_of(&s, "ITEM-01");
            let Some(nx) = next_required(&st).cloned() else { break };
            if let Some(t) = nx.tool {
                let tid = pericia_core::equip::TOOLS.iter().find(|x| x.0 == t).unwrap().1;
                run(&mut s, &format!("pegar {}", tid));
            }
            let alvo = nx.alvos[0].clone();
            let hit = move |a: &Alvo| *a == alvo;
            match uso(&st, &hit, nx.tool, None) {
                Uso::Fazer(x) => run(&mut s, &cmd_for_tool(&x, nx.tool)),
                Uso::Antes { falta, .. } => panic!("travado antes de {}", falta.label),
                Uso::Nada(_) => panic!("nada a fazer em {:?}", nx.label),
            }
        }
        let st = steps_of(&s, "ITEM-01");
        assert!(next_required(&st).is_none(), "pendente: {:?}", next_required(&st).map(|x| x.label.clone()));
        assert!(s.faults.is_empty(), "{:?}", s.faults);
    }

    /// Nenhum caso pode travar na ordem obrigatória: em todos os módulos e dificuldades, o
    /// perito automático conclui todas as etapas de todos os objetos e vestígios, ou para
    /// numa etapa que o próprio motor recusa por um motivo do caso (registrado), nunca por
    /// falta de etapa liberada.
    #[test]
    fn nenhum_caso_trava_na_ordem() {
        let mut travas = vec![];
        for m in [ModuleId::M501, ModuleId::M502, ModuleId::M503, ModuleId::M504, ModuleId::M505] {
            for d in 1..=4u8 {
                for seed in [3u32, 7, 11, 19] {
                    let mut s = Sim::new(generate(m, d, seed));
                    s.exec("luvas calcar");
                    s.exec("pulseira colocar");
                    for _ in 0..400 {
                        let mut progrediu = false;
                        for dono in owners(&s) {
                            let st = steps_of(&s, &dono);
                            let Some(nx) = next_required(&st).cloned() else { continue };
                            if s.equip.gloves != pericia_core::equip::Gloves::Limpas {
                                s.exec(if s.equip.gloves == pericia_core::equip::Gloves::Nenhuma { "luvas calcar" } else { "luvas trocar" });
                            }
                            if let Some(t) = nx.tool {
                                let tid = pericia_core::equip::TOOLS.iter().find(|x| x.0 == t).unwrap().1;
                                s.exec(&format!("pegar {}", tid));
                            }
                            let alvo = nx.alvos[0].clone();
                            let hit = move |a: &Alvo| *a == alvo;
                            match uso(&st, &hit, nx.tool, None) {
                                Uso::Fazer(x) => {
                                    let o = s.exec(&cmd_for_tool(&x, nx.tool));
                                    let depois = steps_of(&s, &dono);
                                    if o.ok && next_required(&depois).map(|y| y.cmd != x.cmd || y.label != x.label).unwrap_or(true) {
                                        progrediu = true;
                                    }
                                }
                                Uso::Antes { falta, .. } => travas.push(format!("{:?} D{} s{} {}: antes de {}", m, d, seed, dono, falta.label)),
                                Uso::Nada(_) => travas.push(format!("{:?} D{} s{} {}: nada liberado para {}", m, d, seed, dono, nx.label)),
                            }
                        }
                        if !progrediu {
                            break;
                        }
                    }
                    for dono in owners(&s) {
                        let st = steps_of(&s, &dono);
                        if let Some(nx) = next_required(&st) {
                            if let Some(t) = nx.tool {
                                let tid = pericia_core::equip::TOOLS.iter().find(|x| x.0 == t).unwrap().1;
                                s.exec(&format!("pegar {}", tid));
                            }
                            let o = s.exec(&cmd_for_tool(nx, nx.tool));
                            travas.push(format!("{:?} D{} s{} {}: parou em '{}' ({})", m, d, seed, dono, nx.label, o.text.lines().next().unwrap_or("")));
                        }
                    }
                }
            }
        }
        assert!(travas.is_empty(), "{} trava(s):\n{}", travas.len(), travas.iter().take(15).cloned().collect::<Vec<_>>().join("\n"));
    }
}

