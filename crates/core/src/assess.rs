//! Avaliação pós-caso. Sem pontuação misteriosa: cada critério mostra o que foi
//! verificado, a decisão tomada e a consequência, com referência ao POP.
//! Encontrar a palavra-chave NÃO compensa destruição de evidência nem conclusão sem base.

use crate::engine::{Output, Sim};
use crate::model::*;

pub struct Criterion {
    pub name: &'static str,
    pub max: u32,
    pub got: u32,
    pub notes: Vec<String>,
}

fn crit(name: &'static str, max: u32) -> Criterion {
    Criterion { name, max, got: max, notes: vec![] }
}

fn lose(c: &mut Criterion, pts: u32, why: impl Into<String>) {
    c.got = c.got.saturating_sub(pts);
    c.notes.push(why.into());
}

pub fn progress(s: &Sim) -> String {
    let mut out = String::from("PROGRESSO (checklist do procedimento)\n");
    for (ok, label) in checklist(s) {
        out.push_str(&format!("  [{}] {}\n", if ok { "x" } else { " " }, label));
    }
    out.push_str(&format!("\nFalhas registradas até agora: {}\n", s.faults.len()));
    out
}

fn any_item(s: &Sim, f: &str) -> bool {
    s.case.items.iter().any(|i| i.flag(f))
}

pub fn checklist(s: &Sim) -> Vec<(bool, String)> {
    let a = &s.audit;
    let mut v = vec![];
    match s.case.module {
        ModuleId::M501 => {
            v.push((any_item(s, "fotografado"), "Fotografar o material lacrado (POP 5.01 §4.1)".into()));
            v.push((a.has("lacre-conferir"), "Conferir lacre e descrição com a documentação".into()));
            v.push((a.has("deslacrar"), "Deslacrar após a conferência".into()));
            v.push((a.has("bloqueio-escrita"), "Usar bloqueador de escrita".into()));
            v.push((a.has("aquisicao") || a.has("espelhamento"), "Adquirir imagem/cópia (exame sobre a cópia)".into()));
            v.push((a.has("verificar-hash"), "Verificar o hash da aquisição".into()));
            v.push((s.keyword_found, "Localizar a expressão de interesse".into()));
            v.push((a.has("lacre-aplicar"), "Apor novo lacre ao material".into()));
            if s.case.media.iter().any(|m| m.fde.is_some()) {
                v.push((a.has("espelhamento") && a.has("bios-ordem"), "Cifragem por hardware: espelhar para destino sanitizado e conferir a ordem de inicialização antes de operar o equipamento".into()));
            }
            if s.case.net.as_ref().map(|n| n.cloud.is_some()).unwrap_or(false) {
                let ok = if s.case.legal.autorizacao_nuvem { s.cloud_access.is_some() } else { !s.faults.iter().any(|f| f.contains("nuvem")) };
                v.push((ok, "Nuvem: respeitar o alcance da autorização judicial (obter os dados se autorizado; registrar a limitação se não)".into()));
            }
        }
        ModuleId::M502 => {
            v.push((any_item(s, "isolado_rf") || any_item(s, "modo_aviao"), "Isolar comunicações (RF/modo avião) (POP 5.02)".into()));
            v.push((a.has("sim-remover"), "Remover SIM registrando o slot".into()));
            v.push((a.has("carregar"), "Garantir alimentação (AFU)".into()));
            v.push((a.has("imei-verificar"), "Conferir IMEI físico × lógico (*#06#)".into()));
            v.push((a.has("extracao"), "Realizar a extração mais completa possível".into()));
            v.push((a.has("verificar-hash") || a.has("hash"), "Registrar hash da extração".into()));
            v.push((s.keyword_found, "Localizar a expressão de interesse".into()));
        }
        ModuleId::M503 => {
            v.push((any_item(s, "fotografado"), "Fixação fotográfica do local/equipamentos (POP 5.03)".into()));
            v.push((any_item(s, "rede_isolada"), "Isolamento digital do equipamento ligado".into()));
            v.push((a.has("memoria"), "Coleta de dados voláteis antes de desligar".into()));
            v.push((a.has("energia-desligar"), "Desligamento por corte de energia após coleta".into()));
            v.push((a.has("aquisicao"), "Aquisição das mídias com bloqueador".into()));
            v.push((a.has("componente-remover") || !s.case.items.iter().any(|i| i.slots.iter().any(|x| x.hidden_by.is_some())), "Verificar mídias ocultas".into()));
            v.push((any_item(s, "embalado"), "Embalagem antiestática e lacre".into()));
            v.push((s.keyword_found, "Localizar a expressão de interesse".into()));
        }
        ModuleId::M504 => {
            v.push((s.net_conn != "institucional", "Usar conexão desvinculada da instituição (POP 5.04)".into()));
            v.push((a.has("dns") && a.has("whois"), "Consultar DNS e WHOIS".into()));
            v.push((a.has("email-cabecalho"), "Analisar o cabeçalho completo do e-mail".into()));
            v.push((a.has("preservar"), "Preservar a página com data/hora/IP".into()));
            v.push((a.has("rede-logs"), "Correlacionar com registros de conexão".into()));
            v.push((s.keyword_found, "Localizar a expressão de interesse".into()));
        }
        ModuleId::M505 => {
            v.push((any_item(s, "fotografado"), "Reconhecer e fixar (fotografar) o dispositivo (POP 5.05)".into()));
            v.push((any_item(s, "etiqueta_camera"), "Etiqueta sobre a câmera frontal".into()));
            v.push((any_item(s, "senhas_solicitadas"), "Solicitar senhas (inclusive segundo espaço/cofres) e verificar smartlock".into()));
            v.push((any_item(s, "isolado_rf") || any_item(s, "modo_aviao"), "Isolar comunicações".into()));
            v.push((a.has("sim-remover"), "Remover SIM registrando slot/operadora/ICCID".into()));
            v.push((a.has("alarmes"), "Desabilitar alarmes (quando possível)".into()));
            v.push((any_item(s, "embalado"), "Embalar individualmente".into()));
            v.push((any_item(s, "lacrado_perito"), "Lacrar".into()));
            v.push((any_item(s, "etiqueta"), "Identificar/etiquetar".into()));
        }
    }
    v.push((!s.report.conclusao.trim().is_empty() || s.case.module == ModuleId::M505, "Redigir o documento final".into()));
    v
}

pub fn finalize(s: &mut Sim) -> Output {
    if s.report.submitted {
        return Output::ok(render(s, &evaluate(s)));
    }
    crate::report::autofill(s);
    s.report.submitted = true;
    s.rec_pub("finalizar", &s.case.id.clone(), "documento emitido; caso encerrado para avaliação");
    let ev = evaluate(s);
    Output::ok(render(s, &ev))
}

pub fn evaluate(s: &Sim) -> Vec<Criterion> {
    let a = &s.audit;
    let m = s.case.module;
    let mut cs = vec![];

    // 1. Conformidade procedimental
    let mut c1 = crit("Conformidade com o procedimento (POP)", 20);
    let cl = checklist(s);
    let missing: Vec<&String> = cl.iter().filter(|(ok, _)| !ok).map(|(_, l)| l).collect();
    for l in &missing {
        lose(&mut c1, 20 / cl.len().max(1) as u32 + 1, format!("Etapa não realizada: {}", l));
    }
    cs.push(c1);

    // 2/3. Preservação e integridade
    let mut c2 = crit("Preservação das evidências e integridade", 20);
    for f in &s.faults {
        let fl = f.to_lowercase();
        if fl.contains("sem bloqueador") || fl.contains("contamin") || fl.contains("apag") || fl.contains("perd") || fl.contains("restaura") || fl.contains("não sanitizado") || fl.contains("insuficiente") {
            lose(&mut c2, 8, f.clone());
        }
    }
    if matches!(m, ModuleId::M501 | ModuleId::M503) && !a.has("aquisicao") && !a.has("espelhamento") {
        lose(&mut c2, 6, "Nenhuma cópia forense foi produzida — o exame deve ser feito sobre a cópia (POP 5.01).");
    }
    cs.push(c2);

    // 4. Registro de ações e custódia
    let mut c4 = crit("Registro de ações e cadeia de custódia", 10);
    if a.verify().is_err() {
        lose(&mut c4, 10, "A trilha de auditoria foi adulterada.");
    }
    if s.case.items.iter().any(|i| i.flag("divergencia_imei")) && !text_of(s).contains("imei") {
        lose(&mut c4, 4, "Divergência de IMEI físico × lógico detectada mas não registrada no documento.");
    }
    if s.case.items.iter().any(|i| i.flag("divergencia_lacre")) {
        if !text_of(s).contains("lacre") && !text_of(s).contains("diverg") {
            lose(&mut c4, 5, "Divergência de lacre detectada mas não mencionada no documento.");
        }
    }
    if s.case.items.iter().any(|i| i.flag("deslacrado")) && !a.has("lacre-aplicar") && m != ModuleId::M504 {
        lose(&mut c4, 3, "Material deslacrado não recebeu novo lacre.");
    }
    cs.push(c4);

    // 5/6. Uso de ferramentas e interpretação
    let mut c5 = crit("Uso adequado das ferramentas e interpretação", 10);
    if s.faults.iter().any(|f| f.contains("não corresponde")) {
        lose(&mut c5, 4, "Apontou expressão incorreta (interpretação precipitada).");
    }
    if s.faults.iter().any(|f| f.contains("alertado")) {
        lose(&mut c5, 4, "Diligência pode ter alertado o investigado (host monitorado sem anonimato).");
    }
    cs.push(c5);

    // 7. Correlação entre fontes
    let mut c7 = crit("Correlação entre fontes", 10);
    if m == ModuleId::M504 && !(a.has("email-cabecalho") && a.has("rede-logs")) {
        lose(&mut c7, 6, "Não correlacionou o cabeçalho do e-mail com os registros de conexão.");
    }
    if m == ModuleId::M501 && s.case.difficulty >= 3 && !a.has("fs-meta") {
        lose(&mut c7, 4, "Não conferiu metadados/datas (há questão de fuso horário ou versões conflitantes).");
    }
    cs.push(c7);

    // 8/9. Fatos x hipóteses; limitações
    let mut c8 = crit("Distinção fato/inferência e reconhecimento de limitações", 10);
    let coh = crate::report::check_coherence(s);
    for p in &coh.problems {
        lose(&mut c8, 3, p.clone());
    }
    if s.case.net.as_ref().map(|n| n.isps.iter().any(|i| i.cgnat)).unwrap_or(false) && !text_of(s).contains("porta") && !text_of(s).contains("cgnat") {
        lose(&mut c8, 3, "Caso com CGNAT: o documento não registra a limitação de atribuição (porta/horário).");
    }
    cs.push(c8);

    // 10. Qualidade documental
    let mut c10 = crit("Qualidade documental", 10);
    if m.document() == DocKind::Laudo {
        if s.report.material.trim().len() < 20 { lose(&mut c10, 3, "Seção MATERIAL incompleta."); }
        if s.report.exame.trim().len() < 20 { lose(&mut c10, 3, "Seção EXAME incompleta."); }
        if s.report.conclusao.trim().len() < 20 { lose(&mut c10, 4, "CONCLUSÃO ausente ou insuficiente."); }
    } else if !s.case.items.iter().all(|i| i.flag("etiqueta") && i.flag("embalado")) {
        lose(&mut c10, 5, "Termo de apreensão: itens sem identificação/embalagem registrada.");
    }
    cs.push(c10);

    // 11. Palavra-chave fundamentada (não se aplica à apreensão)
    if !s.case.keyword.is_empty() {
        let mut c11 = crit("Identificação fundamentada da expressão de interesse", 10);
        if !s.keyword_found {
            lose(&mut c11, 10, "A expressão não foi efetivamente localizada em uma fonte examinada.");
        } else if s.report.keyword_claim.is_empty() {
            lose(&mut c11, 6, "A expressão foi vista mas não foi destacada no documento.");
        } else if s.report.keyword_source.trim().is_empty() {
            lose(&mut c11, 5, "A expressão foi destacada sem indicar a origem.");
        }
        cs.push(c11);
    }

    // Trava: destruição de evidência limita a nota total (a palavra-chave não compensa).
    cs
}

fn text_of(s: &Sim) -> String {
    let r = &s.report;
    format!("{} {} {} {} {}", r.material, r.exame, r.consideracoes, r.conclusao, r.keyword_source).to_lowercase()
}

pub fn total(cs: &[Criterion], s: &Sim) -> (u32, u32, bool) {
    let got: u32 = cs.iter().map(|c| c.got).sum();
    let max: u32 = cs.iter().map(|c| c.max).sum();
    let destroyed = s.faults.iter().any(|f| {
        let f = f.to_lowercase();
        f.contains("apagad") || f.contains("perdid") || f.contains("restauração") || f.contains("apagamento")
    });
    let capped = if destroyed { got.min(max * 60 / 100) } else { got };
    (capped, max, destroyed)
}

pub fn render(s: &Sim, cs: &[Criterion]) -> String {
    let (got, max, destroyed) = total(cs, s);
    let pct = got * 100 / max.max(1);
    let verdict = if destroyed {
        "REPROVADO — houve perda/destruição de evidência"
    } else if pct >= 70 {
        "APROVADO"
    } else {
        "REPROVADO — revise os pontos abaixo"
    };
    let mut out = format!("AVALIAÇÃO DO CASO {}\nResultado: {}/{} ({}%) — {}\n", s.case.id, got, max, pct, verdict);
    if destroyed {
        out.push_str("Observação: com perda de evidência, a nota é limitada a 60%, mesmo que a expressão tenha sido encontrada.\n");
    }
    for c in cs {
        out.push_str(&format!("\n■ {} — {}/{}\n", c.name, c.got, c.max));
        if c.notes.is_empty() {
            out.push_str("   ✓ sem ressalvas\n");
        }
        for n in &c.notes {
            out.push_str(&format!("   – {}\n", n));
        }
    }
    if !s.keyword_where.is_empty() {
        out.push_str(&format!("\nOnde estava a expressão: {}\n", s.keyword_where));
    } else if !s.case.keyword_sources.is_empty() {
        out.push_str("\nFontes da expressão no caso (revisão):\n");
        for k in &s.case.keyword_sources {
            out.push_str(&format!("   • {}\n", k.describe()));
        }
    }
    out
}

/// Orientação adaptativa: quanto maior a dificuldade, menos assistência.
pub fn hint(s: &Sim) -> String {
    let cl = checklist(s);
    let next = cl.iter().find(|(ok, _)| !ok).map(|(_, l)| l.clone());
    match s.case.difficulty {
        1 => {
            let mut out = String::from("MENTOR (nível guiado)\n");
            for (ok, l) in &cl {
                out.push_str(&format!("  [{}] {}\n", if *ok { "x" } else { " " }, l));
            }
            if let Some(n) = next {
                out.push_str(&format!("\nPróximo passo sugerido: {}\nPor quê: {}\n", n, why(&n)));
            }
            out
        }
        2 => match next {
            Some(n) => format!("MENTOR: próximo passo sugerido — {}\nPor quê: {}", n, why(&n)),
            None => "MENTOR: etapas principais cumpridas. Revise o documento.".into(),
        },
        3 => format!("MENTOR (assistência reduzida): {} de {} etapas essenciais cumpridas. Consulte o manual do POP.", cl.iter().filter(|x| x.0).count(), cl.len()),
        _ => "MENTOR: sem orientação neste nível. As decisões e justificativas são suas.".into(),
    }
}

fn why(step: &str) -> &'static str {
    let s = step.to_lowercase();
    if s.contains("fotograf") { "a fixação documenta o estado original antes de qualquer intervenção." }
    else if s.contains("lacre") && s.contains("confer") { "divergências entre lacre e documentação precisam ser registradas antes do deslacre." }
    else if s.contains("bloqueador") { "impede que a leitura altere a mídia original." }
    else if s.contains("imagem") || s.contains("cópia") || s.contains("aquisi") { "o exame é feito sobre a cópia, preservando o original." }
    else if s.contains("hash") { "o hash demonstra a integridade da cópia em relação ao original." }
    else if s.contains("isol") { "evita comandos remotos (apagamento, bloqueio) e novas comunicações." }
    else if s.contains("volát") || s.contains("memória") { "chaves e dados em memória se perdem ao desligar." }
    else if s.contains("oculta") { "mídias podem estar camufladas ou instaladas fora do padrão." }
    else if s.contains("conexão desvinculada") { "evita vincular a diligência ao órgão e alertar o investigado." }
    else if s.contains("preserv") { "páginas podem sair do ar; a preservação registra conteúdo, data/hora e IP usado." }
    else if s.contains("correlac") { "um IP indica uma conexão/assinante; é preciso cruzar fontes e horários." }
    else if s.contains("sim") { "o SIM identifica a linha; slot, operadora e ICCID devem constar no registro." }
    else if s.contains("expressão") { "a expressão precisa ser localizada numa fonte examinada e ter a origem indicada." }
    else if s.contains("documento") { "conclusões devem ser fundamentadas e coerentes com as ações registradas." }
    else { "consulte o manual do POP correspondente." }
}
