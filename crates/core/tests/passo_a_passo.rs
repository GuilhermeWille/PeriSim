//! Modo passo a passo (gerador 1.2.0): ferramenta certa na mão, luvas, etapas físicas,
//! vestígios não digitais e compatibilidade com as regras clássicas.

use pericia_core::casegen::{generate, generate_classic};
use pericia_core::engine::Sim;
use pericia_core::equip::Gloves;
use pericia_core::model::*;

fn run(s: &mut Sim, cmds: &[&str]) {
    for c in cmds {
        let o = s.exec(c);
        assert!(o.ok, "comando '{}' falhou: {}", c, o.text);
    }
}

const FLUXO_501: &[&str] = &[
    "luvas calcar", "pulseira colocar",
    "pegar camera", "fotografar item-01",
    "pegar oficio", "lacre conferir item-01",
    "pegar tesoura", "deslacrar item-01",
    "pegar chave", "desparafusar item-01", "desparafusar item-01", "abrir item-01",
    "cabo desconectar item-01 sata0 dados", "cabo desconectar item-01 sata0 energia",
    "desparafusar item-01 sata0", "desparafusar item-01 sata0", "desparafusar item-01 sata0", "desparafusar item-01 sata0",
    "midia remover item-01 sata0",
    "pegar cabo-sata", "conectar m-01 bloqueador",
    "pegar cabo-usb", "conectar bloqueador estacao",
    "bloqueio m-01", "imagem m-01", "verificar-hash m-01",
    "pegar saco", "embalar item-01",
    "pegar fita-lacre", "lacre aplicar item-01",
    "pegar etiquetas", "etiquetar item-01 Gabinete examinado",
];

#[test]
fn fluxo_completo_501_sem_faltas() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    assert!(s.strict);
    run(&mut s, FLUXO_501);
    assert!(s.faults.is_empty(), "{:?}", s.faults);
    let cl = pericia_core::assess::checklist(&s);
    for etapa in ["Calçar luvas", "pulseira antiestática", "bloqueador", "hash"] {
        assert!(cl.iter().any(|(ok, l)| *ok && l.contains(etapa)), "etapa '{}' não cumprida: {:?}", etapa, cl);
    }
    assert_eq!(s.equip.hand, Some(pericia_core::equip::Tool::Etiquetas));
}

#[test]
fn sem_ferramenta_nada_acontece() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    let antes = s.audit.entries.len();
    for c in ["fotografar item-01", "deslacrar item-01", "inspecionar local", "lacre aplicar item-01", "embalar item-01"] {
        let o = s.exec(c);
        assert!(!o.ok && o.text.contains("segure"), "{}: {}", c, o.text);
    }
    assert_eq!(s.audit.entries.len(), antes, "ação impossível não entra na auditoria");
    // ferramenta errada também não serve
    run(&mut s, &["pegar chave"]);
    assert!(!s.exec("fotografar item-01").ok);
}

#[test]
fn etapas_na_ordem_fisica() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    run(&mut s, &["luvas calcar", "pulseira colocar", "pegar chave"]);
    let o = s.exec("abrir item-01");
    assert!(!o.ok && o.text.contains("parafuso"));
    run(&mut s, &["desparafusar item-01", "desparafusar item-01", "abrir item-01"]);
    let o = s.exec("midia remover item-01 sata0");
    assert!(!o.ok && o.text.contains("cabos"));
    run(&mut s, &["cabo desconectar item-01 sata0 dados", "cabo desconectar item-01 sata0 energia"]);
    let o = s.exec("midia remover item-01 sata0");
    assert!(!o.ok && o.text.contains("parafuso"));
    // aquisição sem a cadeia montada é impossível
    let o = s.exec("imagem m-01");
    assert!(!o.ok, "{}", o.text);
}

#[test]
fn ligar_direto_na_estacao_e_possivel_mas_e_falta() {
    let mut s = Sim::new(generate(ModuleId::M501, 2, 7));
    run(&mut s, &FLUXO_501[..19]);
    run(&mut s, &["pegar cabo-sata", "conectar m-01 estacao", "imagem m-01"]);
    assert!(s.faults.iter().any(|f| f.contains("sem bloqueador")), "{:?}", s.faults);
}

#[test]
fn luvas_e_contaminacao_cruzada() {
    let mut c = generate(ModuleId::M503, 2, 3);
    c.items.iter_mut().find(|i| i.id == "ITEM-01").unwrap().physical.biohazard = true;
    let mut s = Sim::new(c);
    // sem luvas: a ação acontece, mas é falta
    run(&mut s, &["pegar lanterna", "inspecionar item-03"]);
    assert!(s.faults.iter().any(|f| f.contains("sem luvas")));
    run(&mut s, &["luvas calcar", "inspecionar item-01"]);
    assert!(matches!(s.equip.gloves, Gloves::Contaminadas(ref x) if x == "ITEM-01"));
    run(&mut s, &["inspecionar item-03"]);
    assert!(s.faults.iter().any(|f| f.contains("contaminação cruzada")), "{:?}", s.faults);
    // trocar as luvas resolve
    let n = s.faults.len();
    run(&mut s, &["luvas trocar", "inspecionar item-03"]);
    assert_eq!(s.faults.len(), n);
}

#[test]
fn vestigios_sinalizar_fotografar_comunicar() {
    let mut s = Sim::new(generate(ModuleId::M503, 2, 3));
    let ids: Vec<String> = s.case.vestigios.iter().map(|v| v.id.to_lowercase()).collect();
    assert!(!ids.is_empty(), "o local tem vestígios na versão 1.2.0");
    let lbl = "Vestígios não digitais";
    assert!(pericia_core::assess::checklist(&s).iter().any(|(ok, l)| !ok && l.contains(lbl)));
    assert!(!s.exec(&format!("marcar {}", ids[0])).ok, "sem as placas na mão");
    run(&mut s, &["pegar placas"]);
    for v in &ids {
        run(&mut s, &[&format!("marcar {}", v)]);
    }
    run(&mut s, &["pegar camera"]);
    for v in &ids {
        let o = s.exec(&format!("fotografar {}", v));
        assert!(o.ok && o.text.contains("com placa"), "{}", o.text);
    }
    for v in &ids {
        run(&mut s, &[&format!("comunicar {}", v)]);
    }
    assert!(pericia_core::assess::checklist(&s).iter().any(|(ok, l)| *ok && l.contains(lbl)));
}

#[test]
fn vestigios_so_na_versao_nova() {
    for m in ModuleId::ALL {
        for seed in 1..15 {
            assert!(generate_classic(m, 2, seed).vestigios.is_empty());
        }
    }
    // o conteúdo-base não muda entre 1.1.0 e 1.2.0 (mesmas mídias e expressão)
    let a = generate_classic(ModuleId::M501, 2, 7);
    let b = generate(ModuleId::M501, 2, 7);
    assert_eq!(a.keyword, b.keyword);
    assert_eq!(a.media.len(), b.media.len());
}

#[test]
fn celular_cabo_certo_e_isolamento() {
    let mut s = Sim::new(generate(ModuleId::M502, 2, 3));
    let it = s.case.items.iter().find(|i| i.phone.is_some()).unwrap().clone();
    let id = it.id.to_lowercase();
    let certo = pericia_core::equip::phone_cable(it.phone.as_ref().unwrap());
    let errado = if certo == pericia_core::equip::Tool::CaboUsbC { "cabo-lightning" } else { "cabo-usbc" };
    run(&mut s, &["luvas calcar", "pegar faraday", &format!("isolar {} faraday", id)]);
    assert_eq!(s.equip.hand, None, "a Faraday bag fica com o aparelho");
    run(&mut s, &["pegar bateria", &format!("carregar {}", id)]);
    run(&mut s, &[&format!("pegar {}", errado)]);
    let o = s.exec(&format!("extrair {}", id));
    assert!(!o.ok && o.text.contains("segure"), "conector incompatível: {}", o.text);
    run(&mut s, &[&format!("pegar {}", pericia_core::equip::tool_id(certo)), &format!("extrair {}", id)]);
}

#[test]
fn regras_classicas_para_casos_antigos() {
    let mut s = Sim::new(generate_classic(ModuleId::M501, 2, 7));
    assert!(!s.strict);
    run(&mut s, &["fotografar item-01", "lacre conferir item-01", "deslacrar item-01", "abrir item-01", "midia remover item-01 sata0", "bloqueio m-01", "imagem m-01"]);
}

#[test]
fn replay_passo_a_passo_e_deterministico() {
    let mut a = Sim::new(generate(ModuleId::M501, 2, 7));
    let mut b = Sim::new(generate(ModuleId::M501, 2, 7));
    for c in FLUXO_501 {
        a.exec(c);
        b.exec(c);
    }
    assert_eq!(a.audit.last_hash(), b.audit.last_hash());
    assert!(a.audit.verify().is_ok());
}
