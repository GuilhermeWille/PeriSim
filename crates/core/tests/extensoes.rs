//! Extensões do gerador 1.1.0: compatibilidade com 1.0.0, cifragem por hardware com
//! ordem de inicialização, arquivos danificados e cadeia ARC.

use pericia_core::casegen::{generate_classic as generate, generate_v, GEN_VERSION, SUPPORTED_GEN_VERSIONS};
use pericia_core::engine::Sim;
use pericia_core::model::*;

fn json(c: &CaseFile) -> String {
    serde_json::to_string(c).unwrap()
}

#[test]
fn versao_1_0_0_continua_reproduzivel_e_base_e_preservada() {
    assert!(SUPPORTED_GEN_VERSIONS.contains(&"1.0.0") && SUPPORTED_GEN_VERSIONS.contains(&GEN_VERSION));
    for m in ModuleId::ALL {
        for d in 1..=4 {
            for seed in [1u32, 7, 42] {
                let a = generate_v(m, d, seed, "1.0.0");
                assert_eq!(json(&a), json(&generate_v(m, d, seed, "1.0.0")));
                let b = generate(m, d, seed);
                // o conteúdo-base é o mesmo: pessoas, expressão e itens
                assert_eq!(a.keyword, b.keyword);
                assert_eq!(a.people.len(), b.people.len());
                assert_eq!(a.items.len(), b.items.len());
            }
        }
    }
    // caso conhecido da documentação mantém a expressão
    assert_eq!(generate(ModuleId::M501, 1, 7).keyword, "OPERAÇÃO CORVO-71");
}

fn caso_fde() -> Sim {
    for seed in 1..200 {
        let c = generate(ModuleId::M501, 3, seed);
        if c.media.iter().any(|m| m.fde.is_some()) {
            return Sim::new(c);
        }
    }
    panic!("nenhum caso com cifragem por hardware");
}

fn senha_disco(s: &Sim) -> String {
    s.case.media.iter().find_map(|m| m.fde.as_ref().map(|e| e.password.clone())).unwrap()
}

#[test]
fn disco_cifrado_por_hardware_exige_clone_bios_e_senha() {
    let mut s = caso_fde();
    let pw = senha_disco(&s);
    s.exec("fotografar item-01");
    s.exec("lacre conferir item-01");
    s.exec("deslacrar item-01");
    s.exec("bloqueio m-01");
    assert!(!s.exec("imagem m-01").ok, "imagem isolada seria ilegível");
    assert!(!s.exec(&format!("decifrar m-01 {}", pw)).ok, "não se desbloqueia o original");
    s.exec("sanitizar dest-01");
    assert!(s.exec("clone m-01 dest-01").ok);
    assert!(!s.exec("fs montar m-01").ok, "clone ainda cifrado");
    assert!(!s.exec("decifrar m-01 errada").ok);
    assert!(s.exec("bios item-01").ok);
    assert!(s.exec("bios ordem item-01 forense").ok);
    let o = s.exec(&format!("decifrar m-01 {}", pw));
    assert!(o.ok, "{}", o.text);
    assert!(s.exec("fs montar m-01").ok);
    assert!(s.faults.is_empty(), "{:?}", s.faults);
    // a expressão continua alcançável
    let kw = s.case.keyword.to_lowercase();
    let out = s.exec(&format!("fs buscar {}", kw.split_whitespace().next().unwrap())).text;
    assert!(!out.is_empty());
    let cl = pericia_core::assess::checklist(&s);
    assert!(cl.iter().any(|(ok, l)| *ok && l.contains("Cifragem por hardware")));
}

#[test]
fn desbloquear_sem_conferir_a_bios_e_falta() {
    let mut s = caso_fde();
    let pw = senha_disco(&s);
    s.exec("bloqueio m-01");
    s.exec("sanitizar dest-01");
    s.exec("clone m-01 dest-01");
    assert!(s.exec(&format!("decifrar m-01 {}", pw)).ok);
    assert!(s.faults.iter().any(|f| f.contains("ordem de inicialização")));
}

#[test]
fn arquivos_danificados_ensinam_limites() {
    let c = generate(ModuleId::M501, 2, 7);
    let files: Vec<&VFile> = c.media[0].partitions.iter().flat_map(|p| p.files.iter()).collect();
    assert!(files.iter().any(|f| f.deleted && f.recover == Some(Recover::Nenhuma)));
    assert!(files.iter().any(|f| f.bad_sector));
    assert!(files.iter().any(|f| f.content.starts_with("%PDF") && f.path.ends_with(".jpg")));
    // nenhuma extensão contém a expressão (não desloca a resposta do caso)
    assert!(files.iter().filter(|f| f.id.contains("-X")).all(|f| !f.keyword && !f.content.contains(&c.keyword)));
    // a recuperação de sobrescrito é inviável e isso aparece nas limitações
    let mut s = Sim::new(c.clone());
    s.exec("bloqueio m-01");
    s.exec("imagem m-01");
    let gone = files.iter().find(|f| f.deleted && f.recover == Some(Recover::Nenhuma)).unwrap().path.clone();
    let o = s.exec(&format!("recuperar m-01 {}", gone));
    assert!(!o.ok || o.text.to_lowercase().contains("inviável"), "{}", o.text);
}

#[test]
fn assinatura_diferente_da_extensao_aparece_no_file_do_linux() {
    use pericia_core::os::{shell::run_line, Os, Shell};
    let mut sim = Sim::new(generate(ModuleId::M501, 2, 7));
    sim.exec("bloqueio m-01");
    sim.exec("imagem m-01");
    let mut os = Os::new(&sim);
    let mut sh = Shell::new(&os.home);
    os.sync(&sim);
    run_line(&mut os, &mut sim, &mut sh, "sudo mkdir -p /mnt/m01 && sudo mount -o ro,loop ~/casos/C501-D2-00000007/M-01.dd /mnt/m01");
    let out = run_line(&mut os, &mut sim, &mut sh, "file /mnt/m01/Users/*/Pictures/ferias_praia.jpg");
    assert!(out.contains("PDF document"), "{}", out);
}

#[test]
fn mensagem_reencaminhada_tem_arc_e_dkim_quebrado() {
    let mut achou = false;
    for seed in 1..30 {
        let mut s = Sim::new(generate(ModuleId::M504, 3, seed));
        let name = s.case.provided[0].name().to_string();
        if !s.case.provided[0].content.contains("ARC-Seal") {
            continue;
        }
        achou = true;
        assert!(s.case.provided[0].content.contains("dkim=fail"));
        let o = s.exec(&format!("email cabecalho {}", name));
        assert!(o.text.contains("Cadeia ARC") && o.text.contains("intermediário"), "{}", o.text);
        assert!(s.keyword_found, "a expressão continua no assunto/corpo");
    }
    assert!(achou);
}
