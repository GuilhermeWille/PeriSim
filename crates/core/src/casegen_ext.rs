//! Extensões do gerador a partir da versão 1.1.0.
//!
//! O conteúdo-base de um caso (versão 1.0.0) não muda: as extensões são aplicadas por
//! cima, com sementes próprias, e só quando o caso é gerado numa versão que as inclui.
//! Assim, um mesmo ID de caso mantém pessoas, expressão e arquivos-base, e salvamentos
//! da versão 1.0.0 continuam reproduzíveis.
//!
//! Extensões da 1.1.0:
//! * 5.01 D3: disco autocriptografado (cifragem por hardware vinculada ao equipamento);
//! * 5.01/5.03 D2+: mais casos-limite de arquivos — apagado sobrescrito, arquivo em
//!   setores ilegíveis, assinatura que não corresponde à extensão;
//! * 5.04 D3+: mensagem reencaminhada por intermediário, com DKIM quebrado e cadeia ARC.
//!
//! Extensões da 1.2.0 (modo passo a passo):
//! * vestígios físicos na cena e no material (sangue seco, cabelo, bituca, copo, pegada),
//!   que o perito sinaliza, fotografa e comunica sem coletar; itens com vestígio
//!   biológico contaminam as luvas.

use crate::model::*;
use crate::rng::Rng;
use crate::util::{fmt_rfc5322, BRT};

pub fn version_at_least(v: &str, min: &str) -> bool {
    let p = |s: &str| s.split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    p(v) >= p(min)
}

pub fn apply(case: &mut CaseFile, version: &str) {
    if !version_at_least(version, "1.1.0") {
        return;
    }
    apply_11(case);
    if version_at_least(version, "1.2.0") {
        vestigios(case);
    }
    recompute_hashes(case);
}

fn apply_11(case: &mut CaseFile) {
    let mut rng = Rng::for_label(case.seed, &format!("ext-1.1|{}|{}", case.module.code(), case.difficulty));
    let d = case.difficulty;
    match case.module {
        ModuleId::M501 => {
            if d == 3 && rng.chance(0.45) {
                hardware_fde(case, &mut rng);
            }
            if d >= 2 {
                damaged_files(case, &mut rng, "M-01");
            }
        }
        ModuleId::M503 if d >= 2 => {
            if let Some(m) = case.media.iter().find(|m| !m.is_work_media && m.kind != MediaKind::FlashMovel && !m.partitions.is_empty()).map(|m| m.id.clone()) {
                damaged_files(case, &mut rng, &m);
            }
        }
        ModuleId::M504 if d >= 3 => arc_forward(case, &mut rng),
        _ => {}
    }
}

fn recompute_hashes(case: &mut CaseFile) {
    // hashes registrados em documento anterior descrevem a mídia como ela é (com as extensões)
    let ids: Vec<String> = case.documented_hashes.keys().cloned().collect();
    for id in ids {
        if let Some(m) = case.media.iter().find(|m| m.id == id) {
            let h = crate::vfs::media_hash(m);
            case.documented_hashes.insert(id, h);
        }
    }
}

/// Cifragem integral por hardware (SED): o conteúdo só é legível no equipamento de origem,
/// com a senha do disco. A imagem isolada seria ilegível; o caminho do POP é o espelhamento
/// mídia-para-mídia (com destino sanitizado) e o desbloqueio do clone no equipamento.
fn hardware_fde(case: &mut CaseFile, rng: &mut Rng) {
    let pw = format!("{}{}", rng.pick(&["Sol", "Mar", "Rio", "Ipe", "Luz"]), rng.digits(5));
    if let Some(m) = case.media.iter_mut().find(|m| m.id == "M-01") {
        m.fde = Some(Encryption { scheme: "Disco autocriptografado (cifragem por hardware, ATA) vinculado ao equipamento".into(), password: pw.clone(), tpm: true });
    }
    case.provided_passwords.push(("Senha do disco informada pelo investigado (consta no termo de apreensão)".into(), pw));
    case.briefing.push("O termo de apreensão registra uma senha de disco informada pelo investigado.".into());
}

fn vf(id: String, path: &str, ftype: FileType, content: &str, mtime: i64, owner: &str, size: u64) -> VFile {
    VFile {
        id,
        path: path.into(),
        size,
        mtime,
        ctime: mtime,
        atime: mtime,
        owner: owner.into(),
        ftype,
        deleted: false,
        recover: None,
        bad_sector: false,
        content: content.into(),
        ocr: None,
        meta: Default::default(),
        password: None,
        children: vec![],
        container: None,
        keyword: false,
    }
}

/// Casos-limite que exigem reconhecer limitações em vez de presumir conteúdo.
fn damaged_files(case: &mut CaseFile, rng: &mut Rng, media: &str) {
    let start = case.start;
    let Some(m) = case.media.iter_mut().find(|m| m.id == media) else { return };
    let Some(p) = m.partitions.iter_mut().find(|p| p.files.iter().any(|f| f.path.starts_with("/Users/"))) else { return };
    let user = p.files.iter().find_map(|f| f.path.strip_prefix("/Users/").and_then(|r| r.split('/').next()).map(|s| s.to_string())).unwrap_or_else(|| "usuario".into());
    let t = |rng: &mut Rng, a: i64, b: i64| start - rng.int(a * 86400, b * 86400);
    let base = p.files.len();
    // 1) apagado e sobrescrito: só o registro de diretório sobrevive
    let mut gone = vf(format!("{}-X{}", p.id, base + 1), &format!("/Users/{}/Documents/contrato_revisado.docx", user), FileType::Documento, "", t(rng, 3, 20), &user, 48_128);
    gone.deleted = true;
    gone.recover = Some(Recover::Nenhuma);
    gone.meta.insert("Observação".into(), "entrada de diretório preservada; clusters reutilizados por outro arquivo".into());
    // 2) arquivo em região de setores ilegíveis
    let mut bad = vf(format!("{}-X{}", p.id, base + 2), &format!("/Users/{}/Documents/planilha_custos.xlsx", user), FileType::Planilha, "Item;Valor\nMaterial;1.240,00\n[...]", t(rng, 10, 40), &user, 96_512);
    bad.bad_sector = true;
    // 3) extensão que não corresponde à assinatura do arquivo
    let mut sig = vf(
        format!("{}-X{}", p.id, base + 3),
        &format!("/Users/{}/Pictures/ferias_praia.jpg", user),
        FileType::Documento,
        "%PDF-1.7 — Documento: Proposta comercial (rascunho). Sem relação aparente com os fatos.",
        t(rng, 15, 60),
        &user,
        210_944,
    );
    sig.meta.insert("Assinatura".into(), "%PDF (documento PDF) — não corresponde à extensão .jpg".into());
    p.files.push(gone);
    p.files.push(bad);
    p.files.push(sig);
    if m.bad_sectors == 0 {
        m.bad_sectors = 6 + rng.int(0, 20) as u32;
    }
}

/// Mensagem que passou por um intermediário (lista/encaminhador) que alterou o conteúdo:
/// a assinatura DKIM não confere mais no destino, mas a cadeia ARC registra que, no
/// intermediário, SPF e DKIM eram válidos.
fn arc_forward(case: &mut CaseFile, rng: &mut Rng) {
    let Some(f) = case.provided.iter_mut().find(|f| f.path.ends_with(".eml")) else { return };
    let Some(pos) = f.content.find("Authentication-Results:") else { return };
    let line_end = f.content[pos..].find('\n').map(|k| pos + k).unwrap_or(f.content.len());
    let old = f.content[pos..line_end].to_string();
    let dom = old.split("header.d=").nth(1).unwrap_or("remetente.sim").trim().to_string();
    let list = "lista.avisos.sim";
    let t = f.mtime + 60;
    let arc = format!(
        "Received: from {list} (relay.{list} [192.0.2.{h}]) by mx.correio.sim with ESMTPS id {rid}; {dt}\n\
         ARC-Seal: i=1; a=rsa-sha256; d={list}; s=arc1; cv=none; b={sig}\n\
         ARC-Message-Signature: i=1; a=rsa-sha256; d={list}; s=arc1; h=from:to:subject:date; b={sig2}\n\
         ARC-Authentication-Results: i=1; {list}; spf=pass smtp.mailfrom={dom}; dkim=pass header.d={dom}\n\
         Authentication-Results: mx.correio.sim; spf=neutral (encaminhada por {list}); dkim=fail (body hash did not verify) header.d={dom}; arc=pass (i=1)",
        list = list,
        h = rng.int(10, 250),
        rid = rng.hex(12),
        dt = fmt_rfc5322(t, BRT),
        sig = rng.hex(24),
        sig2 = rng.hex(24),
        dom = dom
    );
    f.content.replace_range(pos..line_end, &arc);
    f.content = f.content.replacen("Subject: ", &format!("List-Id: <{}>\nSubject: [aviso] ", list), 1);
}


/// Vestígios não digitais (gerador 1.2.0). Conteúdo fictício e sem exageros: o objetivo
/// é treinar preservação (luvas, sinalização, fotografia, comunicação), não chocar.
fn vestigios(case: &mut CaseFile) {
    let mut rng = Rng::for_label(case.seed, &format!("ext-1.2|vest|{}|{}", case.module.code(), case.difficulty));
    let d = case.difficulty;
    let mut out: Vec<Vestigio> = vec![];
    let mut add = |kind: &str, label: &str, desc: &str, on: Option<String>, bio: bool, spot: &str| {
        out.push(Vestigio { id: String::new(), kind: kind.into(), label: label.into(), desc: desc.into(), on_item: on, biologico: bio, spot: spot.into() });
    };
    let first = case.items.iter().find(|i| !i.hidden).map(|i| (i.id.clone(), i.kind));
    match case.setting {
        Setting::Local => {
            add("sangue", "Gotas avermelhadas no piso", "Gotas secas de coloração avermelhada (aspecto de sangue) no piso, junto à cadeira.", None, true, "piso_mesa");
            if rng.chance(0.8) {
                add("cabelo", "Fio de cabelo no teclado", "Fio de cabelo escuro, de cerca de 20 cm, sobre o teclado da mesa.", None, true, "teclado");
            }
            if rng.chance(0.7) {
                add("bituca", "Bituca de cigarro no cinzeiro", "Bituca recente no cinzeiro da mesa (possível material biológico — saliva).", None, true, "cinzeiro");
            }
            if d >= 2 && rng.chance(0.6) {
                add("pegada", "Pegada parcial sobre as gotas", "Marca parcial de solado sobre uma das gotas do piso.", None, false, "piso_mesa");
            }
            if d >= 3 && rng.chance(0.5) {
                add("copo", "Copo descartável com café", "Copo com resto de café e marca de lábios na borda.", None, true, "mesa");
            }
        }
        Setting::Laboratorio => {
            if let Some((id, kind)) = first {
                let p = match d {
                    1 => 0.4,
                    2 => 0.65,
                    _ => 0.8,
                };
                if rng.chance(p) {
                    let onde = if kind == ItemKind::Celular || kind == ItemKind::Tablet { "na capa do aparelho" } else { "na lateral do equipamento" };
                    add("sangue", "Mancha avermelhada seca", &format!("Mancha seca de coloração avermelhada (aspecto de sangue) {}.", onde), Some(id.clone()), true, "item");
                }
                if rng.chance(p * 0.6) {
                    let onde = if kind.is_computer() { "preso na grade de ventilação" } else { "preso à embalagem" };
                    add("cabelo", "Fio de cabelo", &format!("Fio de cabelo {}.", onde), Some(id), true, "item");
                }
            }
        }
    }
    for (k, v) in out.iter_mut().enumerate() {
        v.id = format!("V-{:02}", k + 1);
    }
    for v in &out {
        if let Some(id) = &v.on_item {
            if let Some(it) = case.items.iter_mut().find(|i| &i.id == id) {
                if v.biologico {
                    it.physical.biohazard = true;
                }
                it.physical.traces.push(format!("{} ({})", v.label.to_lowercase(), v.id));
            }
        }
    }
    if !out.is_empty() {
        let msg = if d <= 2 {
            "Há vestígios físicos (ex.: manchas, cabelo): use luvas, sinalize com placa numerada, fotografe e comunique à equipe de local — não colete nem limpe."
        } else {
            "Preserve vestígios não digitais que encontrar."
        };
        case.briefing.push(msg.to_string());
    }
    case.vestigios = out;
}
