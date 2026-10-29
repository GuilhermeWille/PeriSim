//! Geração de documentos (laudo de exame ou termo de apreensão) em texto e Markdown,
//! e verificação de coerência entre o que o relatório afirma e o que a auditoria registra.

use crate::engine::Sim;
use crate::model::*;
use crate::util::*;

pub fn autofill(s: &mut Sim) {
    let c = &s.case;
    if s.report.historico.is_empty() {
        s.report.historico = c.historico.clone();
    }
    if s.report.objetivo.is_empty() {
        s.report.objetivo = c.objetivo.clone();
    }
    // material: a partir dos itens e artefatos
    if s.report.material.is_empty() {
        let mut m = String::new();
        for it in &c.items {
            if it.hidden && !it.flag("descoberto") {
                continue;
            }
            m.push_str(&format!("{}: {}", it.id, it.description));
            if let Some(seal) = &it.seal {
                m.push_str(&format!(" — lacre {}", seal.number));
            }
            m.push_str(".\n");
        }
        s.report.material = m.trim_end().to_string();
    }
    // exame: a partir das ações registradas
    if s.report.exame.is_empty() {
        let mut steps = vec![];
        for e in &s.audit.entries {
            if matches!(e.action.as_str(), "aquisicao" | "espelhamento" | "extracao" | "hash" | "verificar-hash" | "recuperar" | "decifrar" | "memoria" | "preservar" | "email-cabecalho") {
                steps.push(format!("- {}: {} ({})", fmt_dt(e.sim_time, BRT), e.detail, e.outcome));
            }
        }
        s.report.exame = if steps.is_empty() { "(descreva os exames realizados)".into() } else { steps.join("\n") };
    }
    // considerações: limitações observadas
    if s.report.consideracoes.is_empty() {
        let mut lim = vec![];
        // limitações efetivamente observadas nas ações (sem alegar procedimentos não realizados)
        if s.audit.entries.iter().any(|e| e.action == "recuperar" && e.outcome.contains("parcial")) {
            lim.push("Parte dos arquivos apagados foi obtida apenas parcialmente; o conteúdo ausente não pode ser presumido.".to_string());
        }
        if s.audit.entries.iter().any(|e| e.action == "recuperar" && e.outcome.contains("inviável")) {
            lim.push("Houve arquivos cuja leitura se mostrou inviável (área sobrescrita ou setores ilegíveis).".to_string());
        }
        if s.audit.has("extracao") {
            lim.push("O método de extração do dispositivo móvel condiciona o alcance dos dados obtidos.".to_string());
        }
        if s.case.media.iter().any(|m| m.partitions.iter().any(|p| p.tz_offset != BRT)) && s.audit.has("fs-meta") {
            lim.push("O sistema examinado registrava horários em fuso diferente do horário de Brasília; os horários citados foram convertidos.".to_string());
        }
        if s.case.net.as_ref().map(|n| n.isps.iter().any(|i| i.cgnat)).unwrap_or(false) {
            lim.push("Em ambiente CGNAT, a atribuição de autoria exige a porta lógica de origem e o horário exato; o IP isoladamente não identifica a pessoa.".into());
        }
        s.report.consideracoes = lim.join(" ");
    }
    // origem da palavra-chave
    if s.report.keyword_source.is_empty() && !s.keyword_where.is_empty() {
        s.report.keyword_source = s.keyword_where.clone();
    }
    if s.report.keyword_claim.is_empty() && s.keyword_found {
        s.report.keyword_claim = c.keyword.clone();
    }
}

pub fn preview(s: &Sim) -> String {
    match s.case.module.document() {
        DocKind::TermoApreensao => termo_text(s),
        DocKind::Laudo => laudo_text(s),
    }
}

fn header(s: &Sim) -> String {
    let c = &s.case;
    format!(
        "SIMULAÇÃO DE TREINAMENTO — NÃO É DOCUMENTO OFICIAL\nInstituto de Criminalística (fictício)\nSetor de Perícias de Informática\n\nReferência: {}\nProcedimento: {}\nData: {}\n",
        c.id, c.procedimento, fmt_date(s.clock, BRT)
    )
}

fn laudo_text(s: &Sim) -> String {
    let c = &s.case;
    let r = &s.report;
    let mut out = header(s);
    out.push_str(&format!("\nLAUDO PERICIAL — {} ({})\n", c.module.title(), c.module.code()));
    out.push_str(&format!("\n1. PREÂMBULO\nPerícia realizada em atendimento ao {}, referente a {}.\n", c.procedimento, c.requisitante));
    out.push_str(&format!("\n2. HISTÓRICO\n{}\n", filled(&r.historico, &c.historico)));
    out.push_str(&format!("\n3. OBJETIVO\n{}\n", filled(&r.objetivo, &c.objetivo)));
    out.push_str(&format!("\n4. MATERIAL\n{}\n", filled(&r.material, "(não preenchido — use 'laudo auto' ou 'laudo material <...>')")));
    out.push_str(&format!("\n5. EXAME\n{}\n", filled(&r.exame, "(não preenchido)")));
    if !c.keyword.is_empty() {
        out.push_str("\n6. EXPRESSÃO DE INTERESSE\n");
        if r.keyword_claim.is_empty() {
            out.push_str("(não indicada)\n");
        } else {
            out.push_str(&format!("Expressão localizada: «{}»\n", r.keyword_claim));
            out.push_str(&format!("Origem/obtenção: {}\n", filled(&r.keyword_source, "(origem não indicada)")));
        }
    }
    out.push_str(&format!("\n7. CONSIDERAÇÕES TÉCNICAS\n{}\n", filled(&r.consideracoes, "(sem considerações adicionais)")));
    out.push_str(&format!("\n8. CONCLUSÃO\n{}\n", filled(&r.conclusao, "(não preenchida)")));
    out.push_str("\n9. ANEXOS\n");
    for a in &s.artifacts {
        out.push_str(&format!("- {} {} (origem {}) — SHA-256 {}\n", a.id, a.kind, a.origin, a.hash));
    }
    if s.artifacts.is_empty() {
        out.push_str("(sem anexos)\n");
    }
    out
}

fn termo_text(s: &Sim) -> String {
    let c = &s.case;
    let mut out = header(s);
    out.push_str("\nTERMO DE APREENSÃO DE DISPOSITIVO MÓVEL\n");
    out.push_str(&format!("\nNo âmbito do {}, procedeu-se à apreensão e documentação do(s) seguinte(s) dispositivo(s):\n", c.procedimento));
    for it in &c.items {
        out.push_str(&format!("\nITEM {} — {}\n", it.id, it.label));
        out.push_str(&format!("  Descrição: {}\n", it.description));
        if let Some(ph) = &it.phone {
            out.push_str(&format!("  IMEI: {}\n", ph.imeis.join(" / ")));
            out.push_str(&format!("  Estado: {}\n", ph.boot.label()));
            out.push_str(&format!("  Isolamento de comunicações: {}\n", ph.rf.label()));
            for sim in &ph.sims {
                let st = match sim.place { SimWhere::Slot => "no slot", SimWhere::Removido => "removido e afixado", SimWhere::AfixadoVerso => "afixado ao verso" };
                out.push_str(&format!("  SIM slot {}: operadora {}, ICCID {} ({})\n", sim.slot, sim.operadora, sim.iccid, st));
            }
            if ph.passcode_known(s) {
                out.push_str(&format!("  Senha informada: {}\n", ph.passcode));
            }
        }
        if let Some(seal) = &it.seal {
            out.push_str(&format!("  Lacre aposto: {}\n", seal.number));
        }
        if it.physical.wet {
            out.push_str("  Condição especial: molhado — requer cuidados no processamento.\n");
        }
        if it.physical.battery_swollen {
            out.push_str("  Condição especial: bateria estufada — risco.\n");
        }
        for d in &it.physical.damage {
            out.push_str(&format!("  Dano observado: {}\n", d));
        }
        let flags: Vec<String> = it.flags.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        if !flags.is_empty() {
            out.push_str(&format!("  Registro de ações: {}\n", flags.join(", ")));
        }
    }
    out.push_str("\nObservação: este é um TERMO DE APREENSÃO (não um laudo de exame). O exame pericial, se necessário, será objeto de documento próprio.\n");
    out
}

impl PhoneState {
    fn passcode_known(&self, _s: &Sim) -> bool {
        self.boot == Boot::Desbloqueado || self.lock == "nenhum" || !self.passcode.is_empty()
    }
}

fn filled(v: &str, default: &str) -> String {
    if v.trim().is_empty() {
        default.to_string()
    } else {
        wrap(v, 80).join("\n")
    }
}

pub fn markdown(s: &Sim) -> String {
    let c = &s.case;
    let r = &s.report;
    let mut md = String::new();
    md.push_str("> **SIMULAÇÃO DE TREINAMENTO — NÃO É DOCUMENTO OFICIAL.** Gerado pelo Simulador de Informática Forense.\n\n");
    let doc = c.module.document();
    let title = if doc == DocKind::Laudo { "Laudo Pericial" } else { "Termo de Apreensão" };
    md.push_str(&format!("# {} — {}\n\n", title, c.module.title()));
    md.push_str(&format!("**Referência:** {}  \n**Procedimento:** {}  \n**Módulo POP:** {}  \n**Data:** {}\n\n", c.id, c.procedimento, c.module.code(), fmt_date(s.clock, BRT)));
    if doc == DocKind::Laudo {
        md.push_str(&format!("## 1. Histórico\n\n{}\n\n", filled(&r.historico, &c.historico)));
        md.push_str(&format!("## 2. Objetivo\n\n{}\n\n", filled(&r.objetivo, &c.objetivo)));
        md.push_str(&format!("## 3. Material\n\n{}\n\n", filled(&r.material, "_(não preenchido)_")));
        md.push_str(&format!("## 4. Exame\n\n{}\n\n", filled(&r.exame, "_(não preenchido)_")));
        if !c.keyword.is_empty() {
            md.push_str("## 5. Expressão de interesse\n\n");
            if r.keyword_claim.is_empty() {
                md.push_str("_(não indicada)_\n\n");
            } else {
                md.push_str(&format!("> **«{}»**\n\n", r.keyword_claim));
                md.push_str(&format!("**Origem / forma de obtenção:** {}\n\n", filled(&r.keyword_source, "_(não indicada)_")));
            }
        }
        md.push_str(&format!("## 6. Considerações técnicas\n\n{}\n\n", filled(&r.consideracoes, "_(sem considerações)_")));
        md.push_str(&format!("## 7. Conclusão\n\n{}\n\n", filled(&r.conclusao, "_(não preenchida)_")));
        md.push_str("## 8. Anexos\n\n| ID | Tipo | Origem | SHA-256 |\n|---|---|---|---|\n");
        for a in &s.artifacts {
            md.push_str(&format!("| {} | {} | {} | `{}` |\n", a.id, a.kind, a.origin, a.hash));
        }
        md.push('\n');
    } else {
        md.push_str(&termo_text(s).lines().skip(8).collect::<Vec<_>>().join("\n"));
        md.push('\n');
    }
    md.push_str("## Registro de ações (cadeia de custódia)\n\n");
    for e in &s.audit.entries {
        md.push_str(&format!("- `{}` **{}** — {} _( {} )_\n", fmt_dt(e.sim_time, BRT), e.action, e.detail, e.outcome));
    }
    md
}

/// Verifica coerência entre afirmações do relatório e o histórico de ações.
pub struct Coherence {
    pub problems: Vec<String>,
}

pub fn check_coherence(s: &Sim) -> Coherence {
    let mut problems = vec![];
    let r = &s.report;
    let text = format!("{} {} {} {}", r.exame, r.conclusao, r.consideracoes, r.material).to_lowercase();

    // afirma hash sem ter calculado
    if (text.contains("hash") || text.contains("integridade")) && !s.audit.has("hash") && !s.audit.has("aquisicao") && !s.audit.has("verificar-hash") {
        problems.push("O relatório menciona hash/integridade, mas não há cálculo de hash registrado na auditoria.".into());
    }
    // afirma recuperação sem ter recuperado
    if text.contains("recuper") && !s.audit.has("recuperar") {
        problems.push("O relatório menciona recuperação de arquivos, mas nenhuma recuperação foi realizada.".into());
    }
    // afirma preservação de página sem ter preservado
    if text.contains("preserv") && (s.audit.has("navegar") || s.audit.has("preservar")) && !s.audit.has("preservar") {
        problems.push("O relatório menciona preservação de página, mas não há ação de preservação registrada.".into());
    }
    // conclui autoria por IP de forma categórica
    if text.contains("foi o autor") || text.contains("de autoria de") || text.contains("identifica o investigado") || text.contains("prova que") {
        problems.push("A conclusão afirma autoria de forma categórica — verifique se a evidência sustenta isso (endereço técnico ≠ pessoa).".into());
    }
    // indica expressão sem origem
    if !r.keyword_claim.is_empty() && r.keyword_source.trim().is_empty() {
        problems.push("A expressão de interesse foi indicada sem registrar a origem/forma de obtenção.".into());
    }
    // conclusão vazia
    if r.conclusao.trim().is_empty() {
        problems.push("A conclusão está vazia.".into());
    }
    Coherence { problems }
}
