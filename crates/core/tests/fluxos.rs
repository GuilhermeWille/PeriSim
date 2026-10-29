//! Testes de integração: geração reprodutível, consistência das evidências e
//! fluxos completos de cada módulo POP executados pelo mesmo `exec` usado pela interface.

use pericia_core::casegen::{case_id, generate_classic as generate, parse_case_id, GEN_VERSION};
use pericia_core::engine::Sim;
use pericia_core::model::*;
use pericia_core::{assess, report, vfs};

fn run(s: &mut Sim, cmds: &[&str]) -> Vec<String> {
    cmds.iter().map(|c| s.exec(c).text).collect()
}

fn json(c: &CaseFile) -> String {
    serde_json::to_string(c).unwrap()
}

// ------------------------------------------------------------------ geração

#[test]
fn mesma_semente_reproduz_o_caso() {
    for m in ModuleId::ALL {
        for d in 1..=4 {
            for seed in [1u32, 42, 0xDEADBEEF] {
                assert_eq!(json(&generate(m, d, seed)), json(&generate(m, d, seed)), "{:?} D{} s{}", m, d, seed);
            }
        }
    }
}

#[test]
fn sementes_diferentes_geram_casos_diferentes() {
    for m in ModuleId::ALL {
        let a = generate(m, 2, 10);
        let b = generate(m, 2, 11);
        assert_ne!(json(&a), json(&b));
    }
}

#[test]
fn id_do_caso_ida_e_volta() {
    let id = case_id(ModuleId::M504, 3, 0x00ABCDEF);
    assert_eq!(parse_case_id(&id), Some((ModuleId::M504, 3, 0x00ABCDEF)));
    assert_eq!(parse_case_id("lixo"), None);
    assert_eq!(GEN_VERSION, pericia_core::casegen::generate(ModuleId::M501, 1, 1).gen_version);
}

fn source_exists(c: &CaseFile, src: &SourceRef) -> bool {
    match src {
        SourceRef::File { media, path, .. } => {
            let Some(m) = c.media_by_id(media) else { return false };
            // caminhos "contêiner#interno"
            let leaf = path.rsplit('#').next().unwrap();
            vfs::find_file(m, leaf).map(|(_, f)| f.keyword).unwrap_or(false)
        }
        SourceRef::Page { url } => c.net.as_ref().map(|n| n.pages.iter().any(|p| &p.url == url && p.keyword)).unwrap_or(false),
        SourceRef::Archive { url } => c.net.as_ref().map(|n| n.archive.iter().any(|a| &a.url == url && a.keyword)).unwrap_or(false),
        SourceRef::Cloud { path } => c.net.as_ref().and_then(|n| n.cloud.as_ref()).map(|cl| cl.files.iter().any(|f| &f.path == path)).unwrap_or(false),
        SourceRef::Provided { file } => c.provided.iter().any(|f| f.name() == file && f.keyword),
        SourceRef::Ram { item } => c.item(item).and_then(|i| i.live.as_ref()).map(|l| l.ram_artifacts.iter().any(|a| a.2)).unwrap_or(false),
        SourceRef::Observed { .. } => true,
    }
}

#[test]
fn toda_fonte_da_palavra_chave_existe_e_contem_a_expressao() {
    for m in ModuleId::ALL {
        for d in 1..=4 {
            for seed in 0..40u32 {
                let c = generate(m, d, seed);
                if m == ModuleId::M505 {
                    assert!(c.keyword.is_empty(), "apreensão não deve ter palavra-chave");
                    assert_eq!(c.required_docs, vec![DocKind::TermoApreensao]);
                    continue;
                }
                assert!(!c.keyword_sources.is_empty(), "{} sem fontes", c.id);
                for s in &c.keyword_sources {
                    assert!(source_exists(&c, s), "{}: fonte inexistente {:?}", c.id, s);
                }
                // a expressão aparece literalmente em alguma fonte de conteúdo
                let blob = json(&c);
                assert!(blob.contains(&c.keyword), "{}: expressão ausente do caso", c.id);
                // a isca nunca é igual à expressão
                for d in &c.decoys {
                    assert_ne!(d, &c.keyword);
                }
            }
        }
    }
}

#[test]
fn horarios_coerentes() {
    for m in ModuleId::ALL {
        for seed in 0..20u32 {
            let c = generate(m, 3, seed);
            for med in &c.media {
                for p in &med.partitions {
                    for f in &p.files {
                        assert!(f.mtime <= c.start, "{}: arquivo {} modificado após o início do exame", c.id, f.path);
                    }
                }
            }
            if let Some(n) = &c.net {
                for d in &n.domains {
                    assert!(d.created < c.start && d.expires > d.created);
                }
                for a in &n.archive {
                    assert!(a.at < c.start);
                }
                // DNS A aponta para host existente
                for r in n.dns.iter().filter(|r| r.rtype == "A" && !r.name.starts_with("ns")) {
                    assert!(n.hosts.iter().any(|h| h.ip == r.value), "{}: A {} sem host", c.id, r.value);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ 5.01

#[test]
fn fluxo_501_d1_correto_aprova() {
    let mut s = Sim::new(generate(ModuleId::M501, 1, 7));
    let kw = s.case.keyword.clone();
    run(&mut s, &["fotografar item-01", "lacre conferir item-01", "deslacrar item-01", "bloqueio m-01", "imagem m-01", "verificar-hash m-01", "fs montar m-01"]);
    let busca = s.exec(&format!("fs buscar {}", kw.split(' ').nth(1).unwrap())).text;
    let path = busca.lines().nth(1).unwrap().trim().split(" — ").next().unwrap().to_string();
    let cat = s.exec(&format!("fs cat {}", path));
    assert!(cat.ok && cat.text.contains(&kw), "{}", cat.text);
    assert!(s.keyword_found);
    run(&mut s, &[&format!("achei {}", kw), "laudo auto", "lacre aplicar item-01"]);
    s.exec("laudo conclusao Foi localizado documento contendo a expressão de interesse na imagem forense da mídia M-01, com hash verificado.");
    let out = s.exec("finalizar").text;
    assert!(out.contains("APROVADO"), "{}", out);
    assert!(s.audit.verify().is_ok());
}

#[test]
fn exame_sem_imagem_e_bloqueado_e_sem_bloqueador_e_penalizado() {
    let mut s = Sim::new(generate(ModuleId::M501, 1, 7));
    let r = s.exec("fs montar m-01");
    assert!(!r.ok, "não pode examinar o original");
    s.exec("imagem m-01"); // sem bloqueador
    assert!(s.faults.iter().any(|f| f.contains("sem bloqueador")));
    let ev = assess::evaluate(&s);
    let pres = ev.iter().find(|c| c.name.starts_with("Preservação")).unwrap();
    assert!(pres.got < pres.max);
}

#[test]
fn fluxo_501_d2_recuperacao_e_composto() {
    for seed in 0..10 {
        let mut s = Sim::new(generate(ModuleId::M501, 2, seed));
        run(&mut s, &["fotografar item-01", "lacre conferir item-01", "deslacrar item-01", "bloqueio m-01", "imagem m-01", "fs montar m-01"]);
        let user = pericia_core::content::login(s.case.suspect());
        let p = format!("/Users/{}/Documents/entrega_final.docx", user);
        let cat = s.exec(&format!("fs cat {}", p));
        assert!(!cat.ok && cat.text.contains("apagado"));
        let rec = s.exec(&format!("recuperar m-01 {}", p));
        assert!(rec.ok, "{}", rec.text);
        assert!(s.keyword_found, "seed {}", seed);
        // composto: conteúdo interno só após expandir
        let z = format!("/Users/{}/Downloads/entrega.zip", user);
        assert!(!s.exec("fs cat entrega/resumo.txt").ok);
        assert!(s.exec(&format!("expandir m-01 {}", z)).ok);
        assert!(s.exec("fs cat entrega/resumo.txt").ok);
    }
}

#[test]
fn fluxo_501_d3_conteiner_cifrado_por_dicionario() {
    for seed in 0..25 {
        let mut s = Sim::new(generate(ModuleId::M501, 3, seed));
        if let Some(pw) = s.case.media[0].fde.as_ref().map(|e| e.password.clone()) {
            // gerador 1.1: parte dos casos D3 tem disco cifrado por hardware (clone + BIOS + senha)
            run(&mut s, &["bloqueio m-01", "sanitizar dest-01", "clone m-01 dest-01", "bios ordem item-01 forense", &format!("decifrar m-01 {}", pw), "fs montar m-01"]);
        } else {
            run(&mut s, &["bloqueio m-01", "imagem m-01", "fs montar m-01"]);
        }
        let user = pericia_core::content::login(s.case.suspect());
        let cont = format!("/Users/{}/Documents/backup_fotos.dat", user);
        // sem decifrar, nada do interior é legível
        let inner = s.case.keyword_sources.iter().find_map(|k| match k { SourceRef::File { path, .. } if path.contains('#') => Some(path.rsplit('#').next().unwrap().to_string()), _ => None }).unwrap();
        assert!(!s.exec(&format!("fs cat {}", inner)).ok, "conteúdo cifrado não pode vazar");
        let dic = s.exec("dicionario item-01").text;
        let cands: Vec<String> = dic.lines().skip(1).filter(|l| l.starts_with("  ")).map(|l| l.trim().to_string()).collect();
        let ok = cands.iter().any(|c| s.exec(&format!("decifrar {} {}", cont, c)).ok);
        assert!(ok, "seed {}: dicionário não contém a senha", seed);
        assert!(s.keyword_found);
        assert!(s.exec(&format!("fs cat {}", inner)).ok);
    }
}

#[test]
fn fluxo_501_d4_hash_documentado_confere() {
    let mut s = Sim::new(generate(ModuleId::M501, 4, 3));
    run(&mut s, &["bloqueio m-01", "imagem m-01"]);
    let v = s.exec("verificar-hash m-01").text;
    assert!(v.contains("Documentado antes") && !v.contains("DIVERGE"), "{}", v);
}

// ------------------------------------------------------------------ 5.02

#[test]
fn fluxo_502_isolado_extrai_e_encontra() {
    for d in 1..=3u8 {
        for seed in 0..10 {
            let mut s = Sim::new(generate(ModuleId::M502, d, seed));
            run(&mut s, &["fotografar item-01", "isolar item-01 faraday", "carregar item-01", "imei verificar item-01", "sim remover item-01 1"]);
            if let Some((_, pin)) = s.case.provided_passwords.first().cloned() {
                s.exec(&format!("desbloquear item-01 {}", pin));
            }
            let x = s.exec("extrair item-01");
            assert!(x.ok, "D{} seed {}: {}", d, seed, x.text);
            s.exec("fs montar m-01");
            let cat = s.exec("fs cat /data/data/sim.mensageiro/db/chat.txt");
            assert!(cat.ok, "{}", cat.text);
            assert!(s.keyword_found);
            assert!(!s.faults.iter().any(|f| f.contains("apagamento")), "{:?}", s.faults);
        }
    }
}

#[test]
fn aparelho_ligado_nao_isolado_e_apagado_remotamente() {
    let mut s = Sim::new(generate(ModuleId::M502, 2, 5));
    // ações demoradas sem isolar: o comando remoto chega
    for _ in 0..12 {
        s.exec("fotografar item-01");
        s.exec("item item-01");
        s.exec("lacre conferir item-01");
    }
    s.exec("extrair item-01");
    s.exec("extrair item-01");
    s.exec("extrair item-01");
    s.exec("extrair item-01");
    assert!(s.case.items[0].phone.as_ref().unwrap().wiped, "deveria ter sido apagado");
    assert!(!s.exec("extrair item-01").ok);
    let (_, _, destroyed) = assess::total(&assess::evaluate(&s), &s);
    assert!(destroyed);
}

#[test]
fn tentativas_de_senha_levam_a_apagamento_no_d4() {
    let mut s = Sim::new(generate(ModuleId::M502, 4, 1));
    s.exec("isolar item-01 faraday");
    for i in 0..10 {
        s.exec(&format!("desbloquear item-01 99999{}", i));
    }
    assert!(s.case.items[0].phone.as_ref().unwrap().wiped);
}

// ------------------------------------------------------------------ 5.03

#[test]
fn fluxo_503_coleta_volatil_correta() {
    for d in 1..=4u8 {
        for seed in 0..8 {
            let mut s = Sim::new(generate(ModuleId::M503, d, seed));
            let out = run(&mut s, &["fotografar item-01", "fotografar item-02", "rede isolar item-01", "memoria item-01", "volume copiar item-01", "energia desligar item-01"]);
            assert!(s.keyword_found, "D{} s{}: {:?}", d, seed, out);
            assert!(!s.faults.iter().any(|f| f.contains("perd") || f.contains("apag")), "{:?}", s.faults);
            assert!(s.exec("bloqueio m-01").ok && s.exec("imagem m-01").ok);
            if d >= 2 {
                s.exec("inspecionar local");
                s.exec("inspecionar item-02");
                s.exec("componente remover item-02 dissipador");
                let has_hidden = s.case.items.iter().any(|i| i.slots.iter().any(|x| x.hidden_by.is_some()));
                assert!(!has_hidden, "ainda há slot oculto");
                assert!(s.exec("bloqueio m-03").ok && s.exec("imagem m-03").ok);
            }
        }
    }
}

#[test]
fn desligar_com_volume_montado_perde_evidencia() {
    let mut s = Sim::new(generate(ModuleId::M503, 2, 1));
    s.exec("energia desligar item-01");
    assert!(s.faults.iter().any(|f| f.contains("perdid")));
    assert!(!s.exec("volume copiar item-01").ok);
}

#[test]
fn acesso_remoto_apaga_ram_se_nao_isolado() {
    let mut s = Sim::new(generate(ModuleId::M503, 3, 2));
    for _ in 0..40 {
        s.exec("fotografar item-02");
    }
    assert!(s.faults.iter().any(|f| f.contains("acesso remoto")), "{:?}", s.faults);
}

#[test]
fn midia_em_uso_nao_pode_ser_imageada_ligada() {
    let mut s = Sim::new(generate(ModuleId::M503, 1, 1));
    assert!(!s.exec("imagem m-01").ok);
}

// ------------------------------------------------------------------ 5.04

#[test]
fn fluxo_504_rede_emulada_coerente() {
    for d in 1..=4u8 {
        for seed in 0..8 {
            let mut s = Sim::new(generate(ModuleId::M504, d, seed));
            s.exec("conexao anonima");
            let dom = s.case.net.as_ref().unwrap().domains[0].name.clone();
            let eml = s.case.provided[0].name().to_string();
            let dnsr = s.exec(&format!("dns {}", dom));
            assert!(dnsr.ok && dnsr.text.contains(" A "), "{}", dnsr.text);
            let ip = s.case.net.as_ref().unwrap().dns.iter().find(|r| r.name == dom && r.rtype == "A").unwrap().value.clone();
            assert!(s.exec(&format!("traceroute {}", dom)).text.contains(&ip) || s.exec(&format!("traceroute {}", ip)).ok);
            assert!(s.exec(&format!("whois {}", dom)).ok);
            let h = s.exec(&format!("email cabecalho {}", eml)).text;
            let origin = h.lines().find(|l| l.contains("X-Originating-IP")).unwrap().split('[').nth(1).unwrap().trim_end_matches(']').to_string();
            let logs = s.exec(&format!("rede logs {}", origin)).text;
            assert!(logs.contains("Assinante"), "{}", logs);
            if d >= 3 {
                assert!(logs.contains("CGNAT"));
            }
            let home = format!("https://{}/", dom);
            let nav = s.exec(&format!("navegar {}", home));
            assert!(nav.ok);
            // o painel não indexado aparece como link na página inicial
            assert!(nav.text.contains("/painel"));
            assert!(s.exec(&format!("preservar {}", home)).ok);
            assert!(s.keyword_found);
            assert!(!s.suspect_alerted);
        }
    }
}

#[test]
fn diligencia_sem_anonimato_em_host_monitorado_alerta_e_derruba_pagina() {
    let mut s = Sim::new(generate(ModuleId::M504, 3, 4));
    let dom = s.case.net.as_ref().unwrap().domains[0].name.clone();
    let home = format!("https://{}/", dom);
    assert!(s.exec(&format!("navegar {}", home)).ok);
    assert!(s.suspect_alerted);
    for _ in 0..20 {
        s.exec("fotografar item-01");
    }
    assert!(!s.exec(&format!("navegar {}", home)).ok, "página deveria ter saído do ar");
    assert!(s.exec(&format!("arquivo-web {}", home)).ok, "registro histórico continua disponível");
}

// ------------------------------------------------------------------ 5.05

#[test]
fn fluxo_505_apreensao_gera_termo_nao_laudo() {
    let mut s = Sim::new(generate(ModuleId::M505, 2, 3));
    assert_eq!(s.case.module.document(), DocKind::TermoApreensao);
    run(&mut s, &["fotografar item-01", "camera cobrir item-01", "solicitar-senhas item-01", "isolar item-01 faraday", "carregar item-01", "sim remover item-01 1", "alarmes item-01", "embalar item-01", "lacre aplicar item-01", "etiquetar item-01 Celular do investigado"]);
    assert!(!s.case.provided_passwords.is_empty(), "senhas informadas devem constar no termo");
    let prog = pericia_core::assess::checklist(&s);
    assert!(prog.iter().all(|(ok, _)| *ok), "{:?}", prog);
    let doc = report::preview(&s);
    assert!(doc.contains("TERMO DE APREENSÃO"));
    assert!(!doc.contains("LAUDO PERICIAL"));
    assert!(s.exec("achei qualquer").text.contains("não tem expressão"));
    let out = s.exec("finalizar").text;
    assert!(!out.contains("expressão de interesse —"), "critério de palavra-chave não se aplica");
}

// ------------------------------------------------------------------ auditoria e relatório

#[test]
fn relatorio_incoerente_com_historico_e_detectado() {
    let mut s = Sim::new(generate(ModuleId::M501, 1, 2));
    s.exec("laudo exame Foram recuperados arquivos apagados e calculado o hash de integridade.");
    s.exec("laudo conclusao A análise prova que o investigado foi o autor.");
    let c = report::check_coherence(&s);
    assert!(c.problems.iter().any(|p| p.contains("hash")));
    assert!(c.problems.iter().any(|p| p.contains("recuperação")));
    assert!(c.problems.iter().any(|p| p.contains("categórica")));
}

#[test]
fn palavra_chave_por_acaso_nao_compensa_destruicao() {
    let mut s = Sim::new(generate(ModuleId::M503, 2, 1));
    let kw = s.case.keyword.clone();
    s.exec("energia desligar item-01"); // perde volume
    s.exec(&format!("achei {}", kw));
    s.exec("laudo conclusao Expressão localizada.");
    let ev = assess::evaluate(&s);
    let (got, max, destroyed) = assess::total(&ev, &s);
    assert!(destroyed && got * 100 / max <= 60);
}

#[test]
fn auditoria_detecta_adulteracao_apos_fluxo() {
    let mut s = Sim::new(generate(ModuleId::M501, 1, 2));
    run(&mut s, &["fotografar item-01", "bloqueio m-01", "imagem m-01"]);
    assert!(s.audit.verify().is_ok());
    s.audit.entries[2].outcome = "sem bloqueador? não...".into();
    assert!(s.audit.verify().is_err());
}

#[test]
fn markdown_destaca_palavra_chave_e_lista_hashes() {
    let mut s = Sim::new(generate(ModuleId::M501, 1, 9));
    let kw = s.case.keyword.clone();
    run(&mut s, &["bloqueio m-01", "imagem m-01", &format!("achei {}", kw), "laudo origem arquivo X", "laudo auto"]);
    let md = report::markdown(&s);
    assert!(md.contains(&format!("**«{}»**", kw)));
    assert!(md.contains("| A-001 | imagem-forense |"));
}

// ------------------------------------------------------------------ segurança / isolamento

#[test]
fn terminal_nao_executa_comandos_do_sistema() {
    let mut s = Sim::new(generate(ModuleId::M501, 1, 1));
    for c in ["rm -rf /", "bash -c id", "cmd.exe /c dir", "$(reboot)", "curl https://example.com"] {
        let r = s.exec(c);
        assert!(!r.ok && r.text.contains("desconhecido"), "{}", c);
    }
}

#[test]
fn motor_nao_usa_rede_nem_processos() {
    fn rs_files(d: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() { rs_files(&p, out) } else if p.extension().map(|x| x == "rs").unwrap_or(false) { out.push(p) }
        }
    }
    let mut files = vec![];
    rs_files(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")), &mut files);
    for p in files {
        let src = std::fs::read_to_string(&p).unwrap();
        for bad in ["std::net", "TcpStream", "UdpSocket", "std::process", "Command::new", "reqwest", "hyper", "ureq"] {
            assert!(!src.contains(bad), "{} usa {}", p.display(), bad);
        }
    }
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let deps = manifest.split("[dependencies]").nth(1).unwrap();
    for line in deps.lines().filter(|l| l.contains('=')) {
        let name = line.split('=').next().unwrap().trim();
        assert!(["serde", "serde_json"].contains(&name), "dependência não autorizada no motor: {}", name);
    }
}

#[test]
fn copia_de_volume_nao_libera_o_disco_original() {
    let mut s = Sim::new(generate(ModuleId::M503, 2, 1));
    s.exec("rede isolar item-01");
    assert!(s.exec("volume copiar item-01").ok);
    assert!(!s.exec("fs montar m-01").ok, "o disco original ainda exige aquisição");
    assert!(s.case.media.iter().any(|m| m.is_work_media && m.clone_of.as_deref() == Some("M-01")));
    // arquivo do contêiner original continua inacessível pelo disco; a cópia é lida pela mídia de trabalho
    let p = s.case.media.iter().find(|m| m.is_work_media).unwrap().partitions[0].files[0].path.clone();
    assert!(s.exec(&format!("fs cat {}", p)).ok);
}

#[test]
fn preenchimento_automatico_nao_cria_incoerencias() {
    for m in [ModuleId::M501, ModuleId::M502, ModuleId::M503, ModuleId::M504] {
        for d in 1..=4u8 {
            let mut s = Sim::new(generate(m, d, 3));
            s.exec("laudo auto");
            s.exec("laudo conclusao Conclusão de teste com base nos exames registrados.");
            let c = report::check_coherence(&s);
            assert!(c.problems.is_empty(), "{:?} D{}: {:?}", m, d, c.problems);
        }
    }
}

#[test]
fn bateria_esgota_em_afu_sem_alimentacao_e_perde_estado() {
    // aparelho em AFU, isolado mas sem bateria externa: a busca por sinal drena mais rápido
    let mut found = false;
    for seed in 0..40 {
        let mut s = Sim::new(generate(ModuleId::M502, 3, seed));
        if s.case.items[0].phone.as_ref().unwrap().boot != Boot::Afu { continue; }
        found = true;
        s.exec("isolar item-01 faraday");
        for _ in 0..40 { s.exec("fotografar item-01"); }
        let ph = s.case.items[0].phone.as_ref().unwrap();
        assert_eq!(ph.boot, Boot::Desligado, "bateria deveria ter esgotado (restou {:.1}%)", ph.battery);
        assert!(s.faults.iter().any(|f| f.contains("AFU")));
        break;
    }
    assert!(found, "nenhuma semente gerou AFU no D3");
}

#[test]
fn bateria_externa_preserva_afu() {
    for seed in 0..40 {
        let mut s = Sim::new(generate(ModuleId::M502, 3, seed));
        if s.case.items[0].phone.as_ref().unwrap().boot != Boot::Afu { continue; }
        s.exec("isolar item-01 faraday");
        s.exec("carregar item-01");
        for _ in 0..40 { s.exec("fotografar item-01"); }
        assert_eq!(s.case.items[0].phone.as_ref().unwrap().boot, Boot::Afu);
        return;
    }
}

#[test]
fn imei_fisico_x_logico_detecta_divergencia() {
    let mut seen = false;
    for seed in 0..30 {
        let mut s = Sim::new(generate(ModuleId::M502, 4, seed));
        s.exec("isolar item-01 faraday");
        let out = s.exec("imei verificar item-01").text;
        let ph = s.case.items[0].phone.as_ref().unwrap();
        let diverge = ph.imeis[0] != ph.imei_label[0];
        assert_eq!(out.contains("DIVERGE"), diverge);
        seen |= diverge;
    }
    assert!(seen, "D4 deve sortear etiquetas divergentes em parte dos casos");
}

#[test]
fn ligar_celular_fora_do_isolamento_e_falha_e_desligar_em_afu_descarta_estado() {
    for seed in 0..40 {
        let mut s = Sim::new(generate(ModuleId::M502, 3, seed));
        if s.case.items[0].phone.as_ref().unwrap().boot != Boot::Afu { continue; }
        s.exec("isolar item-01 faraday");
        s.exec("carregar item-01");
        assert!(s.exec("energia desligar item-01").ok);
        assert!(s.faults.iter().any(|f| f.contains("AFU")));
        assert_eq!(s.case.items[0].phone.as_ref().unwrap().boot, Boot::Desligado);
        assert!(s.exec("energia ligar item-01").ok);
        assert_eq!(s.case.items[0].phone.as_ref().unwrap().boot, Boot::Bfu);
        assert!(!s.faults.iter().any(|f| f.contains("fora do isolamento")), "estava isolado");
        return;
    }
}

#[test]
fn comando_remoto_fica_pendente_e_chega_quando_o_aparelho_reconecta() {
    // desligado antes do comando: nada acontece enquanto estiver sem conectividade
    let mut s = Sim::new(generate(ModuleId::M502, 2, 5));
    assert!(s.case.items[0].phone.as_ref().unwrap().remote_wipe_at.is_some());
    s.exec("energia desligar item-01");
    for _ in 0..40 {
        s.exec("fotografar item-01");
    }
    assert!(!s.case.items[0].phone.as_ref().unwrap().wiped);
    assert!(s.case.items[0].phone.as_ref().unwrap().remote_wipe_at.is_some(), "o comando continua pendente");
    // religado fora do isolamento: o comando pendente é entregue
    s.exec("energia ligar item-01");
    s.exec("fotografar item-01");
    assert!(s.case.items[0].phone.as_ref().unwrap().wiped);

    // isolado durante todo o exame: o comando nunca chega
    let mut s = Sim::new(generate(ModuleId::M502, 2, 5));
    s.exec("isolar item-01 faraday");
    s.exec("carregar item-01");
    for _ in 0..40 {
        s.exec("fotografar item-01");
    }
    assert!(!s.case.items[0].phone.as_ref().unwrap().wiped);
}

#[test]
fn sanitizar_midia_questionada_e_recusado_e_registrado() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    // sem bloqueador: recusa + falha de procedimento
    let o = s.exec("sanitizar m-01");
    assert!(!o.ok);
    assert!(s.faults.iter().any(|f| f.contains("sanitizar a mídia questionada")));
    assert!(s.audit.entries.iter().any(|e| e.action == "sanitizar" && e.outcome.starts_with("RECUSADO")));
    // destino próprio: permitido; clone exige destino explícito e não aceita mídia do caso
    assert!(!s.exec("sanitizar").ok);
    assert!(s.exec("sanitizar dest-01").ok);
    assert!(!s.exec("clone m-01").ok);
    assert!(!s.exec("clone m-01 m-01").ok);
    s.exec("bloqueio m-01");
    let o = s.exec("clone m-01 dest-01");
    assert!(o.ok, "{}", o.text);
    assert!(!s.faults.iter().any(|f| f.contains("Espelhamento da mídia M-01 sem bloqueador")), "{:?}", s.faults);
}

#[test]
fn espelhamento_sem_bloqueador_e_registrado() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    s.exec("sanitizar dest-01");
    assert!(s.exec("clone m-01 dest-01").ok);
    assert!(s.faults.iter().any(|f| f.contains("Espelhamento da mídia M-01 sem bloqueador")));
}

#[test]
fn erros_indicam_o_que_faltou_e_sugerem_comando() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    let o = s.exec("energia ligar");
    assert!(o.text.contains("energia ligar <item>") && o.text.contains("ITEM-01"), "{}", o.text);
    let o = s.exec("imagen m-01");
    assert!(o.text.contains("'imagem'"), "{}", o.text);
    // mídia oculta não é revelada pela mensagem de erro
    let mut s = Sim::new(generate(ModuleId::M503, 4, 3));
    let hidden: Vec<String> = s.case.items.iter().flat_map(|i| i.slots.iter().filter(|sl| sl.hidden_by.is_some()).filter_map(|sl| sl.media_id.clone())).collect();
    let o = s.exec("imagem m-99");
    for h in hidden {
        assert!(!o.text.contains(&h), "revelou {}: {}", h, o.text);
    }
}

#[test]
fn relatorio_separa_fatos_hipoteses_limitacoes_e_nao_realizados() {
    let mut s = Sim::new(generate(ModuleId::M501, 1, 7));
    run(&mut s, &["fotografar item-01", "bloqueio m-01", "imagem m-01", "fs montar m-01"]);
    let kw = s.case.keyword.clone();
    s.exec(&format!("fs buscar {}", kw.split_whitespace().next().unwrap()));
    let path = s.case.keyword_sources.iter().find_map(|k| match k { SourceRef::File { path, .. } => Some(path.clone()), _ => None }).unwrap();
    s.exec(&format!("fs cat {}", path));
    s.exec("laudo auto");
    s.exec("laudo hipoteses O documento pode ter sido copiado de outra mídia.");
    s.exec("laudo conclusao Provavelmente o investigado redigiu o documento.");
    let txt = report::preview(&s);
    assert!(txt.contains("FATOS OBSERVADOS") && txt.contains("[fonte:"), "{}", txt);
    assert!(txt.contains("HIPÓTESES NÃO CONFIRMADAS"));
    assert!(txt.contains("PROCEDIMENTOS PREVISTOS NÃO REALIZADOS") && txt.contains("Verificar o hash"));
    let md = report::markdown(&s);
    assert!(md.contains("Procedimentos previstos não realizados") && md.contains("Hipóteses não confirmadas"));
    let coh = report::check_coherence(&s);
    assert!(coh.problems.iter().any(|p| p.contains("linguagem de hipótese")), "{:?}", coh.problems);
    // afirmar achado sem nenhum fato registrado é apontado
    let mut v = Sim::new(generate(ModuleId::M501, 1, 7));
    v.exec("laudo conclusao Foi localizado o documento de interesse.");
    assert!(report::check_coherence(&v).problems.iter().any(|p| p.contains("afirma achados")));
}

#[test]
fn termo_so_registra_senha_efetivamente_informada() {
    for seed in 1..20 {
        let mut s = Sim::new(generate(ModuleId::M505, 2, seed));
        let pc = s.case.items[0].phone.as_ref().unwrap().passcode.clone();
        assert!(!report::preview(&s).contains(&pc), "seed {}: senha não informada apareceu no termo", seed);
        s.exec("solicitar-senhas item-01");
        assert!(report::preview(&s).contains(&pc));
    }
}
