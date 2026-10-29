//! Ferramentas investigativas e comandos do terminal que operam SOBRE o modelo virtual
//! (evidências e rede emulada). Nenhuma delas acessa rede real: tudo é resolvido no caso.

use crate::engine::{Note, Output, Sim};
use crate::model::*;
use crate::util::*;
use crate::vfs;

pub fn help(topic: &str) -> String {
    match topic {
        "fs" => "\
Sistema de arquivos (sobre a IMAGEM adquirida):
  fs montar <mídia>           disponibiliza a imagem para exame
  fs ls [caminho]             lista diretórios/arquivos (inclui apagados marcados com *)
  fs cat <caminho>            mostra o conteúdo de um arquivo
  fs meta <caminho>           metadados (datas, dono, câmera, GPS, hash)
  fs buscar <termo>           procura em nomes e conteúdos
  recuperar <mídia> <caminho> tenta recuperar arquivo apagado
  expandir <mídia> <caminho>  expande arquivo composto (zip)
  decifrar <alvo> <senha>     abre volume/arquivo cifrado
  ocr <mídia> <caminho>       extrai texto de imagem/áudio
  dicionario <item>           gera candidatos de senha a partir de dados do caso"
            .into(),
        "rede" => "\
Rede emulada (internet fictícia, sem acesso externo):
  conexao <tipo>              institucional | dedicada | anonima
  dns <nome>                  consulta registros DNS
  whois <domínio>             dados de registro do domínio
  traceroute <ip|nome>        rota simulada até o destino
  navegar <url>               abre uma página da rede emulada
  preservar <url>             captura e preserva a página (com hash)
  arquivo-web <url>           consulta registro histórico da página
  email cabecalho <arquivo>   analisa o cabeçalho de um .eml
  rede logs <ip>              registros de conexão do provedor (CGNAT)"
            .into(),
        "laudo" => "\
Documento final:
  laudo                       mostra o rascunho atual
  laudo material <texto>      descreve o material examinado
  laudo exame <texto>         descreve os exames realizados
  laudo origem <texto>        caminho/origem da expressão de interesse
  laudo conclusao <texto>     conclusão fundamentada
  laudo consideracoes <texto> considerações técnicas e limitações
  laudo auto                  pré-preenche campos a partir das ações e anotações
  finalizar                   fecha o caso e abre a avaliação"
            .into(),
        _ => "\
COMANDOS (digite 'ajuda <tema>' para detalhes: fs, rede, laudo)

Situação:        caso · itens · item <id> · tempo · notas · custodia · auditoria · progresso
Físico:          fotografar · lacre conferir|aplicar · deslacrar · abrir · componente remover
                 midia listar|remover · bloqueio · imagem · sanitizar · clone · hash · verificar-hash
                 energia desligar · memoria · volume copiar · rede isolar · inspecionar <local|item>
Móveis:          isolar <item> faraday|aluminio · airplane · sim remover · alarmes · carregar
                 camera cobrir <item> · solicitar-senhas <item> · imei verificar <item>
                 desbloquear <item> <senha> · extrair <item> fisica|fs|logica · senha
                 tela <item> inicio|mensagens|telefone|contatos|galeria|arquivos|relogio|config|imei|foto
Coleta:          embalar · etiquetar
Inicialização:   bios <item> · bios ordem <item> forense|disco
Nuvem:           nuvem [status] · nuvem solicitar <conta> · nuvem acessar <conta> <senha> · nuvem ls · nuvem cat <arquivo>
Investigação:    fs ... · recuperar · expandir · decifrar · ocr · dicionario
                 conexao · dns · whois · traceroute · navegar · preservar · arquivo-web · email · rede logs
Relatório:       anotar <texto> · achei <expressão> · laudo ... · finalizar"
            .into(),
    }
}

pub fn describe_case(s: &Sim) -> String {
    let c = &s.case;
    let mut out = String::new();
    out.push_str(&format!("CASO {} — {}\n", c.id, c.title));
    out.push_str(&format!("Módulo POP {} — {}\n", c.module.code(), c.module.title()));
    out.push_str(&format!("Dificuldade: {}/4   Semente: {}   Gerador: v{}\n", c.difficulty, c.seed, c.gen_version));
    out.push_str(&format!("Procedimento: {}   Requisitante: {}\n\n", c.procedimento, c.requisitante));
    out.push_str(&format!("HISTÓRICO\n{}\n\n", wrap(&c.historico, 76).join("\n")));
    out.push_str(&format!("OBJETIVO\n{}\n\n", wrap(&c.objetivo, 76).join("\n")));
    if !c.quesitos.is_empty() {
        out.push_str("QUESITOS\n");
        for q in &c.quesitos {
            out.push_str(&format!("{}\n", wrap(q, 76).join("\n  ")));
        }
        out.push('\n');
    }
    if c.legal.mandado {
        out.push_str(&format!("AUTORIZAÇÃO LEGAL\n{}\n\n", wrap(&c.legal.mandado_texto, 76).join("\n")));
    }
    if !c.keyword.is_empty() {
        out.push_str("OBJETIVO ADICIONAL: localizar a expressão de interesse oculta entre os vestígios,\nidentificar sua origem e fundamentá-la no laudo.\n\n");
    }
    out.push_str("ORIENTAÇÕES\n");
    for b in &c.briefing {
        out.push_str(&format!("  • {}\n", wrap(b, 72).join("\n    ")));
    }
    out
}

pub fn inventory(s: &Sim) -> String {
    let mut rows = vec![];
    for it in &s.case.items {
        if it.hidden && !it.flag("descoberto") {
            continue;
        }
        let estado = if it.kind.is_mobile() {
            it.phone.as_ref().map(|p| p.boot.label().to_string()).unwrap_or_default()
        } else if it.kind.is_computer() {
            if it.power_on { "ligado".into() } else { "desligado".into() }
        } else {
            "—".into()
        };
        rows.push(vec![it.id.clone(), it.kind.label().to_string(), it.label.clone(), estado]);
    }
    let mut out = table(&["ID", "Tipo", "Descrição", "Estado"], &rows);
    out.push_str("\n\nUse 'item <id>' para detalhes. Mídias são examinadas após 'imagem'/'extrair' e 'fs montar'.");
    if s.case.items.iter().any(|i| i.hidden && !i.flag("descoberto")) {
        out.push_str("\n(Pode haver mídias ocultas não listadas — inspecione os equipamentos.)");
    }
    out
}

pub fn item_detail(s: &mut Sim, id: &str) -> Output {
    let Some(it) = s.case.item(id).cloned() else { return Output::err("Item não encontrado.") };
    let mut out = String::new();
    out.push_str(&format!("ITEM {} — {}\n", it.id, it.label));
    out.push_str(&format!("  {} marca {}, modelo {}, série {}\n", it.kind.label(), it.brand, it.model, it.serial));
    if let Some(o) = &it.owner {
        out.push_str(&format!("  Atribuído a: {}\n", o));
    }
    if let Some(seal) = &it.seal {
        out.push_str(&format!("  Lacre: {} ({})\n", seal.number, if seal.intact { "íntegro" } else { "rompido" }));
    }
    if let Some(d) = &it.doc_seal {
        out.push_str(&format!("  Lacre na documentação: {}\n", d));
    }
    if it.kind.is_computer() {
        out.push_str(&format!("  Energia: {}\n", if it.power_on { "LIGADO" } else { "desligado" }));
    }
    if !it.physical.traces.is_empty() {
        out.push_str(&format!("  Vestígios: {}\n", it.physical.traces.join("; ")));
    }
    if it.physical.wet {
        out.push_str("  CONDIÇÃO: molhado — risco elétrico; secar/isolar e sinalizar.\n");
    }
    if it.physical.battery_swollen {
        out.push_str("  CONDIÇÃO: bateria estufada — risco; manuseio cuidadoso.\n");
    }
    if !it.physical.damage.is_empty() {
        out.push_str(&format!("  Danos: {}\n", it.physical.damage.join("; ")));
    }
    if let Some(cf) = &it.physical.camouflaged {
        out.push_str(&format!("  Observação física: {}\n", cf));
    }
    if !it.components.is_empty() {
        out.push_str("  Componentes:\n");
        for c in &it.components {
            out.push_str(&format!("    - {} [{}] ({})\n", c.label, c.id, if c.removed { "removido" } else { "montado" }));
        }
    }
    if !it.slots.is_empty() {
        out.push_str("  Slots:\n");
        for sl in &it.slots {
            let occ = sl.media_id.as_deref().unwrap_or("vazio");
            let hid = if sl.hidden_by.is_some() { " [OCULTO]" } else { "" };
            out.push_str(&format!("    - {} [{}] ({}) → {}{}\n", sl.label, sl.id, sl.iface, occ, hid));
        }
    }
    if let Some(ph) = &it.phone {
        out.push_str("  Telefonia:\n");
        out.push_str(&format!("    SO: {} {}   Estado: {}\n", ph.os, ph.os_version, ph.boot.label()));
        out.push_str(&format!("    IMEI: {}\n", ph.imeis.join(" / ")));
        out.push_str(&format!("    Bateria: {:.0}%  {}  Isolamento: {}\n", ph.battery, if ph.charging { "(carregando)" } else { "" }, ph.rf.label()));
        for sim in &ph.sims {
            let st = match sim.place { SimWhere::Slot => "no slot", SimWhere::Removido => "removido", SimWhere::AfixadoVerso => "afixado ao verso" };
            out.push_str(&format!("    SIM slot {}: {} — ICCID {} ({})\n", sim.slot, sim.operadora, sim.iccid, st));
        }
        if let Some((icc, op)) = &ph.esim {
            out.push_str(&format!("    eSIM: {} — ICCID {}\n", op, icc));
        }
        if !ph.alarms.is_empty() {
            out.push_str(&format!("    Alarmes: {} {}\n", ph.alarms.join(", "), if ph.alarms_disabled { "(desabilitados)" } else { "" }));
        }
    }
    if let Some(live) = &it.live {
        out.push_str("  Estado VOLÁTIL (equipamento ligado):\n");
        out.push_str(&format!("    Usuário: {}   Tela: {}\n", live.logged_user, live.screen));
        if let Some((pid, letra)) = &live.mounted_encrypted {
            out.push_str(&format!("    Volume cifrado MONTADO: {} em {} (chave só na memória)\n", pid, letra));
        }
        out.push_str(&format!("    Rede: {}\n", if live.network_connected { "CONECTADO (risco de acesso remoto)" } else { "isolado" }));
        if !live.processes.is_empty() {
            out.push_str("    Processos:\n");
            for (n, pid, note) in &live.processes {
                out.push_str(&format!("      {} (pid {}){}\n", n, pid, if note.is_empty() { String::new() } else { format!(" — {}", note) }));
            }
        }
    }
    let flags: Vec<String> = it.flags.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    if !flags.is_empty() {
        out.push_str(&format!("  Estado procedimental: {}\n", flags.join(", ")));
    }
    Output::ok(out)
}

pub fn notes(s: &Sim) -> String {
    if s.notes.is_empty() {
        return "Nenhuma anotação ainda. Use 'anotar <texto>'.".into();
    }
    let mut out = String::new();
    for n in &s.notes {
        out.push_str(&format!("[{}] {}\n", fmt_dt(n.at, BRT), n.text));
    }
    out
}

pub fn custody(s: &Sim) -> String {
    let mut out = String::from("CADEIA DE CUSTÓDIA / MOVIMENTAÇÕES\n");
    for e in &s.audit.entries {
        if matches!(e.action.as_str(), "lacre-conferir" | "lacre-aplicar" | "deslacrar" | "midia-remover" | "sim-remover" | "embalar" | "etiquetar" | "aquisicao" | "espelhamento" | "extracao" | "memoria" | "isolar-rf") {
            out.push_str(&format!("[{}] {} — {}\n", fmt_dt(e.sim_time, BRT), e.action, e.detail));
        }
    }
    out.push_str("\nARTEFATOS PRODUZIDOS\n");
    if s.artifacts.is_empty() {
        out.push_str("  (nenhum)\n");
    }
    for a in &s.artifacts {
        out.push_str(&format!("  {} {} ← {}  (SHA-256 {}…)\n", a.id, a.kind, a.origin, &a.hash[..12]));
    }
    out
}

// ----------------------------------------------------------------- fs

pub fn fs(s: &mut Sim, toks: &[String]) -> Output {
    let sub = toks.get(1).map(|x| x.as_str()).unwrap_or("");
    match sub {
        "montar" => {
            let mid = toks.get(2).cloned().unwrap_or_default();
            let Some(m) = s.case.media_by_id(&mid).cloned() else { return Output::err("Mídia não encontrada.") };
            if !s.imaged.contains(&m.id) {
                if s.hashes.contains_key(&format!("fdeclone:{}", m.id)) {
                    return Output::err(format!("O clone de {} contém dados cifrados por hardware: desbloqueie-o no equipamento de origem ('bios ordem <item> forense', 'decifrar {} <senha>').", m.id, m.id));
                }
                return Output::err(format!("A mídia {} ainda não foi adquirida. Faça 'imagem {}' (ou 'extrair' no caso de móvel) antes de examinar — o exame é sobre a cópia.", m.id, m.id));
            }
            s.rec_pub("fs-montar", &m.id, "imagem disponível para exame");
            Output::ok(format!("Imagem de {} montada para exame. Use 'fs ls /'.", m.id))
        }
        "ls" => {
            let path = toks.get(2).cloned().unwrap_or_else(|| "/".into());
            fs_ls(s, &path)
        }
        "cat" => {
            let path = toks.get(2).cloned().unwrap_or_default();
            fs_cat(s, &path)
        }
        "meta" => {
            let path = toks.get(2).cloned().unwrap_or_default();
            fs_meta(s, &path)
        }
        "buscar" => {
            let term = toks[2..].join(" ");
            fs_buscar(s, &term)
        }
        _ => Output::err("Uso: fs montar|ls|cat|meta|buscar ..."),
    }
}

fn mounted_media(s: &Sim) -> Vec<Media> {
    s.case.media.iter().filter(|m| s.imaged.contains(&m.id)).cloned().collect()
}

fn fs_ls(s: &mut Sim, path: &str) -> Output {
    let meds = mounted_media(s);
    if meds.is_empty() {
        return Output::err("Nenhuma imagem montada. Use 'fs montar <mídia>'.");
    }
    let mut out = String::new();
    for m in &meds {
        for p in &m.partitions {
            // partições cifradas não listam sem decifrar
            if let Some(e) = &p.encrypted {
                if !s.decrypted.contains(&p.id) {
                    out.push_str(&format!("[{}] partição {} ({}) — CIFRADA ({}). Use 'decifrar {} <senha>'.\n", m.id, p.label, p.fs, e.scheme, p.id));
                    continue;
                }
            }
            let mut listed = false;
            for f in &p.files {
                if f.path.to_lowercase().starts_with(&path.to_lowercase()) || path == "/" {
                    let mark = if f.deleted { "*" } else { " " };
                    let kind = if f.container.is_some() { "[cifrado]" } else if !f.children.is_empty() { "[composto]" } else { f.ftype.label() };
                    out.push_str(&format!(" {} {:>10}  {}  {}  {}\n", mark, fmt_bytes(f.size), fmt_date(f.mtime, p.tz_offset), kind, f.path));
                    listed = true;
                }
            }
            let total = vfs::total_files(p);
            if listed || path == "/" {
                out.push_str(&format!("   … e {} arquivo(s) de sistema/sem interesse em {} (total {}).\n", p.noise, p.label, total));
            }
        }
    }
    if out.is_empty() {
        out = format!("Nada encontrado em '{}'.", path);
    } else {
        out.push_str("\n(* = arquivo apagado; use 'recuperar <mídia> <caminho>')");
    }
    s.rec_pub("fs-ls", path, "listagem");
    Output::ok(out)
}

fn fs_cat(s: &mut Sim, path: &str) -> Output {
    let meds = mounted_media(s);
    let open = s.open_set();
    for m in &meds {
        if let Some((p, f)) = vfs::find_file_open(m, path, &|id| open.contains(id)) {
            if let Some(e) = &p.encrypted {
                if !s.decrypted.contains(&p.id) {
                    let _ = e;
                    return Output::err("Arquivo em partição cifrada não decifrada.");
                }
            }
            if f.deleted {
                return Output::err(format!("Arquivo apagado. Use 'recuperar {} {}'.", m.id, path));
            }
            if f.bad_sector {
                return Output::err("Arquivo situado em região de setores ilegíveis — conteúdo não recuperável por leitura direta.");
            }
            if f.container.is_some() {
                return Output::err("Arquivo é um contêiner cifrado. Use 'decifrar <caminho> <senha>'.");
            }
            if !f.children.is_empty() {
                return Output::err("Arquivo composto. Use 'expandir <mídia> <caminho>'.");
            }
            let kw = f.keyword;
            let content = f.content.clone();
            let fid = f.id.clone();
            let fpath = f.path.clone();
            let mid = m.id.clone();
            s.rec_pub("fs-cat", path, "conteúdo exibido");
            if kw {
                let src = SourceRef::File { media: mid.clone(), file: fid, path: fpath.clone() };
                register_keyword_hit(s, &src, &format!("arquivo {} (mídia {})", fpath, mid));
            }
            return Output::ok(format!("── {} ──\n{}", path, content));
        }
    }
    Output::err("Arquivo não encontrado (verifique o caminho com 'fs ls').")
}

fn fs_meta(s: &mut Sim, path: &str) -> Output {
    let meds = mounted_media(s);
    let open = s.open_set();
    for m in &meds {
        if let Some((p, f)) = vfs::find_file_open(m, path, &|id| open.contains(id)) {
            let mut out = format!("METADADOS — {}\n", path);
            out.push_str(&format!("  Tipo: {}   Tamanho: {}\n", f.ftype.label(), fmt_bytes(f.size)));
            out.push_str(&format!("  Dono: {}\n", f.owner));
            out.push_str(&format!("  Modificado: {}\n  Criado:     {}\n  Acessado:   {}\n", fmt_dtz(f.mtime, p.tz_offset), fmt_dtz(f.ctime, p.tz_offset), fmt_dtz(f.atime, p.tz_offset)));
            if p.tz_offset != BRT {
                out.push_str(&format!("  ATENÇÃO (fuso): a partição usa {}. {}\n", fmt_offset(p.tz_offset), p.dst_note.clone().unwrap_or_default()));
            }
            out.push_str(&format!("  Estado: {}\n", if f.deleted { "APAGADO" } else { "ativo" }));
            for (k, v) in &f.meta {
                out.push_str(&format!("  {}: {}\n", k, v));
            }
            out.push_str(&format!("  SHA-256 (conteúdo): {}\n", vfs::file_hash(f)));
            s.rec_pub("fs-meta", path, "metadados");
            return Output::ok(out);
        }
    }
    Output::err("Arquivo não encontrado.")
}

fn fs_buscar(s: &mut Sim, term: &str) -> Output {
    if term.is_empty() {
        return Output::err("Uso: fs buscar <termo>");
    }
    let meds = mounted_media(s);
    let t = fold(term);
    let mut hits = vec![];
    for m in &meds {
        for p in &m.partitions {
            if p.encrypted.is_some() && !s.decrypted.contains(&p.id) {
                continue;
            }
            collect_hits(&p.files, &t, &mut hits, &s.decrypted, &s.expanded);
        }
    }
    s.rec_pub("fs-buscar", term, &format!("{} ocorrência(s)", hits.len()));
    if hits.is_empty() {
        return Output::ok(format!("Nenhuma ocorrência de '{}' nas imagens montadas (partições cifradas não entram na busca).", term));
    }
    let mut out = format!("Ocorrências de '{}':\n", term);
    for (path, where_) in &hits {
        out.push_str(&format!("  {} — {}\n", path, where_));
    }
    Output::ok(out)
}

fn collect_hits(files: &[VFile], term: &str, hits: &mut Vec<(String, String)>, dec: &std::collections::BTreeSet<String>, exp: &std::collections::BTreeSet<String>) {
    for f in files {
        if f.deleted {
            continue;
        }
        if fold(&f.path).contains(term) {
            hits.push((f.path.clone(), "nome do arquivo".into()));
        }
        if f.container.is_none() && f.children.is_empty() && fold(&f.content).contains(term) {
            hits.push((f.path.clone(), "conteúdo".into()));
        }
        if let Some(ocr) = &f.ocr {
            if fold(ocr).contains(term) {
                hits.push((f.path.clone(), "texto por OCR".into()));
            }
        }
        if !f.children.is_empty() && exp.contains(&f.id) {
            collect_hits(&f.children, term, hits, dec, exp);
        }
        if let Some(c) = &f.container {
            if dec.contains(&c.id) {
                collect_hits(&c.files, term, hits, dec, exp);
            }
        }
    }
}

pub fn recuperar(s: &mut Sim, mid: &str, path: &str) -> Output {
    let Some(m) = s.case.media_by_id(mid).cloned() else { return Output::err("Uso: recuperar <mídia> <caminho>") };
    if !s.imaged.contains(&m.id) {
        return Output::err("Examine a imagem adquirida (não o original). Faça 'imagem'/'extrair' antes.");
    }
    let open = s.open_set();
    if let Some((_, f)) = vfs::find_file_open(&m, path, &|id| open.contains(id)) {
        if f.bad_sector {
            s.rec_pub("recuperar", path, "em setor ilegível — inviável");
            return Output::err("Arquivo em região de setores ilegíveis — recuperação por leitura direta inviável.");
        }
        let kw = f.keyword;
        let fid = f.id.clone();
        let fpath = f.path.clone();
        let mid2 = m.id.clone();
        let r = vfs::recover(f);
        return match r {
            vfs::RecoverResult::Ok(c) => {
                s.rec_pub("recuperar", path, "recuperação integral");
                if kw {
                    let src = SourceRef::File { media: mid2.clone(), file: fid, path: fpath.clone() };
                    register_keyword_hit(s, &src, &format!("arquivo apagado recuperado {} (mídia {})", fpath, mid2));
                }
                Output::ok(format!("Arquivo recuperado integralmente — {}:\n{}", path, c))
            }
            vfs::RecoverResult::Partial(c) => {
                s.rec_pub("recuperar", path, "recuperação parcial");
                if kw {
                    let src = SourceRef::File { media: mid2.clone(), file: fid, path: fpath.clone() };
                    register_keyword_hit(s, &src, &format!("arquivo apagado (parcial) {} (mídia {})", fpath, mid2));
                }
                Output::ok(format!("Recuperação PARCIAL — {}:\n{}", path, c))
            }
            vfs::RecoverResult::Failed(e) => {
                s.rec_pub("recuperar", path, "inviável");
                Output::err(e)
            }
        };
    }
    Output::err("Arquivo não encontrado para recuperação.")
}

pub fn expandir(s: &mut Sim, mid: &str, path: &str) -> Output {
    let Some(m) = s.case.media_by_id(mid).cloned() else { return Output::err("Uso: expandir <mídia> <caminho>") };
    if !s.imaged.contains(&m.id) {
        return Output::err("Examine a imagem adquirida. Faça 'imagem'/'extrair' antes.");
    }
    let open = s.open_set();
    if let Some((_, f)) = vfs::find_file_open(&m, path, &|id| open.contains(id)) {
        if f.children.is_empty() {
            return Output::err("Arquivo não é composto (nada a expandir).");
        }
        s.expanded.insert(f.id.clone());
        let mut out = format!("Conteúdo de {}:\n", path);
        for c in &f.children {
            out.push_str(&format!("  {}  ({}, {})\n", c.path, c.ftype.label(), fmt_bytes(c.size)));
        }
        out.push_str("Os itens internos podem ser lidos com 'fs cat <caminho interno>' e entram na busca.\n");
        // exibe os conteúdos internos de texto diretamente
        for c in &f.children {
            if c.content.len() < 2000 && c.children.is_empty() {
                out.push_str(&format!("\n── {} ──\n{}\n", c.path, c.content));
                if c.keyword {
                    let src = SourceRef::File { media: m.id.clone(), file: c.id.clone(), path: format!("{}#{}", path, c.path) };
                    register_keyword_hit(s, &src, &format!("arquivo interno {} em {} (mídia {})", c.path, path, m.id));
                }
            }
        }
        s.rec_pub("expandir", path, "arquivo composto expandido");
        return Output::ok(out);
    }
    Output::err("Arquivo não encontrado.")
}

pub fn decifrar(s: &mut Sim, target: &str, pass: &str) -> Output {
    if pass.is_empty() {
        return Output::err("Uso: decifrar <alvo> <senha>  (alvo: caminho do contêiner, id de partição cifrada ou mídia)");
    }
    // disco cifrado por hardware: desbloqueio do clone no equipamento de origem
    if let Some(o) = s.unlock_fde(target, pass) {
        return o;
    }
    // partição cifrada de mídia (ex.: BitLocker-like) ou partição interna
    let meds: Vec<Media> = s.case.media.iter().filter(|m| s.imaged.contains(&m.id)).cloned().collect();
    if meds.is_empty() {
        return Output::err("Nenhuma imagem adquirida. A decifragem é feita sobre a cópia.");
    }
    for m in &meds {
        for p in &m.partitions {
            if p.encrypted.as_ref().map(|e| e.tpm).unwrap_or(false) && p.id.eq_ignore_ascii_case(target) {
                // volume vinculado ao equipamento: mesmo com a senha, a imagem isolada não decifra
                let e = p.encrypted.as_ref().unwrap();
                if pass != e.password {
                    s.rec_pub("decifrar", &p.id, "senha incorreta");
                    return Output::err("Senha incorreta para este volume.");
                }
                s.decrypted.insert(p.id.clone());
                s.rec_pub("decifrar", &p.id, "volume vinculado ao equipamento decifrado com a senha informada (protetor de recuperação)");
                return Output::ok(format!("Volume {} decifrado com a senha constante na documentação. Registre a origem da senha no laudo.", p.id));
            }
            if (p.id.eq_ignore_ascii_case(target)) && p.encrypted.is_some() {
                let e = p.encrypted.as_ref().unwrap();
                if pass == e.password {
                    s.decrypted.insert(p.id.clone());
                    s.rec_pub("decifrar", &p.id, "volume decifrado com a senha informada");
                    return Output::ok(format!("Partição {} decifrada. Use 'fs ls' para ver o conteúdo.", p.id));
                } else {
                    s.rec_pub("decifrar", &p.id, "senha incorreta");
                    return Output::err("Senha incorreta para este volume.");
                }
            }
        }
        // contêiner como arquivo
        if let Some((_, f)) = vfs::find_file(m, target) {
            if let Some(cont) = &f.container {
                if Some(pass.to_string()) == f.password {
                    s.decrypted.insert(cont.id.clone());
                    s.decrypted.insert(f.id.clone());
                    let inner: Vec<String> = cont.files.iter().map(|x| format!("  {} ({})", x.path, x.ftype.label())).collect();
                    // marca keyword em arquivos internos quando lidos; aqui registramos disponibilidade
                    s.rec_pub("decifrar", target, "contêiner aberto com a senha informada");
                    let cid = cont.id.clone();
                    let files = cont.files.clone();
                    let mid = m.id.clone();
                    let tpath = target.to_string();
                    let mut extra = String::new();
                    for cf in &files {
                        if cf.keyword {
                            let src = SourceRef::File { media: mid.clone(), file: cf.id.clone(), path: format!("{}#{}", tpath, cf.path) };
                            register_keyword_hit(s, &src, &format!("arquivo {} dentro do contêiner {} (mídia {})", cf.path, tpath, mid));
                            extra = format!("\n{}", s.keyword_where.clone());
                        }
                    }
                    let _ = (cid, extra);
                    return Output::ok(format!("Contêiner aberto. Conteúdo:\n{}\nLeia com 'fs cat <caminho interno>'.", inner.join("\n")));
                } else {
                    s.rec_pub("decifrar", target, "senha incorreta");
                    return Output::err("Senha incorreta para o contêiner. Dica: 'dicionario <item>' pode sugerir candidatos.");
                }
            }
        }
    }
    Output::err("Alvo cifrado não encontrado.")
}

pub fn ocr(s: &mut Sim, mid: &str, path: &str) -> Output {
    let Some(m) = s.case.media_by_id(mid).cloned() else { return Output::err("Uso: ocr <mídia> <caminho>") };
    if !s.imaged.contains(&m.id) {
        return Output::err("Examine a imagem adquirida. Faça 'imagem'/'extrair' antes.");
    }
    let open = s.open_set();
    if let Some((_, f)) = vfs::find_file_open(&m, path, &|id| open.contains(id)) {
        if let Some(text) = f.ocr.clone() {
            s.rec_pub("ocr", path, "texto extraído de imagem/áudio");
            return Output::ok(format!("Texto reconhecido em {}:\n{}", path, text));
        }
        return Output::err("Nenhum texto reconhecível neste arquivo.");
    }
    Output::err("Arquivo não encontrado.")
}

pub fn dicionario(s: &mut Sim, _id: &str) -> Output {
    // gera candidatos a partir de dados biográficos do investigado (técnica do POP 5.01 §4.5)
    let p = s.case.suspect().clone();
    let mut cands = vec![];
    let pet = &p.pet;
    let yy = p.birth_year % 100;
    cands.push(format!("{}{}", pet, p.birth_year));
    cands.push(format!("{}{:02}!", pet.to_lowercase(), yy));
    cands.push(format!("{}@{}", p.team.replace(' ', ""), p.birth_year));
    cands.push(crate::content::leet(pet) + &yy.to_string());
    cands.push(format!("{}#{}", pet, p.birth_year % 10000));
    cands.push(format!("{}{}", p.team.replace(' ', ""), yy));
    s.rec_pub("dicionario", &p.name, &format!("{} candidatos gerados", cands.len()));
    let mut out = String::from("Candidatos de senha a partir de dados do caso (nome do pet, nascimento, time):\n");
    for c in &cands {
        out.push_str(&format!("  {}\n", c));
    }
    out.push_str("Teste com 'decifrar <alvo> <candidato>'.");
    Output::ok(out)
}

// ----------------------------------------------------------------- rede emulada

fn net<'a>(s: &'a Sim) -> Option<&'a NetWorld> {
    s.case.net.as_ref()
}

pub fn dns(s: &mut Sim, name: &str) -> Output {
    let Some(n) = net(s) else { return Output::err("Não há componente de rede neste caso.") };
    if name.is_empty() {
        return Output::err("Uso: dns <nome|ip>");
    }
    let recs: Vec<&DnsRecord> = n.dns.iter().filter(|r| r.name.eq_ignore_ascii_case(name)).collect();
    if recs.is_empty() {
        s.rec_pub("dns", name, "sem registros");
        return Output::ok(format!("Nenhum registro DNS para '{}' na rede emulada.", name));
    }
    let mut out = format!("Registros DNS de {}:\n", name);
    for r in &recs {
        out.push_str(&format!("  {:<6} {:<30} TTL {}\n", r.rtype, r.value, r.ttl));
    }
    s.rec_pub("dns", name, &format!("{} registro(s)", recs.len()));
    Output::ok(out)
}

pub fn whois(s: &mut Sim, dom: &str) -> Output {
    let Some(n) = net(s) else { return Output::err("Não há componente de rede neste caso.") };
    let Some(d) = n.domains.iter().find(|d| d.name.eq_ignore_ascii_case(dom)) else {
        s.rec_pub("whois", dom, "sem registro");
        return Output::err("Domínio não encontrado na rede emulada.");
    };
    let r = &d.registrant;
    let mut out = format!("WHOIS — {}\n", d.name);
    out.push_str(&format!("  Registrar: {}\n", d.registrar));
    out.push_str(&format!("  Criado:   {}\n  Expira:   {}\n", fmt_date(d.created, BRT), fmt_date(d.expires, BRT)));
    out.push_str(&format!("  Titular:  {}\n  E-mail:   {}\n  Telefone: {}\n  País:     {}\n", r.name, r.email, r.phone, r.country));
    out.push_str(&format!("  Nameservers: {}\n", d.nameservers.join(", ")));
    if r.privacy {
        out.push_str("  OBS: dados protegidos por serviço de privacidade — titularidade não confirmada por aqui.\n");
    }
    out.push_str("  OBS: dados de WHOIS podem ser falsos; não identificam a pessoa por si sós.\n");
    s.rec_pub("whois", dom, "consulta realizada");
    Output::ok(out)
}

pub fn traceroute(s: &mut Sim, target: &str) -> Output {
    let Some(n) = net(s) else { return Output::err("Não há componente de rede neste caso.") };
    // resolve nome -> ip se necessário
    let ip = if target.chars().all(|c| c.is_ascii_digit() || c == '.') {
        target.to_string()
    } else {
        n.dns.iter().find(|r| r.name.eq_ignore_ascii_case(target) && r.rtype == "A").map(|r| r.value.clone()).unwrap_or_default()
    };
    if ip.is_empty() {
        return Output::err("Não foi possível resolver o destino. Faça 'dns <nome>' primeiro.");
    }
    let src = n.expert_ips.get(&s.net_conn).cloned().unwrap_or_default();
    let host = n.hosts.iter().find(|h| h.ip == ip);
    let mut out = format!("traceroute de {} ({}) até {}:\n", s.net_conn, src, ip);
    let mut hops = vec![("1".to_string(), "gw-perito.local".to_string(), "1 ms".to_string()), ("2".to_string(), "borda-isp.sim".to_string(), "8 ms".to_string())];
    if let Some(h) = host {
        hops.push(("3".to_string(), h.asn.clone(), "15 ms".to_string()));
        hops.push(("4".to_string(), h.ptr.clone().unwrap_or_else(|| ip.clone()), "16 ms".to_string()));
    } else {
        hops.push(("3".to_string(), "* * *".to_string(), "sem resposta".to_string()));
    }
    for (n_, host_, lat) in &hops {
        out.push_str(&format!("  {}  {:<32} {}\n", n_, host_, lat));
    }
    if let Some(h) = host {
        out.push_str(&format!("Destino: {} — {} ({})\n", h.org, h.geo, h.asn));
    }
    s.rec_pub("traceroute", &ip, "rota simulada");
    Output::ok(out)
}

pub fn navegar(s: &mut Sim, url: &str) -> Output {
    let Some(n) = net(s).cloned() else { return Output::err("Não há componente de rede neste caso.") };
    if url.is_empty() {
        return Output::err("Uso: navegar <url>");
    }
    let now = s.clock;
    let conn = s.net_conn.clone();
    let Some(page) = n.pages.iter().find(|p| p.url.eq_ignore_ascii_case(url)) else {
        s.rec_pub("navegar", url, "não encontrada");
        return Output::err("Página não encontrada na rede emulada (verifique a URL; páginas não indexadas aparecem em links de outras páginas).");
    };
    if let Some(ra) = page.removed_at {
        if now >= ra {
            return Output::err("A página saiu do ar (não está mais acessível). Consulte 'arquivo-web <url>'.");
        }
    }
    // host monitorado + conexão não anônima → alerta o investigado (consequência)
    let host_ip = n.dns.iter().find(|r| r.rtype == "A" && page.url.contains(&r.name)).map(|r| r.value.clone());
    if let Some(ip) = &host_ip {
        if let Some(h) = n.hosts.iter().find(|h| &h.ip == ip) {
            if h.monitors && conn != "anonima" && !s.suspect_alerted {
                s.suspect_alerted = true;
                s.add_fault_pub("Acesso a host monitorado por conexão não anônima pode ter alertado o investigado.");
                // agenda a retirada das páginas que somem quando alertado
                let delay = n.pages.iter().filter_map(|p| p.removed_if_alerted_after).min().unwrap_or(0);
                s.schedule_alert_removal(now + delay);
            }
        }
    }
    let kw = page.keyword;
    let body = page.body.clone();
    let title = page.title.clone();
    let links = page.links.clone();
    let purl = page.url.clone();
    let mut out = format!("── {} ──\n{}\n\n{}\n", purl, title, body);
    if !links.is_empty() {
        out.push_str(&format!("\nLinks: {}\n", links.join("  ")));
    }
    out.push_str("\n(Para preservar como evidência: 'preservar <url>'.)");
    s.rec_pub("navegar", &purl, "página acessada");
    if kw {
        let src = SourceRef::Page { url: purl.clone() };
        register_keyword_hit(s, &src, &format!("página {}", purl));
    }
    Output::ok(out)
}

pub fn preservar(s: &mut Sim, url: &str) -> Output {
    let Some(n) = net(s).cloned() else { return Output::err("Não há componente de rede neste caso.") };
    let Some(page) = n.pages.iter().find(|p| p.url.eq_ignore_ascii_case(url)) else {
        return Output::err("Página não encontrada para preservação.");
    };
    let now = s.clock;
    if page.removed_at.map(|ra| now >= ra).unwrap_or(false) {
        return Output::err("A página já saiu do ar; nada a preservar agora.");
    }
    let h = crate::sha256::sha256(&format!("{}|{}|{}", page.url, page.title, page.body));
    let ip = n.expert_ips.get(&s.net_conn).cloned().unwrap_or_default();
    let purl = page.url.clone();
    let aid = s.new_artifact_pub("pagina-preservada", &purl, &h, &format!("captura em {} via IP {}", fmt_dtz(now, BRT), ip));
    s.rec_pub("preservar", &purl, &format!("preservada ({}), SHA-256 {}…", aid, &h[..12]));
    Output::ok(format!(
        "Página preservada ({}).\n  URL: {}\n  Data/hora: {}\n  Conexão/IP usado: {} / {}\n  SHA-256 do conteúdo: {}",
        aid, purl, fmt_dtz(now, BRT), s.net_conn, ip, h
    ))
}

pub fn archive(s: &mut Sim, url: &str) -> Output {
    let Some(n) = net(s).cloned() else { return Output::err("Não há componente de rede neste caso.") };
    let snaps: Vec<&Snapshot> = n.archive.iter().filter(|a| a.url.eq_ignore_ascii_case(url)).collect();
    if snaps.is_empty() {
        return Output::err("Nenhum registro histórico para essa URL.");
    }
    let mut out = String::new();
    let mut kwsrc = None;
    for a in &snaps {
        out.push_str(&format!("── cópia histórica de {} em {} ──\n{}\n{}\n", a.url, fmt_dtz(a.at, BRT), a.title, a.body));
        if a.keyword {
            kwsrc = Some((SourceRef::Archive { url: a.url.clone() }, a.at));
        }
    }
    s.rec_pub("arquivo-web", url, &format!("{} cópia(s) histórica(s)", snaps.len()));
    if let Some((src, at)) = kwsrc {
        register_keyword_hit(s, &src, &format!("registro histórico de {} ({})", url, fmt_date(at, BRT)));
    }
    Output::ok(out)
}

pub fn net_logs(s: &mut Sim, ip: &str) -> Output {
    let Some(n) = net(s).cloned() else { return Output::err("Não há componente de rede neste caso.") };
    if ip.is_empty() {
        return Output::err("Uso: rede logs <ip>");
    }
    let mut out = String::new();
    let mut found = false;
    for isp in &n.isps {
        for r in &isp.logs {
            if r.public_ip == ip {
                found = true;
                out.push_str(&format!("Provedor: {} (ASN {})\n", isp.name, isp.asn));
                out.push_str(&format!("  IP público: {}\n", r.public_ip));
                if isp.cgnat {
                    out.push_str(&format!("  CGNAT: SIM — a atribuição exige a PORTA de origem e o horário.\n"));
                    out.push_str(&format!("  Faixa de portas: {}–{}\n", r.port_start, r.port_end));
                    out.push_str(&format!("  IP privado atrás do CGNAT: {}\n", r.private_ip));
                }
                out.push_str(&format!("  Período: {} a {}\n", fmt_dtz(r.start, BRT), fmt_dtz(r.end, BRT)));
                out.push_str(&format!("  Assinante: {}\n", r.subscriber));
            }
        }
    }
    s.rec_pub("rede-logs", ip, if found { "registros encontrados" } else { "sem registros" });
    if !found {
        return Output::ok("Nenhum registro de conexão para esse IP.");
    }
    out.push_str("\nATENÇÃO: o registro aponta uma CONEXÃO/assinante, não necessariamente o indivíduo que praticou o ato.\n");
    Output::ok(out)
}

pub fn email(s: &mut Sim, toks: &[String]) -> Output {
    let sub = toks.get(1).map(|x| x.as_str()).unwrap_or("");
    if sub != "cabecalho" && sub != "headers" {
        return Output::err("Uso: email cabecalho <arquivo.eml>");
    }
    let name = toks.get(2).cloned().unwrap_or_default();
    let Some(f) = s.case.provided.iter().find(|f| f.name().eq_ignore_ascii_case(&name) || f.path.eq_ignore_ascii_case(&name)).cloned() else {
        let avail: Vec<String> = s.case.provided.iter().map(|f| f.name().to_string()).collect();
        return Output::err(format!("Arquivo .eml não encontrado. Disponíveis: {}", if avail.is_empty() { "(nenhum)".into() } else { avail.join(", ") }));
    };
    // separa cabeçalho do corpo
    let header_end = f.content.find("\n\n").unwrap_or(f.content.len());
    let headers = &f.content[..header_end];
    let mut out = format!("CABEÇALHO de {}:\n{}\n\n", f.name(), headers);
    out.push_str("LEITURA (campos Received, de baixo para cima = da origem ao destino):\n");
    let received: Vec<&str> = headers.lines().filter(|l| l.starts_with("Received:")).collect();
    for (i, r) in received.iter().rev().enumerate() {
        out.push_str(&format!("  salto {}: {}\n", i + 1, r.trim_start_matches("Received:").trim()));
    }
    if let Some(l) = headers.lines().find(|l| l.starts_with("X-Originating-IP")) {
        out.push_str(&format!("  {}\n", l));
    }
    if let Some(l) = headers.lines().find(|l| l.starts_with("Authentication-Results")) {
        out.push_str(&format!("  {}\n", l));
    }
    let arc: Vec<&str> = headers.lines().filter(|l| l.starts_with("ARC-")).collect();
    if !arc.is_empty() {
        out.push_str("  Cadeia ARC (registro de autenticação feito por intermediário):\n");
        for l in &arc {
            out.push_str(&format!("    {}\n", l));
        }
        out.push_str("  INTERPRETAÇÃO: a mensagem passou por um intermediário (lista/encaminhador) que a alterou; por isso o DKIM falha no destino.\n");
        out.push_str("  A cadeia ARC registra que, no intermediário, SPF e DKIM eram válidos — confiar nisso depende de confiar no intermediário que assinou o ARC.\n");
    }
    if headers.lines().any(|l| l.starts_with("X-Hop-Check")) {
        out.push_str("  ATENÇÃO: há inconsistência entre o IP de origem e o servidor de saída — verifique.\n");
    }
    out.push_str("\nCorrelacione o IP de origem com 'rede logs <ip>' e confira os horários/fusos.\n");
    s.rec_pub("email-cabecalho", f.name(), "cabeçalho analisado");
    if f.keyword {
        let src = SourceRef::Provided { file: f.name().to_string() };
        register_keyword_hit(s, &src, &format!("mensagem eletrônica {}", f.name()));
    }
    Output::ok(out)
}

// ----------------------------------------------------------------- laudo

pub fn laudo(s: &mut Sim, toks: &[String]) -> Output {
    let sub = toks.get(1).map(|x| x.as_str()).unwrap_or("");
    // "¶" representa quebra de linha (o editor envia campos multilinha por comando, mantendo o replay)
    let rest = toks[2.min(toks.len())..].join(" ").replace(" ¶ ", "\n").replace('¶', "\n");
    match sub {
        "" | "ver" => Output::ok(crate::report::preview(s)),
        "material" => { s.report.material = rest; Output::ok("Campo 'material' atualizado.") }
        "exame" => { s.report.exame = rest; Output::ok("Campo 'exame' atualizado.") }
        "origem" => { s.report.keyword_source = rest; Output::ok("Origem da expressão de interesse registrada.") }
        "conclusao" => { s.report.conclusao = rest; Output::ok("Campo 'conclusão' atualizado.") }
        "consideracoes" => { s.report.consideracoes = rest; Output::ok("Campo 'considerações' atualizado.") }
        "historico" => { s.report.historico = rest; Output::ok("Campo 'histórico' atualizado.") }
        "hipoteses" => { s.report.hipoteses = rest; Output::ok("Hipóteses registradas (aparecem separadas da conclusão, como não confirmadas).") }
        "auto" => { crate::report::autofill(s); Output::ok("Campos pré-preenchidos a partir das ações e anotações. Revise com 'laudo'.") }
        _ => Output::err("Uso: laudo [material|exame|origem|conclusao|consideracoes|hipoteses|historico|auto] <texto>"),
    }
}

// ----------------------------------------------------------------- apoio

pub(crate) fn register_keyword_hit(s: &mut Sim, src: &SourceRef, where_desc: &str) {
    let is_target = s.case.keyword_sources.iter().any(|k| k == src) || matches!(src, SourceRef::File{..}|SourceRef::Page{..}|SourceRef::Archive{..}|SourceRef::Cloud{..}|SourceRef::Provided{..}) && where_contains_keyword(s, src);
    if is_target {
        s.mark_keyword_found(where_desc);
        s.notes.push(Note { at: s.clock, text: format!("Expressão de interesse localizada em: {}", where_desc), source: Some(src.clone()) });
    }
}

fn where_contains_keyword(s: &Sim, src: &SourceRef) -> bool {
    s.case.keyword_sources.iter().any(|k| k == src)
}

// Métodos auxiliares expostos no Sim (para manter o dispatch em engine.rs enxuto).
impl Sim {
    pub fn rec_pub(&mut self, action: &str, detail: &str, outcome: &str) {
        let actor = self.actor.clone();
        self.audit.record(self.clock, &actor, action, detail, outcome);
    }
    pub fn add_fault_pub(&mut self, f: &str) {
        self.faults.push(format!("[{}] {}", fmt_dt(self.clock, BRT), f));
    }
    pub fn new_artifact_pub(&mut self, kind: &str, origin: &str, hash: &str, note: &str) -> String {
        self.artifact_seq += 1;
        let id = format!("A-{:03}", self.artifact_seq);
        self.artifacts.push(crate::engine::Artifact { id: id.clone(), kind: kind.into(), origin: origin.into(), hash: hash.into(), created: self.clock, note: note.into() });
        id
    }
    /// Conjunto de ids de contêineres/compostos abertos (decifrados ou expandidos).
    pub fn open_set(&self) -> std::collections::BTreeSet<String> {
        self.decrypted.union(&self.expanded).cloned().collect()
    }
    pub fn schedule_alert_removal(&mut self, at: i64) {
        let urls: Vec<String> = self.case.net.as_ref().map(|n| n.pages.iter().filter(|p| p.removed_if_alerted_after.is_some()).map(|p| p.url.clone()).collect()).unwrap_or_default();
        if let Some(n) = &mut self.case.net {
            for p in &mut n.pages {
                if p.removed_if_alerted_after.is_some() && p.removed_at.is_none() {
                    p.removed_at = Some(at);
                }
            }
        }
        let _ = urls;
    }
}

/// Linha do tempo correlacionada: arquivos das imagens examinadas, rede emulada e ações do perito.
pub fn timeline(s: &Sim) -> Vec<(i64, String, String)> {
    let mut ev: Vec<(i64, String, String)> = vec![];
    let open = s.open_set();
    fn walk(files: &[VFile], open: &std::collections::BTreeSet<String>, mid: &str, out: &mut Vec<(i64, String, String)>) {
        for f in files {
            let st = if f.deleted { " (apagado)" } else { "" };
            out.push((f.mtime, format!("arquivo {}", mid), format!("modificado: {}{}", f.path, st)));
            if !f.children.is_empty() && open.contains(&f.id) {
                walk(&f.children, open, mid, out);
            }
            if let Some(c) = &f.container {
                if open.contains(&c.id) {
                    walk(&c.files, open, mid, out);
                }
            }
        }
    }
    for m in s.case.media.iter().filter(|m| s.imaged.contains(&m.id)) {
        for p in &m.partitions {
            if p.encrypted.is_some() && !s.decrypted.contains(&p.id) {
                continue;
            }
            walk(&p.files, &open, &m.id, &mut ev);
        }
    }
    if let Some(n) = &s.case.net {
        if s.audit.has("whois") {
            for d in &n.domains {
                ev.push((d.created, "rede".into(), format!("domínio {} registrado", d.name)));
            }
        }
        if s.audit.has("arquivo-web") {
            for a in &n.archive {
                ev.push((a.at, "rede".into(), format!("cópia histórica de {}", a.url)));
            }
        }
        if s.audit.has("rede-logs") {
            for i in &n.isps {
                for l in &i.logs {
                    ev.push((l.start, "provedor".into(), format!("início de conexão {} ({})", l.public_ip, i.name)));
                }
            }
        }
    }
    if s.audit.has("email-cabecalho") {
        for f in &s.case.provided {
            ev.push((f.mtime, "e-mail".into(), format!("mensagem {} (data do cabeçalho)", f.name())));
        }
    }
    for e in &s.audit.entries {
        if !matches!(e.action.as_str(), "fs-ls" | "fs-cat" | "fs-meta" | "fs-buscar" | "anotar") {
            ev.push((e.sim_time, "perito".into(), format!("{} — {}", e.action, e.detail)));
        }
    }
    ev.sort_by_key(|x| x.0);
    ev
}

pub fn timeline_text(s: &Sim) -> String {
    let ev = timeline(s);
    if ev.is_empty() {
        return "Linha do tempo vazia.".into();
    }
    let mut out = String::from("LINHA DO TEMPO (horário de Brasília; arquivos convertidos do fuso da partição)\n");
    for (t, src, txt) in ev {
        out.push_str(&format!("{}  [{}] {}\n", fmt_dt(t, BRT), src, txt));
    }
    out
}
